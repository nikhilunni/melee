"""Recorded traces on disk, plain or zstd-compressed.

Recorders write `<name>.jsonl`; `compress_verified` then replaces a large one
with `<name>.jsonl.zst`. Readers always name the plain path and go through
`open_text` / `read_text` / `exists`, which prefer the plain file and fall back
to the compressed sibling, yielding identical bytes either way. The Rust side
mirrors this in crates/melee-trace-io.

Only `.jsonl` files are ever compressed. `.done` markers, `.json` sidecars,
logs and anything smaller than `MIN_SIZE` stay plain.
"""
from __future__ import annotations

import io
import os
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO, Iterable, TextIO

SUFFIX = ".zst"
# zstd level for recorded traces: ~10x on particle dumps at a few hundred MB/s
# multithreaded; higher levels cost far more time for a few percent.
DEFAULT_LEVEL = 12
# Files below this stay plain: the saving is negligible and they stay greppable.
MIN_SIZE = 1 << 20
CHUNK = 1 << 20


def compressed_path(path: Path | str) -> Path:
    """`<path>.zst`, the compressed sibling of a plain trace path."""
    return Path(str(path) + SUFFIX)


def resolve(path: Path | str) -> Path | None:
    """The file holding the trace named by `path`: the plain file when present,
    else its `.zst` sibling, else None. A path ending in `.zst` names itself."""
    path = Path(path)
    if path.is_file():
        return path
    compressed = compressed_path(path)
    return compressed if compressed.is_file() else None


def exists(path: Path | str) -> bool:
    return resolve(path) is not None


def open_binary(path: Path | str) -> BinaryIO:
    """The trace's plain bytes as a binary stream (plain or decompressed)."""
    found = resolve(path)
    if found is None:
        raise FileNotFoundError(f"trace not found: {path} (nor {compressed_path(path)})")
    if found.suffix != SUFFIX:
        return found.open("rb")
    return _decompressing(found)


def _decompressing(path: Path) -> BinaryIO:
    """Decompress `path` whatever its name (the verifier reads `.zst.tmp`)."""
    import zstandard  # lazily: plain reads work where zstandard is absent

    # closefd=True: closing the reader closes the underlying file.
    return zstandard.ZstdDecompressor().stream_reader(path.open("rb"), read_across_frames=True, closefd=True)


def open_text(path: Path | str, encoding: str = "utf-8") -> TextIO:
    """The trace as text lines, like `Path.open()` on the plain file."""
    return io.TextIOWrapper(io.BufferedReader(open_binary(path)), encoding=encoding)


def read_bytes(path: Path | str) -> bytes:
    with open_binary(path) as stream:
        return stream.read()


def read_text(path: Path | str, encoding: str = "utf-8") -> str:
    return read_bytes(path).decode(encoding)


def compressible(path: Path | str, min_size: int = MIN_SIZE) -> bool:
    """A plain `.jsonl` trace large enough to be worth compressing."""
    path = Path(path)
    return (path.name.endswith(".jsonl") and path.is_file() and not path.is_symlink()
            and path.stat().st_size >= min_size)


@dataclass(frozen=True)
class Compressed:
    plain: Path
    compressed: Path
    plain_bytes: int
    compressed_bytes: int


def _same_bytes(plain: Path, compressed: Path) -> bool:
    with plain.open("rb") as expected, _decompressing(compressed) as actual:
        while True:
            want = expected.read(CHUNK)
            got = actual.read(len(want)) if want else actual.read(1)
            # stream_reader.read(n) may return short reads; top up to len(want).
            while want and len(got) < len(want):
                more = actual.read(len(want) - len(got))
                if not more:
                    break
                got += more
            if want != got:
                return False
            if not want:
                return True


def compress_verified(path: Path | str, level: int = DEFAULT_LEVEL, threads: int = -1) -> Compressed:
    """Replace plain `path` with `<path>.zst`.

    Streams into `<path>.zst.tmp`, decompresses that and compares it byte for
    byte with `path`, then renames it into place (replacing any stale `.zst`)
    and only then removes `path`. On any failure the plain file is untouched
    and the temporary file is removed. `threads=-1` uses every core.
    """
    plain = Path(path)
    if not plain.is_file() or plain.is_symlink():
        raise FileNotFoundError(f"not a plain trace file: {plain}")
    final = compressed_path(plain)
    tmp = Path(str(final) + ".tmp")
    stat = plain.stat()
    import zstandard

    try:
        compressor = zstandard.ZstdCompressor(level=level, threads=threads, write_checksum=True)
        with plain.open("rb") as source, tmp.open("wb") as sink:
            compressor.copy_stream(source, sink, size=stat.st_size, read_size=CHUNK, write_size=CHUNK)
            sink.flush()
            os.fsync(sink.fileno())
        if plain.stat().st_size != stat.st_size or plain.stat().st_mtime_ns != stat.st_mtime_ns:
            raise RuntimeError(f"{plain} changed while compressing")
        if not _same_bytes(plain, tmp):
            raise RuntimeError(f"{tmp} does not decompress to {plain}")
        os.utime(tmp, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        os.replace(tmp, final)
    except BaseException:
        tmp.unlink(missing_ok=True)
        raise
    compressed_bytes = final.stat().st_size
    plain.unlink()
    return Compressed(plain, final, stat.st_size, compressed_bytes)


def compress_outputs(paths: Iterable[Path | str], level: int = DEFAULT_LEVEL, threads: int = -1,
                     min_size: int = MIN_SIZE, log=print) -> list[Compressed]:
    """Compress each existing, large enough plain `.jsonl` among `paths`."""
    done = []
    seen = set()
    for path in paths:
        path = Path(path)
        if path in seen or not compressible(path, min_size):
            continue
        seen.add(path)
        result = compress_verified(path, level, threads)
        if log:
            log(f"   compressed {path.name}: {human(result.plain_bytes)} -> {human(result.compressed_bytes)}")
        done.append(result)
    return done


def human(n: float) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return f"{n:.1f} {unit}"
        n /= 1024
    return f"{n:.1f} TB"
