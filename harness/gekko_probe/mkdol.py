#!/usr/bin/env python3
"""Hand-assemble the Gekko estimate probe into a bootable GameCube .dol.

The program:
  1. sets MSR[FP], writes ALIVE_MAGIC so the host can see it booted;
  2. spins until the host writes START_MAGIC at CTRL_START;
  3. loop A: for i in 0..N_frsqrte:  f64 in -> frsqrte -> f64 out
     loop B: for i in 0..N_fres32:   f32 in (lfs) -> fres -> f32 out (stfs)
     loop C: for i in 0..N_fres64:   f64 in (lfd) -> fres -> f64 out (stfd)
  4. writes DONE_MAGIC at CTRL_DONE and spins forever.

Only the handful of instructions we need are encoded, straight from the
PowerPC ISA field layouts. `python3 mkdol.py out.dol` writes the file;
`python3 mkdol.py --hex` prints the words for cross-checking with a
disassembler (e.g. llvm-mc --disassemble -triple=powerpc).
"""
from __future__ import annotations

import struct
import sys

import layout as L

# ---------------------------------------------------------------- encoders


def _s16(v: int) -> int:
    assert -0x8000 <= v <= 0xFFFF, v
    return v & 0xFFFF


def d_form(opcd: int, rt: int, ra: int, imm: int) -> int:
    return (opcd << 26) | (rt << 21) | (ra << 16) | _s16(imm)


def x_form(opcd: int, rt: int, ra: int, rb: int, xo: int, rc: int = 0) -> int:
    return (opcd << 26) | (rt << 21) | (ra << 16) | (rb << 11) | (xo << 1) | rc


def a_form(opcd: int, frt: int, fra: int, frb: int, frc: int, xo: int, rc: int = 0) -> int:
    return (opcd << 26) | (frt << 21) | (fra << 16) | (frb << 11) | (frc << 6) | (xo << 1) | rc


# integer
def addi(rt, ra, imm):  return d_form(14, rt, ra, imm)
def addis(rt, ra, imm): return d_form(15, rt, ra, imm)
def lis(rt, imm):       return addis(rt, 0, imm)
def ori(ra, rs, imm):   return d_form(24, rs, ra, imm)          # ori rA,rS,UIMM
def lwz(rt, ra, d):     return d_form(32, rt, ra, d)
def stw(rs, ra, d):     return d_form(36, rs, ra, d)
def cmpw(ra, rb):       return x_form(31, 0, ra, rb, 0)         # cmp cr0,0,rA,rB
def cmpwi(ra, imm):     return d_form(11, 0, ra, imm)           # cmpi cr0,0,rA,SIMM
def mfmsr(rt):          return x_form(31, rt, 0, 0, 83)
def mtmsr(rs):          return x_form(31, rs, 0, 0, 146)
def mtctr(rs):          return x_form(31, rs, 9, 0, 467)         # mtspr 9,rS (spr field split: 9 -> ra=9,rb=0)
def isync():            return x_form(19, 0, 0, 0, 150)
def sync():             return x_form(31, 0, 0, 0, 598)

# float load/store
def lfs(frt, ra, d):    return d_form(48, frt, ra, d)
def lfd(frt, ra, d):    return d_form(50, frt, ra, d)
def stfs(frs, ra, d):   return d_form(52, frs, ra, d)
def stfd(frs, ra, d):   return d_form(54, frs, ra, d)

# estimates
def frsqrte(frt, frb):  return a_form(63, frt, 0, frb, 0, 26)
def fres(frt, frb):     return a_form(59, frt, 0, frb, 0, 24)


# branches (offsets in bytes, relative to the branch instruction)
def b(off):
    assert off % 4 == 0 and -0x2000000 <= off < 0x2000000
    return (18 << 26) | (off & 0x03FFFFFC)


def bc(bo, bi, off):
    assert off % 4 == 0 and -0x8000 <= off < 0x8000
    return (16 << 26) | (bo << 21) | (bi << 16) | (off & 0xFFFC)


def bne(off):  return bc(0b00100, 2, off)   # cr0[EQ]==0
def beq(off):  return bc(0b01100, 2, off)   # cr0[EQ]==1
def bdnz(off): return bc(0b10000, 0, off)   # decrement CTR, branch if CTR!=0


def hi(a: int) -> int:
    return (a >> 16) & 0xFFFF


def lo(a: int) -> int:
    return a & 0xFFFF


# ---------------------------------------------------------------- program


