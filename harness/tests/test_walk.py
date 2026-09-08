"""Unit tests for the HSD_GObj fighter-list walk, using a dict-backed fake memory."""
from __future__ import annotations

import struct
from pathlib import Path

import yaml

import walk

HARNESS = Path(__file__).resolve().parents[1]

ENTITIES_SYM = 0x804D782C   # HSD_GObj_Entities (.sbss), symbols.txt:29535
ENTITIES_STRUCT = 0x8046B000
GOBJ_A = 0x80450000
GOBJ_B = 0x80450100
FIGHTER_A = 0x80453080
FIGHTER_B = 0x80455500


class FakeMemory:
    """Big-endian memory backed by a dict of address -> byte.

    Exposes exactly the `dolphin.memory` subset walk.py relies on: read_u32
    and read_u8 (the scripting fork has no bulk read). Reads of unwritten
    addresses return 0, like zeroed RAM.
    """

    def __init__(self) -> None:
        self.mem: dict[int, int] = {}
        self.calls: list[tuple[str, int]] = []

    def write_u32(self, addr: int, value: int) -> None:
        for i, b in enumerate(struct.pack(">I", value)):
            self.mem[addr + i] = b

    def write_bytes(self, addr: int, data: bytes) -> None:
        for i, b in enumerate(data):
            self.mem[addr + i] = b

    def read_u8(self, addr: int) -> int:
        self.calls.append(("u8", addr))
        return self.mem.get(addr, 0)

    def read_u32(self, addr: int) -> int:
        self.calls.append(("u32", addr))
        assert addr & 3 == 0, "read_u32 must be aligned"
        return int.from_bytes(bytes(self.mem.get(addr + i, 0) for i in range(4)), "big")


def build_two_fighter_world() -> FakeMemory:
    m = FakeMemory()
    m.write_u32(ENTITIES_SYM, ENTITIES_STRUCT)
    m.write_u32(ENTITIES_STRUCT + walk.GOBJLIST_FIGHTERS_OFF, GOBJ_A)
    # GObj A -> GObj B -> NULL
    m.write_u32(GOBJ_A + walk.GOBJ_NEXT_OFF, GOBJ_B)
    m.write_u32(GOBJ_A + walk.GOBJ_USER_DATA_OFF, FIGHTER_A)
    m.write_u32(GOBJ_B + walk.GOBJ_NEXT_OFF, 0)
    m.write_u32(GOBJ_B + walk.GOBJ_USER_DATA_OFF, FIGHTER_B)
    return m


def test_two_fighters():
    m = build_two_fighter_world()
    assert walk.fighter_gobjs(m, ENTITIES_SYM) == [GOBJ_A, GOBJ_B]
    assert walk.fighter_bases(m, ENTITIES_SYM) == [FIGHTER_A, FIGHTER_B]


def test_null_entities_pointer():
    m = FakeMemory()
    assert walk.fighter_bases(m, ENTITIES_SYM) == []


def test_empty_list():
    m = FakeMemory()
    m.write_u32(ENTITIES_SYM, ENTITIES_STRUCT)
    assert walk.fighter_bases(m, ENTITIES_SYM) == []


def test_skips_null_user_data():
    m = build_two_fighter_world()
    m.write_u32(GOBJ_A + walk.GOBJ_USER_DATA_OFF, 0)
    assert walk.fighter_bases(m, ENTITIES_SYM) == [FIGHTER_B]


def test_cycle_is_cut():
    m = build_two_fighter_world()
    m.write_u32(GOBJ_B + walk.GOBJ_NEXT_OFF, GOBJ_A)  # B -> A loop
    assert walk.fighter_bases(m, ENTITIES_SYM) == [FIGHTER_A, FIGHTER_B]


