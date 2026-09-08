"""Print a function's retail PowerPC disassembly.

The decomp's `dtk dol split` writes one `.s` file per translation unit under
`third_party/melee-decomp/build/GALE01/asm/`, with every function labelled:

    .fn sinf, global
    /* 803263D4 00323188  7C 08 02 A6 */	mflr r0
    ...
    .endfn sinf

This tool finds the unit that owns a symbol (via `symbols.txt` for the
address and `splits.txt` for the unit's address range), pulls the function
out of that `.s` file, and prints it. See docs/ASM.md for how to produce the
split output; it takes a few seconds.

    uv run python asm.py sinf                 # whole function
    uv run python asm.py 0x80326470           # by address, anywhere inside
    uv run python asm.py sinf --fused         # only fmadd-family lines
    uv run python asm.py atan2f --calls       # bl targets
"""
from __future__ import annotations

import argparse
import bisect
import re
import sys
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path

import symbols

ROOT = Path(__file__).resolve().parents[1]
DECOMP = ROOT / "third_party" / "melee-decomp"
SPLITS = DECOMP / "config" / "GALE01" / "splits.txt"
ASM_DIR = DECOMP / "build" / "GALE01" / "asm"

#: Every fused multiply-add mnemonic, single and double precision.
FUSED_MNEMONICS = frozenset(
    {
        "fmadd", "fmadds",
        "fmsub", "fmsubs",
        "fnmadd", "fnmadds",
        "fnmsub", "fnmsubs",
    }
)

# `/* 803263D4 00323188  7C 08 02 A6 */	mflr r0`
_INSN = re.compile(
    r"^/\* ([0-9A-Fa-f]{8}) [0-9A-Fa-f]{8}\s+((?:[0-9A-Fa-f]{2} ?){4}) \*/\s*(\S+)\s*(.*)$"
)
# `.L_80326424:`
_LABEL = re.compile(r"^(\.L_[0-9A-Fa-f]+):\s*$")
# `melee/lb/lbtrigf.c:` heading a unit block in splits.txt
_UNIT = re.compile(r"^(\S+):\s*$")
# `	.text       start:0x80022C30 end:0x8002305C`
_RANGE = re.compile(r"^\s+(\S+)\s+start:0x([0-9A-Fa-f]+)\s+end:0x([0-9A-Fa-f]+)")


@dataclass(frozen=True)
class Instruction:
    addr: int
    raw: bytes
    mnemonic: str
    operands: str

    def format(self) -> str:
        text = f"{self.mnemonic} {self.operands}".rstrip()
        return f"{self.addr:08X}  {self.raw.hex().upper()}  {text}"


@dataclass(frozen=True)
class Label:
    name: str

    def format(self) -> str:
        return f"{self.name}:"


@dataclass(frozen=True)
class Function:
    name: str
    lines: tuple[Instruction | Label, ...]

    @property
    def instructions(self) -> list[Instruction]:
        return [x for x in self.lines if isinstance(x, Instruction)]

    def fused(self) -> list[Instruction]:
        return [i for i in self.instructions if i.mnemonic in FUSED_MNEMONICS]

    def calls(self) -> list[tuple[int, str]]:
        """`(address, target)` for every `bl`, in program order."""
        return [(i.addr, i.operands) for i in self.instructions if i.mnemonic == "bl"]


# ---------------------------------------------------------------------------
# splits.txt: unit -> code address ranges
# ---------------------------------------------------------------------------


def parse_splits(text: str) -> list[tuple[int, int, str]]:
    """`(start, end, unit)` for every code range in splits.txt, sorted by start.

    Only `.init` and `.text` ranges are kept; data ranges never hold functions.
    """
    ranges: list[tuple[int, int, str]] = []
    unit: str | None = None
    for raw in text.splitlines():
        if not raw.strip() or raw.startswith("Sections:"):
            continue
        m = _UNIT.match(raw)
        if m:
            unit = m.group(1)
            continue
        m = _RANGE.match(raw)
        if m and unit and m.group(1) in (".init", ".text"):
            ranges.append((int(m.group(2), 16), int(m.group(3), 16), unit))
    ranges.sort()
    return ranges


def unit_for_address(ranges: list[tuple[int, int, str]], addr: int) -> str | None:
    starts = [r[0] for r in ranges]
    i = bisect.bisect_right(starts, addr) - 1
    if i >= 0 and ranges[i][0] <= addr < ranges[i][1]:
        return ranges[i][2]
    return None


