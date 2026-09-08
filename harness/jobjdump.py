"""Read retail HSD_JObj subtrees without importing Dolphin or changing memory.

Offsets use the 32-bit Gekko ABI, not the host ABI. Header paths below are
relative to third_party/melee-decomp/src/; see docs/M2_GATE.md for the layout
derivation. Float words are read with read_u32 so even signaling NaN payloads
and signed zero survive Python's float conversion.
"""
from __future__ import annotations

from dataclasses import dataclass
import math
import struct
from typing import Iterable, Iterator, Protocol


class Memory(Protocol):
    def read_u32(self, addr: int) -> int: ...
    def read_u8(self, addr: int) -> int: ...
    def read_f32(self, addr: int) -> float: ...


# sysdolphin/baselib/jobj.h:104-125. HSD_Obj is 4 + 2 + 2 bytes.
JOBJ_NEXT = 0x08
JOBJ_PARENT = 0x0C
JOBJ_CHILD = 0x10
JOBJ_FLAGS = 0x14
JOBJ_ROTATE = 0x1C
JOBJ_SCALE = 0x2C
JOBJ_TRANSLATE = 0x38
JOBJ_MTX = 0x44
JOBJ_AOBJ = 0x7C
JOBJ_SIZE = 0x88
JOBJ_INSTANCE = 1 << 12  # jobj.h:72
JOBJ_MTX_DIRTY = 1 << 6  # jobj.h:69
AOBJ_CURR_FRAME = 0x04  # aobj.h:40-42: one u32 precedes curr_frame

# sysdolphin/baselib/gobj.h:12,28-44; melee/ft/types.h:1126-1140,1148,1294.
GOBJ_CLASS_FIGHTER = 4
GOBJ_HSD_OBJ = 0x28
GOBJ_USER_DATA = 0x2C
FIGHTER_GOBJ = 0x00
FIGHTER_KIND = 0x04
FIGHTER_ANIM_ID = 0x14
FIGHTER_FACING = 0x2C
FIGHTER_POSITION = 0xB0
FIGHTER_CUR_ANIM_FRAME = 0x894

# Cached GameCube MEM1 range, as in dolphin/walk.py. Fail on corruption
# instead of returning a truncated trace that could look like a passing gate.
MEM1_LO, MEM1_HI = 0x80000000, 0x81800000
MAX_JOINTS = 4096


def _check_ptr(ptr: int, size: int = 4) -> None:
    if ptr & 3 or not MEM1_LO <= ptr <= MEM1_HI - size:
        raise ValueError(f"invalid MEM1 pointer 0x{ptr:08X}")


def _is_fighter_gobj(mem: Memory, ptr: int) -> bool:
    # classifier is a big-endian u16 at +0; scalar-only Dolphin API.
    return (mem.read_u8(ptr) << 8 | mem.read_u8(ptr + 1)) == GOBJ_CLASS_FIGHTER


def fighter_root_jobj(mem: Memory, fighter_or_gobj_ptr: int) -> int:
    """Accept a Fighter* or a fighter HSD_GObj*; return its root, or zero.

    Fighter starts with a MEM1 GObj pointer, whereas GObj starts with u16
    classifier=4, so these layouts are unambiguous. Check the Fighter/GObj
    backlink before accepting a Fighter*. Fighter.+0x28 is NOT a JObj*.
    """
    ptr = fighter_or_gobj_ptr
    if not ptr:
        return 0
    _check_ptr(ptr)
    if _is_fighter_gobj(mem, ptr):
        gobj = ptr
    else:
        gobj = mem.read_u32(ptr + FIGHTER_GOBJ)
        if not gobj:
            return 0
        _check_ptr(gobj, GOBJ_USER_DATA + 4)
        if not _is_fighter_gobj(mem, gobj) or mem.read_u32(gobj + GOBJ_USER_DATA) != ptr:
            raise ValueError(f"0x{ptr:08X} is not a Fighter or fighter GObj")
    _check_ptr(gobj, GOBJ_USER_DATA + 4)
    root = mem.read_u32(gobj + GOBJ_HSD_OBJ)
    if root:
        _check_ptr(root, JOBJ_SIZE)
    return root


