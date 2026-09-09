"""Sample completed scheduler ticks. See docs/DOLPHIN_RUN.md for the contract.

Uses MELEE_SCENARIO / MELEE_RAW_OUT and scenario.frames as the tick limit.
Only neutral scenarios are supported: VI input injection cannot time scripted
input transitions to the game's pad queue. No Dolphin dependency in tests.
"""
from __future__ import annotations

import json
import sys
import time
import traceback
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
from trace_common import Tracer, event, run  # noqa: E402
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
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        if self.savestate_path is None:
            raise ValueError("tick tracing requires a match savestate")
        if type(self.scenario["frames"]) is not int or self.scenario["frames"] <= 0:
            raise ValueError("frames must be a positive tick count")
        if any(step.get("buttons") for step in self.inputs):
            raise ValueError("tick tracing currently supports neutral inputs only")
        self.vi_frame = -1
        self.last_tick_vi = 0
        self.last_tick: int | None = None
        self.initial_tick: int | None = None
        self.installed = False
        self.in_callback = False
        self.pending_finish = False
        self.duplicates = 0
        self.reentrant = 0

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
            self.ctl.set_gc_buttons(0, remote_proto.neutral_inputs())
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
            if before != self.last_tick or value != (before + 1) & MASK:
                raise ValueError(f"tick discontinuity at ordinal {self.frame}: "
                                 f"last={self.last_tick}, memory={before}, callback={value}")
            record = self.record("frame_end")
            if not record["fighters"]:
                raise ValueError(f"no fighters at tick ordinal {self.frame}")
            record.update(tick=value, vi_frame=self.vi_frame,
                          watch_address=WATCH_ADDR, watch_value=before)
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
        super().finish()

    def summary(self) -> dict:
        return {**super().summary(), "sampling": "scheduler_end_pre_counter_store",
                "ticks": self.frame, "vi_frames": self.vi_frame + 1,
                "watch_address": WATCH_ADDR, "store_pc": STORE_PC,
                "initial_tick": self.initial_tick, "last_tick": self.last_tick,
                "duplicate_callbacks": self.duplicates, "reentrant_callbacks": self.reentrant}


# Only when Dolphin runs this file directly; importing it (rng_ledger.py) must not start a tracer.
if event is not None and __name__ == "__main__":
    run(TickTracer)
