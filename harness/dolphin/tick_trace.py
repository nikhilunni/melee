"""Sample completed scheduler ticks. See docs/DOLPHIN_RUN.md for the contract.

Uses MELEE_SCENARIO / MELEE_RAW_OUT and scenario.frames as the tick limit.

Scripted inputs: scenario `inputs` steps ({frame, port?, buttons}) are issued
at VI callbacks counted from the savestate load. VI timing cannot be aligned
to the game's pad queue exactly, so each tick record also carries `pad_game`,
the HSD_PadGameStatus array the tick actually consumed (renewed by
lb_80019900 right before HSD_GObj_80390CFC runs the tick); the port replays
that, not the VI schedule. No Dolphin dependency in tests.

Tick input clock (`input_clock = "tick"`): step `frame`s count tick records
instead of VI callbacks, and each step carries the raw PADStatus values in
`raw`. HSD_PadRenewMasterStatus dequeues one raw sample per tick from the
queue HSD_PadRenewRawStatus fills at every VI poll, and keeps the previous
status when the queue is empty, so which poll a tick sees depends on queue
depth. Before the first tick and at every tick end (the game is stopped in the
memcheck callback) the tracer therefore rewrites the queue to hold exactly one
entry: the pad the next tick must consume. Records then carry exactly the
scheduled pads, which replay_to_scenario.py --verify checks.

Controller-fix Gecko codes (UCF, harness/gecko.py) also read older queue
entries: each record's `pad_queue_x` holds, per port, the raw stickX of
entries qread-1 and qread-3 exactly as UCF's FETCH_INPUT indexes them, and
`pad_queue_sticks` entry qread-1's four stick bytes (UCF 0.84's buffer). With
codes installed (MELEE_GECKO_HOOKS), the tick clock also rewrites the two
entries before the injected one with the two previous ticks' pads, so the
queue holds one poll per tick whatever Dolphin's own polls did, and the
first record checks every injection holds its branch.

After-map sample (`after_map = true`): each record also carries `after_map`,
every fighter's struct as it stood when its map proc returned (Fighter_procMap,
0x8006C27C, s_link 6), where Slippi before 3.4.0 reads Post Frame (hook
0x8006C5D8, the proc's common exit). A later proc of the same tick may
overwrite what the map proc left (the accessory pins a captured fighter back
to its captor), so the tick's end state cannot show it.

The scripting API has memory breakpoints only, and a breakpoint makes Dolphin
take its slow memory path for the whole 128 KiB page around it. On a heap
page (a fighter's struct or JObj) that changes emulated float results by an
ulp, so the hook must be a word beside the ones already watched: the GObj
proc loop (HSD_GObj_80390CFC) names the running GObj and proc in
HSD_GObj_804D781C / HSD_GObj_804D7838 (.sbss, the RNG seed's page) and
clears them when the proc returns (0x80390E7C, 0x80390E80). A zero stored to
the first while the second still holds a proc whose callback is
Fighter_procMap is the end of that fighter's map proc.
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
from trace_common import FIGHTER_SIZE, GOBJ_USER_DATA_OFF, Tracer, event, read_items, read_stage_io, run  # noqa: E402
from item_kinds import ITEM_KIND_NAMES  # noqa: E402
import remote_proto  # noqa: E402
import symbols  # noqa: E402
import walk  # noqa: E402

sys.path.insert(0, str(HERE.parent))
import gecko  # noqa: E402

WATCH_ADDR = symbols.addr("gm_80479D58")  # unk_0, +0: scheduler tick count
STORE_PC = 0x801A4FB8
MASK = 0xFFFFFFFF
SATURATED = 0xFFFFFFFE  # gm_1A45.c:341 deliberately stops incrementing here
MAX_VI_WITHOUT_TICK = 120
# Retail instructions around gm_1A45.c:340-342, checked after savestate load.
BOUNDARY_CODE = {0x801A4FA0: 0x481EBD5D, 0x801A4FB4: 0x38030001,
                 STORE_PC: 0x90190000}
# controller.h PadLibData: qnum +0, qread +1, qwrite +2, qcount +3, queue +8.
PAD_LIB_ADDR = symbols.addr("HSD_PadLibData")
# psdisp.c:1857-1861: psFrameNum advances once per particle display pass
# (wrapping 0xFF -> 1). A change between ticks means the lists were re-sorted.
PS_FRAME_ADDR = symbols.addr("psFrameNum")
PAD_STATUS_BYTES = 12  # SDK PADStatus: button u16, 4 x s8 sticks, 4 x u8 analog, s8 err
PAD_ENTRY_BYTES = 4 * PAD_STATUS_BYTES
RAW_KEYS = ("button", "stickX", "stickY", "substickX", "substickY", "triggerL", "triggerR")
PAD_PORTS = 4
#: UCF's FETCH_INPUT wraps a negative queue index by this constant (qnum 5).
UCF_QUEUE_WRAP = 5
PAD_BUTTON_A, PAD_BUTTON_B = 0x100, 0x200
# gobj.c:112-135 (HSD_GObj_80390CFC): the proc loop stores the GObj and the
# proc it is about to run (0x80390DE8, 0x80390DEC) and zeroes both, in that
# order, when the proc returns (0x80390E7C, 0x80390E80).
CURRENT_GOBJ_ADDR = symbols.addr("HSD_GObj_804D781C")
CURRENT_PROC_ADDR = symbols.addr("HSD_GObj_804D7838")
PROC_LOOP_CODE = {0x80390E7C: 0x900DC17C, 0x80390E80: 0x900DC198}
# gobjproc.h HSD_GObjProc: gobj +0x10, on_invoke +0x14.
PROC_GOBJ_OFF, PROC_ON_INVOKE_OFF = 0x10, 0x14
FIGHTER_MAP_PROC = symbols.addr("Fighter_procMap")   # 0x8006C27C, fighter.c:902


def raw_pad_bytes(raw: dict) -> bytes:
    """PADStatus bytes 0..9 for a step's `raw` table (err is left untouched).
    Dolphin reports analog A/B as 0xFF while the digital button is held."""
    button = int(raw.get("button", 0))
    signed = [int(raw.get(k, 0)) & 0xFF for k in ("stickX", "stickY", "substickX", "substickY")]
    return (struct.pack(">H", button) + bytes(signed)
            + bytes([int(raw.get("triggerL", 0)), int(raw.get("triggerR", 0)),
                     0xFF if button & PAD_BUTTON_A else 0, 0xFF if button & PAD_BUTTON_B else 0]))


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
        stage_io = read_stage_io(mem)
        if stage_io is not None:
            record["stage_io"] = stage_io
        if self.after_map:
            record["after_map"] = self.take_after_map(record["fighters"])
        return record

    def watch_map_procs(self) -> None:
        """Break on the proc loop's current-GObj word (see the module header)."""
        for addr, expected in PROC_LOOP_CODE.items():
            actual = self.mem.read_u32(addr)
            if actual != expected:
                raise ValueError(f"retail code mismatch at 0x{addr:08X}: "
                                 f"0x{actual:08X} != 0x{expected:08X}")
        self.mem.add_memcheck(CURRENT_GOBJ_ADDR)

    def sample_after_map(self, is_write: bool, value: int) -> None:
        """The proc loop clearing its current GObj: when the proc that just
        returned is a fighter's map proc, keep that fighter's struct."""
        if not is_write or value != 0 or self.done or self.pending_finish or not self.installed:
            return
        proc = self.mem.read_u32(CURRENT_PROC_ADDR)
        if not walk.MEM1_LO <= proc < walk.MEM1_HI or proc & 3:
            return
        if self.mem.read_u32(proc + PROC_ON_INVOKE_OFF) != FIGHTER_MAP_PROC:
            return
        gobj = self.mem.read_u32(proc + PROC_GOBJ_OFF)
        base = self.mem.read_u32(gobj + GOBJ_USER_DATA_OFF)
        self.after_map_samples[base] = walk.read_bytes(self.mem, base, FIGHTER_SIZE).hex()
        self.after_map_count += 1

    def take_after_map(self, fighters: list[dict]) -> list[dict]:
        """This tick's samples, each with its index in the record's fighter list."""
        bases = [int(fighter["base"], 16) for fighter in fighters]
        samples, self.after_map_samples = self.after_map_samples, {}
        unknown = sorted(set(samples) - set(bases))
        if unknown:
            raise ValueError(f"tick ordinal {self.frame}: map procs ran for fighters not in the "
                             f"fighter list: {[f'0x{base:08X}' for base in unknown]}")
        return [{"fighter": index, "bytes": samples[base]}
                for index, base in enumerate(bases) if base in samples]

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
        self.tick_clock = self.scenario.get("input_clock", "vi") == "tick"
        if self.tick_clock:
            if self.human_ports:
                raise ValueError("the tick input clock drives scripted ports only")
            for step in self.inputs:
                if set(step.get("raw", {})) - set(RAW_KEYS):
                    raise ValueError(f"bad raw pad in step {step!r}")
            self.tick_steps = sorted(self.inputs, key=lambda step: step["frame"])
            self.tick_step_index = 0
            self.tick_held: dict[int, dict] = {0: {}, 1: {}}
            self.injected_ticks = 0
            # The held steps of the last two injected ticks, newest first.
            self.tick_history: list[dict[int, dict]] = []
        self.gecko_hooks = gecko.hooks_from_env()
        # `after_map = true`: the current tick's samples (Fighter* -> struct hex).
        self.after_map = bool(self.scenario.get("after_map", False))
        self.after_map_samples: dict[int, str] = {}
        self.after_map_count = 0
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
        if self.after_map:
            self.watch_map_procs()
        self.installed = True
        if self.tick_clock:
            self.inject_tick_pads(0)

    def advance_tick_steps(self, ordinal: int) -> None:
        """Held per-port step state for tick record `ordinal` (monotonic)."""
        steps = self.tick_steps
        while self.tick_step_index < len(steps) and steps[self.tick_step_index]["frame"] <= ordinal:
            step = steps[self.tick_step_index]
            self.tick_held[int(step.get("port", 0))] = step
            self.tick_step_index += 1

    def inject_tick_pads(self, ordinal: int) -> None:
        """Make tick record `ordinal` consume exactly its scheduled raw pads."""
        self.advance_tick_steps(ordinal)
        qnum = self.mem.read_u8(PAD_LIB_ADDR)
        qwrite = self.mem.read_u8(PAD_LIB_ADDR + 2)
        queue = self.mem.read_u32(PAD_LIB_ADDR + 8)
        slot = (qwrite + qnum - 1) % qnum  # the latest poll; ports 2-3 and err stay as polled
        entries = [(slot, self.tick_held)]
        if self.gecko_hooks:
            if qnum != UCF_QUEUE_WRAP:
                raise ValueError(f"pad queue holds {qnum} entries; UCF assumes {UCF_QUEUE_WRAP}")
            # Entries slot-1, slot-2: the previous ticks' pads (neutral before the first).
            for back in (1, 2):
                held = self.tick_history[back - 1] if len(self.tick_history) >= back \
                    else {port: {} for port in self.tick_held}
                entries.append(((slot - back) % qnum, held))
        for entry, held in entries:
            for port, step in sorted(held.items()):
                base = queue + entry * PAD_ENTRY_BYTES + port * PAD_STATUS_BYTES
                for offset, byte in enumerate(raw_pad_bytes(step.get("raw", {}))):
                    self.mem.write_u8(base + offset, byte)
        self.tick_history = [dict(self.tick_held), *self.tick_history][:2]
        self.mem.write_u8(PAD_LIB_ADDR + 1, slot)  # qread
        self.mem.write_u8(PAD_LIB_ADDR + 3, 1)     # qcount
        self.injected_ticks += 1

    def read_pad_queue_x(self, mem=None) -> list[list[int]]:
        """Per port, the signed stickX of queue entries qread-1 and qread-3,
        indexed as UCF's FETCH_INPUT does (index-1, plus 5 when negative)."""
        mem = self.mem if mem is None else mem
        qread = mem.read_u8(PAD_LIB_ADDR + 1)
        queue = mem.read_u32(PAD_LIB_ADDR + 8)

        def entry(index: int) -> int:
            index -= 1
            return index + UCF_QUEUE_WRAP if index < 0 else index

        def stick_x(index: int, port: int) -> int:
            byte = mem.read_u8(queue + entry(index) * PAD_ENTRY_BYTES + port * PAD_STATUS_BYTES + 2)
            return byte - 0x100 if byte & 0x80 else byte

        return [[stick_x(qread, port), stick_x(qread - 2, port)] for port in range(PAD_PORTS)]

    def read_pad_queue_sticks(self, mem=None) -> list[list[int]]:
        """Per port, entry qread-1's signed stickX, stickY, substickX, substickY."""
        mem = self.mem if mem is None else mem
        qread = mem.read_u8(PAD_LIB_ADDR + 1)
        queue = mem.read_u32(PAD_LIB_ADDR + 8)
        entry = qread - 1 + UCF_QUEUE_WRAP if qread == 0 else qread - 1

        def signed(addr: int) -> int:
            byte = mem.read_u8(addr)
            return byte - 0x100 if byte & 0x80 else byte

        base = queue + entry * PAD_ENTRY_BYTES
        return [[signed(base + port * PAD_STATUS_BYTES + 2 + i) for i in range(4)]
                for port in range(PAD_PORTS)]

    def check_gecko_hooks(self) -> None:
        """Every Gecko C2 injection must hold its branch by the first record."""
        missing = [f"0x{addr:08X}" for addr in self.gecko_hooks
                   if not gecko.is_branch(self.mem.read_u32(addr))]
        if missing:
            raise ValueError(f"Gecko codes not installed at {missing}: enable cheats "
                             "(DOLPHIN_CHEATS=1) and check the user folder's GALE01.ini")

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
            if self.tick_clock:
                # Keep Dolphin's own polls equal to the injected pad; the queue
                # rewrite at each tick end is what the game actually consumes.
                for port, step in sorted(self.tick_held.items()):
                    self.ctl.set_gc_buttons(port, {**remote_proto.neutral_inputs(),
                                                   **step.get("buttons", {})})
            else:
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
        if self.after_map and addr == CURRENT_GOBJ_ADDR:
            if self.in_callback:
                return
            self.in_callback = True   # a failure must not remove memchecks from here
            try:
                self.sample_after_map(is_write, value)
            except Exception:
                self.fail(traceback.format_exc())
            finally:
                self.in_callback = False
            return
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
            elif self.frame and value == 0 and before == self.last_tick \
                    and (self.human_ports or self.tick_clock):
                # A human or bridged match ends when a player runs out of stocks: the
                # GAME scene resets the counter ~113 frames after the last KO. Stop
                # there and report the tick count actually recorded.
                self.ended_early = True
                self.scenario["frames"] = self.frame
                self.pending_finish = True
                return
            elif before != self.last_tick or value != (before + 1) & MASK:
                raise ValueError(f"tick discontinuity at ordinal {self.frame}: "
                                 f"last={self.last_tick}, memory={before}, callback={value}")
            if self.frame == 0 and self.gecko_hooks:
                self.check_gecko_hooks()
            record = self.record("frame_end")
            if not record["fighters"]:
                raise ValueError(f"no fighters at tick ordinal {self.frame}")
            record.update(tick=value, vi_frame=self.vi_frame,
                          ps_frame=self.mem.read_u8(PS_FRAME_ADDR),
                          watch_address=WATCH_ADDR, watch_value=before,
                          pad_game=self.read_game_pads(),
                          pad_queue_x=self.read_pad_queue_x(),
                          pad_queue_sticks=self.read_pad_queue_sticks())
            self.out.write(json.dumps(record) + "\n")
            self.out.flush()
            self.last_tick = value
            self.last_tick_vi = self.vi_frame
            self.frame += 1
            if self.frame == self.scenario["frames"]:
                self.pending_finish = True
            elif self.tick_clock:
                self.inject_tick_pads(self.frame)
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
            if self.after_map:
                self.mem.remove_memcheck(CURRENT_GOBJ_ADDR)
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
                "counter_reset_at_start": self.counter_reset_at_start,
                "input_clock": "tick" if self.tick_clock else "vi",
                **({"after_map_samples": self.after_map_count} if self.after_map else {}),
                **({"injected_ticks": self.injected_ticks} if self.tick_clock else {})}


# Only when Dolphin runs this file directly; importing it (rng_ledger.py) must not start a tracer.
if event is not None and __name__ == "__main__":
    run(TickTracer)
