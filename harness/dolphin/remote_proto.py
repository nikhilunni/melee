"""Protocol shared by remote.py (inside Dolphin) and drive.py (a normal shell).

No Dolphin import here, so everything is unit-testable (tests/test_remote.py).

Files, all under harness/roms/.remote/ (gitignored with the rest of roms/):

    cmd.json     written atomically by drive.py:
                 {"seq": <int, increases>, "commands": [<command>, ...]}
                 remote.py re-reads it when the mtime changes and applies the
                 commands once when `seq` is greater than the last one applied.
    status.json  written atomically by remote.py every STATUS_EVERY frames and
                 after every command batch. See `build_status` for the shape.
    log.txt      one line per error, appended by remote.py.

Commands are JSON objects with an "op" key:

    {"op": "input", "inputs": {"A": true, "StickX": 1.0}, "frames": 3, "port": 0}
        queue `frames` frames holding these GC inputs on that controller
        (0-based Dolphin port, default 0), then go neutral
    {"op": "wait", "frames": 30}          queue `frames` neutral frames
    {"op": "clear"}                       drop every queued input step
    {"op": "save", "path": "/abs/x.sav"}  savestate + sidecar /abs/x.sav.json
    {"op": "load", "path": "/abs/x.sav"}
    {"op": "save_when_wait", "path": "/abs/x.sav"}
                                          save at the first frame where every
                                          fighter's motion_id == MS_WAIT (the
                                          first playable frame of a match)
    {"op": "pause"} / {"op": "resume"}    dolphin.emulation
    {"op": "osd", "text": "..."}          on-screen message
    {"op": "shot", "path": "/abs/x.png"}  save the next rendered frame as PNG
                                          (event.on_framedrawn; works even when
                                          the Dolphin window is covered)
    {"op": "watch", "addr": 2153342740, "size": 4}
                                          include these bytes (hex) in every
                                          status.json under "watch"
    {"op": "unwatch", "addr": ... | null} drop one watch, or all
    {"op": "status"}                      write status.json now
    {"op": "stop"}                        unregister the frame callback

`parse_command` turns the shell grammar used by drive.py into those objects:

    press A                  press A+Start 3        (buttons, default 2 frames)
    hold StickX 1.0 30       hold StickX 255 30     (-1..1 float, or raw 0..255)
    stick 0 -1 5             main stick x y for N frames
    press A 3 @1             any input command may end with @<port> (P2 is @1)
    watch 0x804D6714 4       unwatch 0x804D6714     unwatch all
    save-when-wait /abs/x.sav
    wait 30 | clear | save P | load P | shot P | pause | resume | osd text... | status | stop
"""
from __future__ import annotations

import json
import math
import os
import struct
import time
import zlib
from collections import deque
from pathlib import Path
from typing import Protocol

# --- GC input keys, exactly as dolphin.controller.set_gc_buttons accepts them ---
GC_BUTTONS = ("A", "B", "X", "Y", "Z", "Start", "Up", "Down", "Left", "Right", "L", "R")
GC_STICKS = ("StickX", "StickY", "CStickX", "CStickY")   # float -1..1
GC_TRIGGERS = ("TriggerLeft", "TriggerRight")            # float 0..1
GC_KEYS = GC_BUTTONS + GC_STICKS + GC_TRIGGERS

DEFAULT_PRESS_FRAMES = 2
STATUS_EVERY = 10          # frames between status.json writes
RAW_STICK_CENTER = 128     # GC hardware units, 0..255

# Fighter fields read for the status file (harness/schema/fighter.yaml).
FIGHTER_KIND_OFF = 0x004     # s32 FighterKind
FIGHTER_PLAYER_ID_OFF = 0x00C
FIGHTER_MOTION_ID_OFF = 0x010  # s32 action state
FIGHTER_POS_OFF = 0x0B0      # Vec3 cur_pos
FIGHTER_KIND_NAMES = [       # ft/forward.h FighterKind, index == value
    "Mario", "Fox", "Captain", "Donkey", "Kirby", "Koopa", "Link", "Seak", "Ness",
    "Peach", "Popo", "Nana", "Pikachu", "Samus", "Yoshi", "Purin", "Mewtwo", "Luigi",
    "Mars", "Zelda", "CLink", "DrMario", "Falco", "Pichu", "GameWatch", "Ganon",
    "Emblem", "MasterHand", "CrazyHand", "Boy", "Girl", "GigaKoopa", "Sandbag",
]
# ftCommon forward.h ftCommon_MotionState: DeadDown = 0 ... Wait = 14.
MS_WAIT = 14

