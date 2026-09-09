"""Independent literal Gekko offsets; fake memory cannot read host floats."""
import json
from pathlib import Path
from types import SimpleNamespace
import sys

import pytest

import particle_dump as dump
import dolphin_particle_snippet as snippet
import symbols
import tick_trace as tick
from test_jobjdump import FakeMemory
from test_tick_trace import Events, store
from test_walk import _FakeController, _FakeStates, build_two_fighter_world

GEN0, GEN1 = 0x80400000, 0x80400100
PART0, PART1, PART2 = 0x80401000, 0x80401100, 0x80401200
TABLE, DESC, APP, PENDING = 0x80402000, 0x80402100, 0x80403000, 0x80404000


def put16(mem, address, value):
    mem.data[address] = value >> 8
    mem.data[address + 1] = value & 255


def world():
    mem = FakeMemory()
    mem.write_u32(symbols.addr("hsd_804D78FC"), GEN0)
    mem.write_u32(GEN0, GEN1)
    mem.write_u32(symbols.addr("hsd_804D0908") + 4, PART0)
    mem.write_u32(PART0, PART1)
    mem.write_u32(symbols.addr("hsd_804D0908") + 4 * 15, PART2)
    mem.write_u32(symbols.addr("psCmdListArray"), 1)
    mem.write_u32(symbols.addr("ptclref_804D0E5C"), TABLE)
    mem.write_u32(TABLE - 8, 0)
    mem.write_u32(TABLE - 4, 1)
    mem.write_u32(TABLE, DESC)
    for pointer in (GEN0, GEN1, PART0, PART1, PART2):
        mem.write_u32(pointer + 0x20, DESC + 0x3C)
    for pointer, link in ((PART0, 1), (PART1, 1), (PART2, 15)):
        mem.data[pointer + 0x1D] = link
    mem.write_u32(PART0 + 0x88, GEN1)
    mem.write_u32(PART1 + 0x88, GEN0)
    mem.write_u32(PART0 + 0x8C, APP)
    mem.write_u32(GEN1 + 0x54, APP)
    mem.write_u32(APP + 4, GEN1)
    mem.write_u32(symbols.addr("hsd_804D78F4"), PENDING)
    mem.write_u32(PENDING + 4, GEN1)
    return mem


def test_lists_retain_global_order_and_optional_generator_references():
    captured = dump.snapshot(world())
    assert [g["pointer"] for g in captured["generators"]] == [GEN0, GEN1]
    assert len(captured["particle_lists"]) == 16
    assert [p["pointer"] for p in captured["particle_lists"][1]] == [PART0, PART1]
    assert captured["particle_lists"][15][0]["pointer"] == PART2
    assert len(captured["appsrts"]) == 1
    state = dump.record(7, captured)["state"]
    assert state["particles.link[1].particle[0].generator_index"] == {"t": "u", "v": 1}
    assert state["particles.link[1].particle[1].generator_index"] == {"t": "u", "v": 0}
    assert state["particles.link[15].particle[0].generator_index"] == {"t": "null"}
    assert state["particles.pending[0].generator_index"] == {"t": "u", "v": 1}
    assert state["particles.generator[1].appsrt_index"] == {"t": "u", "v": 0}
    assert state["particles.generator[0].program_kind"] == {"t": "u", "v": 0}
    assert all("pointer" not in path for path in state)


