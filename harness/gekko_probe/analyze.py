#!/usr/bin/env python3
"""Infer the Gekko frsqrte / fres estimate structure from captured pairs.

Reads harness/traces/{frsqrte_probe,fres64_probe,fres_probe}.jsonl (written by
probe.py) and, from the pairs alone:

  * finds which input mantissa bits influence the result,
  * fits the output exponent as a function of the input exponent,
  * fits the output mantissa as a table of (base, slope) entries indexed by
    the top mantissa bits, with a linear correction from the following bits,
  * reports the special-value behaviour (zero, inf, NaN, negative, denormal),
  * rebuilds a bit-level model from the fitted numbers and checks that it
    reproduces every captured pair.

Nothing here comes from any emulator source; every number is fitted from the
data.  Usage:  python3 analyze.py [traces_dir] [--table]
"""
from __future__ import annotations

import json
import os
import sys
from collections import defaultdict

TRACES = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else \
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "traces")
SHOW_TABLE = "--table" in sys.argv

M52 = (1 << 52) - 1
M23 = (1 << 23) - 1


def load(name: str) -> list[tuple[int, int]]:
    path = os.path.join(TRACES, name)
    with open(path) as f:
        return [(int(d["input_bits"], 16), int(d["output_bits"], 16))
                for d in map(json.loads, f)]


def split64(b: int) -> tuple[int, int, int]:
    return b >> 63, (b >> 52) & 0x7FF, b & M52


def join64(s: int, e: int, m: int) -> int:
    return (s << 63) | (e << 52) | m


def split32(b: int) -> tuple[int, int, int]:
    return b >> 31, (b >> 23) & 0xFF, b & M23


def join32(s: int, e: int, m: int) -> int:
    return (s << 31) | (e << 23) | m


# ------------------------------------------------------------------ helpers

def min_trailing_zeros(mants: list[int]) -> int:
    tz = 64
    for m in mants:
        if m:
            tz = min(tz, (m & -m).bit_length() - 1)
    return tz


def bits_that_matter(rows, mbits: int, keyf) -> int:
    """Smallest M such that (keyf(exp), top-M mantissa bits) determines the output."""
    for M in range(1, mbits + 1):
        g = defaultdict(set)
        for e, m, out in rows:
            g[(keyf(e), m >> (mbits - M))].add(out)
        if all(len(v) == 1 for v in g.values()):
            return M
    return mbits


def fit_entry(pts, kmax: int = 3):
    """Fit out == (B - S*low) >> k exactly. Returns (k, S, [valid B values]) or None."""
    pts = sorted(pts)
    (l0, o0), (l1, o1) = pts[0], pts[-1]
    if l1 == l0:
        return None
    for k in range(kmax + 1):
        approx = ((o0 - o1) << k) / (l1 - l0)
        for S in range(int(approx) - 2, int(approx) + 3):
            good = [B for r in range(1 << k)
                    for B in [((o0 << k) + r) + S * l0]
                    if all(o == ((B - S * l) >> k) for l, o in pts)]
            if good:
                return k, S, good
    return None


def fit_table(rows, mbits: int, top: int, low: int, shift: int, keyf, key_has_parity: bool):
    """rows: (exp, mant, outmant). Returns {key: (k, S, Bs, npts)} or None if inconsistent."""
    g = defaultdict(dict)
    for e, m, om in rows:
        key = (keyf(e) if key_has_parity else 0, m >> (mbits - top))
        lo = (m >> (mbits - top - low)) & ((1 << low) - 1)
        v = om >> shift
        if (om & ((1 << shift) - 1)) or g[key].get(lo, v) != v:
            return None
        g[key][lo] = v
    table = {}
    for key, d in g.items():
        r = fit_entry(d.items())
        if r is None:
            return None
        table[key] = (*r, len(d))
    return table


def nan_rule(nan_pairs, quiet_bit: int, default_nan: int):
    """Pick the NaN propagation rule that matches every NaN input/output pair."""
    rules = [
        ("NaN passes through unchanged", lambda b: b),
        ("NaN passes through with the quiet bit set", lambda b: b | quiet_bit),
        ("any NaN -> default qNaN", lambda b: default_nan),
    ]
    for label, f in rules:
        if all(f(i) == o for i, o in nan_pairs):
            return label, f
    return "no simple NaN rule matches", lambda b: default_nan


def fit_exponent_linear(pairs_e_oe, cands):
    """pairs: set of (e, oe). cands: iterable of (label, f). Returns matching labels."""
    return [lab for lab, f in cands if all(f(e) == oe for e, oe in pairs_e_oe)]


# ------------------------------------------------------------------ frsqrte

