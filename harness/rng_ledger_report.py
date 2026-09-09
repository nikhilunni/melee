"""Summarise an rng_ledger raw trace: which functions draw RNG, per tick.

    uv run python rng_ledger_report.py ../harness/traces/idle_fd_fox.ledger.raw.jsonl [--ticks N]

Maps each draw's `lr` (the caller's return address) and `pc` (which HSD_Rand*
variant) to the containing function from the decomp's symbols.txt.
"""
from __future__ import annotations

import argparse
import bisect
import json
from collections import Counter
from pathlib import Path

import symbols


class SymbolMap:
    def __init__(self) -> None:
        table = symbols.load()
        entries = sorted((v["addr"], int(str(v.get("size", "4")), 0), name)
                         for name, v in table.items()
                         if v.get("type") == "function" or v.get("section", "").startswith(".text"))
        self.starts = [e[0] for e in entries]
        self.entries = entries

    def containing(self, addr: int) -> str:
        i = bisect.bisect_right(self.starts, addr) - 1
        while i >= 0:
            start, size, name = self.entries[i]
            if start <= addr < start + max(size, 4):
                return f"{name}+0x{addr - start:X}"
            if start + 0x10000 < addr:
                break
            i -= 1
        return f"0x{addr:08X}"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("raw", type=Path)
    ap.add_argument("--ticks", type=int, default=8, help="how many ticks to print in full")
    args = ap.parse_args()
    sm = SymbolMap()
    per_tick: list[list[str]] = []
    callers = Counter()
    for line in args.raw.read_text().splitlines():
        rec = json.loads(line)
        names = []
        for d in rec.get("rng_draws", []):
            caller = sm.containing(d["lr"] - 4)   # the bl instruction itself
            kind = sm.containing(d["pc"]).split("+")[0]
            names.append(f"{caller} <{kind}>")
            callers[caller] += 1
        per_tick.append(names)
    print(f"records: {len(per_tick)}; total draws: {sum(callers.values())}")
    print("draws per tick histogram:", dict(sorted(Counter(len(t) for t in per_tick).items())))
    print("\ncallers (bl site -> draws):")
    for caller, n in callers.most_common():
        print(f"  {n:6d}  {caller}")
    print(f"\nfirst {args.ticks} ticks:")
    for i, names in enumerate(per_tick[: args.ticks]):
        print(f"  tick {i}: {len(names)} draws")
        for n in names:
            print(f"      {n}")


if __name__ == "__main__":
    main()
