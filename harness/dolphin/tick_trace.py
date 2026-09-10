"""Sample completed scheduler ticks. See docs/DOLPHIN_RUN.md for the contract.

Uses MELEE_SCENARIO / MELEE_RAW_OUT and scenario.frames as the tick limit.

Scripted inputs: scenario `inputs` steps ({frame, port?, buttons}) are issued
at VI callbacks counted from the savestate load. VI timing cannot be aligned
to the game's pad queue exactly, so each tick record also carries `pad_game`,
the HSD_PadGameStatus array the tick actually consumed (renewed by
lb_80019900 right before HSD_GObj_80390CFC runs the tick); the port replays
that, not the VI schedule. No Dolphin dependency in tests.
"""
from __future__ import annotations

import json
import os
import struct
import sys
import time
import traceback
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
from trace_common import Tracer, event, read_items, run  # noqa: E402
from item_kinds import ITEM_KIND_NAMES  # noqa: E402
import remote_proto  # noqa: E402
import symbols  # noqa: E402

WATCH_ADDR = symbols.addr("gm_80479D58")  # unk_0, +0: scheduler tick count
STORE_PC = 0x801A4FB8
MASK = 0xFFFFFFFF
SATURATED = 0xFFFFFFFE  # gm_1A45.c:341 deliberately stops incrementing here
MAX_VI_WITHOUT_TICK = 120
# Retail instructions around gm_1A45.c:340-342, checked after savestate load.
BOUNDARY_CODE = {0x801A4FA0: 0x481EBD5D, 0x801A4FB4: 0x38030001,
                 STORE_PC: 0x90190000}


