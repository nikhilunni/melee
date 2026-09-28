"""Explore, bridge, record and triage in one command.

    cd harness && uv run python explore_batch.py <out-dir> <count> <skip> [--sudden-death]
                          [--boundary NAME ...] [--per-fault N] [--samples K] [--jobs 8]

1. Builds and runs the corpus explorer (`melee-replay` example `explore`) for
   `count` seeds after `skip`, from the given registered boundaries
   (boundaries.toml; default: the Fox-Marth FD workload); recordings land in
   <out-dir>/recordings, which must not exist yet (the explorer never
   overwrites evidence).
2. Bridges faulted cases to retail scenarios with replay_to_scenario.py: the
   N shortest per distinct fault message (default 2; 0 bridges every fault),
   plus K clean samples (evenly spread) as exactness checks.
3. Records all of them at once with record_many.py, sets each scenario's
   `frames` to its trace length, and runs `melee-sim triage` on each.
4. Writes <out-dir>/summary.md: the explorer's outcome counts, each bridged
   case's gate result, and a triage report file per divergent case.

Scenario names follow the corpus convention: corpus_v3_s<swap>_e<seed>_p<profile>
for the Fox-Marth workload (corpus_sd_... for Sudden Death), and
corpus_v3_<boundary>_e<seed>_p<profile> for other boundaries (the name without
`start_`). Existing scenarios of the same name are left
alone and reused. Nothing is registered in the gates or committed: a fixed
fault's scenario is added to m5_gate by hand, with its reason.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import data_root  # noqa: E402
import trace_io  # noqa: E402

CASE = re.compile(r"^(?P<case>v3(?:sd)?-(?P<tag>\S+?)-explore(?P<seed>[0-9a-f]+)-profile(?P<profile>\d)): "
                  r"ticks=(?P<ticks>\d+) status=(?P<status>\S+) fault=(?P<fault>.*)$")


def scenario_name(case: str) -> str:
    m = re.match(r"v3(sd)?-(\S+?)-explore([0-9a-f]+)-profile(\d)", case)
    kind = "sd" if m.group(1) else "v3"
    swap = re.fullmatch(r"swap(\d)", m.group(2))
    where = f"s{swap.group(1)}" if swap else m.group(2)
    return f"corpus_{kind}_{where}_e{m.group(3)}_p{m.group(4)}"


def faulted(m: re.Match) -> bool:
    return m.group("status") == "Faulted"


def pick_faults(faults: list[re.Match], per_fault: int) -> list[re.Match]:
    """The `per_fault` shortest cases of each distinct fault message (all when 0)."""
    if per_fault <= 0:
        return faults
    groups: dict[str, list[re.Match]] = {}
    for m in faults:
        groups.setdefault(m.group("fault"), []).append(m)
    picked = [m for group in groups.values()
              for m in sorted(group, key=lambda m: int(m.group("ticks")))[:per_fault]]
    return sorted(picked, key=faults.index)


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, text=True, capture_output=True, **kw)


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out", type=Path)
    ap.add_argument("count", type=int)
    ap.add_argument("skip", type=int)
    ap.add_argument("--sudden-death", action="store_true")
    ap.add_argument("--boundary", action="append", default=[], help="registered boundary (repeatable)")
    ap.add_argument("--per-fault", type=int, default=2, help="bridge the N shortest cases per fault (0: all)")
    ap.add_argument("--samples", type=int, default=0, help="also bridge K clean cases")
    ap.add_argument("--jobs", type=int, default=8)
    a = ap.parse_args(argv)
    out = a.out.resolve()
    recordings = out / "recordings"
    if recordings.exists():
        sys.exit(f"{recordings} exists: pick a new output directory")
    out.mkdir(parents=True, exist_ok=True)

    print("== building explorer and melee-sim", flush=True)
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "melee-replay", "--example", "explore"],
                   cwd=ROOT, check=True)
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "melee-sim"], cwd=ROOT, check=True)
    explore = ROOT / "target/release/examples/explore"
    sim = ROOT / "target/release/melee-sim"

    print(f"== exploring {a.count} seeds after {a.skip}", flush=True)
    log = out / "explore.log"
    with log.open("w") as f:
        # Exit status 1 only means "faults found"; the log says which.
        subprocess.run([str(explore), str(data_root.ROMS / "files"), str(recordings), str(a.count), str(a.skip),
                        *(["sudden-death"] if a.sudden_death else []),
                        *[arg for name in a.boundary for arg in ("--boundary", name)]],
                       cwd=ROOT, stdout=f, stderr=subprocess.STDOUT)
    cases = [m for line in log.read_text().splitlines() if (m := CASE.match(line))]
    if not cases:
        sys.exit(f"the explorer ran no cases; see {log}")
    faults = pick_faults([m for m in cases if faulted(m)], a.per_fault)
    clean = [m for m in cases if not faulted(m)]
    step = max(1, len(clean) // a.samples) if a.samples else 0
    samples = clean[::step][: a.samples] if a.samples else []
    outcomes = Counter(m.group("fault") if faulted(m) else m.group("status").split("(")[0] for m in cases)
    print(f"   {len(cases)} cases: {sum(map(faulted, cases))} faulted ({len(outcomes)} outcomes), "
          f"bridging {len(faults) + len(samples)}", flush=True)

    bridged: list[tuple[str, str, str]] = []  # (scenario, case, why)
    for m, why in [(m, "fault") for m in faults] + [(m, "sample") for m in samples]:
        name = scenario_name(m.group("case"))
        path = data_root.SCENARIOS / f"{name}.toml"
        if not path.exists():
            r = run([sys.executable, str(HERE / "replay_to_scenario.py"),
                     str(recordings / f"{m.group('case')}.json"), "--name", name], cwd=HERE)
            if r.returncode:
                print(f"   bridge failed: {name}: {r.stderr.strip()[-300:]}", flush=True)
                continue
        bridged.append((name, m.group("case"), why))
    if not bridged:
        (out / "summary.md").write_text(summary(a, outcomes, []))
        print(f"== nothing to record; {out / 'summary.md'}")
        return

    print(f"== recording {len(bridged)} scenarios ({a.jobs} jobs)", flush=True)
    rec = run([sys.executable, str(HERE / "record_many.py"),
               *[str(data_root.SCENARIOS / f"{n}.toml") for n, _, _ in bridged], "--jobs", str(a.jobs)], cwd=HERE)
    print(rec.stdout.strip().splitlines()[-1] if rec.stdout.strip() else rec.stderr[-300:], flush=True)

    results = []
    for name, case, why in bridged:
        scenario = data_root.SCENARIOS / f"{name}.toml"
        expected = data_root.TRACES / f"{name}.tick.expected.jsonl"
        if not trace_io.exists(expected):
            results.append((name, case, why, "not recorded", None))
            continue
        ticks = sum(1 for _ in trace_io.open_text(expected))
        text = re.sub(r"^frames = \d+$", f"frames = {ticks}", scenario.read_text(), count=1, flags=re.M)
        scenario.write_text(text)
        gate = run([str(sim), "gate", str(scenario)], cwd=ROOT)
        verdict = gate.stdout.strip().splitlines()[-1] if gate.returncode == 0 else "DIVERGES"
        report = None
        if gate.returncode:
            report = out / f"{name}.triage.txt"
            t = run([str(sim), "triage", str(scenario)], cwd=ROOT)
            report.write_text(t.stdout + t.stderr)
            first = next((l for l in t.stdout.splitlines() if l.startswith("==")), "").removeprefix("== ")
            verdict = first or (gate.stderr.strip().splitlines() or ["error"])[0]
        results.append((name, case, why, verdict, report))
        print(f"   {name} ({why}): {verdict}", flush=True)
    (out / "summary.md").write_text(summary(a, outcomes, results))
    print(f"== {out / 'summary.md'}")


def summary(a, outcomes: Counter, results: list) -> str:
    where = ", ".join(a.boundary) or "the Fox-Marth FD workload"
    lines = [f"# Explorer batch: {a.count} seeds after {a.skip} from {where}"
             f"{' (Sudden Death)' if a.sudden_death else ''}", "",
             "| Outcome | Cases |", "|---|---:|"]
    lines += [f"| {k} | {v} |" for k, v in outcomes.most_common()]
    lines += ["", "| Scenario | Case | Why | Result | Triage |", "|---|---|---|---|---|"]
    for name, case, why, verdict, report in results:
        lines.append(f"| `{name}` | {case} | {why} | {verdict} | {report.name if report else ''} |")
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    main()
