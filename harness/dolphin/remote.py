"""Remote control for a running Dolphin: runs INSIDE the scripting fork.

Launch (or use `drive.py launch`, which runs exactly this):

    ~/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
        -e /abs/harness/roms/GALE01.iso --script /abs/harness/dolphin/remote.py

Every frame it
  (a) re-reads harness/roms/.remote/cmd.json when its mtime changed and applies
      the commands (see remote_proto.py for the protocol),
  (b) re-issues the queued GC inputs for this frame,
  (c) every STATUS_EVERY frames writes status.json: frame counter, wall-clock
      fps, RNG seed, the fighters found by walk.py (address, kind, position),
      and the raw pad state the game sees,
  (d) appends errors to log.txt instead of dying.

Drive it from a shell with drive.py:  cd harness && uv run python dolphin/drive.py "press Start 3"

Dolphin does not define __file__ for --script files; the code object's filename
is used instead (docs/DOLPHIN_RUN.md).
"""
from __future__ import annotations

import json
import os
import sys
import time
import traceback
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE.parent))  # harness/  -> symbols
sys.path.insert(0, str(HERE))         # harness/dolphin -> walk, remote_proto
import remote_proto as proto  # noqa: E402
import symbols  # noqa: E402
import walk  # noqa: E402

try:
    from dolphin import controller, emulation, event, gui, memory, savestate  # type: ignore
except ImportError:  # importable outside Dolphin for tests
    controller = emulation = event = gui = memory = savestate = None

REMOTE_DIR = Path(os.environ.get("MELEE_REMOTE_DIR", HERE.parent / "roms" / ".remote"))
CMD_FILE = REMOTE_DIR / "cmd.json"
STATUS_FILE = REMOTE_DIR / "status.json"
LOG_FILE = REMOTE_DIR / "log.txt"
PORT = int(os.environ.get("MELEE_REMOTE_PORT", "0"))   # pad whose raw state goes in status

SEED_ADDR = symbols.addr("seed")
ENTITIES_ADDR = symbols.addr("HSD_GObj_Entities")
PAD_MASTER_ADDR = symbols.addr("HSD_PadMasterStatus")


