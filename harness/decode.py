"""Decode raw memory dumps into the canonical JSONL trace format.

Input: one JSON object per line from harness/dolphin/trace_scenario.py:
    {"frame": 12, "phase": "post_fighter", "seed": 2745024,
     "fighters": [{"base": "0x80453080", "bytes": "<hex of 0x23EC bytes>"}, ...]}

Output: melee-diff Records:
    {"frame": 12, "phase": "post_fighter",
     "state": {"rng.seed": {"t":"u","v":2745024},
               "p0.cur_pos.x": {"t":"f32","v":{"bits":..., "approx":...}}, ...}}

Float values carry their bit pattern so comparison is exact.
"""
from __future__ import annotations

import json
import math
import struct
import sys
from pathlib import Path

import yaml

from item_kinds import ITEM_KIND_NAMES
import trace_io

HERE = Path(__file__).resolve().parent
_FMT = {"u8": ">B", "s8": ">b", "u16": ">H", "s16": ">h", "u32": ">I", "s32": ">i",
        "f32": ">f", "f64": ">d", "ptr": ">I"}


def _approx(x: float) -> float:
    """The diagnostic approx is 0 for NaN and infinities, which JSON cannot
    carry; the bits stay exact (as jobjdump.py). Stale item memory holds them."""
    return x if math.isfinite(x) else 0.0


def _val(kind: str, raw: bytes):
    if kind == "f32":
        (x,) = struct.unpack(">f", raw)
        (bits,) = struct.unpack(">I", raw)
        return {"t": "f32", "v": {"bits": bits, "approx": _approx(x)}}
    if kind == "f64":
        (x,) = struct.unpack(">d", raw)
        (bits,) = struct.unpack(">Q", raw)
        return {"t": "f64", "v": {"bits": bits, "approx": _approx(x)}}
    (x,) = struct.unpack(_FMT[kind], raw)
    return {"t": "u" if kind.startswith(("u", "ptr")) else "i", "v": x}


def decode_struct(schema: dict, blob: bytes, prefix: str) -> dict:
    out = {}
    for name, f in schema["fields"].items():
        off, kind = f["offset"], f["type"]
        if kind == "vec3":
            for i, axis in enumerate("xyz"):
                out[f"{prefix}.{name}.{axis}"] = _val("f32", blob[off + 4 * i: off + 4 * i + 4])
        else:
            size = struct.calcsize(_FMT[kind])
            out[f"{prefix}.{name}"] = _val(kind, blob[off: off + size])
    return out


# HSD_PadStatus (sysdolphin/baselib/controller.h), 0x44 bytes per port.
PAD_FIELDS = [
    ("button", 0x00, "u32"), ("last_button", 0x04, "u32"), ("trigger", 0x08, "u32"),
    ("repeat", 0x0C, "u32"), ("release", 0x10, "u32"), ("repeat_count", 0x14, "s32"),
    ("stickX", 0x18, "s8"), ("stickY", 0x19, "s8"), ("subStickX", 0x1A, "s8"),
    ("subStickY", 0x1B, "s8"), ("analogL", 0x1C, "u8"), ("analogR", 0x1D, "u8"),
    ("analogA", 0x1E, "u8"), ("analogB", 0x1F, "u8"),
    ("nml_stickX", 0x20, "f32"), ("nml_stickY", 0x24, "f32"),
    ("nml_subStickX", 0x28, "f32"), ("nml_subStickY", 0x2C, "f32"),
    ("nml_analogL", 0x30, "f32"), ("nml_analogR", 0x34, "f32"),
    ("nml_analogA", 0x38, "f32"), ("nml_analogB", 0x3C, "f32"),
    ("cross_dir", 0x40, "u8"), ("err", 0x41, "s8"),
]
PAD_STATUS_SIZE = 0x44

# All offsets are relative to Item*, big-endian; source lines are in
# third_party/melee-decomp/src/melee/it/types.h unless otherwise stated.
ITEM_SIZE = 0xFCC  # :669 ASSERT_SIZE; :666-667 includes the entire kind union
ITEM_FIELDS = [
    ("entity", 0x004, "ptr"),          # :216-217
    ("spawn_kind", 0x00C, "s32"),      # :221-222
    ("kind", 0x010, "s32"),            # :224-225
    ("spawn_id", 0x01C, "u32"),        # :231 x1C; item.c:570 counter bits
    ("motion_id", 0x024, "s32"),       # :240-241 msid
    ("anim_id", 0x028, "s32"),         # :243-244
    ("facing_dir", 0x02C, "f32"),      # :246-247
    ("vel", 0x040, "vec3"),           # :261-262 x40_vel
    ("pos", 0x04C, "vec3"),           # :264-265
    ("external_vel", 0x058, "vec3"),  # :267-268; item.c:1423 additive displacement
    ("ground_vel", 0x064, "vec3"),    # :270-271; item.c:1424-1434 platform displacement
    ("nudge", 0x070, "vec3"),         # :273-274
    ("ground_or_air", 0x0C0, "s32"),  # :281-284 (after two 4-byte pointers)
    ("prev_pos", 0x388, "vec3"),      # :290 CollData +0x10; lb/types.h:202-206
    ("env_flags", 0x4AC, "u32"),      # :290 CollData +0x134; lb/types.h:232
    ("owner", 0x518, "ptr"),          # :293-295 (raw GObj*, not player slot)
    ("hitbox0.state", 0x5D4, "s32"),  # :315-320; lb/types.h:32 HitCapsule +0
    ("hitbox0.damage", 0x5E0, "f32"), # :316; lb/types.h:35 HitCapsule +0xC
    ("hitbox1.state", 0x710, "s32"),  # :316
    ("hitbox2.state", 0x84C, "s32"),  # :316
    ("hitbox3.state", 0x988, "s32"),  # :316
    ("reflect_gobj", 0xC64, "ptr"),   # :377
    ("hitlag_frames", 0xCBC, "f32"),  # :397
    ("atk_victim", 0xD04, "ptr"),     # :420-422
    ("physics_callback", 0xD18, "ptr"),  # :437-438
    ("collision_callback", 0xD1C, "ptr"),  # :440-441
    ("life_timer", 0xD44, "f32"),     # :478 (not necessarily enabled for every kind)
    ("flags", 0xDC8, "u32"),         # :530 (x9 hitlag, x13 held; item.c:1396-1402)
]
ITEM_SCHEMA = {"fields": {name: {"offset": off, "type": kind}
                          for name, off, kind in ITEM_FIELDS}}
