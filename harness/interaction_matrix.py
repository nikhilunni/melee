"""Motion transitions witnessed by the gated retail traces (docs/INTERACTION_MATRIX.md).

    cd harness && uv run python interaction_matrix.py <out-dir> [--stage FinalDestination]
                                                      [--characters Fox Marth]

For every scenario on `--stage` named in the m4/m5 gates with a local tick
trace, read each fighter's motion id at frame_end over the gated window and
write transitions.tsv (character, from, to, directed count, corpus count,
directed witnesses) and visits.tsv (states entered) to the output directory,
for the fighters of `--characters` (the port's names). Same-tick motion changes
merge; branches that keep the motion id are invisible.

Common states are named from melee-types' motion_state.rs, special states
(id >= 341) from the decomp's per-character enum (`ftMs_MS_SpecialNStart =
ftCo_MS_Count, ...` in ft/kinds/ft<Kind>/), prefixed with its short name.
"""
from __future__ import annotations

import argparse
import collections
import json
import re
import tomllib
from pathlib import Path

import sys

import trace_io

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "harness/dolphin"))
from remote_proto import FIGHTER_KIND_NAMES  # noqa: E402  retail FighterKind names, index == kind

KINDS_DIR = ROOT / "third_party/melee-decomp/src/melee/ft/kinds"
FIRST_SPECIAL = 341   # ftCo_MS_Count
# The port's character names where they differ from the retail kind names.
RETAIL_NAMES = {"CaptainFalcon": "Captain", "DonkeyKong": "Donkey", "GameAndWatch": "GameWatch",
                "Bowser": "Koopa", "Marth": "Mars", "IceClimbers": "Popo", "Jigglypuff": "Purin",
                "YoungLink": "CLink", "Roy": "Emblem", "Ganondorf": "Ganon", "Sheik": "Seak"}
# Kinds whose special states are another kind's enum (ftfalco.c:25).
SHARED_STATES = {"Falco": "Fox"}
PLAYABLE = FIGHTER_KIND_NAMES[:FIGHTER_KIND_NAMES.index("Emblem") + 1]
CHARACTERS = sorted({next((port for port, retail in RETAIL_NAMES.items() if retail == k), k)
                     for k in PLAYABLE if k != "Nana"})


def common_names(path: Path) -> dict[int, str]:
    names: dict[int, str] = {}
    for m in re.finditer(r"^\s+([A-Z][A-Za-z0-9_]+) = (-?\d+)", path.read_text(), re.M):
        names.setdefault(int(m.group(2)), m.group(1))
    return names


def special_names(retail: str) -> tuple[str, dict[int, str]]:
    """(short prefix, id -> name) from the decomp's motion-state enum for a kind."""
    retail = SHARED_STATES.get(retail, retail)
    for path in sorted((KINDS_DIR / f"ft{retail}").glob("*.[ch]")):
        m = re.search(r"\b(ft\w+?)_MS_(\w+) = ftCo_MS_Count,(.*?)\}", path.read_text(), re.S)
        if m:
            prefix = m.group(1)
            rest = [n for n in re.findall(rf"\b{prefix}_MS_(\w+)\s*,", m.group(3)) if not n.endswith("Count")]
            return prefix.removeprefix("ft"), {FIRST_SPECIAL + i: n for i, n in enumerate([m.group(2), *rest])}
    return retail, {}


def gated_scenarios(stage: str) -> dict[str, tuple[int | None, Path]]:
    gated: set[str] = set()
    for test in ("m5_gate.rs", "m4_gate.rs"):
        gated |= set(re.findall(r'"([a-z0-9_]+)"', (ROOT / "crates/melee-sim/tests" / test).read_text()))
    out = {}
    for name in sorted(gated):
        scenario = ROOT / "harness/scenarios" / f"{name}.toml"
        trace = ROOT / "harness/traces" / f"{name}.tick.expected.jsonl"
        if not scenario.exists() or not trace_io.exists(trace):
            continue
        data = tomllib.loads(scenario.read_text())
        if data.get("stage", "FinalDestination") == stage:
            out[name] = (data.get("frames"), trace)
    return out


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out", type=Path)
    ap.add_argument("--stage", default="FinalDestination")
    ap.add_argument("--characters", nargs="+", default=["Fox", "Marth"], choices=CHARACTERS)
    a = ap.parse_args(argv)
    a.out.mkdir(parents=True, exist_ok=True)
    common = common_names(ROOT / "crates/melee-types/src/motion_state.rs")
    retail = {c: RETAIL_NAMES.get(c, c) for c in a.characters}
    kinds = {FIGHTER_KIND_NAMES.index(retail[c]): c for c in a.characters}
    specials = {k: special_names(retail[c]) for k, c in kinds.items()}

    def name(kind: int, motion: int) -> str:
        if motion >= FIRST_SPECIAL:
            prefix, names = specials[kind]
            return f"{prefix}.{names.get(motion, motion)}"
        return common.get(motion, str(motion))

    scenarios = gated_scenarios(a.stage)
    trans: dict[tuple, dict[str, bool]] = collections.defaultdict(dict)
    visit: dict[tuple, set[str]] = collections.defaultdict(set)
    for n, (frames, trace) in scenarios.items():
        prev: dict[tuple, int] = {}
        for i, line in enumerate(trace_io.open_text(trace)):
            if frames and i >= frames:
                break
            state = json.loads(line)["state"]
            for p in range(4):
                k, m = state.get(f"p{p}.kind"), state.get(f"p{p}.motion_id")
                if not k or not m or k["v"] not in kinds:
                    continue
                k, m = k["v"], m["v"]
                visit[(k, m)].add(n)
                if (k, p) in prev and prev[(k, p)] != m:
                    trans[(k, prev[(k, p)], m)][n] = True
                prev[(k, p)] = m

    def split(names) -> tuple[list[str], list[str]]:
        return [x for x in names if not x.startswith("corpus")], [x for x in names if x.startswith("corpus")]

    with (a.out / "transitions.tsv").open("w") as out:
        for (k, src, dst), ns in sorted(trans.items()):
            directed, corpus = split(ns)
            out.write(f"{kinds[k]}\t{name(k, src)}\t{name(k, dst)}\t{len(directed)}\t{len(corpus)}\t"
                      f"{','.join(directed)}\n")
    with (a.out / "visits.tsv").open("w") as out:
        for (k, m), ns in sorted(visit.items()):
            directed, corpus = split(ns)
            out.write(f"{kinds[k]}\t{m}\t{name(k, m)}\t{len(directed)}\t{len(corpus)}\t"
                      f"{','.join(sorted(directed)[:4])}\n")
    print(len(scenarios), "scenarios;", len(trans), "transitions;", len(visit), "states")


if __name__ == "__main__":
    main()
