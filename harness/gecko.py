"""Gecko codes for a recording: a scenario's `gecko = ["ucf-0.8"]` list.

The code text is not in this repository (the Slippi codes are GPL-3). It is
read at record time from the directory named by MELEE_GECKO_DIR, one file per
code: `<name>.txt` (Dolphin/Slippi text, e.g. Slippi's
Output/Console/g_ucf.txt) or `<name>.bin` (the raw code list, 8 bytes per
line, e.g. g_ucf.bin). For UCF, from a slippi-ssbm-asm checkout:

    ucf-0.73.bin   git show 823067b^:Binary/UCF/Ucf0.73Beta.bin
    ucf-0.74.bin   git show b89e160^:Output/Console/g_ucf.bin
    ucf-0.8.txt    Output/Console/g_ucf.txt
    ucf-0.84.txt   Output/Console/g_ucf_084.txt
    dween.txt      the C206B028 block of Output/Console/g_toggles.bin at 009b155
                   (the toggle set's "Arduino" code), then `004DEC08 00030002`
                   (each port's toggle byte at 0x804DEC08 set to 2)

`install` writes the codes into a private Dolphin user folder's
GameSettings/GALE01.ini ([Gecko] and [Gecko_Enabled]); the recorder then
launches Dolphin with cheats enabled (dolphin_config: DOLPHIN_CHEATS=1).
Dolphin runs the code handler at every VI, so codes apply from the first
frame after a savestate that was saved without them. The tracer verifies
each C2 injection holds a branch (MELEE_GECKO_HOOKS).
"""
from __future__ import annotations

import os
import re
from pathlib import Path

DIR_ENV = "MELEE_GECKO_DIR"
HOOKS_ENV = "MELEE_GECKO_HOOKS"
GAME_INI = Path("GameSettings") / "GALE01.ini"
_LINE = re.compile(r"^([0-9A-Fa-f]{8})\s+([0-9A-Fa-f]{8})\b")
#: Gecko "insert ASM" (C2) codes: the address's instruction becomes a branch.
INSERT_ASM = 0xC2
#: PowerPC `b`/`bl`: primary opcode 18.
BRANCH_OPCODE = 18
#: Codes the port names with a scenario flag (melee-sim scenario.rs: a `gecko`
#: list naming the code and the flag go together; a cold twin, which has no
#: `gecko` list, carries the flag alone).
SCENARIO_FLAGS = {"ps-preload": "stadium_preload", "ps-frozen": "stadium_frozen",
                  "frozen-stages": "frozen_stages", "widescreen": "widescreen"}


def scenario_flags(codes: list[str] | None) -> dict[str, bool]:
    """The scenario flags a code list sets."""
    return {flag: True for code, flag in SCENARIO_FLAGS.items() if code in (codes or [])}


def scenario_flag_lines(flags: dict[str, bool]) -> str:
    """The set flags as top-level scenario TOML lines, in SCENARIO_FLAGS order."""
    return "".join(f"{flag} = true\n" for flag in SCENARIO_FLAGS.values() if flags.get(flag))


def code_lines(name: str, directory: Path | None = None) -> list[str]:
    """The code's `XXXXXXXX YYYYYYYY` lines, comments and titles dropped."""
    if directory is None:
        env = os.environ.get(DIR_ENV)
        if not env:
            raise SystemExit(f"scenario needs Gecko code {name!r}: set {DIR_ENV} to the directory "
                             f"holding {name}.txt or {name}.bin (see harness/gecko.py)")
        directory = Path(env)
    text, binary = directory / f"{name}.txt", directory / f"{name}.bin"
    if text.exists():
        lines = []
        for line in text.read_text().splitlines():
            match = _LINE.match(line.strip())
            if match:
                lines.append(f"{match[1].upper()} {match[2].upper()}")
        return lines
    if binary.exists():
        data = binary.read_bytes()
        if len(data) % 8:
            raise ValueError(f"{binary}: not a whole number of 8-byte code lines")
        return [f"{data[i:i + 4].hex().upper()} {data[i + 4:i + 8].hex().upper()}"
                for i in range(0, len(data), 8)]
    raise SystemExit(f"Gecko code {name!r} missing: neither {text} nor {binary} exists")


def injection_addresses(lines: list[str]) -> list[int]:
    """Each C2 code's target instruction (the ASM body lines are skipped)."""
    addresses, index = [], 0
    while index < len(lines):
        word, count = (int(part, 16) for part in lines[index].split())
        index += 1
        if word >> 24 == INSERT_ASM:
            addresses.append(0x80000000 | (word & 0x01FFFFFF))
            index += count  # the inserted instructions
    return addresses


def game_ini(codes: dict[str, list[str]]) -> str:
    """GALE01.ini enabling every code, in the given order."""
    out = ["[Gecko]"]
    for name, lines in codes.items():
        out += [f"${name}", *lines]
    out += ["", "[Gecko_Enabled]", *(f"${name}" for name in codes), ""]
    return "\n".join(out)


def install(user_dir: Path, names: list[str], directory: Path | None = None) -> list[int]:
    """Write the codes into `user_dir` and return their injection addresses."""
    codes = {name: code_lines(name, directory) for name in names}
    ini = user_dir / GAME_INI
    ini.parent.mkdir(parents=True, exist_ok=True)
    ini.write_text(game_ini(codes))
    return [addr for lines in codes.values() for addr in injection_addresses(lines)]


def hooks_from_env() -> list[int]:
    """The injection addresses the recorder expects to find patched."""
    value = os.environ.get(HOOKS_ENV, "")
    return [int(part, 16) for part in value.split(",") if part]


def is_branch(word: int) -> bool:
    return word >> 26 == BRANCH_OPCODE
