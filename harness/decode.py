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
import struct
import sys
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
_FMT = {"u8": ">B", "s8": ">b", "u16": ">H", "s16": ">h", "u32": ">I", "s32": ">i",
        "f32": ">f", "f64": ">d", "ptr": ">I"}


def _val(kind: str, raw: bytes):
    if kind == "f32":
        (x,) = struct.unpack(">f", raw)
        (bits,) = struct.unpack(">I", raw)
        return {"t": "f32", "v": {"bits": bits, "approx": x}}
    if kind == "f64":
        (x,) = struct.unpack(">d", raw)
        (bits,) = struct.unpack(">Q", raw)
        return {"t": "f64", "v": {"bits": bits, "approx": x}}
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


def main(inp: Path, out: Path) -> None:
    fighter = yaml.safe_load((HERE / "schema" / "fighter.yaml").read_text())
    with inp.open() as fi, out.open("w") as fo:
        for line in fi:
            if not line.strip():
                continue
            d = json.loads(line)
            state = {"rng.seed": {"t": "u", "v": d["seed"]}}
            for i, f in enumerate(d.get("fighters", [])):
                state.update(decode_struct(fighter, bytes.fromhex(f["bytes"]), f"p{i}"))
            record = {"frame": d["frame"], "phase": d["phase"], "state": state}
            for key in ("tick", "vi_frame", "watch_address", "watch_value"):
                if key in d:
                    record[key] = d[key]
            if "pad_game" in d:
                record["inputs"] = decode_pads(bytes.fromhex(d["pad_game"]))
            fo.write(json.dumps(record) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: decode.py <raw.jsonl> <trace.jsonl>")
    main(Path(sys.argv[1]), Path(sys.argv[2]))
