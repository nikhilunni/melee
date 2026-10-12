"""Record a scenario's particle-family ledger (dolphin/family_ledger.py) into
a file of the caller's choosing; nothing is written to harness/traces.

    cd harness && MELEE_DATA_ROOT=<checkout with the data> \\
        uv run python family_ledger_record.py scenarios/<name>.toml --out <file.jsonl> [--from TICK]

Each record lists the effect families retail created on that tick with the
call chain of each (saved link registers, innermost first): which effect
call made a generator and which proc it ran in. `symbols.py`-style names:
`uv run python family_ledger_record.py --show <file.jsonl> --tick N`.
"""
from __future__ import annotations

import argparse
import json
import os
import tomllib
from pathlib import Path

import dolphin_config
import record
import symbols

HERE = Path(__file__).resolve().parent


def nearest(table: list[tuple[int, str]], addr: int) -> str:
    """The function holding `addr`, as name+offset."""
    lo, hi = 0, len(table)
    while lo < hi:
        mid = (lo + hi) // 2
        if table[mid][0] <= addr:
            lo = mid + 1
        else:
            hi = mid
    if lo == 0:
        return f"{addr:08X}"
    base, name = table[lo - 1]
    return f"{addr:08X} {name}+0x{addr - base:X}"


def show(path: Path, tick: int) -> None:
    table = sorted((s["addr"], n) for n, s in symbols.load().items() if s.get("type") == "function")
    for line in path.open():
        rec = json.loads(line)
        if rec.get("frame") != tick:
            continue
        for family in rec["families"]:
            print(f"family {family['id']}  current GObj {family['cur']:08X}")
            for addr in (family["pc"], family["lr"], *family["stack"]):
                print("   ", nearest(table, addr))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenario", type=Path, nargs="?")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--from", dest="first", type=int, default=0, help="first tick whose families are kept")
    ap.add_argument("--timeout", type=float, default=1800.0)
    ap.add_argument("--show", type=Path, help="print a recorded ledger's families of --tick with symbol names")
    ap.add_argument("--tick", type=int)
    a = ap.parse_args()
    if a.show:
        show(a.show, a.tick)
        return
    scenario_path = a.scenario.resolve()
    scenario = tomllib.loads(scenario_path.read_text())
    out = a.out.resolve()
    slots = [int(f.get("slot", i)) for i, f in enumerate(scenario.get("fighters", []))]
    if slots != list(range(len(slots))):
        os.environ[dolphin_config.SI_PORTS_ENV] = ",".join(str(s) for s in slots)
    with dolphin_config.isolated_user_dir() as user_dir:
        os.environ["DOLPHIN_USER_DIR"] = str(user_dir)
        with record.gecko_codes(scenario.get("gecko", [])):
            record.run_dolphin_until(HERE / "dolphin/family_ledger.py",
                                     {"MELEE_SCENARIO": str(scenario_path), "MELEE_RAW_OUT": str(out),
                                      "MELEE_FAMILY_FROM": str(a.first)},
                                     Path(str(out) + ".done"), Path(str(out) + ".err"),
                                     Path(str(out) + ".dolphin.out"), a.timeout, None)
    print(f"{scenario['name']}: family ledger -> {out}")


if __name__ == "__main__":
    main()