class Remote:
    """One instance per Dolphin process; `on_frame` is the frameadvance callback."""

    def __init__(self, mem=None, ctl=None, states=None, emu=None, osd=None, events=None) -> None:
        self.mem = memory if mem is None else mem
        self.ctl = controller if ctl is None else ctl
        self.states = savestate if states is None else states
        self.emu = emulation if emu is None else emu
        self.gui = gui if osd is None else osd
        self.events = event if events is None else events
        self.shot_path: Path | None = None   # pending screenshot, consumed by on_frame
        self.watches: dict[int, int] = {}    # addr -> size, dumped into every status
        self.save_when_wait_path: str | None = None
        self.save_when_fighters_path: str | None = None
        self.frame = 0
        self.queue = proto.InputQueue()
        self.cmds = proto.CommandFile(CMD_FILE)
        self.fps = proto.FpsMeter()
        self.last_seed: int | None = None
        self.held: dict | None = None
        self.errors = 0
        self.note = "started"
        self.stopped = False
        REMOTE_DIR.mkdir(parents=True, exist_ok=True)

    # --- logging -----------------------------------------------------------
    def log(self, msg: str) -> None:
        self.errors += 1
        with LOG_FILE.open("a") as f:
            f.write(f"{time.strftime('%H:%M:%S')} frame={self.frame} {msg}\n")

    # --- proto.Host --------------------------------------------------------
    def save_state(self, path: str) -> None:
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        self.states.save_to_file(path)
        sidecar = {
            "savestate": path,
            "saved_at": time.strftime("%Y-%m-%d %H:%M:%S"),
            "frame": self.frame,
            "seed": self.read_seed(),
            "fighters": self.read_fighters(),
        }
        proto.write_json_atomic(Path(path + ".json"), sidecar)
        self.note = f"saved {path}"

    def load_state(self, path: str) -> None:
        self.states.load_from_file(path)
        self.note = f"loaded {path}; seed now 0x{self.read_seed():08X}"

    def save_when_wait(self, path: str) -> None:
        """Arm a save for the first frame where every fighter is in Wait."""
        self.save_when_wait_path = path
        self.note = f"armed save-when-wait {path}"

    def save_when_fighters(self, path: str) -> None:
        """Arm a save for the first frame where any valid fighter exists (match start)."""
        self.save_when_fighters_path = path
        self.note = f"armed save-when-fighters {path}"

    def check_save_when_fighters(self) -> None:
        if self.save_when_fighters_path is None:
            return
        fighters = self.read_fighters()
        # The first VI frame with a fighter GObj can precede Fighter init
        # (garbage motion id, zero position, second fighter absent); wait for
        # a full, initialised roster.
        if len(fighters) >= 2 and all(proto.fighter_looks_valid(f) and 0 <= f["motion_id"] < 1000
                                      for f in fighters):
            path, self.save_when_fighters_path = self.save_when_fighters_path, None
            self.save_state(path)
            self.note = f"save-when-fighters fired at frame {self.frame}: {path}"

    def check_save_when_wait(self) -> None:
        if self.save_when_wait_path is None:
            return
        if proto.all_waiting(self.read_fighters()):
            path, self.save_when_wait_path = self.save_when_wait_path, None
            self.save_state(path)
            self.note = f"save-when-wait fired at frame {self.frame}: {path}"
            self.write_status()

    def pause(self) -> None:
        self.emu.pause()

    def resume(self) -> None:
        self.emu.resume()

    def osd(self, text: str) -> None:
        self.gui.add_osd_message(text, 3000)

    def shot(self, path: str) -> None:
        """Save the next presented frame as PNG (the emulator's own pixels, not the screen).

        `event.on_framedrawn` listeners can only be replaced, never removed
        (None is rejected), and a permanent one forces a GPU readback every
        frame. `await event.framedrawn()` is a one-shot listener instead; the
        coroutine is started by returning it from the frameadvance callback.
        """
        self.shot_path = Path(path)
        self.shot_path.parent.mkdir(parents=True, exist_ok=True)

    SHOT_WARMUP_FRAMES = 3   # frame dumping only runs while a listener exists;
                             # the first readbacks return the stale buffer from
                             # the previous shot (verified: shots lagged by one)

    async def take_shot(self, path: Path):
        try:
            for _ in range(self.SHOT_WARMUP_FRAMES):
                width, height, data = await self.events.framedrawn()
            proto.write_png(path, width, height, data)
            self.note = f"shot {path} ({width}x{height})"
        except Exception as e:  # noqa: BLE001
            self.log(f"screenshot failed: {e!r}")
        self.write_status()

    def poke(self, addr: int, width: int, value: int, or_into: bool) -> None:
        """Write (or OR into) one u8/u16/u32 of emulated memory. Used to flip
        save-data bits such as the stage unlock mask; never part of a trace."""
        read = getattr(self.mem, f"read_u{width}")
        write = getattr(self.mem, f"write_u{width}")
        before = read(addr)
        new = (before | value) if or_into else value
        write(addr, new)
        self.note = f"poke 0x{addr:08X} u{width}: 0x{before:X} -> 0x{read(addr):X}"
        self.log(self.note)

    def watch(self, addr: int, size: int) -> None:
        self.watches[addr] = size

    def unwatch(self, addr: int | None) -> None:
        if addr is None:
            self.watches.clear()
        else:
            self.watches.pop(addr, None)

    def read_watches(self) -> dict[str, str]:
        out = {}
        for addr, size in self.watches.items():
            try:
                out[f"0x{addr:08X}"] = walk.read_bytes(self.mem, addr, size).hex()
            except Exception as e:  # noqa: BLE001
                out[f"0x{addr:08X}"] = f"error: {e!r}"
        return out

    def stop(self) -> None:
        self.stopped = True
        self.note = "stopped"
        self.write_status()
        # Listeners cannot be removed, only replaced: park a no-op.
        self.events.on_frameadvance(lambda: None)

    # --- memory ------------------------------------------------------------
    def read_seed(self) -> int:
        return self.mem.read_u32(SEED_ADDR)

    def read_fighters(self) -> list[dict]:
        # The entity list is garbage before a match; treat any failure as "none".
        try:
            bases = walk.fighter_bases(self.mem, ENTITIES_ADDR)
            out = [proto.fighter_summary(self.mem, b) for b in bases]
            return [f for f in out if proto.fighter_looks_valid(f)]
        except Exception as e:  # noqa: BLE001
            self.log(f"fighter walk failed: {e!r}")
            return []

    def read_pad(self) -> dict | None:
        try:
            return proto.pad_summary(self.mem, PAD_MASTER_ADDR, PORT)
        except Exception as e:  # noqa: BLE001
            self.log(f"pad read failed: {e!r}")
            return None

    # --- status ------------------------------------------------------------
    def write_status(self) -> None:
        seed = self.read_seed()
        status = proto.build_status(
            pid=os.getpid(), frame=self.frame, fps=self.fps.sample(self.frame),
            seed=seed, seed_changed=(self.last_seed is not None and seed != self.last_seed),
            last_seq=self.cmds.last_seq, queue=self.queue, fighters=self.read_fighters(),
            pad=self.read_pad(), errors=self.errors, held=self.held, note=self.note,
            watch=self.read_watches(),
        )
        self.last_seed = seed
        proto.write_json_atomic(STATUS_FILE, status)

    # --- per frame ---------------------------------------------------------
    def poll_commands(self) -> bool:
        try:
            batch = self.cmds.poll()
        except (OSError, json.JSONDecodeError) as e:
            self.log(f"cmd.json unreadable: {e!r}")
            return False
        if batch is None:
            return False
        seq, commands = batch
        for cmd in commands:
            try:
                proto.apply_command(cmd, self.queue, self)
            except Exception as e:  # noqa: BLE001
                self.log(f"seq {seq} command {cmd!r} failed: {e!r}")
        return True

    def on_frame(self):
        """Frameadvance callback. Returns a coroutine (a pending screenshot) or None."""
        try:
            self.frame += 1
            got_commands = self.poll_commands()
            self.check_save_when_wait()      # before this frame's inputs: the saved state is untouched
            self.check_save_when_fighters()
            step = self.queue.next_frame()
            if step is not None:
                port, inputs = step
                self.ctl.set_gc_buttons(port, inputs)
                self.held = {"port": port, **inputs}
            else:
                self.held = None
            if got_commands or self.frame % proto.STATUS_EVERY == 0:
                self.write_status()
            if self.shot_path is not None:
                path, self.shot_path = self.shot_path, None
                return self.take_shot(path)
        except Exception:  # noqa: BLE001
            self.log(traceback.format_exc())
        return None


def main() -> None:
    remote = Remote()
    for stale in (CMD_FILE, STATUS_FILE):
        stale.unlink(missing_ok=True)   # commands from a previous run must not replay
    LOG_FILE.write_text("")
    event.on_frameadvance(remote.on_frame)
    print(f"remote.py: listening on {REMOTE_DIR} (pid {os.getpid()})")


if event is not None:
    main()
