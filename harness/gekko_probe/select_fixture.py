#!/usr/bin/env python3
"""Select a compact, representative golden fixture from the probe captures.

Reads harness/traces/{frsqrte_probe,fres64_probe,fres_probe}.jsonl and writes
crates/gekko-math/tests/data/{frsqrte,fres64,fres32}_pairs.txt: one
`<input_hex> <output_hex>` pair per line, sorted by input bits, no prefix.

Selection per instruction (deterministic, seeded):
  * every special: signed zero, infinities, NaNs (quiet and signalling),
    negative denormals; a spread of positive denormals and negatives;
  * fres: every pair whose input exponent is within 2 of a clamp threshold
    (0x37F / 0x47C for doubles, the equivalent single exponents), capped;
  * every table index (frsqrte: exponent parity x top 4 mantissa bits;
    fres: top 5 mantissa bits) with an even spread over the linear-term
    bits (next 11 / next 10);
  * random fill from the remaining pairs up to the per-instruction budget.

Usage: python3 select_fixture.py [traces_dir] [out_dir]
"""
from __future__ import annotations

import json
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TRACES = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "traces")
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
    HERE, "..", "..", "crates", "gekko-math", "tests", "data")

BUDGET = {"frsqrte": 3300, "fres64": 3300, "fres32": 4000}
PER_KEY = {"frsqrte": 60, "fres64": 50, "fres32": 60}
M52 = (1 << 52) - 1
M23 = (1 << 23) - 1


def load(name):
    with open(os.path.join(TRACES, name)) as f:
        return [(int(d["input_bits"], 16), int(d["output_bits"], 16))
                for d in map(json.loads, f)]


def classify64(i):
    s, e, m = i >> 63, (i >> 52) & 0x7FF, i & M52
    if e == 0x7FF:
        return "nan" if m else "inf"
    if e == 0 and m == 0:
        return "zero"
    if e == 0:
        return "negdenorm" if s else "denorm"
    return "negative" if s else "normal"


def classify32(i):
    s, e, m = i >> 31, (i >> 23) & 0xFF, i & M23
    if e == 0xFF:
        return "nan" if m else "inf"
    if e == 0 and m == 0:
        return "zero"
    if e == 0:
        return "negdenorm" if s else "denorm"
    return "negative" if s else "normal"


def key64_frsqrte(i):
    e, m = (i >> 52) & 0x7FF, i & M52
    return (e & 1, m >> 48), (m >> 37) & 0x7FF


def key64_fres(i):
    m = i & M52
    return (0, m >> 47), (m >> 37) & 0x3FF


def key32_fres(i):
    m = i & M23
    return (0, m >> 18), (m >> 8) & 0x3FF


def spread(items, n):
    """n items evenly spaced over the list (which is sorted by low bits)."""
    if len(items) <= n:
        return list(items)
    step = (len(items) - 1) / (n - 1)
    return [items[round(k * step)] for k in range(n)]


def select(name, pairs, classify, keyf, clamp_exps, exp_of, rng):
    chosen = {}
    by_class = {}
    for i, o in pairs:
        by_class.setdefault(classify(i), []).append((i, o))

    def take(lst, n=None):
        lst = sorted(lst)
        if n is not None and len(lst) > n:
            lst = spread(lst, n)
        for i, o in lst:
            chosen[i] = o

    for cls in ("zero", "inf", "nan", "negdenorm"):
        take(by_class.get(cls, []))
    take(by_class.get("denorm", []), 48)
    take(by_class.get("negative", []), 96)

    normals = by_class.get("normal", [])
    # Clamp neighbourhoods (fres only).
    if clamp_exps:
        near = [(i, o) for i, o in normals
                if any(abs(exp_of(i) - c) <= 2 for c in clamp_exps)]
        take(near, 400)
    # Every table key with a spread over the linear-term bits.
    by_key = {}
    for i, o in normals:
        key, low = keyf(i)
        by_key.setdefault(key, []).append((low, i, o))
    for key in sorted(by_key):
        pts = sorted(by_key[key])
        for _, i, o in spread(pts, PER_KEY[name]):
            chosen[i] = o
    # Random fill.
    rest = [(i, o) for i, o in pairs if i not in chosen]
    rng.shuffle(rest)
    for i, o in rest[:max(0, BUDGET[name] - len(chosen))]:
        chosen[i] = o
    return sorted(chosen.items())


def write(name, sel, width):
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, f"{name}_pairs.txt")
    with open(path, "w") as f:
        for i, o in sel:
            f.write(f"{i:0{width}x} {o:0{width}x}\n")
    print(f"{path}: {len(sel)} pairs, {os.path.getsize(path)} bytes")


def main():
    rng = random.Random(0x6EEC0)
    fr = load("frsqrte_probe.jsonl")
    f64 = load("fres64_probe.jsonl")
    f32 = load("fres_probe.jsonl")
    write("frsqrte", select("frsqrte", fr, classify64, key64_frsqrte, (), None, rng), 16)
    write("fres64", select("fres64", f64, classify64, key64_fres, (0x37F, 0x47C),
                           lambda i: (i >> 52) & 0x7FF, rng), 16)
    write("fres32", select("fres32", f32, classify32, key32_fres, (0xFC,),
                           lambda i: (i >> 23) & 0xFF, rng), 8)


if __name__ == "__main__":
    main()
