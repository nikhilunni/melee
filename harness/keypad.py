"""Terminal gamepad for human-port recordings: Ghostty keys -> a pad-state file.

    cd harness && uv run python keypad.py            # keep this terminal focused while playing

macOS Secure Keyboard Entry blanks the global key state that Dolphin's keyboard
backend polls, but the focused terminal still receives its own key events. This
program asks the terminal for the kitty keyboard protocol with press/repeat/release
events (Ghostty supports it), tracks the held keys and writes the GameCube pad state
to `roms/.remote/keypad.json` on every change. The tick tracer, run with
MELEE_KEYPAD pointing at that file, applies it to the human ports each VI frame.

Keys: arrows = control stick (Shift = half tilt), X/Z/C/S = A/B/X/Y, D = Z,
Q/W = L/R (digital, full press), I/J/K/L = C-stick, T/F/G/H = D-pad.
Ctrl-C quits (and releases everything).
"""
from __future__ import annotations

import json
import os
import re
import sys
import termios
import tty
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = Path(os.environ.get("MELEE_KEYPAD", HERE / "roms" / ".remote" / "keypad.json"))

# kitty functional key codes for arrows (legacy CSI A/B/C/D carry code 1).
ARROWS = {"A": "Up", "B": "Down", "C": "Right", "D": "Left"}
BUTTONS = {"x": "A", "z": "B", "c": "X", "s": "Y", "d": "Z", "q": "L", "w": "R",
           "t": "Up", "g": "Down", "f": "DLeft", "h": "DRight"}
CSTICK = {"i": ("CStickY", 1.0), "k": ("CStickY", -1.0), "j": ("CStickX", -1.0), "l": ("CStickX", 1.0)}
CSI = re.compile(rb"\x1b\[([0-9:;]*)([uABCDHF~])")


def pad_state(held: set[str], shift: bool) -> dict:
    tilt = 0.5 if shift else 1.0
    x = (tilt if "Right" in held else 0.0) - (tilt if "Left" in held else 0.0)
    y = (tilt if "Up" in held else 0.0) - (tilt if "Down" in held else 0.0)
    pad = {"A": False, "B": False, "X": False, "Y": False, "Z": False, "Start": False,
           "Up": False, "Down": False, "Left": False, "Right": False, "L": False, "R": False,
           "StickX": x, "StickY": y, "CStickX": 0.0, "CStickY": 0.0,
           "TriggerLeft": 0.0, "TriggerRight": 0.0}
    for key, button in BUTTONS.items():
        if key in held:
            if button == "DLeft":
                pad["Left"] = True
            elif button == "DRight":
                pad["Right"] = True
            elif button in ("Up", "Down"):
                pad[button] = True  # D-pad via T/G; the stick uses the arrow names
            else:
                pad[button] = True
    for key, (axis, value) in CSTICK.items():
        if key in held:
            pad[axis] = value
    pad["TriggerLeft"] = 1.0 if pad["L"] else 0.0
    pad["TriggerRight"] = 1.0 if pad["R"] else 0.0
    return pad


def write(pad: dict, seq: int) -> None:
    tmp = OUT.with_suffix(".tmp")
    tmp.write_text(json.dumps({"seq": seq, "pad": pad}))
    os.replace(tmp, OUT)


def main() -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    fd = sys.stdin.fileno()
    saved = termios.tcgetattr(fd)
    held: set[str] = set()
    shift = False
    seq = 0
    write(pad_state(held, shift), seq)
    # Push kitty flags 1 (disambiguate) | 2 (event types) | 8 (all keys as escapes).
    sys.stdout.write("\x1b[>11u")
    sys.stdout.write("keypad: writing %s. Keep this window focused. Ctrl-C quits.\r\n" % OUT)
    sys.stdout.flush()
    tty.setraw(fd)
    buf = b""
    last = None
    try:
        while True:
            chunk = os.read(fd, 1024)
            if not chunk:
                break
            buf += chunk
            while True:
                m = CSI.search(buf)
                if not m:
                    if len(buf) > 64:
                        buf = b""
                    break
                buf = buf[m.end():]
                params, final = m.group(1).decode(), m.group(2).decode()
                fields = params.split(";") if params else []
                code = fields[0].split(":")[0] if fields else "1"
                mods, event = 1, 1
                if len(fields) > 1:
                    parts = fields[1].split(":")
                    mods = int(parts[0] or 1)
                    event = int(parts[1]) if len(parts) > 1 and parts[1] else 1
                if final in ARROWS:
                    name = ARROWS[final]
                elif final == "u" and code.isdigit():
                    cp = int(code)
                    if cp == 3 and (mods - 1) & 4:
                        raise KeyboardInterrupt
                    if cp in (57441, 57447):   # left/right shift as keys
                        shift = event != 3
                        name = None
                    elif cp == 99 and (mods - 1) & 4:
                        raise KeyboardInterrupt
                    else:
                        name = chr(cp).lower() if 32 < cp < 127 else None
                else:
                    name = None
                shift = shift or bool((mods - 1) & 1) if event != 3 else shift
                if name is not None:
                    if event == 3:
                        held.discard(name)
                    else:
                        held.add(name)
                pad = pad_state(held, bool((mods - 1) & 1) or shift)
                if pad != last:
                    seq += 1
                    write(pad, seq)
                    last = pad
                    shown = " ".join(k for k, v in pad.items() if v is True)
                    sys.stdout.write(f"\r\x1b[Kstick ({pad['StickX']:+.1f},{pad['StickY']:+.1f}) {shown}")
                    sys.stdout.flush()
    except KeyboardInterrupt:
        pass
    finally:
        write(pad_state(set(), False), seq + 1)
        termios.tcsetattr(fd, termios.TCSADRAIN, saved)
        sys.stdout.write("\x1b[<u\r\nkeypad: released all keys.\r\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
