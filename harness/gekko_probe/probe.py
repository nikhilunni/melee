"""Dolphin --script: drive the gekko_probe .dol and capture estimate outputs.

Runs inside Felk's Dolphin scripting fork. `__file__` is not defined there,
so all paths come from the environment:

  GEKKO_PROBE_DIR   directory holding layout.py (this directory)
  GEKKO_PROBE_OUT   directory for the *.jsonl results and status files

Frame 1: confirm the guest booted (ALIVE_MAGIC), write the input sweeps and
counts, then write START_MAGIC. Later frames: poll DONE_MAGIC; when set, read
the outputs, write JSONL files, and exit the process.
"""
from __future__ import annotations

import json
import os
import random
import sys
import traceback

PROBE_DIR = os.environ.get("GEKKO_PROBE_DIR", "/Users/nikhilunni/Projects/melee/harness/gekko_probe")
OUT_DIR = os.environ.get("GEKKO_PROBE_OUT", "/Users/nikhilunni/Projects/melee/harness/traces")
sys.path.insert(0, PROBE_DIR)
import layout as L  # noqa: E402

from dolphin import event, memory  # type: ignore  # noqa: E402

STATUS = os.path.join(OUT_DIR, "gekko_probe.status")


def status(msg: str) -> None:
    print("gekko_probe:", msg)
    with open(STATUS, "a") as f:
        f.write(msg + "\n")


# ------------------------------------------------------------- input sweeps

def f64_bits(sign: int, exp: int, mant: int) -> int:
    assert 0 <= exp < 0x800 and 0 <= mant < (1 << 52)
    return (sign << 63) | (exp << 52) | mant


def f32_bits(sign: int, exp: int, mant: int) -> int:
    assert 0 <= exp < 0x100 and 0 <= mant < (1 << 23)
    return (sign << 31) | (exp << 23) | mant


def frsqrte_inputs(rng: random.Random) -> list[int]:
    xs: list[int] = []
    # Full sweep of the top 11 mantissa bits (51..41) for several exponents of
    # each parity; lower bits zero.
    for e in (0x3FE, 0x3FF, 0x400, 0x401, 0x3F0, 0x3F1, 0x001, 0x7FE):
        for top in range(2048):
            xs.append(f64_bits(0, e, top << 41))
    # Do the lower 41 bits matter?  Fixed top bits, random low bits.
    for _ in range(2048):
        e = rng.choice((0x3FE, 0x3FF, 0x400))
        top = rng.randrange(2048)
        xs.append(f64_bits(0, e, (top << 41) | rng.getrandbits(41)))
    # Random positive normals across the whole exponent range.
    for _ in range(4096):
        xs.append(f64_bits(0, rng.randrange(1, 0x7FF), rng.getrandbits(52)))
    # Specials.
    xs += [
        0x0000000000000000, 0x8000000000000000,             # +0, -0
        0x7FF0000000000000, 0xFFF0000000000000,             # +inf, -inf
        0x7FF8000000000000, 0x7FF0000000000001,             # qNaN, sNaN
        0x7FF800000000BEEF, 0xFFF8000000000000,             # NaN payloads
        0xBFF0000000000000, 0xC004000000000000,             # -1, -2.5
        0x8000000000000001, 0x800FFFFFFFFFFFFF,             # tiny negatives
        0x0000000000000001, 0x0000000000000002,             # min denormals
        0x0008000000000000, 0x000FFFFFFFFFFFFF,             # large denormals
        0x7FEFFFFFFFFFFFFF, 0x0010000000000000,             # max/min normal
        0x3FF0000000000000, 0x4010000000000000, 0x3FD0000000000000,  # 1, 4, .25
    ]
    for _ in range(512):                                     # random denormals
        xs.append(f64_bits(0, 0, rng.getrandbits(52)))
    for _ in range(256):                                     # random negatives
        xs.append(f64_bits(1, rng.randrange(1, 0x7FF), rng.getrandbits(52)))
    assert len(xs) <= L.MAX_F64, len(xs)
    return xs


