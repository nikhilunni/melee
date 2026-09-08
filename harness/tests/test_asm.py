"""Unit tests for asm.py's parsing and lookup, on a fixture snippet.

The real split output needs the disc (see docs/ASM.md); these tests only use
the small `.s` excerpt below, which is the `sinf` small-argument branch from
`build/GALE01/asm/MSL/trigf.s` plus a second function, in dtk's exact
output format.
"""
from __future__ import annotations

import pytest

import asm

TRIGF_SNIPPET = """\
.include "macros.inc"
.file "trigf.c"

# 0x803261BC..0x803265A8 | size: 0x3EC
.text
.balign 4

# .text:0x0 | 0x803261BC | size: 0x44
.fn tanf, global
/* 803261BC 00322D9C  7C 08 02 A6 */	mflr r0
/* 803261C0 00322DA0  90 01 00 04 */	stw r0, 0x4(r1)
/* 803261C8 00322DA8  DB E1 00 10 */	stfd f31, 0x10(r1)
/* 803261CC 00322DAC  FF E0 08 90 */	fmr f31, f1
/* 803261D0 00322DB0  48 00 00 4D */	bl sin__Ff
/* 803261D4 00322DB4  FC 40 08 90 */	fmr f2, f1
/* 803261D8 00322DB8  FC 20 F8 90 */	fmr f1, f31
/* 803261DC 00322DBC  48 00 00 49 */	bl cos__Ff
/* 803261E0 00322DC0  EC 22 08 24 */	fdivs f1, f2, f1
/* 803261FC 00322DDC  4E 80 00 20 */	blr
.endfn tanf

# .text:0x218 | 0x803263D4 | size: 0x1A4
.fn sinf, global
/* 803263D4 00323188  7C 08 02 A6 */	mflr r0
/* 80326404 003231B8  EC 20 01 B2 */	fmuls f1, f0, f6
/* 80326408 003231BC  41 82 00 1C */	beq .L_80326424
/* 80326414 003231C8  FC 00 00 1E */	fctiwz f0, f0
/* 80326420 003231D4  48 00 00 18 */	b .L_80326438
.L_80326424:
/* 80326428 003231DC  EC 00 08 2A */	fadds f0, f0, f1
.L_80326438:
/* 8032646C 00323220  EC 06 00 28 */	fsubs f0, f6, f0
/* 80326470 00323224  EC 02 01 BA */	fmadds f0, f2, f6, f0
/* 80326474 00323228  EC 03 01 BA */	fmadds f0, f3, f6, f0
/* 80326484 00323238  4B FF FD 31 */	bl fabsf__Ff
/* 803264B4 00323268  EC 3F 00 72 */	fmuls f1, f31, f1
/* 803264BC 00323270  EC 22 00 7A */	fmadds f1, f2, f1, f0
/* 80326318 003230F8  EC 3F 00 7C */	fnmsubs f1, f31, f1, f0
/* 80326368 00323148  EC 24 09 FE */	fnmadds f1, f4, f2, f1
/* 80326574 00323328  4E 80 00 20 */	blr
.endfn sinf
"""

SPLITS_SNIPPET = """\
Sections:
	.init       type:code align:32
	.text       type:code align:32
	.sdata2     type:rodata align:32

melee/lb/lbtrigf.c:
	.text       start:0x80022C30 end:0x8002305C
	.rodata     start:0x803B7300 end:0x803B73B8
	.sdata2     start:0x804D7DB0 end:0x804D7DD8

MSL/trigf.c:
	.text       start:0x803261BC end:0x803265A8
	.ctors      start:0x803B7244 end:0x803B7248

Runtime/__start.c:
	.init       start:0x80003100 end:0x800034E0
"""


def test_parse_function_keeps_instructions_and_labels_in_order():
    fn = asm.parse_function(TRIGF_SNIPPET, "sinf")
    assert fn is not None
    assert fn.name == "sinf"
    first = fn.lines[0]
    assert isinstance(first, asm.Instruction)
    assert (first.addr, first.mnemonic, first.operands) == (0x803263D4, "mflr", "r0")
    assert first.raw == bytes.fromhex("7C0802A6")
    labels = [x.name for x in fn.lines if isinstance(x, asm.Label)]
    assert labels == [".L_80326424", ".L_80326438"]
    # A label sits between the instructions that surround it in the file.
    kinds = [type(x).__name__ for x in fn.lines[4:8]]
    assert kinds == ["Instruction", "Label", "Instruction", "Label"]


