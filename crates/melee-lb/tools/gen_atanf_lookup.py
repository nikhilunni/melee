#!/usr/bin/env python3
"""Transcribe `atanf_lookup` from the decomp's lbtrigf.c into Rust.

The table is 46 `float`s written as double literals in the C. Each literal
is the shortest round-trip decimal of an f32, so parsing it as f32 (what
Rust does) and parsing it as f64 then rounding to f32 (what MWCC did) give
the same bits; this script checks that claim for every entry before
emitting anything.

Usage:
    gen_atanf_lookup.py            # print the Rust const block
    gen_atanf_lookup.py --check    # verify crates/melee-lb/src/trigf.rs matches

The native-C oracle (tests/ref_oracle.rs, op `atanf_lookup`) also compares
the table bit for bit at test time; this script exists so the block in
trigf.rs can be regenerated rather than retyped if the decomp changes.
"""
import re
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
DECOMP_SRC = REPO / "third_party" / "melee-decomp" / "src" / "melee" / "lb" / "lbtrigf.c"
TRIGF_RS = REPO / "crates" / "melee-lb" / "src" / "trigf.rs"

TABLE_RE = re.compile(
    r"static const float atanf_lookup\[\] = \{(.*?)\};", re.S
)


def f32_bits(text: str) -> int:
    """Bits of the C literal as a float: parse as double, round to single."""
    d = float(text)
    return struct.unpack("<I", struct.pack("<f", d))[0]


def parse_table() -> list[str]:
    src = DECOMP_SRC.read_text()
    m = TABLE_RE.search(src)
    if not m:
        sys.exit(f"atanf_lookup not found in {DECOMP_SRC}")
    entries = [e.strip() for e in m.group(1).split(",") if e.strip()]
    if len(entries) != 46:
        sys.exit(f"expected 46 entries, found {len(entries)}")
    for e in entries:
        # Rust parses the literal straight to f32. Confirm that agrees with
        # the double-then-single rounding MWCC performed.
        direct = struct.unpack("<I", struct.pack("<f", float(e)))[0]
        via_double = f32_bits(e)
        if direct != via_double:
            sys.exit(f"double rounding differs for {e}: {direct:08x} vs {via_double:08x}")
    return entries


def render(entries: list[str]) -> str:
    """Emit the block in rustfmt's layout: trailing comments aligned one
    space past the longest literal."""
    lits = [(e if ("." in e or "e" in e) else e + ".0") + "," for e in entries]
    width = max(len(lit) for lit in lits)
    lines = ["pub const ATANF_LOOKUP: [f32; 46] = ["]
    for i, (e, lit) in enumerate(zip(entries, lits)):
        lines.append(f"    {lit:<{width}} // [{i}] = 0x{f32_bits(e):08X}")
    lines.append("];")
    return "\n".join(lines) + "\n"


def main() -> None:
    entries = parse_table()
    block = render(entries)
    if "--check" in sys.argv[1:]:
        rs = TRIGF_RS.read_text()
        if block not in rs:
            sys.exit(f"{TRIGF_RS} does not contain the generated table block; regenerate it")
        print(f"ok: {TRIGF_RS} matches {DECOMP_SRC}")
        return
    sys.stdout.write(block)


if __name__ == "__main__":
    main()
