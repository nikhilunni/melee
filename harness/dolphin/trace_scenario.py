"""Dolphin-side script: drive a scenario and dump raw state every frame.

Target runtime: a scripting-capable Dolphin build (Felk's "dolphin scripting"
fork exposes `dolphin.event`, `dolphin.memory`, `dolphin.controller`,
`dolphin.savestate`). The GDB stub in mainline Dolphin is the fallback and
would need a different driver.

The memory-touching walk lives in walk.py (no Dolphin dependency) so it can be
unit-tested; this file only wires it to `dolphin.memory` and the frame hook.

Facts about the scripting fork this relies on:
  * `dolphin.memory` has typed reads (read_u8/u16/u32/u64, read_s*, read_f32/
    f64) and matching writes, but no bulk read; walk.read_bytes composes one.
  * `controller.set_gc_buttons(port, dict)` lasts a single frame, so the held
    input state is re-issued on every frame advance.
  * `event.on_codebreakpoint(cb)` exists, but breakpoints cannot be added from
    Python. TODO: the intra-frame phases in docs/ORACLE.md (input, fighter
    update, collision, items, camera) need breakpoints set via the debugger UI
    or a small C++ patch before this script can dump more than `frame_end`.

Run inside Dolphin with:  --script harness/dolphin/trace_scenario.py
Environment:
  MELEE_SCENARIO  path to a scenario TOML
  MELEE_RAW_OUT   path to write raw JSONL

Note: this directory is also called `dolphin`, but it has no __init__.py on
purpose. A regular package (Dolphin's real `dolphin` module) always wins over a
namespace directory during import, so `from dolphin import memory` below still
resolves to the emulator's module even with harness/ on sys.path.
"""
from __future__ import annotations

import json
import os
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))  # harness/  -> symbols
sys.path.insert(0, str(HERE))         # harness/dolphin -> walk
import symbols  # noqa: E402
import walk  # noqa: E402

try:
    from dolphin import controller, event, memory, savestate  # type: ignore
except ImportError:  # allow importing for tests outside Dolphin
    controller = event = memory = savestate = None

FIGHTER_SIZE = 0x23EC
GOBJ_USER_DATA_OFF = walk.GOBJ_USER_DATA_OFF

SEED_ADDR = symbols.addr("seed")
ENTITIES_ADDR = symbols.addr("HSD_GObj_Entities")


def fighter_bases(mem=None) -> list[int]:
    """Walk HSD_GObj_Entities->fighters and return Fighter* for each."""
    return walk.fighter_bases(memory if mem is None else mem, ENTITIES_ADDR)


class Tracer:
    def __init__(self, scenario: dict, out) -> None:
        self.scenario = scenario
        self.out = out
        self.frame = 0
        self.inputs = sorted(scenario.get("inputs", []), key=lambda s: s["frame"])
        self.held: dict = {}  # current pad state; a step at frame N holds until the next step

    def pad_state(self, f: int) -> dict:
        for step in self.inputs:
            if step["frame"] == f:
                self.held = dict(step.get("buttons", {}))
        return self.held

    def apply_inputs(self, f: int, ctl=None) -> None:
        ctl = controller if ctl is None else ctl
        # set_gc_buttons only lasts one frame: re-issue the held state every frame.
        ctl.set_gc_buttons(0, self.pad_state(f))

    def record(self, phase: str, mem=None) -> dict:
        mem = memory if mem is None else mem
        return {
            "frame": self.frame,
            "phase": phase,
            "seed": mem.read_u32(SEED_ADDR),
            "fighters": [
                {"base": f"0x{b:08X}", "bytes": walk.read_bytes(mem, b, FIGHTER_SIZE).hex()}
                for b in fighter_bases(mem)
            ],
        }

    def dump(self, phase: str) -> None:
        self.out.write(json.dumps(self.record(phase)) + "\n")

    def on_frame(self) -> None:
        self.apply_inputs(self.frame)
        self.dump("frame_end")
        self.frame += 1
        if self.frame >= self.scenario["frames"]:
            self.out.close()
            sys.exit(0)


def main() -> None:
    scenario = tomllib.loads(Path(os.environ["MELEE_SCENARIO"]).read_text())
    out = open(os.environ["MELEE_RAW_OUT"], "w")
    tracer = Tracer(scenario, out)
    if "savestate" in scenario:
        savestate.load_from_file(scenario["savestate"])
    event.on_frameadvance(tracer.on_frame)


if event is not None:
    main()
