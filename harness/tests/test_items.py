"""Item oracle fixtures: no Dolphin process or game-data writes."""
import copy
import json
import struct

import pytest

import decode
import gen_item_kinds
from item_kinds import ITEM_KIND_NAMES
import trace_common
import validate_ticks
from test_tick_trace import capture, store  # noqa: F401 (pytest fixture)
from test_validate_ticks import record
from test_walk import (ENTITIES_STRUCT, ENTITIES_SYM, FIGHTER_A, FIGHTER_B,
                       GOBJ_A, GOBJ_B, FakeMemory)

ITEM_GOBJ = 0x80500000
ITEM_BASE = 0x80600000
FOX_LASER = 54  # it/forward.h ItemKind, after Mario fire (48)


def item_image(gobj=ITEM_GOBJ, spawn_id=123, x=10.0, prev_x=8.0, owner=GOBJ_A):
    raw = bytearray(0xFCC)
    for off, value in [(0x04, gobj), (0x0C, 3), (0x10, FOX_LASER), (0x1C, spawn_id),
                       (0x24, 1), (0x28, 2), (0xC0, 1), (0x518, owner),
                       (0x5D4, 1), (0x84C, 3), (0xD18, 0x8029C9CC), (0xD1C, 0x8029C9EC)]:
        struct.pack_into(">I", raw, off, value)
    for off, value in [(0x2C, -1.0), (0x40, 2.0), (0x4C, x), (0x50, 5.0),
                       (0x388, 7.0), (0x38C, 4.0), (0x5E0, 3.5), (0xD44, 90.0),
                       (0xDE0, prev_x), (0xDE4, 5.0)]:
        struct.pack_into(">f", raw, off, value)
    raw[-4:] = b"\x12\x34\x56\x78"  # complete per-kind union tail survives capture
    return bytes(raw)


def item_row(**kwargs):
    raw = item_image(**kwargs)
    gobj = kwargs.get("gobj", ITEM_GOBJ)
    return {"gobj": f"0x{gobj:08X}", "base": f"0x{ITEM_BASE:08X}",
            "kind": FOX_LASER, "kind_name": "It_Kind_Fox_Laser", "owner": 0,
            "bytes": raw.hex(), "state": decode.decode_item(raw)}


def item_trace():
    rows = [record(i, i) for i in range(4)]
    for i, row in enumerate(rows):
        row.update(frame=i, tick=100+i, vi_frame=i, watch_value=99+i,
                   watch_address=0x80479D58, items=[])
    rows[1]["items"] = [item_row()]
    rows[2]["items"] = [item_row(x=12.0, prev_x=10.0)]
    return rows


def add_items(mem, count):
    gobjs = [ITEM_GOBJ + i * 0x40 for i in range(count)]
    mem.write_u32(ENTITIES_SYM, ENTITIES_STRUCT)
    mem.write_u32(ENTITIES_STRUCT + 0x24, gobjs[0] if gobjs else 0)
    expected = []
    for i, gobj in enumerate(gobjs):
        base = ITEM_BASE + i * 0x1000
        raw = item_image(gobj=gobj, spawn_id=i)
        mem.write_u32(gobj + 0x08, gobjs[i+1] if i+1 < count else 0)
        mem.write_u32(gobj + 0x2C, base)
        mem.write_bytes(base, raw)
        expected.append((gobj, base, raw))
    return expected


@pytest.mark.parametrize("count", [0, 1, 2, 128])
def test_read_items_preserves_full_bytes_and_retail_order(count):
    mem = FakeMemory()
    expected = add_items(mem, count)
    assert trace_common.read_items(mem) == expected
    if count == 0:
        assert mem.calls == [("u32", ENTITIES_SYM), ("u32", ENTITIES_STRUCT + 0x24)]


def test_item_order_is_link_order_not_address_order():
    mem = FakeMemory()
    a, b = add_items(mem, 2)
    mem.write_u32(ENTITIES_STRUCT + 0x24, b[0])
    mem.write_u32(b[0] + 8, a[0])
    mem.write_u32(a[0] + 8, 0)
    assert trace_common.read_items(mem) == [b, a]


def test_null_entities_and_unconstructed_item():
    assert trace_common.read_items(FakeMemory()) == []
    mem = FakeMemory()
    _, b = add_items(mem, 2)
    mem.write_u32(ITEM_GOBJ + 0x2C, 0)
    assert trace_common.read_items(mem) == [b]


@pytest.mark.parametrize("offset,value", [(8, ITEM_GOBJ), (8, 0x1234), (0x2C, 0x817FFFFC)])
def test_corrupt_item_lists_fail_instead_of_truncating(offset, value):
    mem = FakeMemory()
    add_items(mem, 1)
    mem.write_u32(ITEM_GOBJ + offset, value)
    with pytest.raises(ValueError, match="cycle|invalid item-list pointer"):
        trace_common.read_items(mem)


