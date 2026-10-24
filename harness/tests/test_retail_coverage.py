"""Retail coverage: Branch Watch dumps mapped onto retail functions, the
determinism check, scenario selection, and the tracer's start/stop window.
Never launches Dolphin."""
import json
import struct
import tomllib

import coverage_trace
import pytest
import tick_trace as tick
from test_tick_trace import Events, store
from test_walk import _FakeController, _FakeStates, build_two_fighter_world

import retail_coverage as rc

SYMBOLS = """\
__start = .init:0x80003100; // type:function size:0x20 scope:global
caller = .text:0x80005000; // type:function size:0x20 scope:global
leaf = .text:0x80005020; // type:function size:0x8 scope:global
recurse = .text:0x80005028; // type:function size:0x10 scope:global
untouched = .text:0x80005038; // type:function size:0x8 scope:global
seed = .sdata:0x804D5F90; // type:object size:0x4 scope:global
"""
SPLITS = """\
Sections:
	.init       type:code align:4

melee/a.c:
	.text       start:0x80005000 end:0x80005028

melee/b.c:
	.text       start:0x80005028 end:0x80005040
	.data       start:0x80400000 end:0x80400010
"""
BL, B, BLR, BEQ, BNE_LR, NOP = 0x48000001, 0x48000000, 0x4E800020, 0x41820008, 0x4C820020, 0x60000000


def layout() -> rc.Layout:
    # caller: bl leaf; beq +8; nop...; blr | leaf: nop; blr | recurse: bl recurse; bnelr; nop; blr
    words = {0x80005000: BL, 0x80005004: BEQ, 0x8000501C: BLR, 0x80005024: BLR,
             0x80005028: BL, 0x8000502C: BNE_LR, 0x80005034: BLR, 0x8000503C: BLR}
    blob = b"".join(struct.pack(">I", words.get(0x80005000 + 4 * i, NOP)) for i in range(16))
    return rc.Layout(rc.parse_functions(SYMBOLS), rc.parse_units(SPLITS), [(0x80005000, blob)])


def test_functions_are_the_code_symbols_with_unit_and_branch_sites():
    lay = layout()
    rows = [(f.name, f.unit, f.sites, f.cond_sites) for f in lay.functions]
    assert rows == [("__start", "?", 0, 0), ("caller", "melee/a.c", 3, 1), ("leaf", "melee/a.c", 1, 0),
                    ("recurse", "melee/b.c", 3, 1), ("untouched", "melee/b.c", 1, 0)]
    assert lay.containing(0x80005024).name == "leaf"
    assert lay.containing(0x80005040) is None


@pytest.mark.parametrize("word, kind", [
    (BL, "uncond"), (B, "uncond"), (BLR, "uncond"), (0x4E800420, "uncond"),  # bctr
    (BEQ, "cond"), (BNE_LR, "cond"), (0x42000010, "cond"),                   # bdnz
    (NOP, None), (0x7C0802A6, None),                                         # mflr
])
def test_branch_kinds(word, kind):
    assert rc.branch_kind(word) == kind


def test_branch_watch_lines_parse_with_the_taken_flag():
    text = "80005000 80005020 48000001 5 0 3\n80005004 80005008 41820008 2 0 1\nbad line\n"
    assert rc.parse_branch_watch(text) == [
        rc.Branch(0x80005000, 0x80005020, BL, 5, True),
        rc.Branch(0x80005004, 0x80005008, BEQ, 2, False)]


def test_entries_count_calls_and_outside_jumps_but_not_loops():
    lay = layout()
    hits = rc.function_hits(lay, [
        rc.Branch(0x80005000, 0x80005020, BL, 5, True),       # caller -> leaf, 5 calls
        rc.Branch(0x80005024, 0x80005004, BLR, 5, True),      # leaf returns
        rc.Branch(0x80005004, 0x80005008, BEQ, 3, False),     # caller's beq not taken
        rc.Branch(0x80005004, 0x8000500C, BEQ, 2, True),      # ... and taken
        rc.Branch(0x80005028, 0x80005028, BL, 4, True),       # recursion: a call to itself
        rc.Branch(0x8000502C, 0x80005030, BNE_LR, 0, True),   # zero hits: ignored
    ])
    assert sorted(hits) == [0x80005000, 0x80005020, 0x80005028]
    assert hits[0x80005020].entries == 5 and hits[0x80005020].branch_execs == 5
    caller = hits[0x80005000]
    assert caller.entries == 0 and caller.branch_execs == 10 and caller.both_ways == 1
    assert caller.sites == {0x80005000, 0x80005004}
    assert hits[0x80005028].entries == 4


def test_function_tables_round_trip(tmp_path):
    lay = layout()
    hits = rc.function_hits(lay, [rc.Branch(0x80005000, 0x80005020, BL, 5, True)])
    rc.write_function_table(tmp_path / "t.tsv", lay, hits)
    assert rc.read_function_table(tmp_path / "t.tsv") == {0x80005000: (0, 5, 1, 0), 0x80005020: (5, 0, 0, 0)}


