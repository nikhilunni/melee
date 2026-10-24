"""parity_inventory.py's parsing and classification, on a small synthetic
decomp (symbols, splits, C sources, a motion-state table) and crate tree."""
from __future__ import annotations

from pathlib import Path

import pytest

import parity_inventory as pi

SYMBOLS = """\
memset = .init:0x80003100; // type:function size:0x30 scope:global
ftCo_80001000 = .text:0x80001000; // type:function size:0x40 scope:global
ftCo_Wait_Anim = .text:0x80001040; // type:function size:0x20 scope:global
ftCo_Wait_Phys = .text:0x80001060; // type:function size:0x20 scope:global
ftCo_80001080 = .text:0x80001080; // type:function size:0x40 scope:global
grX_80002000 = .text:0x80002000; // type:function size:0x10 scope:global
mnY_80003000 = .text:0x80003000; // type:function size:0x10 scope:global
it_80004000 = .text:0x80004000; // type:function size:0x10 scope:global
it_80004010 = .text:0x80004010; // type:function size:0x10 scope:global
ftData_MotionStateList = .data:0x80400000; // type:object size:0x100 scope:global
"""

SPLITS = """\
Sections:
\t.text       type:code align:32

melee/ft/ftcommon.c:
\t.text       start:0x80001000 end:0x80002000
\t.data       start:0x80400000 end:0x80400100

melee/gr/grx.c:
\t.text       start:0x80002000 end:0x80003000

melee/mn/mny.c:
\t.text       start:0x80003000 end:0x80004000

melee/it/kinds/itz.c:
\t.text       start:0x80004000 end:0x80005000
"""

FTCOMMON_C = """\
#include "ftcommon.h"

void ftCo_80001000(Fighter* fp)
{
    if (fp->x) {
        fp->y = 1;
    }
}

static void ftCo_80001080(Fighter* fp,
                          int a)
{
    return;
}
"""

MOTION_C = """\
MotionState ftData_MotionStateList[2] = {
    {
        // ftCo_MS_Wait = 0
        0, 0, 0,
        ftCo_Wait_Anim,
        ftCo_Wait_Phys,
    },
    {
        // ftCo_MS_Other = 1
        0, 0, 0,
        ftCo_80001080,
    },
};
"""

ITEM_C = """\
ItemStateTable it_803F0000[] = {
    { 0, it_80004000, it_80004010, NULL },
};
"""

RUST = '''\
//! Fighter procs.

/// ftCo_80001000 (80001000): the port of it. ftCo_80001080 is a callee.
pub fn port(x: u32) -> u32 {
    // grX_80002000 runs here too.
    if x == 3 {
        unimplemented!("ftcommon.c:6: the y branch");
    }
    let c = 'a';
    let _s: &'static str = "it_80004000 in a string";
    x
}

/// A field that mentions mnY_80003000 and HSD_Unknown_Name.
pub struct S {
    pub a: u32,
}
'''


@pytest.fixture
def tree(tmp_path: Path) -> tuple[Path, Path]:
    decomp = tmp_path / "decomp"
    cfg = decomp / "config" / "GALE01"
    cfg.mkdir(parents=True)
    (cfg / "symbols.txt").write_text(SYMBOLS)
    (cfg / "splits.txt").write_text(SPLITS)
    src = decomp / "src" / "melee"
    (src / "ft").mkdir(parents=True)
    (src / "ft" / "ftcommon.c").write_text(FTCOMMON_C)
    (src / "ft" / "ftmotionstates.c").write_text(MOTION_C)
    (src / "it").mkdir()
    (src / "it" / "itz.c").write_text(ITEM_C)
    crates = tmp_path / "code" / "crates"
    (crates / "melee-ft" / "src").mkdir(parents=True)
    (crates / "melee-ft" / "src" / "lib.rs").write_text(RUST)
    return decomp, crates


def test_symbols_keep_section_size_and_type():
    syms = pi.parse_symbols(SYMBOLS)
    by = {s.name: s for s in syms}
    assert by["ftCo_80001000"].addr == 0x80001000
    assert by["ftCo_80001000"].size == 0x40
    assert by["memset"].section == "init"
    assert by["ftData_MotionStateList"].kind == "object"


