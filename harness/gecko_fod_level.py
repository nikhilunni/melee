"""Build the Gecko codes of the consoles with level Fountain of Dreams platforms.

    cd harness && MELEE_GECKO_DIR=~/melee-data/gecko uv run python gecko_fod_level.py

Six tournament games of 2020-03-04 (docs/SLIPPI.md, "Consoles with level
Fountain of Dreams platforms") ran a code set whose text is not public. Their
frames fix what it did; these two codes do exactly that and nothing else, so
retail can be recorded doing it. Each is written into MELEE_GECKO_DIR unless
a file of its name is already there; none is overwritten.

`neutral-spawn-2019-entry.txt` is the same construction for the consoles
that entered by spawn order without the level platforms (late 2019 to
March 2020, `spawn = "neutral-2019-entry"`): `neutral-spawn-2019.txt` (the
table with late 2019's Dream Land row, harness/gecko.py) with
SetSpawn_AdjustEntryFrames back and no row changed.

`neutral-spawn-entry-fod-level.txt` is `neutral-spawn.txt` (slippi-ssbm-asm
External/NeutralSpawn/NeutralSpawn.asm, the C2 block at 0x8016E510) with two
changes:

- SetSpawn_AdjustEntryFrames, which the source has commented out since
  4dc7447 (2020-01-14), is back after SetSpawn stores the facing direction:

      mr    r3, r31            # REG_PlayerSlot
      mulli r4, r30, 5         # REG_SpawnID
      branchl r12, 0x80035FDC  # Player_SetUnk4C: the entry delay

  The relative branches that cross the six new words are moved by 24 bytes.
- Fountain of Dreams' first two singles rows are (-41.25, 24) and (41.25, 24).

`fod-level.txt` replaces the two loads of grIzumi_801CBE64 that pass a side
platform's creation height to grIzumi_801CCBDC (`lfs f1, 0x0(r6)` at
0x801CC02C and `lfs f1, 0x8(r6)` at 0x801CC084, r6 = yakumono_param) with a
load of 24.0:

      bl    .+8                # lr = the constant's address
      .float 24.0
      mflr  r6                 # r6 is dead after the load; the caller's `bl`
      lfs   f1, 0(r6)          # follows, and its lr is saved in its frame
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

import gecko

#: SetSpawn's facing store (branchl r12, 0x80033094) and the first word of
#: its epilogue (lmw r20, 0xB0(r1)): the entry code goes between them.
FACING_STORE = [0x3D808003, 0x618C3094, 0x7D8903A6, 0x4E800421, 0xBA8100B0]
ENTRY_FRAMES = [0x7FE3FB78,              # mr r3, r31
                0x1C9E0005,              # mulli r4, r30, 5
                0x3D808003, 0x618C5FDC,  # lis/ori r12, 0x80035FDC
                0x7D8903A6, 0x4E800421]  # mtctr r12; bctrl
#: Fountain of Dreams' singles rows 0 and 1: stage id 2, then (x, y) pairs.
FOUNTAIN_ROWS = [0x00000002, 0xC2250000, 0x41A80000, 0x42250000, 0x41D80000]
LEVEL = 0x41C00000  # 24.0
BLR = 0x4E800020
#: Primary opcodes of the relative branches `b`/`bl` and `bc`, with the mask
#: of each one's displacement.
BRANCH_FIELDS = {18: 0x03FFFFFC, 16: 0x0000FFFC}
ABSOLUTE = 2

LEVEL_LOAD = ["48000009 41C00000",  # bl .+8; .float 24.0
              "7CC802A6 C0260000",  # mflr r6; lfs f1, 0(r6)
              "60000000 00000000"]  # nop; the handler's branch back
LEVEL_SITES = ["C21CC02C 00000003", "C21CC084 00000003"]


def find(words: list[int], sequence: list[int]) -> int:
    hits = [i for i in range(len(words) - len(sequence) + 1)
            if words[i:i + len(sequence)] == sequence]
    if len(hits) != 1:
        sys.exit(f"neutral-spawn.txt is not the expected NeutralSpawn block: "
                 f"{len(hits)} matches of {[f'{w:08X}' for w in sequence]}")
    return hits[0]


def displacement(word: int) -> int | None:
    """The byte displacement of a relative branch instruction, else None."""
    mask = BRANCH_FIELDS.get(word >> 26)
    if mask is None or word & ABSOLUTE:
        return None
    value = word & mask
    sign = (mask + 4) >> 1  # the field's top bit
    return value - 2 * sign if value & sign else value


def with_displacement(word: int, value: int) -> int:
    mask = BRANCH_FIELDS[word >> 26]
    half = (mask + 4) >> 1
    assert -half <= value < half, "branch out of range"
    return (word & ~mask) | (value & mask)


def spawn_code(lines: list[str], level_fountain: bool = True) -> list[str]:
    words = [int(part, 16) for line in lines for part in line.split()]
    if words[0] != 0xC216E510 or words[1] * 2 != len(words) - 2:
        sys.exit("neutral-spawn.txt must be the one C2 block at 0x8016E510")
    body = words[2:]
    at = find(body, FACING_STORE) + len(FACING_STORE) - 1
    # Everything before the insertion is code; what follows is SetSpawn's
    # epilogue, then tables and the injection's exit, with no branch back.
    for i in range(at):
        offset = displacement(body[i])
        if offset is not None and i + offset // 4 >= at:
            body[i] = with_displacement(body[i], offset + 4 * len(ENTRY_FRAMES))
    epilogue = body[at:body.index(BLR, at)]
    assert all(displacement(word) is None for word in epilogue)
    body[at:at] = ENTRY_FRAMES
    if level_fountain:
        row = find(body, FOUNTAIN_ROWS)
        body[row + 2] = body[row + 4] = LEVEL
    assert len(body) % 2 == 0 and body[-1] == 0
    return [f"C216E510 {len(body) // 2:08X}",
            *(f"{body[i]:08X} {body[i + 1]:08X}" for i in range(0, len(body), 2))]


def main() -> None:
    env = os.environ.get(gecko.DIR_ENV)
    if not env:
        sys.exit(f"set {gecko.DIR_ENV} to the directory holding neutral-spawn.txt")
    directory = Path(env)
    codes = {
        "neutral-spawn-2019-entry.txt": [
            "$NeutralSpawn (2019 Dream Land row), entry by spawn order "
            "(constructed by harness/gecko_fod_level.py from neutral-spawn-2019.txt)",
            *spawn_code(gecko.code_lines("neutral-spawn-2019", directory), level_fountain=False)],
        "neutral-spawn-entry-fod-level.txt": [
            "$NeutralSpawn, entry by spawn order, level Fountain of Dreams rows "
            "(constructed by harness/gecko_fod_level.py from neutral-spawn.txt)",
            *spawn_code(gecko.code_lines("neutral-spawn", directory))],
        "fod-level.txt": [
            "$Level Fountain of Dreams platforms (constructed by harness/gecko_fod_level.py)",
            *(line for site in LEVEL_SITES for line in (site, *LEVEL_LOAD))],
    }
    for name, lines in codes.items():
        if (directory / name).exists():
            print(f"kept {directory / name}: it exists and is not overwritten")
            continue
        (directory / name).write_text("\n".join(lines) + "\n")
        print(f"wrote {directory / name}: {len(lines) - 1} code lines")


if __name__ == "__main__":
    main()
