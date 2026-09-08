"""Exercise gen_schema's comment scanner on a synthetic header covering every comment form."""
from __future__ import annotations

from pathlib import Path

import gen_schema as gs

HEADER = """
typedef int enum_t;
typedef enum Kind { K_A, K_B } Kind;
typedef void (*Event)(int);
typedef struct { float x, y, z; } Vec, Vec3;

struct Inner {
    /*  +0 */ u32 buttons;
    /*  +4 */ int level;
    /* +8:0 */ u8 flag_a : 1;
    /* +8:1 */ u8 flag_b : 2;
    /* 0xC */ Kind kind;
};
ASSERT_SIZE(struct Inner, 0x10);

struct Outer {
    /* fp+0 */ HSD_GObj* gobj;
    /*   fp+4 */ Vec3 pos;
    /// @at{10} @sz{4}
    float speed;
    /** @at{14} @sz{4} * @brief doc block */
    Event cb;
    /* fp+18 */ struct dmg {
        /* fp+18 */ float percent;
        /* fp+1C */ enum_t kb_angle;
    } dmg;
    /* fp+20 */ struct Inner inner;
    /* fp+x30 */ u8 (*tbl)[2];
    /* fp+34 */ union {
        u32 flags;
        struct { u8 b0 : 1; };
    };
    /* fp+38 */ u32 arr[4];
    /* fp+48 */ struct { int a; } pair[2];
    /* fp+58 */ int late;
    /* fp+50 */ int early;
    /* fp+60 */ int past_end;
};
ASSERT_SIZE(struct Outer, 0x60);
"""


def make_header(tmp_path: Path) -> gs.Header:
    p = tmp_path / "types.h"
    p.write_text(HEADER)
    t = gs.TypedefTable()
    t._scan(HEADER)
    hdr = gs.Header.__new__(gs.Header)
    hdr.path = p
    hdr.rel = "types.h"
    hdr.raw = HEADER
    hdr.text = gs.preprocess(HEADER)
    hdr.typedefs = t
    hdr.notes, hdr.warnings = [], []
    return hdr


def fields(root: gs.Node) -> dict[str, gs.Node]:
    return {n.name: n for n in root.leaves()}


def test_offset_comment_forms():
    assert gs.parse_offset_comment(" fp+B0 ") == gs.Off(0xB0, True)
    assert gs.parse_offset_comment("  +10 ") == gs.Off(0x10, False)
    assert gs.parse_offset_comment(" 0x1A88 ") == gs.Off(0x1A88, False)
    assert gs.parse_offset_comment(" fp+x1A88 ") == gs.Off(0x1A88, True)
    assert gs.parse_offset_comment(" fp+2218:3 ") == gs.Off(0x2218, True, 3)
    assert gs.parse_offset_comment("* @at{D38} @sz{4} * @brief x") == gs.Off(0xD38, False)
    assert gs.parse_offset_comment(" Effects? ") is None


def test_scan_and_types(tmp_path):
    hdr = make_header(tmp_path)
    root = hdr.scan("Outer")
    f = fields(root)
    assert (f["gobj"].offset, f["gobj"].schema_type) == (0x0, "ptr")
    assert (f["pos"].offset, f["pos"].schema_type) == (0x4, "vec3")
    assert (f["speed"].offset, f["speed"].schema_type) == (0x10, "f32")      # /// @at{}
    assert (f["cb"].offset, f["cb"].schema_type) == (0x14, "ptr")           # /** @at{} */, funcptr typedef
    assert (f["dmg.percent"].offset, f["dmg.kb_angle"].schema_type) == (0x18, "s32")
    # struct Inner recursed with base 0x20 and relative offsets
    assert f["inner.buttons"].offset == 0x20
    assert (f["inner.level"].offset, f["inner.level"].schema_type) == (0x24, "s32")
    assert (f["inner.flag_b"].offset, f["inner.flag_b"].bit, f["inner.flag_b"].width) == (0x28, 1, 2)
    assert f["inner.flag_b"].schema_type == "unknown"
    assert (f["inner.kind"].offset, f["inner.kind"].schema_type) == (0x2C, "s32")  # enum
    assert (f["tbl"].offset, f["tbl"].schema_type) == (0x30, "ptr")         # fp+x typo, pointer-to-array
    assert (f["_anon_34"].offset, f["_anon_34"].ctype) == (0x34, "union")   # anonymous union, no inner offsets
    assert (f["arr"].schema_type, f["arr"].ctype) == ("unknown", "u32[4]")
    assert (f["pair"].schema_type, f["pair"].ctype) == ("unknown", "struct[2]")
    assert hdr.assert_size("Outer") == 0x60


def test_validation_flags_decrease_and_overflow(tmp_path):
    hdr = make_header(tmp_path)
    root = hdr.scan("Outer")
    violations, _ = gs.validate(root, 0x60, hdr)
    assert len(violations) == 2
    assert any("early" in v and "decrease" in v for v in violations)
    assert any("past_end" in v and ">= size" in v for v in violations)
