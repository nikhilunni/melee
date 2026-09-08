"""Scalar-only fake MEM1 verifies the runtime layout independently of constants."""
import io
import json
from pathlib import Path
import struct
import sys
from types import SimpleNamespace

import pytest
import yaml

import jobjdump as j
import dolphin_bones_snippet as snippet

ROOT, CHILD, SIBLING = 0x80400000, 0x80400100, 0x80400200
GOBJ, FIGHTER, AOBJ = 0x80410000, 0x80420000, 0x80430000


class FakeMemory:
    def __init__(self):
        self.data = {}
        self.reads = []

    def write_u32(self, addr, word):
        self.data.update((addr + i, b) for i, b in enumerate(word.to_bytes(4, "big")))

    def read_u8(self, addr):
        self.reads.append(("u8", addr))
        return self.data.get(addr, 0)

    def read_u32(self, addr):
        assert addr % 4 == 0
        self.reads.append(("u32", addr))
        return int.from_bytes(bytes(self.data.get(addr + i, 0) for i in range(4)), "big")

    def read_f32(self, addr):
        raise AssertionError("float reads can change NaN payload bits")


def three_joints():
    mem = FakeMemory()
    # HSD_JObj child/next at 0x10/0x08; parent at 0x0c.
    mem.write_u32(ROOT + 0x10, CHILD)
    mem.write_u32(CHILD + 0x08, SIBLING)
    mem.write_u32(CHILD + 0x0C, ROOT)
    mem.write_u32(SIBLING + 0x0C, ROOT)
    for bone, ptr in enumerate((ROOT, CHILD, SIBLING)):
        mem.write_u32(ptr + 0x14, 8 | bone)
        for offset, count in ((0x1C, 4), (0x2C, 3), (0x38, 3), (0x44, 12)):
            for component in range(count):
                mem.write_u32(ptr + offset + 4 * component, 0x3F800000 + 256 * bone + offset + component)
    mem.write_u32(CHILD + 0x7C, AOBJ)
    mem.write_u32(AOBJ, 0x20000000)  # flags must not be mistaken for curr_frame
    mem.write_u32(AOBJ + 4, 0x3FC00000)  # 1.5
    return mem


def fighter(mem):
    mem.write_u32(GOBJ, 0x00040800)  # u16 classifier=4, u8 p_link=8
    mem.write_u32(GOBJ + 0x28, ROOT)
    mem.write_u32(GOBJ + 0x2C, FIGHTER)
    mem.write_u32(FIGHTER, GOBJ)
    mem.write_u32(FIGHTER + 4, 1)  # Fox
    mem.write_u32(FIGHTER + 0x14, 2)  # Wait1 row
    mem.write_u32(FIGHTER + 0x28, 0x80450000)  # not the root
    mem.write_u32(FIGHTER + 0x894, 0x3FC00000)


def test_three_joint_preorder_and_all_words():
    mem = three_joints()
    joints = j.jobj_tree(mem, ROOT)
    assert [joint.pointer for joint in joints] == [ROOT, CHILD, SIBLING]
    assert [joint.flags for joint in joints] == [8, 9, 10]
    assert [joint.aobj_curr_frame for joint in joints] == [None, 0x3FC00000, None]
    for bone, joint in enumerate(joints):
        for field, offset, count in (("rotate", 0x1C, 4), ("scale", 0x2C, 3),
                                      ("translate", 0x38, 3), ("mtx", 0x44, 12)):
            assert getattr(joint, field) == tuple(0x3F800000 + 256 * bone + offset + i for i in range(count))
    assert {kind for kind, _ in mem.reads} == {"u32"}


def test_root_siblings_and_instance_children_are_excluded():
    mem = three_joints()
    mem.write_u32(ROOT + 8, 1)  # invalid, but not in the requested subtree
    mem.write_u32(CHILD + 0x14, 1 << 12)
    mem.write_u32(CHILD + 0x10, 1)  # INSTANCE child is an external reference
    assert [joint.pointer for joint in j.jobj_tree(mem, ROOT)] == [ROOT, CHILD, SIBLING]


def test_grandchildren_precede_parent_siblings():
    mem = three_joints()
    grandchild = ROOT + 0x300
    mem.write_u32(CHILD + 0x10, grandchild)
    mem.write_u32(grandchild + 0x0C, CHILD)
    assert [joint.pointer for joint in j.jobj_tree(mem, ROOT)] == [ROOT, CHILD, grandchild, SIBLING]


def test_fighter_and_gobj_resolve_same_root():
    mem = three_joints()
    fighter(mem)
    assert j.fighter_root_jobj(mem, GOBJ) == ROOT
    assert j.fighter_root_jobj(mem, FIGHTER) == ROOT
    assert j.fighter_root_jobj(mem, 0) == 0
    mem.write_u32(GOBJ + 0x28, 0)
    assert j.fighter_root_jobj(mem, FIGHTER) == 0
    mem.write_u32(GOBJ + 0x2C, FIGHTER + 4)
    with pytest.raises(ValueError, match="not a Fighter"):
        j.fighter_root_jobj(mem, FIGHTER)