# HSD_PadStatus (sysdolphin/baselib/controller.h): button u32 at +0, stickX s8 at +0x18.
PAD_STATUS_SIZE = 0x2C
PAD_BUTTON_OFF = 0x00
PAD_STICK_X_OFF = 0x18
PAD_STICK_Y_OFF = 0x19


def neutral_inputs() -> dict:
    out: dict = {k: False for k in GC_BUTTONS}
    out.update({k: 0.0 for k in GC_STICKS})
    out.update({k: 0.0 for k in GC_TRIGGERS})
    return out


class CommandError(ValueError):
    """Bad shell-grammar command."""


def _analog_value(text: str) -> float:
    """Parse an analog value: -1..1 as given, or raw GC units 0..255 (ints > 1)."""
    try:
        v = float(text)
    except ValueError as e:
        raise CommandError(f"analog value must be a number, got {text!r}") from e
    if -1.0 <= v <= 1.0:
        return v
    if text.lstrip("+").isdigit() and 0 <= v <= 255:
        return (v - RAW_STICK_CENTER) / RAW_STICK_CENTER
    raise CommandError(f"analog value {text!r} not in -1..1 or 0..255")


def _frames(text: str | None, default: int | None = None) -> int:
    if text is None:
        if default is None:
            raise CommandError("frame count required")
        return default
    if not text.isdigit() or int(text) <= 0:
        raise CommandError(f"frame count must be a positive integer, got {text!r}")
    return int(text)


def _port(text: str) -> int:
    """`@N` suffix: 0-based Dolphin controller index (GameCube has 4 ports)."""
    if not text[1:].isdigit() or not 0 <= int(text[1:]) <= 3:
        raise CommandError(f"port must be @0..@3, got {text!r}")
    return int(text[1:])


def _address(text: str) -> int:
    """Guest address: hex (0x8xxxxxxx) or decimal, inside MEM1's cached range."""
    try:
        addr = int(text, 0)
    except ValueError as e:
        raise CommandError(f"address must be an integer, got {text!r}") from e
    if not 0x8000_0000 <= addr < 0x8180_0000:
        raise CommandError(f"address {text} outside MEM1 (0x80000000..0x817FFFFF)")
    return addr


_KEY_BY_LOWER = {k.lower(): k for k in GC_KEYS}


def _check_key(key: str) -> str:
    """Canonical GC key name (case-insensitive lookup)."""
    try:
        return _KEY_BY_LOWER[key.lower()]
    except KeyError as e:
        raise CommandError(f"unknown GC input {key!r}; known: {', '.join(GC_KEYS)}") from e


