"""Memory-walking helpers for the Dolphin oracle, kept free of any Dolphin import.

Everything here takes a `mem` object with `read_u32(addr) -> int` and
`read_u8(addr) -> int` (the subset of `dolphin.memory` we use; the scripting
fork has typed read_u8/u16/u32/u64/s*/f32/f64 but no bulk read), so the walk
can be exercised in tests with a dict-backed fake. `read_bytes` below composes
a bulk read out of those two.

Layout facts are transcribed from the decomp header
third_party/melee-decomp/src/sysdolphin/baselib/gobj.h and mirrored in
harness/schema/globals.yaml (`structs:`); tests/test_walk.py asserts the two
stay in sync.

    HSD_GObj_Entities            .sbss 0x804D782C, `HSD_GObjList*` (gobj.h:113)
    HSD_GObjList.fighters        +0x20 (gobj.h:75)  -- really HSD_GObj*[] indexed
                                                       by p_link; fighters are
                                                       created with p_link 8
                                                       (ft/fighter.c:852), 8*4 = 0x20
    HSD_GObjList.items           +0x24 (gobj.h:76)
    HSD_GObj.next                +0x08 (gobj.h:36)  -- singly walked, NULL-terminated
    HSD_GObj.user_data           +0x2C (gobj.h:44)  -- Fighter* (GET_FIGHTER, ft/inlines.h:40)
"""
from __future__ import annotations

from typing import Protocol


class Memory(Protocol):
    def read_u32(self, addr: int) -> int: ...
    def read_u8(self, addr: int) -> int: ...


def read_bytes(mem: Memory, addr: int, n: int) -> bytes:
    """Read `n` bytes at `addr` (big-endian memory image) using read_u32/read_u8.

    Aligned 4-byte chunks go through read_u32; the unaligned head and the
    tail (n % 4) go through read_u8.
    """
    out = bytearray()
    end = addr + n
    cur = addr
    while cur < end and cur & 3:
        out.append(mem.read_u8(cur))
        cur += 1
    while cur + 4 <= end:
        out += mem.read_u32(cur).to_bytes(4, "big")
        cur += 4
    while cur < end:
        out.append(mem.read_u8(cur))
        cur += 1
    return bytes(out)


# HSD_GObjList (gobj.h:66-86)
GOBJLIST_FIGHTERS_OFF = 0x20  # gobj.h:75
GOBJLIST_ITEMS_OFF = 0x24     # gobj.h:76

# HSD_GObj (gobj.h:28-47)
GOBJ_NEXT_OFF = 0x08          # gobj.h:36
GOBJ_PREV_OFF = 0x0C          # gobj.h:37
GOBJ_USER_DATA_OFF = 0x2C     # gobj.h:44
GOBJ_SIZE = 0x38

# Melee never has more than 6 fighter GObjs (4 players + 2 Nanas); 8 is a
# generous cap that still stops a corrupted list from spinning forever.
MAX_FIGHTERS = 8

# GameCube MEM1 as seen by the game (cached virtual range).
MEM1_LO = 0x8000_0000
MEM1_HI = 0x8180_0000


def is_valid_ptr(p: int) -> bool:
    return MEM1_LO <= p < MEM1_HI


def walk_gobj_list(mem: Memory, head: int, cap: int = MAX_FIGHTERS) -> list[int]:
    """Follow HSD_GObj.next from `head` until NULL; return the GObj addresses.

    Stops early on a NULL/invalid pointer, a revisited node (cycle), or after
    `cap` nodes.
    """
    out: list[int] = []
    seen: set[int] = set()
    cur = head
    while cur and is_valid_ptr(cur) and len(out) < cap:
        if cur in seen:
            break
        seen.add(cur)
        out.append(cur)
        cur = mem.read_u32(cur + GOBJ_NEXT_OFF)
    return out


def fighter_gobjs(mem: Memory, entities_addr: int, cap: int = MAX_FIGHTERS) -> list[int]:
    """Return the HSD_GObj* of every entry in HSD_GObj_Entities->fighters."""
    entities = mem.read_u32(entities_addr)
    if not entities or not is_valid_ptr(entities):
        return []
    head = mem.read_u32(entities + GOBJLIST_FIGHTERS_OFF)
    return walk_gobj_list(mem, head, cap)


def fighter_bases(mem: Memory, entities_addr: int, cap: int = MAX_FIGHTERS) -> list[int]:
    """Return the Fighter* (gobj->user_data) for every fighter GObj, in list order.

    GObjs with a NULL user_data are skipped (a fighter mid-construction).
    """
    bases: list[int] = []
    for gobj in fighter_gobjs(mem, entities_addr, cap):
        ud = mem.read_u32(gobj + GOBJ_USER_DATA_OFF)
        if ud and is_valid_ptr(ud):
            bases.append(ud)
    return bases
