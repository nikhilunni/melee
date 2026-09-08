"""Exercise the GameCube FST parser and extractor on a small in-memory disc image.

No real disc is involved: `build_disc` lays out a header, a two-section
main.dol, a tiny apploader, and an FST with a nested directory, exactly as
`extract_fst` documents the format.
"""
from __future__ import annotations

import io
import struct
from pathlib import Path, PurePosixPath

import pytest

import extract_fst as fst

FST_OFFSET = 0x10000
DOL_OFFSET = 0x8000
FILE_DATA_OFFSET = 0x20000


def be32(*values: int) -> bytes:
    return struct.pack(f">{len(values)}I", *values)


def fst_entry(is_dir: bool, name_offset: int, a: int, b: int) -> bytes:
    return be32(((1 if is_dir else 0) << 24) | name_offset, a, b)


def build_fst(files: dict[str, bytes]) -> tuple[bytes, dict[str, int]]:
    """An FST for: root / { top.bin, audio/ { us/ { voice.ssm }, bgm.hps }, GrNLa.dat }.

    Returns the table bytes and each file's disc offset. Entry order matches
    the layout the retail disc uses: a directory's entries follow it and its
    next-sibling index is one past its last descendant.
    """
    names = ["top.bin", "audio", "us", "voice.ssm", "bgm.hps", "GrNLa.dat"]
    strings = b""
    name_off: dict[str, int] = {}
    for n in names:
        name_off[n] = len(strings)
        strings += n.encode() + b"\0"

    offsets: dict[str, int] = {}
    cursor = FILE_DATA_OFFSET
    for path in ("top.bin", "audio/us/voice.ssm", "audio/bgm.hps", "GrNLa.dat"):
        offsets[path] = cursor
        cursor += (len(files[path]) + 0x7FFF) & ~0x7FFF  # 32 KiB alignment, like retail

    # index: 0 root, 1 top.bin, 2 audio/, 3 us/, 4 voice.ssm, 5 bgm.hps, 6 GrNLa.dat
    total = 7
    entries = b"".join(
        [
            fst_entry(True, 0, 0, total),
            fst_entry(False, name_off["top.bin"], offsets["top.bin"], len(files["top.bin"])),
            fst_entry(True, name_off["audio"], 0, 6),
            fst_entry(True, name_off["us"], 2, 5),
            fst_entry(False, name_off["voice.ssm"], offsets["audio/us/voice.ssm"], len(files["audio/us/voice.ssm"])),
            fst_entry(False, name_off["bgm.hps"], offsets["audio/bgm.hps"], len(files["audio/bgm.hps"])),
            fst_entry(False, name_off["GrNLa.dat"], offsets["GrNLa.dat"], len(files["GrNLa.dat"])),
        ]
    )
    return entries + strings, offsets


def build_dol() -> bytes:
    """A DOL with one text section at 0x100 (0x40 bytes) and one data section at 0x180 (0x20)."""
    header = bytearray(fst.DOL_HEADER_SIZE)
    struct.pack_into(">I", header, fst.DOL_SECTION_OFFSETS_FIELD, 0x100)
    struct.pack_into(">I", header, fst.DOL_SECTION_SIZES_FIELD, 0x40)
    data_idx = fst.DOL_TEXT_SECTIONS  # first data section
    struct.pack_into(">I", header, fst.DOL_SECTION_OFFSETS_FIELD + 4 * data_idx, 0x180)
    struct.pack_into(">I", header, fst.DOL_SECTION_SIZES_FIELD + 4 * data_idx, 0x20)
    body = bytearray(0x1A0 - fst.DOL_HEADER_SIZE)
    body[0:0x40] = b"T" * 0x40
    body[0x80:0xA0] = b"D" * 0x20
    return bytes(header) + bytes(body)


FILES = {
    "top.bin": b"top-level file",
    "audio/us/voice.ssm": b"voice" * 100,
    "audio/bgm.hps": b"",  # zero-length files exist on real discs
    "GrNLa.dat": bytes(range(256)),
}


def build_disc() -> tuple[bytes, dict[str, int]]:
    table, offsets = build_fst(FILES)
    dol = build_dol()
    apploader_code, apploader_trailer = b"A" * 0x30, b"R" * 0x10

    image = bytearray(FILE_DATA_OFFSET + 0x20000)
    image[0:6] = b"GALE01"
    image[fst.TITLE_OFFSET : fst.TITLE_OFFSET + 5] = b"Melee"
    struct.pack_into(">I", image, fst.DOL_OFFSET_FIELD, DOL_OFFSET)
    struct.pack_into(">I", image, fst.FST_OFFSET_FIELD, FST_OFFSET)
    struct.pack_into(">I", image, fst.FST_SIZE_FIELD, len(table))
    struct.pack_into(">I", image, fst.FST_MAX_SIZE_FIELD, len(table))
    image[fst.BI2_BIN_OFFSET : fst.BI2_BIN_OFFSET + 4] = b"bi2!"
    app = fst.APPLOADER_OFFSET
    struct.pack_into(">II", image, app + 0x14, len(apploader_code), len(apploader_trailer))
    image[app + 0x20 : app + 0x20 + len(apploader_code) + len(apploader_trailer)] = (
        apploader_code + apploader_trailer
    )
    image[DOL_OFFSET : DOL_OFFSET + len(dol)] = dol
    image[FST_OFFSET : FST_OFFSET + len(table)] = table
    for path, data in FILES.items():
        image[offsets[path] : offsets[path] + len(data)] = data
    return bytes(image), offsets