FOX_LASER_KIND = next(k for k, name in ITEM_KIND_NAMES.items() if name == "It_Kind_Fox_Laser")


def decode_item(blob: bytes) -> dict:
    if len(blob) != ITEM_SIZE:
        raise ValueError(f"Item image must be {ITEM_SIZE:#x} bytes, got {len(blob):#x}")
    state = {key.removeprefix("item."): value
             for key, value in decode_struct(ITEM_SCHEMA, blob, "item").items()}
    # Four slots, no stored active-count field (:315-320). Disabled == 0,
    # lb/forward.h:71-78; count the active HitCapsules, not xAC8_hurtboxNum.
    state["hitbox_count"] = {"t": "u", "v": sum(
        state[f"hitbox{i}.state"]["v"] != 0 for i in range(4))}
    if state["kind"]["v"] == FOX_LASER_KIND:
        # :568,667 union foxlaser; it/itCharItems.h:240 pos at union +0xC.
        # itfoxlaser.c:95-96 saves this tick's pre-physics position here.
        laser = {"fields": {"prev_pos": {"offset": 0xDE0, "type": "vec3"}}}
        state.update(decode_struct(laser, blob, "laser"))
    return state


def decode_pads(blob: bytes) -> dict:
    """`pad_game` (HSD_PadGameStatus[4]) -> {"p0": {...}, ...}. Inputs, not state."""
    out = {}
    for port in range(len(blob) // PAD_STATUS_SIZE):
        base = port * PAD_STATUS_SIZE
        out[f"p{port}"] = {
            name: _val(kind, blob[base + off: base + off + struct.calcsize(_FMT[kind])])
            for name, off, kind in PAD_FIELDS
        }
    return out


# grStadium_GroundVars::xDC: the controller polls the form read in phase 1.
STADIUM_LOADING = 1


def decode_events(stage_io: dict, previous: dict | None) -> dict:
    """External events the tick's game logic consumed (inputs, not state).

    `stage_read_completed`: grStadium_801D42B8's poll this tick found the
    form archive read complete. The controller polls exactly once per tick
    in phase 1 and leaves it on success, so a tick that starts in phase 1
    and ends elsewhere is the tick whose poll succeeded; every other tick is
    false (no poll, or the read still in flight).
    """
    was_loading = previous is not None and previous["stadium_phase"] == STADIUM_LOADING
    return {"stage_read_completed": was_loading and stage_io["stadium_phase"] != STADIUM_LOADING}


def main(inp: Path, out: Path) -> None:
    fighter = yaml.safe_load((HERE / "schema" / "fighter.yaml").read_text())
    previous_io = None
    with trace_io.open_text(inp) as fi, out.open("w") as fo:
        for line in fi:
            if not line.strip():
                continue
            d = json.loads(line)
            state = {"rng.seed": {"t": "u", "v": d["seed"]}}
            for i, f in enumerate(d.get("fighters", [])):
                state.update(decode_struct(fighter, bytes.fromhex(f["bytes"]), f"p{i}"))
            record = {"frame": d["frame"], "phase": d["phase"], "state": state}
            for key in ("tick", "vi_frame", "ps_frame", "watch_address", "watch_value"):
                if key in d:
                    record[key] = d[key]
            if "pad_game" in d:
                record["inputs"] = decode_pads(bytes.fromhex(d["pad_game"]))
            if "stage_io" in d:
                record["events"] = decode_events(d["stage_io"], previous_io)
                previous_io = d["stage_io"]
            if "items" in d:
                record["items"] = [
                    {**item, "state": decode_item(bytes.fromhex(item["bytes"]))}
                    for item in d["items"]
                ]
            fo.write(json.dumps(record, allow_nan=False) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: decode.py <raw.jsonl> <trace.jsonl>")
    main(Path(sys.argv[1]), Path(sys.argv[2]))