def test_lifetime_pc_counters_color_and_float_bits_at_literal_offsets():
    mem = world()
    expected = {"command_wait": 321, "pc": 0x1234, "mark_pc": 0x5678, "loop_pc": 0x9ABC,
                "life": 600, "rotation_count": 4, "primary_remaining": 8, "alpha_compare_remaining": 9}
    for offset, value in zip((0x1A, 0x24, 0x26, 0x28, 0x2A, 0x5E, 0x68, 0x78), expected.values()):
        put16(mem, PART0 + offset, value)
    mem.data[PART0 + 0x1C] = 17
    mem.data[PART0 + 0x12] = 211
    special = [0x80000000, 0x7F801234, 0x7F800000]
    for index, bits in enumerate(special):
        mem.write_u32(PART0 + 0x2C + index * 4, bits)
    captured = dump.snapshot(mem)
    fields = captured["particle_lists"][1][0]["fields"]
    assert {name: fields[name] for name in expected} == expected
    assert fields["loop_count"] == 17 and fields["primary_color"][0] == 211
    assert fields["velocity"] == special
    row = dump.record(2, captured)
    assert row["frame"] == 2 and row["phase"] == "particles"
    values = [row["state"][f"particles.link[1].particle[0].velocity[{i}]"]["v"]["bits"] for i in range(3)]
    assert values == special
    assert json.loads(json.dumps(row, allow_nan=False)) == row
    assert len(captured["generators"][0]["raw_hex"]) == 2 * 0x94
    assert len(captured["particle_lists"][1][0]["raw_hex"]) == 2 * 0x98


def test_rect_aux_flag_and_shared_app_matrix_have_correct_offsets():
    mem = world()
    put16(mem, GEN0 + 0x16, 5)
    mem.write_u32(GEN0 + 0x60, 0x3F800000)
    mem.write_u32(GEN0 + 0x8C, 0x40000000)
    put16(mem, GEN0 + 0x90, 7)
    mem.write_u32(APP + 0x34 + 11 * 4, 0x40400000)
    captured = dump.snapshot(mem)
    fields = captured["generators"][0]["fields"]
    assert fields["aux.x"] == 0x3F800000
    assert fields["aux.zz"] == 0x40000000 and fields["aux.flags"] == 7
    assert captured["appsrts"][0]["fields"]["matrix"][11] == 0x40400000


@pytest.mark.parametrize("target", [GEN0, 1, 0x81800000, 0x817FFFFC])
def test_invalid_generator_links_and_cycles_fail(target):
    mem = world()
    mem.write_u32(GEN1, target)
    with pytest.raises(ValueError):
        dump.snapshot(mem)


def test_shared_particle_node_and_wrong_link_fail():
    mem = world()
    mem.write_u32(PART1, PART0)
    with pytest.raises(ValueError, match="cyclic, shared"):
        dump.snapshot(mem)
    mem.write_u32(PART1, 0)
    mem.data[PART1 + 0x1D] = 7
    with pytest.raises(ValueError, match="list 1 contains link 7"):
        dump.snapshot(mem)


def test_unknown_program_is_explicitly_unresolved_and_invalid_bank_fails():
    mem = world()
    mem.write_u32(GEN0 + 0x20, DESC + 0x3D)
    state = dump.record(0, dump.snapshot(mem))["state"]
    assert state["particles.generator[0].program_kind"] == {"t": "null"}
    assert state["particles.generator[0].program_resolved"] == {"t": "u", "v": 0}
    mem.data[GEN0 + 0x18] = 65
    with pytest.raises(ValueError, match="invalid particle bank"):
        dump.snapshot(mem)


def test_empty_lists_still_emit_seed_and_all_counts():
    mem = FakeMemory()
    mem.write_u32(symbols.addr("seed"), 1234)
    state = dump.record(0, dump.snapshot(mem))["state"]
    assert state["rng.seed"] == {"t": "u", "v": 1234}
    assert len(state) == 20
    with pytest.raises(ValueError, match="frame must fit"):
        dump.record(-1, dump.snapshot(mem))


