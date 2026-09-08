"""Extract the filesystem of a GameCube disc image.

A GameCube disc is a flat 1.46 GB image with four system regions at fixed
or header-named offsets, followed by the game's files wherever the FST
(file string table) says they are. Every multi-byte field is big-endian.

    disc offset          contents
    -------------------  ---------------------------------------------------
    0x0000               boot.bin, 0x440 bytes: disc header
      0x0000  char[6]      game id ("GALE01")
      0x0020  char[0x3E0]  title, NUL padded
      0x0420  u32          offset of main.dol
      0x0424  u32          offset of the FST
      0x0428  u32          size of the FST in bytes
      0x042C  u32          maximum FST size (multi-disc games)
    0x0440               bi2.bin, 0x2000 bytes: boot info (region, debug flags)
    0x2440               apploader.img: 0x20-byte header, then code + trailer
      0x2450  u32          apploader code size
      0x2454  u32          apploader trailer size
    <dol offset>         main.dol; its length comes from its own section table
    <fst offset>         fst.bin: 12-byte entries, then a NUL-separated
                         string table

    FST entry (12 bytes)     file                    directory
      byte 0   flags           0                       1
      bytes 1-3                name offset into the string table
      u32 at 4                 disc offset of data     index of parent dir
      u32 at 8                 size in bytes           index of next sibling
                                                       (one past the last
                                                       descendant)

Entry 0 is the root directory; its "next sibling" field is therefore the
total entry count, and the string table begins right after the last entry.
A directory's descendants are the entries between it and its next-sibling
index, so one linear pass with a stack of open directories recovers the
whole tree.

The image is read in chunks and never held in memory at once.

Usage (from harness/):
    uv run python extract_fst.py roms/GALE01.iso            # extract next to the ISO
    uv run python extract_fst.py roms/GALE01.iso --out roms  # explicit destination
    uv run python extract_fst.py roms/GALE01.iso --list      # print the tree only

Output layout, matching what Dolphin's "Extract Entire Disc" produces:
    <out>/sys/{boot.bin,bi2.bin,apploader.img,main.dol,fst.bin}
    <out>/files/<tree from the FST>
"""
from __future__ import annotations

import argparse
import hashlib
import struct
import sys
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import BinaryIO, Iterable

BOOT_BIN_SIZE = 0x440
BI2_BIN_OFFSET = 0x440
BI2_BIN_SIZE = 0x2000
APPLOADER_OFFSET = 0x2440
APPLOADER_HEADER_SIZE = 0x20

GAME_ID_OFFSET = 0x00
GAME_ID_SIZE = 6
TITLE_OFFSET = 0x20
TITLE_SIZE = 0x3E0
DOL_OFFSET_FIELD = 0x420
FST_OFFSET_FIELD = 0x424
FST_SIZE_FIELD = 0x428
FST_MAX_SIZE_FIELD = 0x42C

FST_ENTRY_SIZE = 12
FST_FLAG_DIRECTORY = 1

# main.dol section table: 7 text and 11 data sections. Offsets first, then
# load addresses, then sizes; the file ends where the furthest section ends.
DOL_TEXT_SECTIONS = 7
DOL_DATA_SECTIONS = 11
DOL_SECTION_OFFSETS_FIELD = 0x00
DOL_SECTION_SIZES_FIELD = 0x90
DOL_HEADER_SIZE = 0x100

COPY_CHUNK = 1 << 20


class DiscFormatError(ValueError):
    """The image does not look like a GameCube disc, or its FST is malformed."""


@dataclass(frozen=True)
class DiscHeader:
    """The fields of boot.bin the extractor needs."""

    game_id: str
    title: str
    dol_offset: int
    fst_offset: int
    fst_size: int
    fst_max_size: int


@dataclass(frozen=True)
class DiscFile:
    """One regular file named by the FST."""

    path: PurePosixPath
    """Path relative to the `files/` root, e.g. `audio/us/1padv.ssm`."""
    offset: int
    """Absolute disc offset of the first byte."""
    size: int


@dataclass(frozen=True)
class Fst:
    """The parsed file table: every directory and file, in FST order."""

    dirs: tuple[PurePosixPath, ...]
    files: tuple[DiscFile, ...]

    @property
    def total_bytes(self) -> int:
        return sum(f.size for f in self.files)


@dataclass(frozen=True)
class ExtractReport:
    """What `extract` wrote."""

    out_dir: Path
    sys_files: dict[str, int]
    """`sys/<name>` -> bytes written."""
    file_count: int
    total_bytes: int
    main_dol_sha1: str


# --------------------------------------------------------------------------
# Header and system files
# --------------------------------------------------------------------------


def _read_exact(f: BinaryIO, offset: int, size: int) -> bytes:
    f.seek(offset)
    data = f.read(size)
    if len(data) != size:
        raise DiscFormatError(
            f"short read: wanted {size:#x} bytes at {offset:#x}, got {len(data):#x}"
        )
    return data