def fres32_inputs(rng: random.Random) -> list[int]:
    xs: list[int] = []
    for e in (0x7E, 0x7F, 0x80, 0x81, 0x70, 0x01, 0xFE, 0x40):
        for top in range(2048):
            xs.append(f32_bits(0, e, top << 12))
    for _ in range(2048):                                    # low 12 bits random
        e = rng.choice((0x7E, 0x7F, 0x80))
        xs.append(f32_bits(0, e, (rng.randrange(2048) << 12) | rng.getrandbits(12)))
    for _ in range(4096):
        xs.append(f32_bits(0, rng.randrange(1, 0xFF), rng.getrandbits(23)))
    xs += [
        0x00000000, 0x80000000, 0x7F800000, 0xFF800000,
        0x7FC00000, 0x7F800001, 0x7FC0BEEF, 0xFFC00000,
        0xBF800000, 0xC0200000, 0x80000001, 0x807FFFFF,
        0x00000001, 0x00000002, 0x00400000, 0x007FFFFF,
        0x7F7FFFFF, 0x00800000, 0x3F800000, 0x40800000, 0x3E800000,
    ]
    for _ in range(512):
        xs.append(f32_bits(0, 0, rng.getrandbits(23)))
    for _ in range(256):
        xs.append(f32_bits(1, rng.randrange(1, 0xFF), rng.getrandbits(23)))
    # exponents whose reciprocal lands in the single denormal/overflow zone
    for e in range(0xF0, 0xFF):
        for top in range(0, 2048, 64):
            xs.append(f32_bits(0, e, top << 12))
    for e in range(0x01, 0x10):
        for top in range(0, 2048, 64):
            xs.append(f32_bits(0, e, top << 12))
    for e in (0xFB, 0xFC, 0xFD, 0x01, 0x02):
        for top in range(0, 2048, 16):
            xs.append(f32_bits(0, e, (top << 12) | rng.getrandbits(12)))
            xs.append(f32_bits(1, e, (top << 12) | rng.getrandbits(12)))
    assert len(xs) <= L.MAX_F32, len(xs)
    return xs


def fres64_inputs(rng: random.Random) -> list[int]:
    xs: list[int] = []
    for e in (0x3FE, 0x3FF, 0x400, 0x3F0, 0x001, 0x7FE, 0x200):
        for top in range(2048):
            xs.append(f64_bits(0, e, top << 41))
    for _ in range(2048):                                    # low 41 bits random
        e = rng.choice((0x3FE, 0x3FF, 0x400))
        xs.append(f64_bits(0, e, (rng.randrange(2048) << 41) | rng.getrandbits(41)))
    for _ in range(4096):
        xs.append(f64_bits(0, rng.randrange(1, 0x7FF), rng.getrandbits(52)))
    xs += [
        0x0000000000000000, 0x8000000000000000,
        0x7FF0000000000000, 0xFFF0000000000000,
        0x7FF8000000000000, 0x7FF0000000000001,
        0x7FF800000000BEEF, 0xFFF8000000000000,
        0xBFF0000000000000, 0xC004000000000000,
        0x8000000000000001, 0x800FFFFFFFFFFFFF,
        0x0000000000000001, 0x0000000000000002,
        0x0008000000000000, 0x000FFFFFFFFFFFFF,
        0x7FEFFFFFFFFFFFFF, 0x0010000000000000,
        0x3FF0000000000000, 0x4010000000000000, 0x3FD0000000000000,
    ]
    for _ in range(512):
        xs.append(f64_bits(0, 0, rng.getrandbits(52)))
    for _ in range(256):
        xs.append(f64_bits(1, rng.randrange(1, 0x7FF), rng.getrandbits(52)))
    # exponent edges: the double range, and the band where the reciprocal
    # leaves the single-precision range (result exp 1023+127 / 1023-126),
    # both signs, so the clamp thresholds are pinned to the exponent.
    for e in list(range(0x7F0, 0x7FF)) + list(range(0x001, 0x010)):
        for top in range(0, 2048, 64):
            xs.append(f64_bits(0, e, top << 41))
    for e in list(range(0x37A, 0x385)) + list(range(0x478, 0x483)):
        for top in range(0, 2048, 32):
            xs.append(f64_bits(0, e, (top << 41) | rng.getrandbits(41)))
            xs.append(f64_bits(1, e, (top << 41) | rng.getrandbits(41)))
    assert len(xs) <= L.MAX_F64, len(xs)
    return xs