def test_sites_merge_to_both_ways(tmp_path):
    union = {}
    rc.merge_sites(union, {1: "T", 2: "N", 3: "B"})
    rc.merge_sites(union, {1: "N", 2: "N", 4: "T"})
    assert union == {1: "B", 2: "N", 3: "B", 4: "T"}
    hits = {0: rc.FunctionHits(sites={8, 12}, taken={8, 12}, not_taken={12})}
    rc.write_sites(tmp_path / "s.txt", hits)
    assert rc.read_sites(tmp_path / "s.txt") == {8: "T", 12: "B"}


def rec(frame, seed, vi):
    return json.dumps({"frame": frame, "phase": "frame_end", "seed": seed, "fighters": [],
                       "tick": frame, "vi_frame": vi, "ps_frame": 1}) + "\n"


def test_traces_compare_without_host_timing():
    recorded = [rec(0, 5, 1), rec(1, 6, 2)]
    assert rc.compare_traces(recorded, [rec(0, 5, 7), rec(1, 6, 9)]) == "identical (2 records)"
    assert rc.compare_traces(recorded, [rec(0, 5, 1), rec(1, 7, 2)]) == "DIFFERS at record 1: seed"
    assert rc.compare_traces(recorded, recorded[:1]).startswith("DIFFERS in length")
    # A key only the newer tracer writes is not a difference.
    newer = [line.replace('"ps_frame": 1', '"ps_frame": 1, "pad_queue_sticks": []') for line in recorded]
    assert rc.compare_traces(recorded, newer) == "identical (2 records)"


def test_m5_names_read_every_list_once():
    text = """\
const A: [(&str, usize); 2] = [
    // a comment ("not_this", 3)
    ("first", 10),
    ("second", 20),
];
static B: &[(&str, usize)] = &[
    ("second", 20),
    ("slp_x_cold", 5),
];
const C: [&str; 1] = [
    "bare",
];
fn f() { let _ = ("not_a_list", 1); }
"""
    assert rc.m5_names(text) == ["first", "second", "slp_x_cold", "bare"]


def test_cold_twins_resolve_to_their_recorded_bridge(tmp_path):
    for name in ("plain", "slp_x", "slp_x_cold"):
        (rc.data_root.SCENARIOS / f"{name}.toml").write_text(f'name = "{name}"\n')
    for name in ("plain", "slp_x"):
        (rc.data_root.TRACES / f"{name}.tick.raw.jsonl").write_text("")
    found, missing = rc.scenario_paths(["plain", "slp_x_cold", "slp_x", "gone"])
    assert [p.stem for p in found] == ["plain", "slp_x"] and missing == ["gone"]


def test_scope_scenarios_are_cpu_only_tick_scenes(tmp_path):
    setup = rc.SCOPE_SETUPS[1]
    scenario = tomllib.loads(rc.scope_scenario(setup, tmp_path / "s.sav"))
    assert scenario["name"] == setup.name and scenario["frames"] == setup.ticks
    assert [f["controller"] for f in scenario["fighters"]] == ["cpu"] * 4
    assert "inputs" not in scenario


class FakeEmulation:
    def __init__(self):
        self.calls = []

    def __getattr__(self, name):
        return lambda *args: self.calls.append((name, *args))


def test_branch_watch_covers_exactly_the_recorded_ticks(tmp_path):
    mem = build_two_fighter_world()
    for addr, word in tick.BOUNDARY_CODE.items():
        mem.write_u32(addr, word)
    mem.write_u32(tick.WATCH_ADDR, 100)
    mem.add_memcheck = mem.remove_memcheck = lambda addr: None
    emu = FakeEmulation()
    out = tmp_path / "branches.txt"
    tracer = coverage_trace.CoverageTracer(
        {"frames": 2}, (tmp_path / "raw.jsonl").open("w"), tmp_path / "m.sav", {"seed": 123},
        tmp_path / "raw.jsonl.done", mem, _FakeController(), _FakeStates(mem, 123), Events(),
        emu=emu, branch_out=str(out))
    tracer.on_frame()
    assert [c[0] for c in emu.calls] == ["branch_watch_clear", "branch_watch_start"]
    store(tracer, mem, 101)
    assert len(emu.calls) == 2
    store(tracer, mem, 102)  # the last record: stop inside its callback
    assert emu.calls[-1] == ("branch_watch_stop",)
    tracer.on_frame()        # finish: save before .done
    assert emu.calls[-1] == ("branch_watch_save", str(out))
    assert [c[0] for c in emu.calls].count("branch_watch_stop") == 1
    assert json.loads((tmp_path / "raw.jsonl.done").read_text())["branch_watch_ticks"] == 2