@lru_cache(maxsize=1)
def _code_ranges() -> list[tuple[int, int, str]]:
    return parse_splits(SPLITS.read_text())


def asm_path_for_unit(unit: str) -> Path:
    """`melee/lb/lbtrigf.c` -> `build/GALE01/asm/melee/lb/lbtrigf.s`."""
    return ASM_DIR / Path(unit).with_suffix(".s")


# ---------------------------------------------------------------------------
# .s parsing
# ---------------------------------------------------------------------------


def parse_function(text: str, name: str) -> Function | None:
    """Extract `.fn name` ... `.endfn name` from a dtk `.s` file."""
    lines: list[Instruction | Label] = []
    inside = False
    for raw in text.splitlines():
        if not inside:
            if raw.startswith(f".fn {name},"):
                inside = True
            continue
        if raw.startswith(f".endfn {name}"):
            return Function(name, tuple(lines))
        m = _INSN.match(raw)
        if m:
            addr, hexbytes, mnem, ops = m.groups()
            lines.append(
                Instruction(int(addr, 16), bytes.fromhex(hexbytes.replace(" ", "")), mnem, ops)
            )
            continue
        m = _LABEL.match(raw)
        if m:
            lines.append(Label(m.group(1)))
    return None


# ---------------------------------------------------------------------------
# symbol / address resolution
# ---------------------------------------------------------------------------


@lru_cache(maxsize=1)
def _functions_by_address() -> tuple[list[int], list[tuple[str, int]]]:
    funcs = sorted(
        (meta["addr"], name, int(meta.get("size", "0"), 16))
        for name, meta in symbols.load().items()
        if meta.get("type") == "function"
    )
    return [a for a, _, _ in funcs], [(n, s) for _, n, s in funcs]


def function_containing(addr: int) -> tuple[str, int] | None:
    """`(name, start)` of the function whose range contains `addr`."""
    starts, info = _functions_by_address()
    i = bisect.bisect_right(starts, addr) - 1
    if i < 0:
        return None
    name, size = info[i]
    if starts[i] <= addr < starts[i] + max(size, 4):
        return name, starts[i]
    return None


def resolve(query: str) -> tuple[str, int]:
    """A symbol name or a hex address (with or without `0x`) -> `(name, addr)`."""
    if re.fullmatch(r"(0x)?[0-9A-Fa-f]{8}", query):
        addr = int(query, 16)
        hit = function_containing(addr)
        if hit is None:
            raise KeyError(f"no function in symbols.txt contains 0x{addr:08X}")
        return hit
    return query, symbols.addr(query)


def load_function(query: str) -> Function:
    if not ASM_DIR.is_dir():
        raise FileNotFoundError(
            f"{ASM_DIR} is missing. Run the decomp split first (docs/ASM.md):\n"
            f"  cd {DECOMP} && python3 configure.py && ninja build/GALE01/config.json"
        )
    name, addr = resolve(query)
    unit = unit_for_address(_code_ranges(), addr)
    if unit is None:
        raise KeyError(f"{name} (0x{addr:08X}) is not inside any code range in {SPLITS}")
    path = asm_path_for_unit(unit)
    fn = parse_function(path.read_text(), name)
    if fn is None:
        raise KeyError(f"{name} not found in {path}")
    return fn


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def format_function(fn: Function, *, fused: bool = False, calls: bool = False) -> str:
    if calls:
        return "\n".join(f"{addr:08X}  bl {target}" for addr, target in fn.calls())
    if fused:
        return "\n".join(i.format() for i in fn.fused())
    return "\n".join(x.format() for x in fn.lines)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("query", nargs="+", help="function symbol or 0x-address inside it")
    p.add_argument("--fused", action="store_true", help="only fmadd/fmsub/fnmadd/fnmsub (and -s) lines")
    p.add_argument("--calls", action="store_true", help="list bl targets")
    args = p.parse_args(argv)

    status = 0
    for i, q in enumerate(args.query):
        try:
            fn = load_function(q)
        except (KeyError, FileNotFoundError) as e:
            print(f"error: {e}", file=sys.stderr)
            status = 1
            continue
        if len(args.query) > 1:
            if i:
                print()
            unit = unit_for_address(_code_ranges(), fn.instructions[0].addr)
            print(f"== {fn.name} ({unit})")
        print(format_function(fn, fused=args.fused, calls=args.calls))
    return status


if __name__ == "__main__":
    sys.exit(main())