def test_generated_kind_table_covers_aliases_last_kind_and_unknown():
    assert gen_item_kinds.OUTPUT.read_text() == gen_item_kinds.generate(gen_item_kinds.SOURCE.read_text())
    assert ITEM_KIND_NAMES[FOX_LASER] == "It_Kind_Fox_Laser"
    assert ITEM_KIND_NAMES[0xA1] == "It_PKind_Tosakinto"
    assert ITEM_KIND_NAMES[0xBF] == "It_Kind_Chicorita_Leaf"
    assert ITEM_KIND_NAMES[236] == "It_Kind_Kyasarin_Egg"
    assert ITEM_KIND_NAMES[-999] == "It_Kind_None"


def test_decode_hand_built_item_image():
    state = decode.decode_item(item_image())
    for key, expected in {"entity": ITEM_GOBJ, "kind": FOX_LASER, "spawn_kind": 3,
                          "spawn_id": 123, "motion_id": 1, "anim_id": 2,
                          "owner": GOBJ_A, "hitbox_count": 2}.items():
        assert state[key]["v"] == expected
    for key, bits in {"facing_dir": 0xBF800000, "pos.x": 0x41200000,
                      "pos.y": 0x40A00000, "vel.x": 0x40000000,
                      "prev_pos.x": 0x40E00000, "laser.prev_pos.x": 0x41000000,
                      "hitbox0.damage": 0x40600000, "life_timer": 0x42B40000}.items():
        assert state[key]["t"] == "f32" and state[key]["v"]["bits"] == bits


@pytest.mark.parametrize("size", [0, 0xFCB, 0xFCD])
def test_decode_rejects_incomplete_or_oversized_items(size):
    with pytest.raises(ValueError, match="Item image must be"):
        decode.decode_item(bytes(size))


def test_nonlaser_union_is_not_mislabeled_as_laser_position():
    raw = bytearray(item_image())
    struct.pack_into(">i", raw, 0x10, -999)
    assert "laser.prev_pos.x" not in decode.decode_item(raw)


def test_tick_capture_items_owner_slot_changes_and_full_decode(capture, tmp_path):
    tracer, mem, raw, _, _ = capture
    # List positions 0/1 deliberately differ from player slots 2/5.
    mem.write_u32(FIGHTER_A, GOBJ_A)
    mem.write_bytes(FIGHTER_A + 0xC, b"\x02")
    mem.write_u32(FIGHTER_B, GOBJ_B)
    mem.write_bytes(FIGHTER_B + 0xC, b"\x05")
    add_items(mem, 2)
    mem.write_u32(ITEM_BASE + 0x1000 + 0x518, 0)
    tracer.on_frame()
    store(tracer, mem, 101)
    mem.write_u32(ITEM_BASE + 0x518, GOBJ_B)
    mem.write_u32(ITEM_BASE + 0x1000 + 0x518, ITEM_GOBJ)  # item owner, not fighter
    mem.write_u32(ITEM_BASE + 0x1000 + 0x10, 9999)
    store(tracer, mem, 102)
    mem.write_u32(ENTITIES_STRUCT + 0x24, 0)
    store(tracer, mem, 103)
    tracer.on_frame()
    records = [json.loads(line) for line in raw.read_text().splitlines()]
    assert [[item["owner"] for item in row["items"]] for row in records] == [[2, None], [5, None], []]
    assert records[1]["items"][1]["kind_name"] == "?"
    assert records[0]["items"][0]["bytes"].endswith("12345678")
    decoded = tmp_path / "decoded.jsonl"
    decode.main(raw, decoded)
    rows = [json.loads(line) for line in decoded.read_text().splitlines()]
    assert all(len(row["state"]) == 49 for row in rows)
    assert rows[0]["items"][0]["state"]["owner"]["v"] == GOBJ_A
    assert rows[0]["items"][0]["bytes"] == records[0]["items"][0]["bytes"]
    assert rows[-1]["items"] == []


def test_validator_reports_spawn_despawn_and_checks_velocity(tmp_path, capsys):
    rows = item_trace()
    result = validate_ticks.validate(rows)
    assert not result["violations"]
    assert [(e["event"], e["tick"]) for e in result["item_events"]] == [("spawn", 101), ("despawn", 103)]
    assert result["item_motion_checked"] == 1
    path = tmp_path / "trace.jsonl"
    path.write_text("\n".join(map(json.dumps, rows)))
    assert validate_ticks.main([str(path), "--scripted", "--items"]) == 0
    output = capsys.readouterr().out
    assert "item spawn tick 101" in output and "item despawn tick 103" in output
    assert "motion checked=1, skipped=0" in output


@pytest.mark.parametrize("key,value,message", [
    ("pos.x", 99.0, "does not match velocity"),
    ("laser.prev_pos.x", 9.0, "is not previous tick position"),
    ("pos.y", float("nan"), "is not finite"),
])
def test_validator_detects_broken_motion_using_bits(key, value, message):
    rows = item_trace()
    rows[2]["items"][0]["state"][key] = decode._val("f32", struct.pack(">f", value))
    assert any(message in error for error in validate_ticks.validate(rows)["violations"])


def test_validator_ignores_approximate_float_metadata():
    rows = item_trace()
    rows[2]["items"][0]["state"]["vel.x"]["v"]["approx"] = -999
    result = validate_ticks.validate(rows)
    assert not result["violations"] and result["item_motion_checked"] == 1


