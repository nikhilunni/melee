"""Dolphin-side script: drive a scenario and dump raw state every frame.

Target runtime: a scripting-capable Dolphin build (Felk's "dolphin scripting"
fork exposes `dolphin.event`, `dolphin.memory`, `dolphin.controller`,
`dolphin.savestate`). The GDB stub in mainline Dolphin is the fallback and
would need a different driver.

The memory-touching walk lives in walk.py (no Dolphin dependency) so it can be
unit-tested; this file only wires it to `dolphin.memory` and the frame hook.

Facts about the scripting fork this relies on (verified 2026-09-08, see
docs/DOLPHIN_RUN.md):
  * `__file__` is NOT defined for --script files; the code object's filename
    is used to find this directory.
  * `dolphin.memory` has typed reads (read_u8/u16/u32/u64, read_s*, read_f32/
    f64) and matching writes, but no bulk read; walk.read_bytes composes one.
  * `controller.set_gc_buttons(port, dict)` lasts a single frame, so the held
    input state is re-issued on every frame advance.
  * `savestate.load_from_file` called from inside the frameadvance callback
    runs synchronously (State::LoadAs -> Core::RunOnCPUThread, which runs the
    job inline when already on the CPU thread). Called from top-level script
    code the core is not running yet and the load is a no-op, so the load
    happens on the first frame callback. Right after it returns, memory IS the
    saved frame boundary, which becomes trace frame 0.
  * An exception (including SystemExit) inside a callback is only printed; the
    callback stays registered. Listeners can be replaced but not removed
    (`on_frameadvance(None)` raises ValueError), so finishing parks a no-op.
  * `event.on_codebreakpoint(cb)` exists, but breakpoints cannot be added from
    Python. TODO: the intra-frame phases in docs/ORACLE.md (input, fighter
    update, collision, items, camera) need breakpoints set via the debugger UI
    or a small C++ patch before this script can dump more than `frame_end`.

Run inside Dolphin with:  --script harness/dolphin/trace_scenario.py
(or from a shell: `uv run python dolphin/run_scenario.py scenarios/x.toml`)
Environment:
  MELEE_SCENARIO  path to a scenario TOML
  MELEE_RAW_OUT   path to write raw JSONL; `<path>.done` (JSON) is written
                  when the run is complete, `<path>.err` on failure

Note: this directory is also called `dolphin`, but it has no __init__.py on
purpose. A regular package (Dolphin's real `dolphin` module) always wins over a
namespace directory during import, so `from dolphin import memory` below still
resolves to the emulator's module even with harness/ on sys.path.
"""
from __future__ import annotations

import json
import os
import sys
import time
import tomllib
import traceback
from pathlib import Path


def script_dir(g: dict) -> Path:
    """Directory of this script, with or without `__file__` (Dolphin omits it)."""
    f = g.get("__file__") or sys._getframe(1).f_code.co_filename  # noqa: SLF001
    return Path(f).resolve().parent


HERE = script_dir(globals())
REPO = HERE.parent.parent
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


def resolve_savestate(scenario: dict, repo: Path = REPO) -> Path | None:
    """Scenario savestate paths are relative to the repo root (Dolphin's cwd is not)."""
    if "savestate" not in scenario:
        return None
    p = Path(scenario["savestate"])
    return p if p.is_absolute() else repo / p


def read_sidecar(sav: Path | None) -> dict | None:
    """remote.py writes `<sav>.json` with the seed/frame at save time."""
    if sav is None:
        return None
    try:
        return json.loads(Path(str(sav) + ".json").read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None


class Tracer:
    def __init__(self, scenario: dict, out, savestate_path: Path | None = None,
                 sidecar: dict | None = None, done_path: Path | None = None,
                 mem=None, ctl=None, states=None, events=None) -> None:
        self.scenario = scenario
        self.out = out
        self.savestate_path = savestate_path
        self.sidecar = sidecar
        self.done_path = done_path
        self.mem = memory if mem is None else mem
        self.ctl = controller if ctl is None else ctl
        self.states = savestate if states is None else states
        self.events = event if events is None else events
        self.frame = 0
        self.inputs = sorted(scenario.get("inputs", []), key=lambda s: s["frame"])
        self.held: dict = {}  # current pad state; a step at frame N holds until the next step
        self.needs_load = savestate_path is not None
        self.load_info: dict = {}
        self.done = False
        self.t_start: float | None = None

    def pad_state(self, f: int) -> dict:
        for step in self.inputs:
            if step["frame"] == f:
                self.held = dict(step.get("buttons", {}))
        return self.held

    def apply_inputs(self, f: int, ctl=None) -> None:
        ctl = self.ctl if ctl is None else ctl
        # set_gc_buttons only lasts one frame: re-issue the held state every frame.
        ctl.set_gc_buttons(0, self.pad_state(f))

    def record(self, phase: str, mem=None) -> dict:
        mem = self.mem if mem is None else mem
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

    def load_savestate(self) -> None:
        """Synchronous inside the frame callback; afterwards memory is the saved boundary."""
        self.states.load_from_file(str(self.savestate_path))
        self.needs_load = False
        seed = self.mem.read_u32(SEED_ADDR)
        self.load_info = {"seed_after_load": seed}
        if self.sidecar is not None:
            self.load_info["sidecar_seed"] = self.sidecar.get("seed")
            self.load_info["synced"] = seed == self.sidecar.get("seed")

    def on_frame(self) -> None:
        if self.done:
            return
        try:
            if self.needs_load:
                self.load_savestate()
            if self.t_start is None:
                self.t_start = time.monotonic()
            self.apply_inputs(self.frame)
            self.dump("frame_end")
            self.frame += 1
            if self.frame >= self.scenario["frames"]:
                self.finish()
        except Exception:  # noqa: BLE001
            self.fail(traceback.format_exc())

    def summary(self) -> dict:
        elapsed = time.monotonic() - self.t_start if self.t_start else 0.0
        return {
            "frames": self.frame,
            "elapsed_s": round(elapsed, 3),
            "fps": round(self.frame / elapsed, 2) if elapsed > 0 else None,
            "savestate": str(self.savestate_path) if self.savestate_path else None,
            **self.load_info,
        }

    def finish(self) -> None:
        self.done = True
        self.out.close()
        if self.done_path is not None:
            self.done_path.write_text(json.dumps(self.summary(), indent=1))
        self.unregister()

    def unregister(self) -> None:
        if self.events is not None:
            self.events.on_frameadvance(lambda: None)   # None is rejected by the fork

    def fail(self, text: str) -> None:
        self.done = True
        if self.done_path is not None:
            Path(str(self.done_path)[: -len(".done")] + ".err").write_text(text)
        try:
            self.out.close()
        finally:
            self.unregister()


def main() -> None:
    scenario = tomllib.loads(Path(os.environ["MELEE_SCENARIO"]).read_text())
    raw_out = Path(os.environ["MELEE_RAW_OUT"])
    raw_out.parent.mkdir(parents=True, exist_ok=True)
    sav = resolve_savestate(scenario)
    tracer = Tracer(scenario, raw_out.open("w"), sav, read_sidecar(sav),
                    Path(str(raw_out) + ".done"))
    event.on_frameadvance(tracer.on_frame)


if event is not None:
    main()