def _u32(data: bytes, offset: int) -> int:
    return struct.unpack_from(">I", data, offset)[0]


def read_header(f: BinaryIO) -> DiscHeader:
    """Decode boot.bin from the start of the image."""
    boot = _read_exact(f, 0, BOOT_BIN_SIZE)
    game_id = boot[GAME_ID_OFFSET : GAME_ID_OFFSET + GAME_ID_SIZE]
    if not game_id.isalnum():
        raise DiscFormatError(f"game id {game_id!r} is not alphanumeric; not a GameCube image?")
    title = boot[TITLE_OFFSET : TITLE_OFFSET + TITLE_SIZE].split(b"\0", 1)[0]
    return DiscHeader(
        game_id=game_id.decode("ascii"),
        title=title.decode("ascii", errors="replace"),
        dol_offset=_u32(boot, DOL_OFFSET_FIELD),
        fst_offset=_u32(boot, FST_OFFSET_FIELD),
        fst_size=_u32(boot, FST_SIZE_FIELD),
        fst_max_size=_u32(boot, FST_MAX_SIZE_FIELD),
    )


def dol_size(f: BinaryIO, dol_offset: int) -> int:
    """Length of main.dol: the end of whichever section reaches furthest.

    The DOL header has no length field, so this is how every extractor
    (Dolphin included) sizes it.
    """
    header = _read_exact(f, dol_offset, DOL_HEADER_SIZE)
    count = DOL_TEXT_SECTIONS + DOL_DATA_SECTIONS
    end = DOL_HEADER_SIZE
    for i in range(count):
        offset = _u32(header, DOL_SECTION_OFFSETS_FIELD + 4 * i)
        size = _u32(header, DOL_SECTION_SIZES_FIELD + 4 * i)
        if size:
            end = max(end, offset + size)
    return end


def apploader_size(f: BinaryIO) -> int:
    """Length of apploader.img: header, code, and trailer."""
    header = _read_exact(f, APPLOADER_OFFSET, APPLOADER_HEADER_SIZE)
    code_size = _u32(header, 0x14)
    trailer_size = _u32(header, 0x18)
    return APPLOADER_HEADER_SIZE + code_size + trailer_size


# --------------------------------------------------------------------------
# FST
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class _RawEntry:
    is_dir: bool
    name_offset: int
    offset_or_parent: int
    size_or_next: int

    @classmethod
    def unpack(cls, raw: bytes, index: int) -> "_RawEntry":
        base = index * FST_ENTRY_SIZE
        flags_and_name = _u32(raw, base)
        return cls(
            is_dir=(flags_and_name >> 24) == FST_FLAG_DIRECTORY,
            name_offset=flags_and_name & 0x00FF_FFFF,
            offset_or_parent=_u32(raw, base + 4),
            size_or_next=_u32(raw, base + 8),
        )


def _entry_name(raw: bytes, strings_start: int, name_offset: int) -> str:
    start = strings_start + name_offset
    end = raw.find(b"\0", start)
    if start >= len(raw) or end < 0:
        raise DiscFormatError(f"FST name at string offset {name_offset:#x} runs off the table")
    name = raw[start:end].decode("shift_jis", errors="replace")
    if not name or "/" in name or name in (".", ".."):
        raise DiscFormatError(f"FST contains an unsafe entry name {name!r}")
    return name


def parse_fst(raw: bytes) -> Fst:
    """Turn the bytes of fst.bin into a directory and file list.

    Files and directories are returned in table order, which is the order
    the disc stores them in and the order Dolphin lists them.
    """
    if len(raw) < FST_ENTRY_SIZE:
        raise DiscFormatError("FST is shorter than one entry")
    root = _RawEntry.unpack(raw, 0)
    if not root.is_dir:
        raise DiscFormatError("FST entry 0 is not a directory")
    total = root.size_or_next
    strings_start = total * FST_ENTRY_SIZE
    if strings_start > len(raw):
        raise DiscFormatError(
            f"FST declares {total} entries ({strings_start:#x} bytes) but is only {len(raw):#x} bytes"
        )

    dirs: list[PurePosixPath] = []
    files: list[DiscFile] = []
    # Open directories: (index one past the last descendant, path).
    open_dirs: list[tuple[int, PurePosixPath]] = [(total, PurePosixPath())]
    for index in range(1, total):
        while index >= open_dirs[-1][0]:
            open_dirs.pop()
        parent_end, parent_path = open_dirs[-1]
        entry = _RawEntry.unpack(raw, index)
        path = parent_path / _entry_name(raw, strings_start, entry.name_offset)
        if entry.is_dir:
            end = entry.size_or_next
            if end <= index or end > parent_end:
                raise DiscFormatError(
                    f"FST directory {path} at entry {index} has next-sibling {end}, "
                    f"outside its parent's range ({index + 1}..{parent_end})"
                )
            dirs.append(path)
            open_dirs.append((end, path))
        else:
            files.append(DiscFile(path, entry.offset_or_parent, entry.size_or_next))
    return Fst(tuple(dirs), tuple(files))