@pytest.mark.parametrize("key,value", [("flags", 1 << (31-9)), ("flags", 1 << (31-0x13)),
                                      ("owner", GOBJ_B), ("kind", 0),
                                      ("env_flags", 0x8000),
                                      ("physics_callback", 0), ("collision_callback", 0)])
def test_affected_or_unaudited_motion_is_reported_as_skipped(key, value):
    rows = item_trace()
    after = rows[2]["items"][0]
    after["state"][key]["v"] = value
    if key == "kind":
        for row in rows[1:3]:
            row["items"][0]["kind"] = value
            row["items"][0]["state"][key]["v"] = value
    after["state"]["pos.x"] = decode._val("f32", struct.pack(">f", 99.0))
    result = validate_ticks.validate(rows)
    assert not result["violations"] and result["item_motion_skipped"] == 1


def test_surviving_spawn_id_cannot_move_to_a_new_gobj():
    rows = item_trace()
    rows[2]["items"][0] = item_row(gobj=ITEM_GOBJ + 0x40)
    assert any("live item changed" in e for e in validate_ticks.validate(rows)["violations"])


def test_reused_gobj_is_a_new_generation():
    rows = item_trace()
    rows[2]["items"][0] = item_row(spawn_id=124)
    result = validate_ticks.validate(rows)
    assert not result["violations"]
    assert [(e["event"], e["tick"], e["spawn_id"]) for e in result["item_events"]] == [
        ("spawn", 101, 123), ("despawn", 102, 123), ("spawn", 102, 124), ("despawn", 103, 124)]


def test_duplicate_identity_or_disappearing_capture_is_invalid():
    rows = item_trace()
    rows[1]["items"].append(copy.deepcopy(rows[1]["items"][0]))
    del rows[3]["items"]
    errors = validate_ticks.validate(rows)["violations"]
    assert any("duplicate item identity" in e for e in errors)
    assert any("items field disappeared" in e for e in errors)


def test_initial_items_are_marked_as_already_present():
    rows = item_trace()[1:3]
    result = validate_ticks.validate(rows)
    assert result["item_events"][0]["initial"]
    assert len(result["item_events"]) == 1  # capture end is not a despawn


def test_empty_tick_preserves_legacy_record_bytes(tmp_path):
    import tick_trace
    from test_walk import build_two_fighter_world

    mem = build_two_fighter_world()
    old = trace_common.Tracer({"frames": 1}, None, mem=mem).record("frame_end")
    new = tick_trace.TickTracer({"frames": 1}, None, tmp_path / "unused.sav", mem=mem).record("frame_end")
    assert new.pop("items") == []
    assert json.dumps(new) == json.dumps(old)


def test_record_runner_decode_validate_path_without_dolphin(tmp_path, monkeypatch, capsys):
    import record as recorder
    import run_scenario

    harness = tmp_path / "harness"
    harness.mkdir()
    monkeypatch.setattr(recorder, "HERE", harness)
    monkeypatch.setattr(recorder, "ROOT", tmp_path)
    monkeypatch.setattr(run_scenario, "HARNESS", harness)
    scenario = tmp_path / "laser.toml"
    scenario.write_text('name = "laser"\nframes = 4\nsavestate = "idle.sav"\n'
                        'inputs = [{ frame = 1, buttons = { B = true } }]\n')
    raw_rows = []
    for row in item_trace():
        raw_rows.append({key: value for key, value in row.items() if key != "state"})
        raw_rows[-1].update(seed=123, fighters=[{"base": "0x80453080", "bytes": bytes(0x23EC).hex()}],
                            pad_game=bytes(4 * 0x44).hex())
        for item in raw_rows[-1]["items"]:
            del item["state"]

    class FakeDolphin:
        pid = 1

        def __init__(self, command, **kwargs):
            from pathlib import Path

            assert command[command.index("--script") + 1].endswith("tick_trace.py")
            raw = Path(kwargs["env"]["MELEE_RAW_OUT"])
            assert raw.is_relative_to(tmp_path)
            raw.write_text("\n".join(map(json.dumps, raw_rows)))
            Path(str(raw) + ".done").write_text(json.dumps({"ticks": 4}))

        def poll(self):
            return 0

    def run_tick_runner(command, **kwargs):
        assert command[1].endswith("dolphin/run_scenario.py")
        assert "--tick-trace" in command and kwargs["check"]
        run_scenario.main(command[2:])

    monkeypatch.setattr(run_scenario.subprocess, "Popen", FakeDolphin)
    monkeypatch.setattr(recorder.subprocess, "run", run_tick_runner)
    recorder.main([str(scenario), "--no-ledger", "--no-particles"])
    output = capsys.readouterr().out
    assert "PASS: 0 violations" in output and "== laser: done" in output
    decoded = harness / "traces/laser.tick.expected.jsonl"
    assert validate_ticks.main([str(decoded), "--scripted", "--items"]) == 0
    assert "motion checked=1, skipped=0" in capsys.readouterr().out