class Asm:
    def __init__(self, base: int):
        self.base = base
        self.words: list[int] = []
        self.labels: dict[str, int] = {}
        self.fixups: list[tuple[int, str, str]] = []

    @property
    def pc(self) -> int:
        return self.base + 4 * len(self.words)

    def emit(self, w: int) -> None:
        assert 0 <= w <= 0xFFFFFFFF
        self.words.append(w)

    def label(self, name: str) -> None:
        self.labels[name] = self.pc

    def branch(self, kind: str, target: str) -> None:
        self.fixups.append((len(self.words), kind, target))
        self.emit(0)

    def resolve(self) -> None:
        enc = {"b": b, "bne": bne, "beq": beq, "bdnz": bdnz}
        for idx, kind, target in self.fixups:
            off = self.labels[target] - (self.base + 4 * idx)
            self.words[idx] = enc[kind](off)

    def load_addr(self, r: int, a: int) -> None:
        self.emit(lis(r, hi(a)))
        self.emit(ori(r, r, lo(a)))


def build_program() -> list[int]:
    a = Asm(L.TEXT_ADDR)
    # r10 = CTRL block
    a.emit(mfmsr(3))
    a.emit(ori(3, 3, 0x2000))          # MSR[FP]
    a.emit(mtmsr(3))
    a.emit(isync())
    a.load_addr(10, L.CTRL_ADDR)
    a.load_addr(4, L.ALIVE_MAGIC)
    a.emit(stw(4, 10, L.CTRL_ALIVE - L.CTRL_ADDR))
    a.load_addr(4, L.START_MAGIC)
    a.label("wait")
    a.emit(lwz(3, 10, L.CTRL_START - L.CTRL_ADDR))
    a.emit(cmpw(3, 4))
    a.branch("bne", "wait")

    def loop(name: str, count_off: int, src: int, dst: int, op, ld, st, step: int) -> None:
        a.emit(lwz(5, 10, count_off))
        a.load_addr(6, src)
        a.load_addr(7, dst)
        a.emit(cmpwi(5, 0))
        a.branch("beq", name + "_end")
        a.emit(mtctr(5))
        a.label(name)
        a.emit(ld(1, 6, 0))
        a.emit(op(2, 1))
        a.emit(st(2, 7, 0))
        a.emit(addi(6, 6, step))
        a.emit(addi(7, 7, step))
        a.branch("bdnz", name)
        a.label(name + "_end")

    loop("frsqrte", L.CTRL_N_FRSQRTE - L.CTRL_ADDR, L.FRSQRTE_IN, L.FRSQRTE_OUT,
         frsqrte, lfd, stfd, 8)
    loop("fres32", L.CTRL_N_FRES32 - L.CTRL_ADDR, L.FRES32_IN, L.FRES32_OUT,
         fres, lfs, stfs, 4)
    loop("fres64", L.CTRL_N_FRES64 - L.CTRL_ADDR, L.FRES64_IN, L.FRES64_OUT,
         fres, lfd, stfd, 8)

    a.emit(sync())
    a.load_addr(4, L.DONE_MAGIC)
    a.emit(stw(4, 10, L.CTRL_DONE - L.CTRL_ADDR))
    a.label("spin")
    a.branch("b", "spin")
    a.resolve()
    return a.words


# ---------------------------------------------------------------- DOL


def make_dol(words: list[int], text_addr: int) -> bytes:
    text = b"".join(struct.pack(">I", w) for w in words)
    # pad text to 32 bytes
    text += b"\0" * (-len(text) % 32)
    header_size = 0x100
    text_off = [header_size] + [0] * 6
    data_off = [0] * 11
    text_addrs = [text_addr] + [0] * 6
    data_addrs = [0] * 11
    text_sizes = [len(text)] + [0] * 6
    data_sizes = [0] * 11
    bss_addr, bss_size = 0, 0
    entry = text_addr
    hdr = struct.pack(">7I11I7I11I7I11IIII",
                      *text_off, *data_off, *text_addrs, *data_addrs,
                      *text_sizes, *data_sizes, bss_addr, bss_size, entry)
    hdr += b"\0" * (header_size - len(hdr))
    assert len(hdr) == header_size
    return hdr + text


def main(argv: list[str]) -> int:
    words = build_program()
    if "--hex" in argv:
        for i, w in enumerate(words):
            print(f"{L.TEXT_ADDR + 4 * i:08x}: {w:08x}")
        return 0
    out = argv[1] if len(argv) > 1 else "gekko_probe.dol"
    with open(out, "wb") as f:
        f.write(make_dol(words, L.TEXT_ADDR))
    print(f"wrote {out}: {len(words)} instructions at {L.TEXT_ADDR:#x}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