def parse_command(text: str) -> dict:
    """Shell grammar -> command object. Raises CommandError on bad input."""
    parts = text.strip().split()
    if not parts:
        raise CommandError("empty command")
    op, args = parts[0].lower(), parts[1:]

    port = 0
    if op in ("press", "hold", "stick") and args and args[-1].startswith("@"):
        port = _port(args.pop())

    if op == "press":
        if not args:
            raise CommandError("press needs a button, e.g. `press A` or `press A+Start 3`")
        keys = [_check_key(k) for k in args[0].split("+")]
        for k in keys:
            if k not in GC_BUTTONS:
                raise CommandError(f"press only takes buttons; use `hold {k} <value> <frames>`")
        if len(args) > 2:
            raise CommandError("press takes at most: <buttons> [frames]")
        return {"op": "input", "inputs": {k: True for k in keys},
                "frames": _frames(args[1] if len(args) > 1 else None, DEFAULT_PRESS_FRAMES),
                "port": port}

    if op == "hold":
        if len(args) != 3:
            raise CommandError("hold takes exactly: <key> <value> <frames>")
        key = _check_key(args[0])
        if key in GC_BUTTONS:
            if args[1].lower() not in ("true", "false", "1", "0"):
                raise CommandError(f"{key} is a button; value must be true/false")
            value: bool | float = args[1].lower() in ("true", "1")
        else:
            value = _analog_value(args[1])
            if key in GC_TRIGGERS and value < 0:
                raise CommandError("trigger values are 0..1")
        return {"op": "input", "inputs": {key: value}, "frames": _frames(args[2]), "port": port}

    if op == "stick":
        if len(args) != 3:
            raise CommandError("stick takes exactly: <x> <y> <frames>")
        return {"op": "input",
                "inputs": {"StickX": _analog_value(args[0]), "StickY": _analog_value(args[1])},
                "frames": _frames(args[2]), "port": port}

    if op == "wait":
        if len(args) != 1:
            raise CommandError("wait takes exactly: <frames>")
        return {"op": "wait", "frames": _frames(args[0])}

    if op in ("save", "load", "shot", "save-when-wait"):
        if len(args) != 1:
            raise CommandError(f"{op} takes exactly: <path>")
        path = Path(args[0])
        if not path.is_absolute():
            raise CommandError(f"{op} path must be absolute (Dolphin's cwd is not ours): {args[0]}")
        return {"op": op.replace("-", "_"), "path": str(path)}

    if op == "osd":
        if not args:
            raise CommandError("osd needs text")
        return {"op": "osd", "text": " ".join(args)}

    if op == "watch":
        if len(args) != 2:
            raise CommandError("watch takes exactly: <addr> <size>")
        return {"op": "watch", "addr": _address(args[0]), "size": _frames(args[1])}

    if op == "unwatch":
        if len(args) != 1:
            raise CommandError("unwatch takes exactly: <addr> | all")
        return {"op": "unwatch", "addr": None if args[0].lower() == "all" else _address(args[0])}

    if op in ("poke", "poke-or"):
        # poke <addr> <u8|u16|u32> <value>: write; poke-or: OR the value in.
        if len(args) != 3 or args[1].lower() not in ("u8", "u16", "u32"):
            raise CommandError(f"{op} takes exactly: <addr> <u8|u16|u32> <value>")
        width = int(args[1][1:])
        value = int(args[2], 0)
        if not 0 <= value < (1 << width):
            raise CommandError(f"{op}: value {args[2]} does not fit in {width} bits")
        return {"op": "poke", "addr": _address(args[0]), "width": width,
                "value": value, "or": op == "poke-or"}

    if op in ("clear", "pause", "resume", "status", "stop"):
        if args:
            raise CommandError(f"{op} takes no arguments")
        return {"op": op}

    raise CommandError(f"unknown command {op!r}")


def parse_commands(texts: list[str]) -> list[dict]:
    """Parse several commands; each string may hold several separated by ';'."""
    out = []
    for text in texts:
        for piece in text.split(";"):
            if piece.strip():
                out.append(parse_command(piece))
    return out


# --- atomic JSON files ------------------------------------------------------

def write_json_atomic(path: Path, obj) -> None:
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(obj, indent=1))
    os.replace(tmp, path)   # readers never see a partial file


def read_json(path: Path):
    return json.loads(path.read_text())