def test_header_fields() -> None:
    image, _ = build_disc()
    header = fst.read_header(io.BytesIO(image))
    assert header.game_id == "GALE01"
    assert header.title == "Melee"
    assert header.dol_offset == DOL_OFFSET
    assert header.fst_offset == FST_OFFSET


def test_parse_fst_recovers_tree_in_table_order() -> None:
    image, offsets = build_disc()
    header = fst.read_header(io.BytesIO(image))
    table = fst.read_fst(io.BytesIO(image), header)

    assert table.dirs == (PurePosixPath("audio"), PurePosixPath("audio/us"))
    assert [(str(f.path), f.offset, f.size) for f in table.files] == [
        ("top.bin", offsets["top.bin"], len(FILES["top.bin"])),
        ("audio/us/voice.ssm", offsets["audio/us/voice.ssm"], len(FILES["audio/us/voice.ssm"])),
        ("audio/bgm.hps", offsets["audio/bgm.hps"], 0),
        ("GrNLa.dat", offsets["GrNLa.dat"], 256),
    ]
    assert table.total_bytes == sum(len(d) for d in FILES.values())


def test_format_tree_indents_by_depth() -> None:
    image, _ = build_disc()
    header = fst.read_header(io.BytesIO(image))
    text = fst.format_tree(fst.read_fst(io.BytesIO(image), header))
    assert text.splitlines() == [
        "GrNLa.dat  256 bytes",
        "audio/",
        "  bgm.hps  0 bytes",
        "  us/",
        "    voice.ssm  500 bytes",
        "top.bin  14 bytes",
    ]


def test_dol_size_is_the_furthest_section_end() -> None:
    image, _ = build_disc()
    assert fst.dol_size(io.BytesIO(image), DOL_OFFSET) == 0x1A0


def test_extract_writes_sys_and_files(tmp_path: Path) -> None:
    image, _ = build_disc()
    report = fst.extract(io.BytesIO(image), tmp_path)

    assert report.file_count == 4
    assert report.total_bytes == sum(len(d) for d in FILES.values())
    for path, data in FILES.items():
        assert (tmp_path / "files" / path).read_bytes() == data

    sys_dir = tmp_path / "sys"
    assert (sys_dir / "boot.bin").read_bytes() == image[: fst.BOOT_BIN_SIZE]
    assert (sys_dir / "bi2.bin").read_bytes()[:4] == b"bi2!"
    assert (sys_dir / "apploader.img").read_bytes()[0x20:] == b"A" * 0x30 + b"R" * 0x10
    assert (sys_dir / "main.dol").read_bytes() == build_dol()
    assert (sys_dir / "fst.bin").read_bytes() == image[FST_OFFSET : FST_OFFSET + report.sys_files["fst.bin"]]
    assert report.sys_files["main.dol"] == 0x1A0


@pytest.mark.parametrize(
    "mutate, message",
    [
        # Root flagged as a file.
        (lambda t: b"\0" + t[1:], "entry 0 is not a directory"),
        # Directory whose next-sibling index points before itself.
        (lambda t: t[: 2 * 12] + fst_entry(True, 8, 0, 1) + t[3 * 12 :], "outside its parent"),
        # Entry count larger than the table.
        (lambda t: t[:8] + be32(10_000) + t[12:], "declares 10000 entries"),
    ],
)
def test_malformed_tables_are_rejected(mutate, message) -> None:
    table, _ = build_fst(FILES)
    with pytest.raises(fst.DiscFormatError, match=message):
        fst.parse_fst(mutate(table))


def test_unsafe_names_are_rejected() -> None:
    table, _ = build_fst(FILES)
    strings_start = 7 * fst.FST_ENTRY_SIZE
    bad = bytearray(table)
    bad[strings_start : strings_start + 7] = b"../evil"  # overwrite "top.bin"
    with pytest.raises(fst.DiscFormatError, match="unsafe entry name"):
        fst.parse_fst(bytes(bad))


def test_truncated_image_is_reported(tmp_path: Path) -> None:
    image, offsets = build_disc()
    cut = image[: offsets["GrNLa.dat"] + 16]
    with pytest.raises(fst.DiscFormatError, match="image ended"):
        fst.extract(io.BytesIO(cut), tmp_path)