# ------------------------------------------------------------- frame driver

state = {"frame": 0, "started": False, "in": None, "done": False}


def write_inputs() -> None:
    rng = random.Random(0x6EC0)
    a = frsqrte_inputs(rng)
    b = fres32_inputs(rng)
    c = fres64_inputs(rng)
    state["in"] = (a, b, c)
    for i, v in enumerate(a):
        memory.write_u64(L.FRSQRTE_IN + 8 * i, v)
    for i, v in enumerate(b):
        memory.write_u32(L.FRES32_IN + 4 * i, v)
    for i, v in enumerate(c):
        memory.write_u64(L.FRES64_IN + 8 * i, v)
    # poison the output regions so unwritten slots are visible
    for i in range(len(a)):
        memory.write_u64(L.FRSQRTE_OUT + 8 * i, 0xDEADDEADDEADDEAD)
    for i in range(len(b)):
        memory.write_u32(L.FRES32_OUT + 4 * i, 0xDEADDEAD)
    for i in range(len(c)):
        memory.write_u64(L.FRES64_OUT + 8 * i, 0xDEADDEADDEADDEAD)
    memory.write_u32(L.CTRL_N_FRSQRTE, len(a))
    memory.write_u32(L.CTRL_N_FRES32, len(b))
    memory.write_u32(L.CTRL_N_FRES64, len(c))
    memory.write_u32(L.CTRL_DONE, 0)
    memory.write_u32(L.CTRL_START, L.START_MAGIC)
    status(f"inputs written: frsqrte={len(a)} fres32={len(b)} fres64={len(c)}; START set")


def dump(path: str, ins: list[int], base: int, width: int) -> int:
    rd = memory.read_u64 if width == 8 else memory.read_u32
    fmt = "0x%016x" if width == 8 else "0x%08x"
    n = 0
    with open(path, "w") as f:
        for i, v in enumerate(ins):
            o = rd(base + width * i)
            f.write(json.dumps({"input_bits": fmt % v, "output_bits": fmt % o}) + "\n")
            n += 1
    return n


def collect() -> None:
    a, b, c = state["in"]
    na = dump(os.path.join(OUT_DIR, "frsqrte_probe.jsonl"), a, L.FRSQRTE_OUT, 8)
    nb = dump(os.path.join(OUT_DIR, "fres_probe.jsonl"), b, L.FRES32_OUT, 4)
    nc = dump(os.path.join(OUT_DIR, "fres64_probe.jsonl"), c, L.FRES64_OUT, 8)
    status(f"outputs written: frsqrte={na} fres32={nb} fres64={nc}")
    status("DONE")


def on_frame() -> None:
    state["frame"] += 1
    fr = state["frame"]
    try:
        if not state["started"]:
            alive = memory.read_u32(L.CTRL_ALIVE)
            pc_word = memory.read_u32(L.TEXT_ADDR)
            status(f"frame {fr}: alive={alive:#010x} text[0]={pc_word:#010x}")
            if alive != L.ALIVE_MAGIC:
                if fr < 300:
                    return
                status("guest never set ALIVE_MAGIC; giving up")
                os._exit(3)
            write_inputs()
            state["started"] = True
            return
        done = memory.read_u32(L.CTRL_DONE)
        if done == L.DONE_MAGIC:
            collect()
            sys.stdout.flush()
            os._exit(0)
        if fr % 60 == 0:
            status(f"frame {fr}: waiting, done={done:#010x}")
        if fr > 1800:
            status("timeout waiting for DONE")
            os._exit(4)
    except Exception:
        status("exception:\n" + traceback.format_exc())
        os._exit(5)


with open(STATUS, "w") as _f:
    _f.write("script loaded\n")
event.on_frameadvance(on_frame)
status("registered on_frameadvance")