class CommandFile:
    """Reader side of cmd.json: yields each new batch exactly once."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.last_mtime: float | None = None
        self.last_seq = -1

    def poll(self) -> tuple[int, list[dict]] | None:
        try:
            mtime = self.path.stat().st_mtime_ns
        except FileNotFoundError:
            return None
        if mtime == self.last_mtime:
            return None
        self.last_mtime = mtime
        data = read_json(self.path)
        seq = int(data.get("seq", -1))
        if seq <= self.last_seq:
            return None
        self.last_seq = seq
        return seq, list(data.get("commands", []))


def next_seq(path: Path) -> int:
    """Writer side helper: one more than the seq currently in cmd.json."""
    try:
        return int(read_json(path).get("seq", -1)) + 1
    except (FileNotFoundError, ValueError, json.JSONDecodeError):
        return 0


# --- input queue -------------------------------------------------------------

class InputQueue:
    """FIFO of (inputs, frames, port) steps, consumed one frame at a time.

    `set_gc_buttons` overrides last a single frame, so the current step is
    re-issued each frame. When a step ends the queue emits one explicit
    neutral frame on that port so a held button is released even if the next
    step omits it.
    """

    def __init__(self) -> None:
        self._steps: deque[tuple[dict, int, int]] = deque()
        self._release_port: int | None = None

    def enqueue(self, inputs: dict, frames: int, port: int = 0) -> None:
        if frames <= 0:
            raise ValueError("frames must be positive")
        if not 0 <= port <= 3:
            raise ValueError("port must be 0..3")
        for k in inputs:
            _check_key(k)
        self._steps.append((dict(inputs), frames, port))

    def enqueue_wait(self, frames: int) -> None:
        self.enqueue({}, frames)

    def clear(self) -> None:
        if self._steps:
            self._release_port = self._steps[0][2]
        self._steps.clear()

    @property
    def remaining_frames(self) -> int:
        return sum(n for _, n, _ in self._steps)

    def idle(self) -> bool:
        return not self._steps and self._release_port is None

    def next_frame(self) -> tuple[int, dict] | None:
        """(port, inputs) to apply this frame, or None when nothing is queued."""
        if not self._steps:
            if self._release_port is not None:
                port, self._release_port = self._release_port, None
                return port, neutral_inputs()
            return None
        inputs, frames, port = self._steps[0]
        frame_inputs = neutral_inputs()
        frame_inputs.update(inputs)
        if frames == 1:
            self._steps.popleft()
            self._release_port = port
        else:
            self._steps[0] = (inputs, frames - 1, port)
        return port, frame_inputs


# --- command dispatch --------------------------------------------------------

class Host(Protocol):
    """What remote.py provides to `apply_command`; faked in tests."""

    def save_state(self, path: str) -> None: ...
    def load_state(self, path: str) -> None: ...
    def save_when_wait(self, path: str) -> None: ...
    def pause(self) -> None: ...
    def resume(self) -> None: ...
    def osd(self, text: str) -> None: ...
    def shot(self, path: str) -> None: ...
    def watch(self, addr: int, size: int) -> None: ...
    def unwatch(self, addr: int | None) -> None: ...
    def poke(self, addr: int, width: int, value: int, or_into: bool) -> None: ...
    def stop(self) -> None: ...
    def write_status(self) -> None: ...


def apply_command(cmd: dict, queue: InputQueue, host: Host) -> None:
    op = cmd.get("op")
    if op == "input":
        queue.enqueue(cmd["inputs"], int(cmd["frames"]), int(cmd.get("port", 0)))
    elif op == "wait":
        queue.enqueue_wait(int(cmd["frames"]))
    elif op == "clear":
        queue.clear()
    elif op == "save":
        host.save_state(cmd["path"])
    elif op == "load":
        host.load_state(cmd["path"])
    elif op == "save_when_wait":
        host.save_when_wait(cmd["path"])
    elif op == "pause":
        host.pause()
    elif op == "resume":
        host.resume()
    elif op == "osd":
        host.osd(str(cmd["text"]))
    elif op == "shot":
        host.shot(cmd["path"])
    elif op == "watch":
        host.watch(int(cmd["addr"]), int(cmd["size"]))
    elif op == "poke":
        host.poke(int(cmd["addr"]), int(cmd["width"]), int(cmd["value"]), bool(cmd["or"]))
    elif op == "unwatch":
        host.unwatch(None if cmd.get("addr") is None else int(cmd["addr"]))
    elif op == "status":
        host.write_status()
    elif op == "stop":
        host.stop()
    else:
        raise ValueError(f"unknown op {op!r}")


# --- screenshots ---------------------------------------------------------------

def write_png(path: Path, width: int, height: int, rgb: bytes) -> None:
    """Minimal PNG encoder for the RGB frame `event.on_framedrawn` delivers.

    The fork converts the presenter's RGBA rows to tightly packed RGB
    (eventmodule.cpp PyFrameDrawn), so `len(rgb) == width * height * 3`.
    """
    if len(rgb) != width * height * 3:
        raise ValueError(f"expected {width * height * 3} RGB bytes, got {len(rgb)}")
    raw = b"".join(b"\x00" + rgb[y * width * 3:(y + 1) * width * 3] for y in range(height))

    def chunk(kind: bytes, body: bytes) -> bytes:
        return (struct.pack(">I", len(body)) + kind + body
                + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF))

    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw, 6))
           + chunk(b"IEND", b""))
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_bytes(png)
    os.replace(tmp, path)


# --- status ------------------------------------------------------------------

class Memory(Protocol):
    def read_u8(self, addr: int) -> int: ...
    def read_u32(self, addr: int) -> int: ...
    def read_s32(self, addr: int) -> int: ...
    def read_s8(self, addr: int) -> int: ...
    def read_f32(self, addr: int) -> float: ...


def fighter_summary(mem: Memory, base: int) -> dict:
    kind = mem.read_s32(base + FIGHTER_KIND_OFF)
    pos = [mem.read_f32(base + FIGHTER_POS_OFF + 4 * i) for i in range(3)]
    return {
        "base": f"0x{base:08X}",
        "kind": kind,
        "kind_name": FIGHTER_KIND_NAMES[kind] if 0 <= kind < len(FIGHTER_KIND_NAMES) else "?",
        "player_id": mem.read_u8(base + FIGHTER_PLAYER_ID_OFF),
        "motion_id": mem.read_s32(base + FIGHTER_MOTION_ID_OFF),
        "pos": pos,
    }


def all_waiting(fighters: list[dict]) -> bool:
    """True when there is at least one fighter and every one is in ftCo_MS_Wait."""
    return bool(fighters) and all(f["motion_id"] == MS_WAIT for f in fighters)


def fighter_looks_valid(summary: dict) -> bool:
    return (0 <= summary["kind"] < len(FIGHTER_KIND_NAMES)
            and all(math.isfinite(v) and abs(v) < 1e5 for v in summary["pos"]))


def pad_summary(mem: Memory, pad_master_addr: int, port: int = 0) -> dict:
    base = pad_master_addr + port * PAD_STATUS_SIZE
    return {
        "button": f"0x{mem.read_u32(base + PAD_BUTTON_OFF):08X}",
        "stick_x": mem.read_s8(base + PAD_STICK_X_OFF),
        "stick_y": mem.read_s8(base + PAD_STICK_Y_OFF),
    }


class FpsMeter:
    def __init__(self, now: float | None = None) -> None:
        self.t0 = time.monotonic() if now is None else now
        self.f0 = 0
        self.fps = 0.0

    def sample(self, frame: int, now: float | None = None) -> float:
        """Frames per wall second since the previous sample; windows under
        MIN_WINDOW_S keep the last value (status is sometimes written twice
        in one frame)."""
        now = time.monotonic() if now is None else now
        dt = now - self.t0
        if dt >= self.MIN_WINDOW_S:
            self.fps = (frame - self.f0) / dt
            self.t0, self.f0 = now, frame
        return self.fps

    MIN_WINDOW_S = 0.05


def build_status(*, pid: int, frame: int, fps: float, seed: int, seed_changed: bool,
                 last_seq: int, queue: InputQueue, fighters: list[dict], pad: dict | None,
                 errors: int, held: dict | None, note: str = "",
                 watch: dict[str, str] | None = None) -> dict:
    return {
        "watch": watch or {},
        "pid": pid,
        "wall_time": time.time(),
        "frame": frame,
        "fps": round(fps, 2),
        "seed": seed,
        "seed_hex": f"0x{seed:08X}",
        "seed_changed": seed_changed,
        "last_seq": last_seq,
        "queue_frames": queue.remaining_frames,
        "idle": queue.idle(),
        "held": held or {},
        "fighters": fighters,
        "pad0": pad,
        "errors": errors,
        "note": note,
    }