class TickTracer(Tracer):
    def record(self, phase: str, mem=None) -> dict:
        mem = self.mem if mem is None else mem
        record = super().record(phase, mem)
        items = read_items(mem)
        record["items"] = []
        if items:
            # Use the fighter images from this same CPU callback, not a second
            # walk or the list index (Nana and noncontiguous player slots exist).
            # ft/types.h:1127,1130: Fighter.gobj +0, player_id +0xC.
            owners = {}
            for fighter in record["fighters"]:
                raw = bytes.fromhex(fighter["bytes"])
                owners[struct.unpack_from(">I", raw, 0)[0]] = raw[0xC]
            owners.pop(0, None)
            for gobj, base, raw in items:
                kind = struct.unpack_from(">i", raw, 0x10)[0]  # it/types.h:225
                owner = struct.unpack_from(">I", raw, 0x518)[0]  # it/types.h:295
                record["items"].append({
                    "gobj": f"0x{gobj:08X}", "base": f"0x{base:08X}",
                    "kind": kind, "kind_name": ITEM_KIND_NAMES.get(kind, "?"),
                    "owner": owners.get(owner), "bytes": raw.hex(),
                })
        return record

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        if self.savestate_path is None:
            raise ValueError("tick tracing requires a match savestate")
        if type(self.scenario["frames"]) is not int or self.scenario["frames"] <= 0:
            raise ValueError("frames must be a positive tick count")
        for step in self.inputs:
            unknown = set(step.get("buttons", {})) - set(remote_proto.GC_KEYS)
            if unknown or type(step.get("frame")) is not int or step["frame"] < 0:
                raise ValueError(f"bad input step {step!r}: unknown keys {sorted(unknown)}")
        self.vi_frame = -1
        self.last_tick_vi = 0
        self.last_tick: int | None = None
        self.initial_tick: int | None = None
        self.installed = False
        self.in_callback = False
        self.pending_finish = False
        self.duplicates = 0
        self.reentrant = 0
        self.counter_reset_at_start = False
        self.ended_early = False
        # Human ports: log the pads Dolphin polled, per VI frame, so the RNG-ledger and
        # particle passes can replay them as a scripted schedule (pads_to_inputs.py).
        self.pads_out = open(self.out.name + ".pads.jsonl", "w") if self.human_ports else None
        # keypad.py writes the terminal gamepad's state here; applied to the human ports.
        keypad = os.environ.get("MELEE_KEYPAD")
        self.keypad = Path(keypad) if keypad and self.human_ports else None
        self.keypad_pad: dict | None = None

    def install(self) -> None:
        if self.load_info.get("synced") is False:
            raise ValueError(f"savestate seed does not match sidecar: {self.load_info}")
        for addr, expected in BOUNDARY_CODE.items():
            actual = self.mem.read_u32(addr)
            if actual != expected:
                raise ValueError(f"retail code mismatch at 0x{addr:08X}: "
                                 f"0x{actual:08X} != 0x{expected:08X}")
        self.initial_tick = self.last_tick = self.mem.read_u32(WATCH_ADDR)
        if self.last_tick == SATURATED:
            raise ValueError("game tick counter is saturated")
        self.events.on_memorybreakpoint(self.on_memory)
        self.mem.add_memcheck(WATCH_ADDR)  # binding takes just one positional address
        self.installed = True

    def on_frame(self) -> None:
        # Removal here avoids invalidating TMemCheck::Action's `this` while it
        # is still executing. The final record was already taken on the CPU.
        try:
            if self.done:
                self.unregister()
                return
            if self.pending_finish:
                self.finish()
                return
            self.vi_frame += 1
            if self.needs_load:
                self.load_savestate()
            if not self.installed:
                self.t_start = time.monotonic()
                self.install()
            self.apply_inputs(self.vi_frame, base=remote_proto.neutral_inputs())
            if self.keypad is not None:
                try:
                    self.keypad_pad = json.loads(self.keypad.read_text())["pad"]
                except (OSError, ValueError, KeyError):
                    pass  # mid-replace or not started yet: keep the previous state
                if self.keypad_pad is not None:
                    for port in self.human_ports:
                        self.ctl.set_gc_buttons(port, self.keypad_pad)
            if self.pads_out is not None and self.vi_frame > 0:
                self.log_human_pads(self.vi_frame - 1)
            if self.vi_frame - self.last_tick_vi >= MAX_VI_WITHOUT_TICK:
                raise RuntimeError(f"no scheduler tick for {MAX_VI_WITHOUT_TICK} VI fields; "
                                   "check memcheck support, pause/stepping, and savestate")
        except Exception:
            self.fail(traceback.format_exc())

    def on_memory(self, is_write: bool, addr: int, value: int) -> None:
        if self.done or self.pending_finish or not self.installed:
            return
        if not is_write or addr != WATCH_ADDR:
            return
        if self.in_callback:
            self.reentrant += 1
            return
        self.in_callback = True
        try:
            before = self.mem.read_u32(WATCH_ADDR)
            if self.frame and value == self.last_tick:
                self.duplicates += 1
                return
            if self.frame == 0 and value == 0 and before == self.last_tick:
                # A match scene resets the scheduler tick counter to 0 on its
                # first tick (gm_1A45.c); a savestate taken at match start
                # observes that reset. Accept it once and count from there.
                self.counter_reset_at_start = True
            elif self.frame and value == 0 and before == self.last_tick and self.human_ports:
                # A human match ends when a player runs out of stocks: the GAME scene
                # resets the counter ~113 frames after the last KO. Stop there and
                # report the tick count actually recorded.
                self.ended_early = True
                self.scenario["frames"] = self.frame
                self.pending_finish = True
                return
            elif before != self.last_tick or value != (before + 1) & MASK:
                raise ValueError(f"tick discontinuity at ordinal {self.frame}: "
                                 f"last={self.last_tick}, memory={before}, callback={value}")
            record = self.record("frame_end")
            if not record["fighters"]:
                raise ValueError(f"no fighters at tick ordinal {self.frame}")
            record.update(tick=value, vi_frame=self.vi_frame,
                          watch_address=WATCH_ADDR, watch_value=before,
                          pad_game=self.read_game_pads())
            self.out.write(json.dumps(record) + "\n")
            self.out.flush()
            self.last_tick = value
            self.last_tick_vi = self.vi_frame
            self.frame += 1
            if self.frame == self.scenario["frames"]:
                self.pending_finish = True
            elif value == SATURATED:
                raise ValueError("game tick counter saturated before requested count")
        except Exception:
            self.fail(traceback.format_exc())
        finally:
            self.in_callback = False

    def unregister(self) -> None:
        if self.in_callback:
            return  # next VI removes the memcheck even after an error
        if self.installed:
            self.mem.remove_memcheck(WATCH_ADDR)
            self.installed = False
        # This fork accumulates listeners; done guards park ours without
        # registering more callbacks or relying on on_*(None).

    def finish(self) -> None:
        self.unregister()  # cleanup must succeed before publishing .done
        if self.pads_out is not None:
            self.log_human_pads(self.vi_frame)  # the poll behind the final tick
            self.pads_out.close()
            self.pads_out = None
        super().finish()

    def log_human_pads(self, vi_frame: int) -> None:
        """The pad state Dolphin polled during `vi_frame` (read one VI later, before
        this frame's poll): `controller.get_gc_buttons` returns the last seen input,
        override or real controller alike."""
        pads = {str(port): self.ctl.get_gc_buttons(port) for port in self.human_ports}
        self.pads_out.write(json.dumps({"vi_frame": vi_frame, "pads": pads}) + "\n")

    def summary(self) -> dict:
        return {**super().summary(), "sampling": "scheduler_end_pre_counter_store",
                "ticks": self.frame, "vi_frames": self.vi_frame + 1,
                "ended_early": self.ended_early, "human_ports": self.human_ports,
                "watch_address": WATCH_ADDR, "store_pc": STORE_PC,
                "initial_tick": self.initial_tick, "last_tick": self.last_tick,
                "duplicate_callbacks": self.duplicates, "reentrant_callbacks": self.reentrant,
                "counter_reset_at_start": self.counter_reset_at_start}


# Only when Dolphin runs this file directly; importing it (rng_ledger.py) must not start a tracer.
if event is not None and __name__ == "__main__":
    run(TickTracer)