def analyse_frsqrte():
    pairs = load("frsqrte_probe.jsonl")
    print(f"=== frsqrte: {len(pairs)} pairs")
    normals, specials = [], []
    for i, o in pairs:
        s, e, m = split64(i)
        (normals if (s == 0 and 0 < e < 0x7FF) else specials).append((i, o))
    rows = [(split64(i)[1], split64(i)[2], split64(o)[2]) for i, o in normals]
    print(f"  positive normals: {len(normals)}, specials/denormals/negatives: {len(specials)}")

    # output sign always 0 for positive normals?
    assert all(split64(o)[0] == 0 for _, o in normals)
    # exponent relation
    e_oe = {(split64(i)[1], split64(o)[1]) for i, o in normals}
    labels = fit_exponent_linear(e_oe, [
        (f"out_exp = (0x{A:X} - in_exp) >> 1", (lambda A: lambda e: (A - e) >> 1)(A))
        for A in range(0xB00, 0xD00)])
    print(f"  exponent: {labels} ({len(e_oe)} distinct (in,out) exps)")
    # mantissa granularity and dependence
    tz = min_trailing_zeros([r[2] for r in rows])
    M_par = bits_that_matter(rows, 52, lambda e: e & 1)
    M_full = bits_that_matter(rows, 52, lambda e: e)
    print(f"  out mantissa has {tz} trailing zero bits -> {52 - tz} significant bits")
    print(f"  input mantissa bits that matter: top {M_par} (keyed by exp parity); "
          f"top {M_full} (keyed by full exp) -> mantissa depends on parity only: {M_par == M_full}")
    # table fit
    chosen = None
    for top in range(1, M_par):
        t = fit_table(rows, 52, top, M_par - top, tz, lambda e: e & 1, True)
        if t is not None:
            chosen = (top, M_par - top, t)
            break
    if chosen is None:
        print("  !! no exact (base - slope*low) >> k table fit found")
        return None
    top, low, table = chosen
    ks = {v[0] for v in table.values()}
    print(f"  table: {len(table)} entries = 2 parities x {1 << top} (top {top} mantissa bits); "
          f"linear term over next {low} bits; extra shift k in {sorted(ks)}")
    print(f"  form: out_mant = (base[parity][top{top}] - slope * next{low}) << {tz}")
    uniq = all(len(v[2]) == 1 for v in table.values())
    print(f"  base values uniquely determined: {uniq}")
    if SHOW_TABLE:
        for key in sorted(table):
            k, S, Bs, n = table[key]
            print(f"    parity={key[0]} idx={key[1]:2d}: base=0x{Bs[0]:07X} slope=0x{S:04X} (pts={n})")

    A = int(labels[0].split("0x")[1].split()[0], 16) if labels else None

    def model(bits: int) -> int:
        s, e, m = split64(bits)
        if e == 0x7FF:
            if m == 0:
                return 0x7FF8000000000000 if s else 0
            return bits if not (m >> 51) else bits  # placeholder; specials checked separately
        if e == 0 and m == 0:
            return join64(s, 0x7FF, 0)
        if s:
            return 0x7FF8000000000000
        if e == 0:  # denormal: normalise
            sh = 53 - m.bit_length()
            m = (m << sh) & M52
            e = 1 - sh
        key = (e & 1, m >> (52 - top))
        lo = (m >> (52 - top - low)) & ((1 << low) - 1)
        k, S, Bs, _ = table[key]
        om = ((Bs[0] - S * lo) >> k) << tz
        return join64(0, (A - e) >> 1, om)

    # specials report
    print("  specials observed:")
    seen = {}
    for i, o in specials:
        s, e, m = split64(i)
        if e == 0x7FF and m:
            cls = "NaN"
        elif e == 0x7FF:
            cls = "-inf" if s else "+inf"
        elif e == 0 and m == 0:
            cls = "-0" if s else "+0"
        elif s:
            cls = "negative" + (" denormal" if e == 0 else "")
        else:
            cls = "+denormal"
        seen.setdefault(cls, []).append((i, o))
    for cls, lst in seen.items():
        outs = {o for _, o in lst}
        ex = ", ".join(f"{i:#018x}->{o:#018x}" for i, o in lst[:3])
        print(f"    {cls:18s} n={len(lst):4d} distinct outs={len(outs):4d}  e.g. {ex}")
    nan_label, nan_f = nan_rule(seen.get("NaN", []), 1 << 51, 0x7FF8000000000000)
    print(f"    NaN rule: {nan_label}")

    def model_full(bits: int) -> int:
        s, e, m = split64(bits)
        if e == 0x7FF and m:
            return nan_f(bits)
        return model(bits)

    ok = sum(model_full(i) == o for i, o in pairs)
    bad = [(i, o, model_full(i)) for i, o in pairs if model_full(i) != o]
    print(f"  MODEL REPRODUCES {ok}/{len(pairs)} pairs")
    for i, o, mo in bad[:10]:
        print(f"    mismatch in={i:#018x} got={mo:#018x} want={o:#018x}")
    return {"top": top, "low": low, "shift": tz, "A": A,
            "table": {k: (v[0], v[1], v[2][0]) for k, v in table.items()},
            "nan_rule": nan_label, "ok": ok, "n": len(pairs)}