@pytest.mark.parametrize("bad", [ROOT, CHILD, 1, 0x81800000, 0x817FFFFC])
def test_bad_links_fail_instead_of_truncating(bad):
    mem = three_joints()
    mem.write_u32(SIBLING + 8, bad)
    with pytest.raises(ValueError):
        j.jobj_tree(mem, ROOT)
    assert j.jobj_tree(mem, 0) == []


def test_scalar_records_keep_bits_and_canonical_shape():
    mem = three_joints()
    special = [0x80000000, 0x7F801234, 0x7F800000, 0xFF800000]
    for i, bits in enumerate(special):
        mem.write_u32(ROOT + 0x44 + 4 * i, bits)
    records = list(j.records(7, j.jobj_tree(mem, ROOT)))
    assert len(records) == 3 * 22
    expected_keys = [f"p0.bone[{bone}].{field}[{i}]" for bone in range(3)
                     for field, count in (("mtx", 12), ("rotate", 4), ("scale", 3), ("translate", 3))
                     for i in range(count)]
    assert [list(r["state"])[0] for r in records] == expected_keys
    assert all(r["frame"] == 7 and r["phase"] == "bones" for r in records)
    assert [next(iter(r["state"].values()))["v"]["bits"] for r in records[:4]] == special
    assert json.loads(json.dumps(records, allow_nan=False)) == records
    assert records[4]["state"]["p0.bone[0].mtx[4]"] == {
        "t": "f32", "v": {"bits": 0x3F800048, "approx": struct.unpack(">f", bytes.fromhex("3f800048"))[0]}}


def test_fighter_frame_offset_matches_generated_schema():
    schema = yaml.safe_load((Path(__file__).resolve().parents[1] / "schema/fighter.generated.yaml").read_text())
    assert schema["fields"]["cur_anim_frame"]["offset"] == j.FIGHTER_CUR_ANIM_FRAME
    assert schema["fields"]["gobj"]["offset"] == j.FIGHTER_GOBJ


def fox_world():
    mem = three_joints()
    fighter(mem)
    # Extend the child chain to 72 children plus the root, without game data.
    for bone in range(2, 72):
        mem.write_u32(ROOT + bone * 0x100 + 8, ROOT + (bone + 1) * 0x100)
    mem.write_u32(CHILD + 0x14, 1 << 6)
    return mem


def test_snippet_emits_comparison_and_separate_timing_diagnostics():
    mem = fox_world()
    out, meta = io.StringIO(), io.StringIO()
    snippet.dump_frame(mem, GOBJ, 0, out, meta)
    assert len(out.getvalue().splitlines()) == 73 * 22
    diagnostic = json.loads(meta.getvalue())
    assert diagnostic["cur_anim_frame"] == 0x3FC00000
    assert diagnostic["joints"][1]["aobj_curr_frame"] == 0x3FC00000
    assert diagnostic["dirty_bones"] == [1]
    mem.write_u32(FIGHTER + 0x14, 3)
    with pytest.raises(ValueError, match="Wait1"):
        snippet.dump_frame(mem, GOBJ, 1, out, meta)


def test_standalone_callback_loads_then_captures_and_unregisters(tmp_path, monkeypatch):
    import symbols
    mem = fox_world()
    entities = 0x80460000
    mem.write_u32(symbols.addr("HSD_GObj_Entities"), entities)
    mem.write_u32(entities + 0x20, GOBJ)
    saved, output = tmp_path / "fox.sav", tmp_path / "bones.jsonl"
    saved.touch()
    monkeypatch.setenv("MELEE_BONES_SAVESTATE", str(saved))
    monkeypatch.setenv("MELEE_BONES_OUT", str(output))
    monkeypatch.setenv("MELEE_BONES_FRAMES", "2")
    callbacks, loads = [], []
    fake = SimpleNamespace(
        memory=mem, event=SimpleNamespace(on_frameadvance=callbacks.append),
        controller=SimpleNamespace(set_gc_buttons=lambda *args: None),
        savestate=SimpleNamespace(load_from_file=loads.append),
    )
    monkeypatch.setitem(sys.modules, "dolphin", fake)
    snippet.main()
    callback = callbacks[0]
    callback()
    callback()
    assert loads == [str(saved)]
    assert len(output.read_text().splitlines()) == 2 * 73 * 22
    assert json.loads(Path(str(output) + ".done").read_text()) == {"frames": 2}
    assert callbacks[-1] is not callback
    assert callbacks[-1]() is None