def test_tick_subclass_loads_initial_then_samples_only_scheduler_boundaries(tmp_path):
    mem = build_two_fighter_world()
    for address, value in world().data.items():
        mem.write_bytes(address, bytes([value]))
    for address, word in tick.BOUNDARY_CODE.items():
        mem.write_u32(address, word)
    mem.write_u32(tick.WATCH_ADDR, 100)
    calls = []
    mem.add_memcheck = lambda address: calls.append(("add", address))
    mem.remove_memcheck = lambda address: calls.append(("remove", address))
    output, metadata, initial = (tmp_path / name for name in ("particles.jsonl", "metadata.jsonl", "initial.jsonl"))
    done = Path(str(output) + ".done")
    tracer = snippet.ParticleTracer(
        {"frames": 2}, metadata.open("w"), tmp_path / "idle.sav", {"seed": 123}, done,
        mem, _FakeController(), _FakeStates(mem, 123), Events(),
        particle_out=output.open("w"), initial_path=initial,
    )
    tracer.on_frame()
    assert initial.is_file() and output.read_text() == ""
    initial_row = json.loads(initial.read_text())
    assert initial_row["state"]["particles.link[1].count"]["v"] == 2
    store(tracer, mem, 101)
    store(tracer, mem, 102)
    assert not done.exists() and len(calls) == 1
    tracer.on_frame()
    assert calls[-1] == ("remove", tick.WATCH_ADDR)
    rows = [json.loads(line) for line in output.read_text().splitlines()]
    assert [row["frame"] for row in rows] == [0, 1]
    assert all(row["phase"] == "particles" for row in rows)
    assert json.loads(done.read_text())["ticks"] == 2
    diagnostics = [json.loads(line) for line in metadata.read_text().splitlines()]
    assert [row["tick"] for row in diagnostics] == [101, 102]
    assert diagnostics[0]["particles"]["generators"][0]["pointer"] == GEN0


def test_startup_failure_clears_stale_markers(tmp_path, monkeypatch):
    output = tmp_path / "particles.jsonl"
    done = Path(str(output) + ".done")
    done.write_text("stale success")
    monkeypatch.setenv("MELEE_PARTICLES_OUT", str(output))
    monkeypatch.setenv("MELEE_PARTICLES_SAVESTATE", str(tmp_path / "missing.sav"))
    monkeypatch.setitem(sys.modules, "dolphin", SimpleNamespace(
        controller=None, event=None, memory=None, savestate=None))
    snippet.main()
    assert not done.exists()
    assert "missing.sav" in Path(str(output) + ".err").read_text()


def test_shifted_v42_bank_never_dereferences_entries_below_first_id():
    mem = world()
    header = 0x80500000
    first_id, count = 30000, 5
    biased_table = header + 12 - first_id * 4
    mem.write_u32(symbols.addr("psCmdListArray"), first_id + count)
    mem.write_u32(symbols.addr("ptclref_804D0E5C"), biased_table)
    mem.write_u32(header, 0x00420000)
    mem.write_u32(header + 4, first_id)
    mem.write_u32(header + 8, count)
    for index in range(count):
        mem.write_u32(header + 12 + index * 4, DESC + index * 0x100)
    # Values beneath the actual table are not pointers, and must never be
    # interpreted as descriptor addresses. FD's ID bias is exactly 30000.
    mem.write_u32(biased_table, 1)
    mem.write_u32(header - 4, 0xFFFFFFFF)
    expected = {DESC + index * 0x100 + 0x3C: first_id + index for index in range(count)}
    assert dump.program_indices(mem, 0) == expected
    assert ("u32", biased_table) not in mem.reads
    captured = dump.snapshot(mem)
    assert captured["generators"][0]["program_kind"] == 30000
    assert captured["particle_lists"][1][0]["program_kind"] == 30000


def test_bank_header_recovery_rejects_inconsistent_metadata():
    mem = world()
    mem.write_u32(TABLE - 4, 2)
    with pytest.raises(ValueError, match="cannot recover particle bank"):
        dump.program_indices(mem, 0)


def test_family_id_counter_is_captured_with_its_u16_width():
    mem = world()
    put16(mem, symbols.addr("lbl_804D6368"), 0xFFFF)
    put16(mem, symbols.addr("lbl_804D6368") + 2, 0x1234)
    captured = dump.snapshot(mem)
    assert captured["globals"]["lbl_804D6368"] == 0xFFFF
    assert dump.record(0, captured)["state"]["particles.family_id_counter"] == {"t": "u", "v": 0xFFFF}
