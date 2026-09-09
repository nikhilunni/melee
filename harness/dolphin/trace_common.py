"""Shared raw records, inputs, savestate loading and markers for Dolphin tracers."""
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


HERE = Path(__file__).resolve().parent
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
ITEM_SIZE = 0xFCC  # it/types.h:669, ASSERT_SIZE(struct Item, 0xFCC)
GOBJ_USER_DATA_OFF = walk.GOBJ_USER_DATA_OFF

SEED_ADDR = symbols.addr("seed")
ENTITIES_ADDR = symbols.addr("HSD_GObj_Entities")
# HSD_PadGameStatus[4]: the clamped, scaled pad each fighter reads
# (Fighter_Spaghetti_8006AD10). HSD_PadStatus is 0x44 bytes (controller.h).
PAD_GAME_ADDR = symbols.addr("HSD_PadGameStatus")
PAD_STATUS_SIZE = 0x44
PAD_PORTS = 4


def fighter_bases(mem=None) -> list[int]:
    """Walk HSD_GObj_Entities->fighters and return Fighter* for each."""
    return walk.fighter_bases(memory if mem is None else mem, ENTITIES_ADDR)


def read_items(mem=None) -> list[tuple[int, int, bytes]]:
    """(GObj*, Item*, full memory image), in retail p_link 9 order.

    gobj.h:76,36,44; item.c:957,984 creates p_link 9 and attaches Item*.
    Retail's limits are per hold-kind, loaded from ItemCommonData (item.c:
    132-144,455-528), not a fixed total. Walk to NULL without a fighter-sized
    cap. A seen set bounds corrupt cycles; malformed pointers fail capture
    instead of publishing a silently truncated oracle. Empty list: two reads.
    """
    mem = memory if mem is None else mem
    entities = mem.read_u32(ENTITIES_ADDR)
    if not entities:
        return []

    def checked(ptr: int, size: int) -> int:
        if ptr & 3 or not walk.MEM1_LO <= ptr <= walk.MEM1_HI - size:
            raise ValueError(f"invalid item-list pointer 0x{ptr:08X}")
        return ptr

    head = checked(entities, walk.GOBJLIST_ITEMS_OFF + 4)
    gobj = mem.read_u32(head + walk.GOBJLIST_ITEMS_OFF)
    items, seen = [], set()
    while gobj:
        checked(gobj, walk.GOBJ_SIZE)
        if gobj in seen:
            raise ValueError(f"cycle in item list at 0x{gobj:08X}")
        seen.add(gobj)
        base = mem.read_u32(gobj + GOBJ_USER_DATA_OFF)
        if base:  # Same as fighters: a GObj awaiting user_data is not live yet.
            checked(base, ITEM_SIZE)
            items.append((gobj, base, walk.read_bytes(mem, base, ITEM_SIZE)))
        gobj = mem.read_u32(gobj + walk.GOBJ_NEXT_OFF)
    return items


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
        # TOML gotcha: a key written after a [[fighters]] table belongs to that
        # table. `inputs` must be a top-level key (before the first table).
        if any("inputs" in f for f in scenario.get("fighters", [])):
            raise ValueError("scenario inputs must be top-level, not under a [[fighters]] table")
        # Current pad state per port; a step at frame N holds until that port's
        # next step. Frames count VI callbacks from the savestate load.
        self.held: dict[int, dict] = {0: {}}
        self.needs_load = savestate_path is not None
        self.load_info: dict = {}
        self.done = False
        self.t_start: float | None = None

    def pad_state(self, f: int) -> dict[int, dict]:
        for step in self.inputs:
            if step["frame"] == f:
                self.held[int(step.get("port", 0))] = dict(step.get("buttons", {}))
        return self.held

    def apply_inputs(self, f: int, ctl=None, base: dict | None = None) -> None:
        """Re-issue every port's held state (an override lasts one VI frame).

        `base` is merged under each port's step, so a step only names the
        keys that differ from it (neutral for the tick tracer).
        """
        ctl = self.ctl if ctl is None else ctl
        for port, held in sorted(self.pad_state(f).items()):
            ctl.set_gc_buttons(port, {**(base or {}), **held})

    def read_game_pads(self, mem=None) -> str:
        """Hex of HSD_PadGameStatus[0..3] as consumed by the tick just completed."""
        mem = self.mem if mem is None else mem
        return walk.read_bytes(mem, PAD_GAME_ADDR, PAD_STATUS_SIZE * PAD_PORTS).hex()

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
            # Retained legacy convention. Listeners actually accumulate in
            # this fork; self.done parks the original callback.
            self.events.on_frameadvance(lambda: None)

    def fail(self, text: str) -> None:
        self.done = True
        if self.done_path is not None:
            Path(str(self.done_path)[: -len(".done")] + ".err").write_text(text)
        try:
            self.out.close()
        finally:
            self.unregister()


def run(tracer_type=Tracer) -> None:
    raw_out = Path(os.environ["MELEE_RAW_OUT"])
    raw_out.parent.mkdir(parents=True, exist_ok=True)
    done = Path(str(raw_out) + ".done")
    err = Path(str(raw_out) + ".err")
    done.unlink(missing_ok=True)
    err.unlink(missing_ok=True)
    out = raw_out.open("w")
    try:
        scenario = tomllib.loads(Path(os.environ["MELEE_SCENARIO"]).read_text())
        sav = resolve_savestate(scenario)
        tracer = tracer_type(scenario, out, sav, read_sidecar(sav), done)
        event.on_frameadvance(tracer.on_frame)
    except Exception:
        out.close()
        err.write_text(traceback.format_exc())
