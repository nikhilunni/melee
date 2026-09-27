"""One-shot migration: compress existing plain JSONL traces to verified `.zst`.

    cd harness && uv run python compress_traces.py                 # dry run: list candidates
    cd harness && uv run python compress_traces.py --estimate      # dry run plus a sampled ratio
    cd harness && uv run python compress_traces.py --apply         # compress, verify, remove plain

Walks harness/traces (recursively; --dir overrides) for plain `*.jsonl` files of
at least --min-size bytes and not modified in the last --min-age minutes (a
recording may still be writing them). Each one is compressed with
trace_io.compress_verified: written to `<file>.zst.tmp`, decompressed and
compared byte for byte with the plain file, renamed to `<file>.zst`, and only
then is the plain file removed. `.done` markers, `.json` sidecars, logs and
small files are left alone. Every reader (Rust melee-trace-io, Python
trace_io) accepts either form, so the migration can stop and resume at any point.
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import zstandard

import trace_io

HERE = Path(__file__).resolve().parent
SAMPLE_BYTES = 4 << 20
SAMPLE_FILES = 32


def candidates(root: Path, min_size: int, min_age_s: float) -> list[Path]:
    now = time.time()
    found = []
    for path in sorted(root.rglob("*.jsonl")):
        if not trace_io.compressible(path, min_size):
            continue
        if now - path.stat().st_mtime < min_age_s:
            print(f"   skip (modified recently, may still be recording): {path.name}")
            continue
        found.append(path)
    return found


def estimate_ratio(paths: list[Path], level: int) -> float:
    """Compressed/plain over the first SAMPLE_BYTES of the largest files."""
    largest = sorted(paths, key=lambda p: p.stat().st_size, reverse=True)[:SAMPLE_FILES]
    compressor = zstandard.ZstdCompressor(level=level, threads=-1)
    plain = packed = 0
    for path in largest:
        with path.open("rb") as f:
            chunk = f.read(SAMPLE_BYTES)
        plain += len(chunk)
        packed += len(compressor.compress(chunk))
    return packed / plain if plain else 1.0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dir", type=Path, default=HERE / "traces", help="trace directory (default harness/traces)")
    ap.add_argument("--apply", action="store_true", help="compress; without it only list what would change")
    ap.add_argument("--estimate", action="store_true", help="dry run: estimate the ratio from sampled prefixes")
    ap.add_argument("--level", type=int, default=trace_io.DEFAULT_LEVEL)
    ap.add_argument("--threads", type=int, default=-1, help="zstd worker threads (-1: every core)")
    ap.add_argument("--min-size", type=int, default=trace_io.MIN_SIZE, help="bytes; smaller files stay plain")
    ap.add_argument("--min-age", type=float, default=10.0, help="minutes since last modification")
    a = ap.parse_args(argv)

    root = a.dir.resolve()
    if not root.is_dir():
        print(f"no trace directory at {root}", file=sys.stderr)
        return 1
    paths = candidates(root, a.min_size, a.min_age * 60)
    before = sum(p.stat().st_size for p in paths)
    print(f"{len(paths)} plain traces >= {trace_io.human(a.min_size)} under {root}: {trace_io.human(before)}")
    if not a.apply:
        for path in paths:
            print(f"   {trace_io.human(path.stat().st_size):>10}  {path.relative_to(root)}")
        if a.estimate and paths:
            ratio = estimate_ratio(paths, a.level)
            print(f"estimated after: ~{trace_io.human(before * ratio)} (ratio {1 / ratio:.1f}x, sampled)")
        print("dry run; pass --apply to compress")
        return 0

    after = 0
    failures = []
    t0 = time.monotonic()
    for index, path in enumerate(paths, 1):
        try:
            result = trace_io.compress_verified(path, a.level, a.threads)
        except Exception as error:  # keep going; the plain file is untouched on failure
            failures.append((path, error))
            print(f"[{index}/{len(paths)}] FAILED {path.name}: {error}", file=sys.stderr)
            continue
        after += result.compressed_bytes
        print(f"[{index}/{len(paths)}] {path.relative_to(root)}: "
              f"{trace_io.human(result.plain_bytes)} -> {trace_io.human(result.compressed_bytes)}")
    done = before - sum(p.stat().st_size for p, _ in failures if p.is_file())
    ratio = done / after if after else 0.0
    print(f"compressed {len(paths) - len(failures)} files: {trace_io.human(done)} -> {trace_io.human(after)} "
          f"({ratio:.1f}x) in {time.monotonic() - t0:.0f}s")
    if failures:
        print(f"{len(failures)} failures; their plain files are unchanged", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