def test_splits_attribute_code_ranges_to_files():
    splits = pi.parse_splits(SPLITS)
    assert pi.source_file(0x80001010, splits) == "melee/ft/ftcommon.c"
    assert pi.source_file(0x80002000, splits) == "melee/gr/grx.c"
    assert pi.source_file(0x80009000, splits) == "(unsplit)"


@pytest.mark.parametrize("src, sub, scope", [
    ("melee/ft/kinds/ftKirby/ftkirby.c", "ft/kinds/ftKirby", "in-game"),
    ("melee/mn/mnmain.c", "mn", "menus"),
    ("melee/ft/kinds/ftMasterHand/ftmh.c", "ft/kinds/ftMasterHand", "single-player fighters"),
    ("dolphin/gx/GXInit.c", "dolphin/gx", "rendering"),
    ("MSL/trigf.c", "MSL", "in-game"),
    ("MSL/printf.c", "MSL", "OS, hardware, loading"),
])
def test_subsystem_and_scope(src, sub, scope):
    assert pi.subsystem(src) == sub
    assert pi.scope(src) == scope


def test_c_functions_span_their_bodies():
    funcs = {name: (a, b) for name, a, b in pi.c_functions(FTCOMMON_C)}
    assert funcs["ftCo_80001000"] == (3, 8)
    assert funcs["ftCo_80001080"] == (10, 14)


def test_lexer_separates_comments_strings_and_lifetimes():
    lines = pi.lex_rust(RUST)
    body = next(l for l in lines if "'a'" in l.code or "' '" in l.code)
    assert "a" not in body.comment
    s = next(l for l in lines if "in a string" in l.strings)
    assert "'static" in s.code
    assert lines[0].comment.strip() == "Fighter procs." and not lines[0].doc
    assert lines[2].doc
    assert all(l.depth >= 0 for l in lines)


def test_header_tokens_take_the_symbol_part():
    assert pi.header_tokens("ftCo_80001000 (80001000): the port") == "ftCo_80001000 (80001000)"
    assert pi.header_tokens("`a_1` / `b_2`: both") == "a_1` / `b_2`"
    assert pi.header_tokens("The rows, with ftCo_80001000") == "The"


def test_motion_and_item_tables_list_row_identifiers():
    rows = pi.motion_tables(MOTION_C)[0]
    assert rows[0][-2:] == ["ftCo_Wait_Anim", "ftCo_Wait_Phys"]
    assert "ftCo_80001080" in rows[1]
    items = pi.motion_tables(ITEM_C, pi._ITEM_TABLE)[0]
    assert items[0][0:2] == ["it_80004000", "it_80004010"]


def test_build_classifies_every_function(tree):
    decomp, crates = tree
    rows_tsv = "table\taction\timplemented\ncommon\t0\ttrue\ncommon\t1\tfalse\n"
    inv = pi.build(decomp, crates, rows_tsv)
    status = {r["name"]: r["status"] for r in inv.rows}
    # The header names it and an unimplemented! sits in its body.
    assert status["ftCo_80001000"] == "partial"
    # Named later in the same doc block, and its row is not implemented.
    assert status["ftCo_80001080"] == "body"
    # A comment inside a ported fn.
    assert status["grX_80002000"] == "body"
    # Callbacks of an implemented motion row.
    assert status["ftCo_Wait_Anim"] == "row"
    assert status["ftCo_Wait_Phys"] == "row"
    # A struct's doc only references.
    assert status["mnY_80003000"] == "referenced"
    # A string inside a ported fn's body counts as body; its state row
    # carries the sibling callback along.
    assert status["it_80004000"] == "body"
    assert status["it_80004010"] == "row"
    assert status["memset"] == "not started"
    (site,) = inv.sites
    assert site.func.name == "ftCo_80001000"
    assert site.c_ref == "ftcommon.c:6"
    assert inv.unresolved["HSD_Unknown_Name"] == 1


def test_tsv_outputs_are_keyed_by_address(tree, tmp_path):
    decomp, crates = tree
    inv = pi.build(decomp, crates)
    pi.write_tsv(inv, tmp_path / "out")
    header = (tmp_path / "out" / "inventory.tsv").read_text().splitlines()[0].split("\t")
    assert header[:3] == ["address", "size", "name"]
    sites = (tmp_path / "out" / "fail_closed.tsv").read_text().splitlines()
    assert sites[1].startswith("80001000\tftCo_80001000\t")