# --------------------------------------------------------------------- fres

def analyse_fres64():
    pairs = load("fres64_probe.jsonl")
    print(f"\n=== fres (f64 in/out via lfd/stfd): {len(pairs)} pairs")
    normals, specials = [], []
    for i, o in pairs:
        s, e, m = split64(i)
        (normals if (s == 0 and 0 < e < 0x7FF) else specials).append((i, o))
    print(f"  positive normals: {len(normals)}, others: {len(specials)}")

    # exponent relation, find the un-clamped band
    e_to_out = defaultdict(set)
    for i, o in normals:
        e_to_out[split64(i)[1]].add(o)
    # exps whose outputs are all normal numbers and follow out_exp = C - in_exp
    def normal_out(o):
        return 0 < split64(o)[1] < 0x7FF
    lin = {}
    for C in range(0x700, 0x900):
        good = [e for e, outs in e_to_out.items()
                if all(normal_out(o) and split64(o)[1] == C - e for o in outs)]
        if len(good) > len(lin.get(C, ())):
            lin[C] = good
    C = max(lin, key=lambda c: len(lin[c]))
    lin_exps = sorted(lin[C])
    e_lo, e_hi = lin_exps[0], lin_exps[-1]
    print(f"  exponent: out_exp = 0x{C:X} - in_exp with a normal result for in_exp in "
          f"[0x{e_lo:X}, 0x{e_hi:X}] ({len(lin_exps)} exps seen)")
    lo_clamp = {e: outs for e, outs in e_to_out.items() if e < e_lo}
    hi_clamp = {e: outs for e, outs in e_to_out.items() if e > e_hi}
    lo_vals = set().union(*lo_clamp.values()) if lo_clamp else set()
    hi_vals = set().union(*hi_clamp.values()) if hi_clamp else set()
    print(f"  in_exp < 0x{e_lo:X}: outputs {[hex(v) for v in lo_vals]}  "
          f"(largest such in_exp seen {max(lo_clamp, default=0):#x})")
    print(f"  in_exp > 0x{e_hi:X}: outputs {[hex(v) for v in hi_vals]}  "
          f"(smallest such in_exp seen {min(hi_clamp, default=0):#x})")
    big = next(iter(lo_vals)) if len(lo_vals) == 1 else None
    small = next(iter(hi_vals)) if len(hi_vals) == 1 else None
    print(f"  (result exp at the edges: 0x{C - e_lo:X} = 1023+127 -> largest single exp; "
          f"0x{C - e_hi:X} = 1023-126 -> smallest single normal exp; "
          f"0x47EFFFFFE0000000 is FLT_MAX as f64)")

    rows = [(split64(i)[1], split64(i)[2], split64(o)[2]) for i, o in normals
            if e_lo <= split64(i)[1] <= e_hi]
    tz = min_trailing_zeros([r[2] for r in rows])
    M_full = bits_that_matter(rows, 52, lambda e: e)
    M_none = bits_that_matter(rows, 52, lambda e: 0)
    print(f"  out mantissa has {tz} trailing zero bits -> {52 - tz} significant bits")
    print(f"  input mantissa bits that matter: top {M_none} (ignoring exp); top {M_full} (keyed by exp) "
          f"-> mantissa independent of exponent: {M_full == M_none}")
    chosen = None
    for top in range(1, M_none):
        t = fit_table(rows, 52, top, M_none - top, tz, lambda e: 0, False)
        if t is not None:
            chosen = (top, M_none - top, t)
            break
    if chosen is None:
        print("  !! no exact table fit")
        return None
    top, low, table = chosen
    ks = {v[0] for v in table.values()}
    print(f"  table: {len(table)} entries indexed by top {top} mantissa bits; linear term over next {low} bits; "
          f"fitted extra shift k in {sorted(ks)}")
    # unify to k = kmax representation
    K = max(ks)
    uni = {}
    for key, (k, S, Bs, n) in table.items():
        uni[key] = (S << (K - k), [B << (K - k) for B in Bs], n, k)
    print(f"  form: out_mant = ((base[top{top}] - slope * next{low}) >> {K}) << {tz}")
    ambiguous = sum(1 for v in uni.values() if v[3] < K)
    multi = sum(1 for v in uni.values() if v[3] == K and len(v[1]) > 1)
    print(f"  entries with even slope, whose base LSB is therefore unobservable: {ambiguous}; "
          f"other entries with more than one consistent base: {multi}")
    if hi_clamp and max(lin_exps) + 1 != min(hi_clamp):
        print(f"  !! coverage gap between 0x{max(lin_exps):X} and 0x{min(hi_clamp):X}; threshold not pinned")
    if lo_clamp and min(lin_exps) - 1 != max(lo_clamp):
        print(f"  !! coverage gap between 0x{max(lo_clamp):X} and 0x{min(lin_exps):X}; threshold not pinned")
    if SHOW_TABLE:
        for key in sorted(uni):
            S, Bs, n, k = uni[key]
            print(f"    idx={key[1]:2d}: base=0x{Bs[0]:06X} slope=0x{S:03X} (pts={n}, k={k})")

    nan_pairs = [(i, o) for i, o in specials if split64(i)[1] == 0x7FF and split64(i)[2]]
    nan_label, nan_f = nan_rule(nan_pairs, 1 << 51, 0x7FF8000000000000)

    def model(bits: int) -> int:
        s, e, m = split64(bits)
        if e == 0x7FF:
            return nan_f(bits) if m else join64(s, 0, 0)
        if e == 0 and m == 0:
            return join64(s, 0x7FF, 0)
        if e < e_lo:
            return (s << 63) | big
        if e > e_hi:
            return (s << 63) | small
        key = (0, m >> (52 - top))
        lo = (m >> (52 - top - low)) & ((1 << low) - 1)
        S, Bs, _, _ = uni[key]
        om = ((Bs[0] - S * lo) >> K) << tz
        return join64(s, C - e, om)

    print("  specials observed:")
    seen = {}
    for i, o in specials:
        s, e, m = split64(i)
        if e == 0x7FF and m:
            cls = "NaN"
        elif e == 0x7FF:
            cls = "-inf" if s else "+inf"
        elif e == 0 and m == 0:
            cls = "-0" if s else "+0"
        elif s:
            cls = "negative" + (" denormal" if e == 0 else "")
        else:
            cls = "+denormal"
        seen.setdefault(cls, []).append((i, o))
    for cls, lst in seen.items():
        outs = {o for _, o in lst}
        ex = ", ".join(f"{i:#018x}->{o:#018x}" for i, o in lst[:3])
        print(f"    {cls:18s} n={len(lst):4d} distinct outs={len(outs):4d}  e.g. {ex}")
    print(f"    NaN rule: {nan_label}")

    ok = sum(model(i) == o for i, o in pairs)
    bad = [(i, o, model(i)) for i, o in pairs if model(i) != o]
    print(f"  MODEL REPRODUCES {ok}/{len(pairs)} pairs")
    for i, o, mo in bad[:10]:
        print(f"    mismatch in={i:#018x} got={mo:#018x} want={o:#018x}")
    return {"model": model, "top": top, "low": low, "shift": tz, "K": K, "C": C,
            "e_lo": e_lo, "e_hi": e_hi, "big": big, "small": small, "table": uni,
            "ok": ok, "n": len(pairs)}


