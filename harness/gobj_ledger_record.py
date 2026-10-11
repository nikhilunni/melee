"""Record a scenario's GObj allocation ledger (dolphin/gobj_ledger.py) into a
file of the caller's choosing; nothing is written to harness/traces.

    cd harness && MELEE_DATA_ROOT=<checkout with the data> \\
        uv run python gobj_ledger_record.py scenarios/<name>.toml --out <file.jsonl>
"""
from __future__ import annotations

import argparse
import os
import tomllib
from pathlib import Path

import dolphin_config
import record

HERE = Path(__file__).resolve().parent


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenario", type=Path)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--timeout", type=float, default=1800.0)
    a = ap.parse_args()
    scenario_path = a.scenario.resolve()
    scenario = tomllib.loads(scenario_path.read_text())
    out = a.out.resolve()
    slots = [int(f.get("slot", i)) for i, f in enumerate(scenario.get("fighters", []))]
    if slots != list(range(len(slots))):
        os.environ[dolphin_config.SI_PORTS_ENV] = ",".join(str(s) for s in slots)
    with dolphin_config.isolated_user_dir() as user_dir:
        os.environ["DOLPHIN_USER_DIR"] = str(user_dir)
        with record.gecko_codes(scenario.get("gecko", [])):
            record.run_dolphin_until(HERE / "dolphin/gobj_ledger.py",
                                     {"MELEE_SCENARIO": str(scenario_path), "MELEE_RAW_OUT": str(out)},
                                     Path(str(out) + ".done"), Path(str(out) + ".err"),
                                     Path(str(out) + ".dolphin.out"), a.timeout, None)
    print(f"{scenario['name']}: GObj ledger -> {out}")


if __name__ == "__main__":
    main()