def read_fst(f: BinaryIO, header: DiscHeader) -> Fst:
    return parse_fst(_read_exact(f, header.fst_offset, header.fst_size))


def format_tree(fst: Fst) -> str:
    """The tree as indented text with file sizes, for `--list`."""
    sizes = {f.path: f.size for f in fst.files}
    entries: list[PurePosixPath] = sorted(
        list(fst.dirs) + [f.path for f in fst.files], key=lambda p: p.parts
    )
    lines = []
    for path in entries:
        indent = "  " * (len(path.parts) - 1)
        if path in sizes:
            lines.append(f"{indent}{path.name}  {sizes[path]:,} bytes")
        else:
            lines.append(f"{indent}{path.name}/")
    return "\n".join(lines)


# --------------------------------------------------------------------------
# Extraction
# --------------------------------------------------------------------------


def copy_range(src: BinaryIO, offset: int, size: int, dst: Path) -> None:
    """Stream `size` bytes at `offset` of `src` into the file `dst`."""
    dst.parent.mkdir(parents=True, exist_ok=True)
    src.seek(offset)
    remaining = size
    with dst.open("wb") as out:
        while remaining:
            chunk = src.read(min(COPY_CHUNK, remaining))
            if not chunk:
                raise DiscFormatError(
                    f"image ended {remaining:#x} bytes before the end of {dst.name} "
                    f"(offset {offset:#x}, size {size:#x})"
                )
            out.write(chunk)
            remaining -= len(chunk)


def system_files(f: BinaryIO, header: DiscHeader) -> dict[str, tuple[int, int]]:
    """`sys/<name>` -> (disc offset, size) for the five system files."""
    return {
        "boot.bin": (0, BOOT_BIN_SIZE),
        "bi2.bin": (BI2_BIN_OFFSET, BI2_BIN_SIZE),
        "apploader.img": (APPLOADER_OFFSET, apploader_size(f)),
        "main.dol": (header.dol_offset, dol_size(f, header.dol_offset)),
        "fst.bin": (header.fst_offset, header.fst_size),
    }


def extract(f: BinaryIO, out_dir: Path, *, progress: Iterable[str] | None = None) -> ExtractReport:
    """Write `sys/` and `files/` under `out_dir`.

    `progress`, if given, is fed one line per file for the caller to print.
    """
    header = read_header(f)
    fst = read_fst(f, header)

    sys_dir = out_dir / "sys"
    written: dict[str, int] = {}
    for name, (offset, size) in system_files(f, header).items():
        copy_range(f, offset, size, sys_dir / name)
        written[name] = size

    files_dir = out_dir / "files"
    for d in fst.dirs:
        (files_dir / Path(*d.parts)).mkdir(parents=True, exist_ok=True)
    for entry in fst.files:
        copy_range(f, entry.offset, entry.size, files_dir / Path(*entry.path.parts))
        if progress is not None:
            print(f"  {entry.path}  {entry.size:,}", file=sys.stderr)

    return ExtractReport(
        out_dir=out_dir,
        sys_files=written,
        file_count=len(fst.files),
        total_bytes=fst.total_bytes,
        main_dol_sha1=hashlib.sha1((sys_dir / "main.dol").read_bytes()).hexdigest(),
    )


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("iso", type=Path, help="GameCube disc image (.iso / .gcm)")
    parser.add_argument(
        "--out",
        type=Path,
        help="destination directory; gets sys/ and files/ (default: the image's directory)",
    )
    parser.add_argument("--list", action="store_true", help="print the file tree and exit")
    parser.add_argument("-v", "--verbose", action="store_true", help="print each file as it is written")
    args = parser.parse_args(argv)

    with args.iso.open("rb") as f:
        header = read_header(f)
        print(f"{header.game_id}  {header.title}", file=sys.stderr)
        if args.list:
            fst = read_fst(f, header)
            print(format_tree(fst))
            print(f"{len(fst.files)} files, {fst.total_bytes:,} bytes", file=sys.stderr)
            return 0
        out_dir = args.out if args.out is not None else args.iso.parent
        report = extract(f, out_dir, progress=[] if args.verbose else None)

    sys_total = sum(report.sys_files.values())
    print(f"sys/: {len(report.sys_files)} files, {sys_total:,} bytes", file=sys.stderr)
    for name, size in report.sys_files.items():
        print(f"  {name}  {size:,}", file=sys.stderr)
    print(f"sys/main.dol sha1 {report.main_dol_sha1}", file=sys.stderr)
    print(f"files/: {report.file_count} files, {report.total_bytes:,} bytes", file=sys.stderr)
    print(f"wrote {report.out_dir}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
