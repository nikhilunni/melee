"""Dolphin-side script: drive a scenario and dump raw state every frame.

Target runtime: a scripting-capable Dolphin build (Felk's "dolphin scripting"
fork exposes `dolphin.event`, `dolphin.memory`, `dolphin.controller`,
`dolphin.savestate`). The GDB stub in mainline Dolphin is the fallback and
would need a different driver.

This is a skeleton. The pieces marked TODO need the fighter list walk, which
requires the HSD_GObjEntities layout from the decomp (sysdolphin/baselib/gobj.h)
to be transcribed into harness/schema/globals.yaml.

Run inside Dolphin with:  --script harness/dolphin/trace_scenario.py
Environment:
  MELEE_SCENARIO  path to a scenario TOML
  MELEE_RAW_OUT   path to write raw JSONL
"""
from __future__ import annotations

import json
import os
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import symbols  # noqa: E402

try:
    from dolphin import controller, event, memory, savestate  # type: ignore
except ImportError:  # allow importing for tests outside Dolphin
    controller = event = memory = savestate = None

FIGHTER_SIZE = 0x23EC
GOBJ_USER_DATA_OFF = 0x2C

scenario = tomllib.loads(Path(os.environ["MELEE_SCENARIO"]).read_text())
out = open(os.environ["MELEE_RAW_OUT"], "w")
SEED_ADDR = symbols.addr("seed")
ENTITIES_ADDR = symbols.addr("HSD_GObj_Entities")

frame = 0
inputs = scenario.get("inputs", [])


def fighter_bases() -> list[int]:
    """Walk HSD_GObj_Entities->fighters and return Fighter* for each."""
    # TODO: transcribe HSD_GObjEntities and HSD_GObj layouts (next pointer,
    # user_data at +0x2C) into schema and walk the list here.
    return []


def apply_inputs(f: int) -> None:
    for step in inputs:
        if step["frame"] == f:
            controller.set_gc_buttons(0, step.get("buttons", {}))


def dump(phase: str) -> None:
    rec = {
        "frame": frame,
        "phase": phase,
        "seed": memory.read_u32(SEED_ADDR),
        "fighters": [
            {"base": f"0x{b:08X}", "bytes": memory.read_bytes(b, FIGHTER_SIZE).hex()}
            for b in fighter_bases()
        ],
    }
    out.write(json.dumps(rec) + "\n")


if event is not None:
    if "savestate" in scenario:
        savestate.load_from_file(scenario["savestate"])

    @event.on_frameadvance
    def _on_frame() -> None:
        global frame
        apply_inputs(frame)
        dump("frame_end")
        frame += 1
        if frame >= scenario["frames"]:
            out.close()
            sys.exit(0)