def test_parse_function_stops_at_its_own_endfn():
    tanf = asm.parse_function(TRIGF_SNIPPET, "tanf")
    assert tanf is not None
    assert [i.addr for i in tanf.instructions][-1] == 0x803261FC
    assert asm.parse_function(TRIGF_SNIPPET, "cosf") is None


def test_fused_filter_covers_every_fma_family_mnemonic():
    fn = asm.parse_function(TRIGF_SNIPPET, "sinf")
    assert [(i.addr, i.mnemonic) for i in fn.fused()] == [
        (0x80326470, "fmadds"),
        (0x80326474, "fmadds"),
        (0x803264BC, "fmadds"),
        (0x80326318, "fnmsubs"),
        (0x80326368, "fnmadds"),
    ]
    # Plain multiplies and adds are not fused.
    assert "fmuls" not in asm.FUSED_MNEMONICS
    assert "fadds" not in asm.FUSED_MNEMONICS
    assert {"fmadd", "fmsub", "fnmadd", "fnmsub"} <= asm.FUSED_MNEMONICS


def test_calls_lists_bl_targets_in_program_order():
    assert asm.parse_function(TRIGF_SNIPPET, "tanf").calls() == [
        (0x803261D0, "sin__Ff"),
        (0x803261DC, "cos__Ff"),
    ]
    assert asm.parse_function(TRIGF_SNIPPET, "sinf").calls() == [(0x80326484, "fabsf__Ff")]


def test_format_function_prints_address_bytes_and_text():
    fn = asm.parse_function(TRIGF_SNIPPET, "tanf")
    out = asm.format_function(fn).splitlines()
    assert out[0] == "803261BC  7C0802A6  mflr r0"
    assert out[-1] == "803261FC  4E800020  blr"
    assert asm.format_function(fn, calls=True).splitlines() == [
        "803261D0  bl sin__Ff",
        "803261DC  bl cos__Ff",
    ]
    fused = asm.format_function(asm.parse_function(TRIGF_SNIPPET, "sinf"), fused=True)
    assert fused.splitlines()[0] == "80326470  EC0201BA  fmadds f0, f2, f6, f0"


def test_parse_splits_keeps_only_code_ranges_sorted():
    ranges = asm.parse_splits(SPLITS_SNIPPET)
    assert ranges == [
        (0x80003100, 0x800034E0, "Runtime/__start.c"),
        (0x80022C30, 0x8002305C, "melee/lb/lbtrigf.c"),
        (0x803261BC, 0x803265A8, "MSL/trigf.c"),
    ]


@pytest.mark.parametrize(
    ("addr", "unit"),
    [
        (0x80022C30, "melee/lb/lbtrigf.c"),  # first byte of a range
        (0x80023058, "melee/lb/lbtrigf.c"),  # last instruction
        (0x8002305C, None),  # one past the end: a gap before the next unit
        (0x803263D4, "MSL/trigf.c"),
        (0x80003100, "Runtime/__start.c"),
        (0x80000000, None),
        (0x90000000, None),
    ],
)
def test_unit_for_address(addr, unit):
    ranges = asm.parse_splits(SPLITS_SNIPPET)
    assert asm.unit_for_address(ranges, addr) == unit


def test_asm_path_mirrors_the_unit_path():
    p = asm.asm_path_for_unit("melee/lb/lbtrigf.c")
    assert p == asm.ASM_DIR / "melee" / "lb" / "lbtrigf.s"


def test_resolve_accepts_symbols_and_addresses():
    # Uses the real symbols.txt from the submodule; skip if it is absent.
    if not asm.symbols.SYMBOLS.exists():
        pytest.skip("decomp submodule not checked out")
    assert asm.resolve("sinf") == ("sinf", 0x803263D4)
    assert asm.resolve("0x803263D4") == ("sinf", 0x803263D4)
    assert asm.resolve("80326470") == ("sinf", 0x803263D4), "an address inside the body"
    with pytest.raises(KeyError):
        asm.resolve("not_a_symbol_anywhere")