def analyse_fres32(f64model):
    pairs = load("fres_probe.jsonl")
    print(f"\n=== fres (f32 in via lfs, f32 out via stfs): {len(pairs)} pairs")

    def f32_to_f64(b: int) -> int:
        s, e, m = split32(b)
        if e == 0xFF:
            return join64(s, 0x7FF, m << 29)
        if e == 0:
            if m == 0:
                return join64(s, 0, 0)
            sh = 24 - m.bit_length()          # normalise the single denormal
            m = (m << sh) & M23
            e = 1 - sh
        return join64(s, e + 0x380, m << 29)

    def f64_to_f32(b: int) -> int:
        s, e, m = split64(b)
        if e == 0x7FF:
            return join32(s, 0xFF, m >> 29)
        if e == 0 and m == 0:
            return join32(s, 0, 0)
        e32 = e - 0x380
        assert 0 < e32 < 0xFF, hex(b)
        assert m & ((1 << 29) - 1) == 0, hex(b)   # result already single-representable
        return join32(s, e32, m >> 29)

    def model(b: int) -> int:
        return f64_to_f32(f64model(f32_to_f64(b)))

    ok = sum(model(i) == o for i, o in pairs)
    bad = [(i, o, model(i)) for i, o in pairs if model(i) != o]
    print("  model = widen f32 to f64 (denormals normalised) -> fres64 model -> narrow to f32 (exact)")
    print(f"  MODEL REPRODUCES {ok}/{len(pairs)} pairs")
    for i, o, mo in bad[:10]:
        print(f"    mismatch in={i:#010x} got={mo:#010x} want={o:#010x}")
    return ok, len(pairs)


if __name__ == "__main__":
    r1 = analyse_frsqrte()
    r2 = analyse_fres64()
    if r2:
        analyse_fres32(r2["model"])