def test_cap_applies():
    m = FakeMemory()
    m.write_u32(ENTITIES_SYM, ENTITIES_STRUCT)
    n = walk.MAX_FIGHTERS + 4
    gobjs = [0x80440000 + i * walk.GOBJ_SIZE for i in range(n)]
    m.write_u32(ENTITIES_STRUCT + walk.GOBJLIST_FIGHTERS_OFF, gobjs[0])
    for i, g in enumerate(gobjs):
        m.write_u32(g + walk.GOBJ_NEXT_OFF, gobjs[i + 1] if i + 1 < n else 0)
        m.write_u32(g + walk.GOBJ_USER_DATA_OFF, 0x80460000 + i * 0x2400)
    assert len(walk.fighter_bases(m, ENTITIES_SYM)) == walk.MAX_FIGHTERS


def test_read_bytes_aligned_uses_u32_chunks():
    m = FakeMemory()
    data = bytes((i * 7 + 3) & 0xFF for i in range(0x23EC))
    m.write_bytes(FIGHTER_A, data)
    assert walk.read_bytes(m, FIGHTER_A, 0x23EC) == data
    kinds = {k for k, _ in m.calls}
    assert kinds == {"u32"}  # 0x23EC is a multiple of 4 and the base is aligned


def test_read_bytes_unaligned_head_and_tail():
    m = FakeMemory()
    data = bytes(range(0x20, 0x20 + 10))
    m.write_bytes(0x80450001, data)
    assert walk.read_bytes(m, 0x80450001, 10) == data  # 0x80450001 .. 0x8045000A
    u8s = [a for k, a in m.calls if k == "u8"]
    u32s = [a for k, a in m.calls if k == "u32"]
    assert u8s == [0x80450001, 0x80450002, 0x80450003, 0x80450008, 0x80450009, 0x8045000A]
    assert u32s == [0x80450004]
    assert walk.read_bytes(m, 0x80450001, 0) == b""


def test_offsets_match_globals_schema():
    g = yaml.safe_load((HARNESS / "schema" / "globals.yaml").read_text())
    gobj = g["structs"]["HSD_GObj"]["fields"]
    lst = g["structs"]["HSD_GObjList"]["fields"]
    assert gobj["next"]["offset"] == walk.GOBJ_NEXT_OFF
    assert gobj["prev"]["offset"] == walk.GOBJ_PREV_OFF
    assert gobj["user_data"]["offset"] == walk.GOBJ_USER_DATA_OFF
    assert g["structs"]["HSD_GObj"]["size"] == walk.GOBJ_SIZE
    assert lst["fighters"]["offset"] == walk.GOBJLIST_FIGHTERS_OFF
    assert lst["items"]["offset"] == walk.GOBJLIST_ITEMS_OFF


def test_trace_scenario_imports_without_dolphin():
    import trace_scenario

    assert trace_scenario.memory is None
    assert trace_scenario.ENTITIES_ADDR == ENTITIES_SYM
    m = build_two_fighter_world()
    assert trace_scenario.fighter_bases(m) == [FIGHTER_A, FIGHTER_B]
    rec = trace_scenario.Tracer({"frames": 1}, None).record("frame_end", m)
    assert [f["base"] for f in rec["fighters"]] == ["0x80453080", "0x80455500"]
    assert len(rec["fighters"][0]["bytes"]) == trace_scenario.FIGHTER_SIZE * 2


def test_inputs_reissued_every_frame():
    import trace_scenario

    class FakeController:
        def __init__(self):
            self.log = []

        def set_gc_buttons(self, port, buttons):
            self.log.append((port, dict(buttons)))

    ctl = FakeController()
    scenario = {"frames": 5, "inputs": [{"frame": 1, "buttons": {"A": True}},
                                        {"frame": 3, "buttons": {}}]}
    t = trace_scenario.Tracer(scenario, None)
    for f in range(5):
        t.apply_inputs(f, ctl)
    # set_gc_buttons lasts one frame in the scripting fork, so the held state
    # must be sent on every frame, not only on the frame the step appears.
    assert ctl.log == [(0, {}), (0, {"A": True}), (0, {"A": True}), (0, {}), (0, {})]