def float_bits(mem: Memory, ptr: int, count: int) -> tuple[int, ...]:
    """Read consecutive f32 words without a float round trip."""
    _check_ptr(ptr, count * 4)
    return tuple(mem.read_u32(ptr + 4 * i) for i in range(count))


@dataclass(frozen=True)
class Joint:
    pointer: int
    flags: int
    rotate: tuple[int, ...]
    scale: tuple[int, ...]
    translate: tuple[int, ...]
    mtx: tuple[int, ...]
    aobj_curr_frame: int | None


def jobj_tree(mem: Memory, root_ptr: int) -> list[Joint]:
    """Root, children, siblings: HSD_JObjAddAnimAll (jobj.c:323-347).

    Do not visit root siblings or INSTANCE children. This equals hsd-anim's
    depth_first on Fox's single-root skeleton. No matrix setup is performed:
    mtx is the cached world matrix, possibly dirty (inspect flags).
    """
    pending = [(root_ptr, False)] if root_ptr else []
    seen: set[int] = set()
    joints = []
    while pending:
        ptr, follow_next = pending.pop()
        _check_ptr(ptr, JOBJ_SIZE)
        if ptr in seen or len(seen) >= MAX_JOINTS:
            raise ValueError(f"cyclic, shared, or oversized JObj tree at 0x{ptr:08X}")
        seen.add(ptr)
        flags = mem.read_u32(ptr + JOBJ_FLAGS)
        aobj = mem.read_u32(ptr + JOBJ_AOBJ)
        curr_frame = float_bits(mem, aobj + AOBJ_CURR_FRAME, 1)[0] if aobj else None
        joints.append(Joint(
            ptr, flags,
            float_bits(mem, ptr + JOBJ_ROTATE, 4),
            float_bits(mem, ptr + JOBJ_SCALE, 3),
            float_bits(mem, ptr + JOBJ_TRANSLATE, 3),
            float_bits(mem, ptr + JOBJ_MTX, 12), curr_frame,
        ))
        sibling = mem.read_u32(ptr + JOBJ_NEXT) if follow_next else 0
        if sibling:
            pending.append((sibling, True))
        child = mem.read_u32(ptr + JOBJ_CHILD) if not flags & JOBJ_INSTANCE else 0
        if child:
            pending.append((child, True))
    return joints


def f32_value(bits: int) -> dict:
    """melee-diff Value::F32; non-finite approx uses 0 (bits stay exact).

    JSON NaN/Infinity and serde_json's null cannot round-trip as an f64.
    approx is diagnostic only and never participates in equality.
    """
    approx = struct.unpack(">f", struct.pack(">I", bits))[0]
    return {"t": "f32", "v": {"bits": bits, "approx": approx if math.isfinite(approx) else 0.0}}


def records(frame: int, joints: Iterable[Joint]) -> Iterator[dict]:
    """One singleton-state Record per scalar: 22 records per joint.

    Numeric bone order, then mtx[0..12], rotate[0..4], scale[0..3],
    translate[0..3] (exclusive upper bounds). Frame is a capture ordinal,
    not the floating-point animation time. Pointer/flags/AObj are diagnostics
    and intentionally not part of the portable comparison stream.
    """
    if not 0 <= frame < 1 << 64:
        raise ValueError("frame must fit a u64")
    for bone, joint in enumerate(joints):
        for field in ("mtx", "rotate", "scale", "translate"):
            for i, bits in enumerate(getattr(joint, field)):
                yield {"frame": frame, "phase": "bones", "state": {
                    f"p0.bone[{bone}].{field}[{i}]": f32_value(bits),
                }}
