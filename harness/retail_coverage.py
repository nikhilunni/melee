"""Which retail functions real play executes, and how often, measured in Dolphin.

    cd harness && uv run python retail_coverage.py scenarios/a.toml ... --run NAME [--jobs 8]
    cd harness && uv run python retail_coverage.py --m5 --run m5 [--limit N] [--jobs 8]
    cd harness && uv run python retail_coverage.py --aggregate --run NAME   # rebuild the tables only

Each scenario replays on retail exactly as record.py's tick pass does (its
savestate, inputs, Gecko codes and ports, a private Dolphin user folder) with
dolphin/coverage_trace.py, which records Dolphin's Branch Watch over the
recorded ticks: every executed branch with its hit count, taken and not
taken. Needs the coverage build of the fork (docs/patches/
0004-scripting-branch-watch.patch; MELEE_COVERAGE_DOLPHIN or the default
build-coverage bundle) and runs it in debug mode, where the JITs emit the
recording code.

Determinism: every run's raw tick trace is compared, record by record, with
the scenario's own recording in harness/traces (made without coverage); the
host-timing field `vi_frame` is the only one left out. A run that differs is
reported and its coverage still kept (marked in runs.tsv).

Mapping (symbols.txt functions, splits.txt units, the DOL's branch sites):
  - a function executed iff a branch inside it ran (every function leaves
    through a branch: blr, a tail call, or a loop's back edge);
  - its entries are the taken branches landing on its first instruction from
    outside it, plus calls (`bl`) to it from anywhere (recursion);
  - branch sites hit / total, and conditional sites seen both ways / total,
    are the block-level measure of how much of the function ran.

Outputs, under target/coverage/ of the code checkout:
  <run>.tsv        one row per retail function (hit or not), keyed by address
  <run>.hits.tsv   address x scenario: entries and branch executions
  <run>.units.tsv  per decomp source file: functions, executed, branch sites
  <run>.runs.tsv   per scenario: ticks, wall time, determinism, functions hit
  <run>/<name>.functions.tsv   the per-scenario tables the aggregate is made of
"""
from __future__ import annotations

import argparse
import bisect
import contextlib
import itertools
import json
import os
import re
import signal
import struct
import subprocess
import sys
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import data_root
import dolphin_config
import trace_io

CODE_ROOT = data_root.CODE_ROOT
OUT_ROOT = CODE_ROOT / "target" / "coverage"
DEFAULT_BIN = (Path.home() / "Projects/dolphin-scripting/build-coverage/Binaries"
               / "DolphinCoverage.app/Contents/MacOS/dolphin-emu-nogui")
BIN_ENV = "MELEE_COVERAGE_DOLPHIN"
DEBUG_FLAGS = ["-C", "Dolphin.Interface.DebugModeEnabled=True"]
M5_GATE = CODE_ROOT / "crates" / "melee-sim" / "tests" / "m5_gate.rs"
SPLITS = data_root.DECOMP / "config" / "GALE01" / "splits.txt"
SYMBOLS = data_root.DECOMP / "config" / "GALE01" / "symbols.txt"
DOL = data_root.DECOMP / "orig" / "GALE01" / "sys" / "main.dol"
#: Scenario lists in m5_gate.rs hold `("name", ticks)` tuples or bare names.
M5_ENTRY = re.compile(r'^\s*\(?"([A-Za-z0-9_]+)"')
VI_FRAME = re.compile(r'"vi_frame": -?\d+, ')
LISTED_SCENARIOS = 8  # scenario names spelled out per function in <run>.tsv


# ---------------------------------------------------------------------------
# Retail layout: functions, units, branch sites
# ---------------------------------------------------------------------------

_SYMBOL = re.compile(r"^(\w+)\s*=\s*\.(\w+):0x([0-9A-Fa-f]+);\s*//\s*(.*)$")
_UNIT = re.compile(r"^(\S+):\s*$")
_RANGE = re.compile(r"^\s+(\S+)\s+start:0x([0-9A-Fa-f]+)\s+end:0x([0-9A-Fa-f]+)")


@dataclass
class Function:
    addr: int
    name: str
    size: int
    unit: str = "?"
    sites: int = 0          # branch instructions in the function
    cond_sites: int = 0     # of which conditional


def parse_functions(text: str) -> list[Function]:
    """Every `type:function` symbol of the code sections, sorted by address."""
    out = []
    for raw in text.splitlines():
        m = _SYMBOL.match(raw.strip())
        if not m or m.group(2) not in ("init", "text"):
            continue
        meta = dict(kv.split(":", 1) for kv in m.group(4).split() if ":" in kv)
        if meta.get("type") != "function":
            continue
        out.append(Function(int(m.group(3), 16), m.group(1), int(meta.get("size", "0"), 16)))
    out.sort(key=lambda f: f.addr)
    return out


def parse_units(text: str) -> list[tuple[int, int, str]]:
    """`(start, end, unit)` of every .init/.text range in splits.txt."""
    ranges, unit = [], None
    for raw in text.splitlines():
        if (m := _UNIT.match(raw)) and not raw.startswith("Sections"):
            unit = m.group(1)
        elif (m := _RANGE.match(raw)) and unit and m.group(1) in (".init", ".text"):
            ranges.append((int(m.group(2), 16), int(m.group(3), 16), unit))
    return sorted(ranges)


def dol_code(dol: bytes) -> list[tuple[int, bytes]]:
    """`(address, bytes)` of the DOL's text sections (header: 7 text offsets
    at 0x00, addresses at 0x48, sizes at 0x90)."""
    out = []
    for i in range(7):
        off, addr, size = (struct.unpack_from(">I", dol, base + 4 * i)[0] for base in (0x00, 0x48, 0x90))
        if off and size:
            out.append((addr, dol[off:off + size]))
    return out


def branch_kind(word: int) -> str | None:
    """'cond', 'uncond' or None for a PowerPC instruction word: b/bl, bc,
    bclr and bcctr (BO with bits 0x14 set is branch-always)."""
    op = word >> 26
    if op == 18:
        return "uncond"
    if op == 16 or (op == 19 and (word >> 1) & 0x3FF in (16, 528)):
        bo = (word >> 21) & 0x1F
        return "uncond" if bo & 0x14 == 0x14 else "cond"
    return None


class Layout:
    """Retail functions with their unit and branch-site counts; address lookup."""

    def __init__(self, functions: list[Function], units: list[tuple[int, int, str]],
                 code: list[tuple[int, bytes]]) -> None:
        self.functions = functions
        self.starts = [f.addr for f in functions]
        unit_starts = [u[0] for u in units]
        for f in functions:
            i = bisect.bisect_right(unit_starts, f.addr) - 1
            if i >= 0 and units[i][0] <= f.addr < units[i][1]:
                f.unit = units[i][2]
        for base, blob in code:
            for off in range(0, len(blob) - 3, 4):
                kind = branch_kind(struct.unpack_from(">I", blob, off)[0])
                if kind and (f := self.containing(base + off)):
                    f.sites += 1
                    f.cond_sites += kind == "cond"

    @classmethod
    def load(cls) -> Layout:
        return cls(parse_functions(SYMBOLS.read_text()), parse_units(SPLITS.read_text()),
                   dol_code(DOL.read_bytes()) if DOL.exists() else [])

    def containing(self, addr: int) -> Function | None:
        i = bisect.bisect_right(self.starts, addr) - 1
        if i >= 0:
            f = self.functions[i]
            if f.addr <= addr < f.addr + max(f.size, 4):
                return f
        return None


# ---------------------------------------------------------------------------
# Branch Watch dumps -> per-function counts
# ---------------------------------------------------------------------------


@dataclass
class Branch:
    origin: int
    destination: int
    inst: int
    hits: int
    taken: bool  # Branch Watch's "condition": the true (taken) collection


def parse_branch_watch(text: str) -> list[Branch]:
    """Lines `origin destin inst total_hits hits_snapshot flags` (BranchWatch::Save);
    flags bit 1 is the taken (true-path) collection."""
    out = []
    for raw in text.splitlines():
        parts = raw.split()
        if len(parts) != 6:
            continue
        flags = int(parts[5], 16)
        out.append(Branch(int(parts[0], 16), int(parts[1], 16), int(parts[2], 16),
                          int(parts[3]), bool(flags & 2)))
    return out


@dataclass
class FunctionHits:
    entries: int = 0
    branch_execs: int = 0
    sites: set[int] = field(default_factory=set)
    taken: set[int] = field(default_factory=set)
    not_taken: set[int] = field(default_factory=set)

    @property
    def both_ways(self) -> int:
        return len(self.taken & self.not_taken)


def function_hits(layout: Layout, branches: list[Branch]) -> dict[int, FunctionHits]:
    """Per function start address: entries, branch executions, sites seen."""
    hits: dict[int, FunctionHits] = {}
    for b in branches:
        if b.hits <= 0:
            continue
        if f := layout.containing(b.origin):
            h = hits.setdefault(f.addr, FunctionHits())
            h.branch_execs += b.hits
            h.sites.add(b.origin)
            (h.taken if b.taken else h.not_taken).add(b.origin)
        if b.taken and (g := layout.containing(b.destination)) and g.addr == b.destination:
            call = b.inst >> 26 in (16, 18, 19) and b.inst & 1  # LK: a call, recursion too
            if call or layout.containing(b.origin) is not g:
                hits.setdefault(g.addr, FunctionHits()).entries += b.hits
    return hits


FUNCTION_COLUMNS = ("address", "name", "entries", "branch_execs", "sites_hit", "both_ways")


def write_function_table(path: Path, layout: Layout, hits: dict[int, FunctionHits]) -> None:
    by_addr = {f.addr: f for f in layout.functions}
    with path.open("w") as out:
        out.write("\t".join(FUNCTION_COLUMNS) + "\n")
        for addr in sorted(hits):
            h = hits[addr]
            out.write(f"0x{addr:08X}\t{by_addr[addr].name}\t{h.entries}\t{h.branch_execs}\t"
                      f"{len(h.sites)}\t{h.both_ways}\n")


def write_sites(path: Path, hits: dict[int, FunctionHits]) -> None:
    """Every branch site that ran: `address T|N|B` (taken, not taken, both)."""
    lines = []
    for h in hits.values():
        for site in h.sites:
            lines.append((site, "B" if site in h.taken and site in h.not_taken
                          else "T" if site in h.taken else "N"))
    path.write_text("".join(f"{a:08X} {k}\n" for a, k in sorted(lines)))


def read_sites(path: Path) -> dict[int, str]:
    out = {}
    for line in path.read_text().splitlines():
        a, k = line.split()
        out[int(a, 16)] = k
    return out


def merge_sites(into: dict[int, str], more: dict[int, str]) -> None:
    for a, k in more.items():
        old = into.get(a)
        into[a] = k if old is None or old == k else "B"


def read_function_table(path: Path) -> dict[int, tuple[int, int, int, int]]:
    rows = {}
    for line in path.read_text().splitlines()[1:]:
        a, _, entries, execs, sites, both = line.split("\t")
        rows[int(a, 16)] = (int(entries), int(execs), int(sites), int(both))
    return rows


# ---------------------------------------------------------------------------
# Determinism: the coverage run's trace against the scenario's recording
# ---------------------------------------------------------------------------


def compare_traces(recorded_lines, coverage_lines) -> str:
    """'identical (N records)' or the first difference. `vi_frame` (host
    timing) is ignored; keys only one side has (an older tracer) are too."""
    n = 0
    for n, (a, b) in enumerate(itertools.zip_longest(recorded_lines, coverage_lines), 1):
        if a is None or b is None:
            return f"DIFFERS in length: {'recorded' if b is None else 'coverage'} run is longer ({n - 1} common)"
        if VI_FRAME.sub("", a) == VI_FRAME.sub("", b):
            continue
        ra, rb = json.loads(a), json.loads(b)
        keys = (set(ra) & set(rb)) - {"vi_frame"}
        differ = sorted(k for k in keys if ra[k] != rb[k])
        if differ:
            return f"DIFFERS at record {n - 1}: {', '.join(differ[:6])}"
    return f"identical ({n} records)"


def check_determinism(name: str, coverage_raw: Path) -> str:
    recorded = data_root.TRACES / f"{name}.tick.raw.jsonl"
    if not trace_io.exists(recorded):
        return "no recording"
    with trace_io.open_text(recorded) as a, coverage_raw.open() as b:
        return compare_traces(a, b)


# ---------------------------------------------------------------------------
# One scenario (a child process: private user folder, Gecko codes, ports)
# ---------------------------------------------------------------------------


def coverage_binary() -> Path:
    return Path(os.environ.get(BIN_ENV) or DEFAULT_BIN)


def run_one(scenario_path: Path, run_dir: Path, timeout: float, keep: bool) -> None:
    """Inside the child: run the coverage pass, check it, write the tables."""
    import record  # the tick pass's flags and Gecko setup, shared

    # A stopped run must stop its Dolphin too (the finally below).
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))

    scenario = tomllib.loads(scenario_path.read_text())
    name = scenario["name"]
    slots = [int(f.get("slot", i)) for i, f in enumerate(scenario.get("fighters", []))]
    if slots != list(range(len(slots))) or len(slots) > 2:  # record.py plugs in two by default
        os.environ[dolphin_config.SI_PORTS_ENV] = ",".join(str(s) for s in slots)
    raw = run_dir / f"{name}.tick.raw.jsonl"
    branches = run_dir / f"{name}.branches.txt"
    done, err = Path(f"{raw}.done"), Path(f"{raw}.err")
    t0 = time.monotonic()
    with record.gecko_codes(scenario.get("gecko", [])):
        binary = dolphin_config.binary()
        cmd = [str(binary), "-e", str(record.ISO), "--script", str(HERE / "dolphin/coverage_trace.py"),
               *record.dolphin_flags(binary), *DEBUG_FLAGS]
        env = {**os.environ, "MELEE_SCENARIO": str(scenario_path), "MELEE_RAW_OUT": str(raw),
               "MELEE_BRANCH_WATCH_OUT": str(branches)}
        if not trace_io.exists(data_root.TRACES / f"{name}.tick.raw.jsonl"):
            env["MELEE_COVERAGE_TRACE"] = "0"  # nothing to compare: keep no records
        for p in (done, err):
            p.unlink(missing_ok=True)
        with (run_dir / f"{name}.dolphin.out").open("wb") as log:
            proc = subprocess.Popen(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while not done.exists() and not err.exists():
                    if proc.poll() is not None:
                        sys.exit(f"{name}: Dolphin exited with {proc.returncode}")
                    if time.monotonic() - t0 > timeout:
                        sys.exit(f"{name}: no completion marker after {timeout}s")
                    time.sleep(0.5)
            finally:
                proc.terminate()
                try:
                    proc.wait(10)
                except subprocess.TimeoutExpired:
                    proc.kill()
    if err.exists():
        sys.exit(f"{name}: coverage pass failed:\n{err.read_text()[-2000:]}")
    wall = time.monotonic() - t0
    summary = json.loads(done.read_text())
    verdict = check_determinism(name, raw)
    layout = Layout.load()
    hits = function_hits(layout, parse_branch_watch(branches.read_text()))
    write_function_table(run_dir / f"{name}.functions.tsv", layout, hits)
    write_sites(run_dir / f"{name}.sites.txt", hits)
    (run_dir / f"{name}.run.json").write_text(json.dumps({
        "name": name, "ticks": summary.get("branch_watch_ticks"), "wall_s": round(wall, 1),
        "emulation_s": summary.get("elapsed_s"), "determinism": verdict, "functions_hit": len(hits),
        "input_clock": scenario.get("input_clock", "vi"),
    }))
    if not keep:
        raw.unlink(missing_ok=True)
        branches.unlink(missing_ok=True)


def run_child(scenario: Path, run_dir: Path, timeout: float, keep: bool) -> tuple[Path, int, float]:
    t0 = time.monotonic()
    name = tomllib.loads(scenario.read_text())["name"]
    with dolphin_config.isolated_user_dir() as user_dir, (run_dir / f"{name}.log").open("wb") as log:
        env = {**os.environ, "DOLPHIN_USER_DIR": str(user_dir), "DOLPHIN_BIN": str(coverage_binary())}
        code = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--one", str(scenario),
                               "--run-dir", str(run_dir), "--timeout", str(timeout),
                               *(["--keep"] if keep else [])],
                              env=env, stdout=log, stderr=subprocess.STDOUT).returncode
    return scenario, code, time.monotonic() - t0


# ---------------------------------------------------------------------------
# Scenario selection
# ---------------------------------------------------------------------------


def m5_names(text: str) -> list[str]:
    """Every scenario name the m5 gate lists, in file order, once each."""
    names, seen, inside = [], set(), False
    for line in text.splitlines():
        if re.match(r"^(pub )?(const|static) [A-Z0-9_]+: .*= \[\s*$", line) or \
                re.match(r"^(pub )?(const|static) [A-Z0-9_]+: .*= &\[\s*$", line):
            inside = True
            continue
        if inside and line.startswith("];"):
            inside = False
            continue
        if inside and (m := M5_ENTRY.match(line)) and m.group(1) not in seen:
            seen.add(m.group(1))
            names.append(m.group(1))
    return names


def scenario_paths(names: list[str]) -> tuple[list[Path], list[str]]:
    """Scenario files with a recorded tick trace; the names without one."""
    found, missing = [], []
    for name in names:
        # A Slippi witness gates through its `_cold` twin; the retail
        # recording belongs to the bridge scenario itself.
        for candidate in (name, name.removesuffix("_cold")):
            path = data_root.SCENARIOS / f"{candidate}.toml"
            if path.exists() and trace_io.exists(data_root.TRACES / f"{candidate}.tick.raw.jsonl"):
                if path not in found:
                    found.append(path)
                break
        else:
            missing.append(name)
    return found, missing


# ---------------------------------------------------------------------------
# Out-of-scope setups: what breadth beyond the gate would require
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ScopeSetup:
    """A retail match the port does not cover, played by CPUs (no inputs)."""
    name: str
    stkind: int              # InternalStageId, the stage screen's forced stage
    players: tuple[str, ...]  # make_boundary.CHARACTERS names
    item_frequency: int      # gmm_x1CB0.item_freq: -1 off, 0 very low .. 4 very high
    cpu_level: int = 9
    ticks: int = 7200        # two minutes
    stocks: int = 99         # nobody runs out: the tick tracer stops on a match end


SCOPE_SETUPS = (
    ScopeSetup("scope_fd_falcon_puff_cpu", 0x20, ("CaptainFalcon", "Jigglypuff"), -1),
    ScopeSetup("scope_corneria_4cpu_items", 0x07, ("Fox", "Falco", "Pikachu", "Mario"), 4),
    ScopeSetup("scope_brinstar_4cpu_items", 0x06, ("Samus", "Link", "Kirby", "DonkeyKong"), 4),
    ScopeSetup("scope_hyrule_4cpu_items", 0x0E, ("Ness", "Peach", "Yoshi", "IceClimbers"), 4),
    ScopeSetup("scope_ps_4cpu_items", 0x03, ("Mewtwo", "GameAndWatch", "Ganondorf", "Pichu"), 4),
    ScopeSetup("scope_mutecity_4cpu_items", 0x0A, ("Roy", "YoungLink", "DrMario", "Bowser"), 4),
)


def scope_scenario(setup: ScopeSetup, savestate: Path) -> str:
    """A coverage-only scenario (never registered): the CPUs play, no inputs."""
    fighters = "".join(f'\n[[fighters]]\nslot = {i}\ncharacter = "{p}"\ncontroller = "cpu"\n'
                       for i, p in enumerate(setup.players))
    return (f'name = "{setup.name}"\n# Coverage only (retail_coverage.py --scope): {len(setup.players)} '
            f'level-{setup.cpu_level} CPUs, item frequency {setup.item_frequency}.\n'
            f'savestate = "{savestate}"\nframes = {setup.ticks}\n{fighters}')


def make_scope_scenarios(scope_dir: Path, names: list[str] | None) -> list[Path]:
    """A savestate (made by make_boundary's menu driver) and a scenario per
    setup, under target/coverage/scope; existing ones are reused."""
    import make_boundary

    scope_dir.mkdir(parents=True, exist_ok=True)
    out = []
    for setup in SCOPE_SETUPS:
        if names and setup.name not in names:
            continue
        sav = scope_dir / f"{setup.name}.sav"
        if not sav.exists():
            print(f"{setup.name}: making the start savestate", flush=True)
            make_boundary.make_savestate(
                sav, setup.stkind, [make_boundary.CHARACTERS[p] for p in setup.players], setup.stocks,
                make_boundary.TIMEOUT, extra={"item_frequency": setup.item_frequency,
                                              "cpu_players": list(range(len(setup.players))),
                                              "cpu_level": setup.cpu_level})
        path = scope_dir / f"{setup.name}.toml"
        path.write_text(scope_scenario(setup, sav))
        out.append(path)
    return out


# ---------------------------------------------------------------------------
# Aggregate tables
# ---------------------------------------------------------------------------


def aggregate(run: str, run_dir: Path, layout: Layout) -> dict:
    runs = [json.loads(p.read_text()) for p in sorted(run_dir.glob("*.run.json"))]
    per: dict[str, dict[int, tuple[int, int, int, int]]] = {}
    for r in runs:
        table = run_dir / f"{r['name']}.functions.tsv"
        if table.exists():
            per[r["name"]] = read_function_table(table)
    totals: dict[int, list] = {}
    for name, rows in sorted(per.items()):
        for addr, (entries, execs, _sites, _both) in rows.items():
            t = totals.setdefault(addr, [0, 0, []])
            t[0] += entries
            t[1] += execs
            t[2].append(name)
    # Branch sites: the union over scenarios (a site seen taken in one and not
    # taken in another went both ways).
    union: dict[int, str] = {}
    for name in per:
        if (path := run_dir / f"{name}.sites.txt").exists():
            merge_sites(union, read_sites(path))
    sites: dict[int, int] = {}
    both: dict[int, int] = {}
    for site, kind in union.items():
        if f := layout.containing(site):
            sites[f.addr] = sites.get(f.addr, 0) + 1
            both[f.addr] = both.get(f.addr, 0) + (kind == "B")
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    with (OUT_ROOT / f"{run}.tsv").open("w") as out:
        out.write("address\tname\tunit\tsize\tentries\tbranch_execs\tsites_hit\tsites_total\t"
                  "cond_both_ways\tcond_total\tscenarios_hit\tscenarios\n")
        for f in layout.functions:
            e, x, names = totals.get(f.addr, (0, 0, []))
            listed = ",".join(names[:LISTED_SCENARIOS]) + (",..." if len(names) > LISTED_SCENARIOS else "")
            out.write(f"0x{f.addr:08X}\t{f.name}\t{f.unit}\t{f.size}\t{e}\t{x}\t{sites.get(f.addr, 0)}\t"
                      f"{f.sites}\t{both.get(f.addr, 0)}\t{f.cond_sites}\t{len(names)}\t{listed}\n")
    by_addr = {f.addr: f for f in layout.functions}
    with (OUT_ROOT / f"{run}.hits.tsv").open("w") as out:
        out.write("address\tname\tscenario\tentries\tbranch_execs\n")
        for name, rows in sorted(per.items()):
            for addr, (entries, execs, _s, _b) in sorted(rows.items()):
                out.write(f"0x{addr:08X}\t{by_addr[addr].name}\t{name}\t{entries}\t{execs}\n")
    units: dict[str, list[int]] = {}
    for f in layout.functions:
        u = units.setdefault(f.unit, [0, 0, 0, 0, 0])
        u[0] += 1
        u[1] += f.addr in totals
        u[2] += f.sites
        u[3] += sites.get(f.addr, 0)
        u[4] += totals.get(f.addr, (0, 0))[1]
    with (OUT_ROOT / f"{run}.units.tsv").open("w") as out:
        out.write("unit\tfunctions\texecuted\tsites_total\tsites_hit\tbranch_execs\n")
        for unit, (n, hit, st, sh, x) in sorted(units.items()):
            out.write(f"{unit}\t{n}\t{hit}\t{st}\t{sh}\t{x}\n")
    with (OUT_ROOT / f"{run}.runs.tsv").open("w") as out:
        out.write("scenario\tticks\twall_s\temulation_s\tfunctions_hit\tinput_clock\tdeterminism\n")
        for r in runs:
            out.write(f"{r['name']}\t{r['ticks']}\t{r['wall_s']}\t{r['emulation_s']}\t"
                      f"{r['functions_hit']}\t{r.get('input_clock', '?')}\t{r['determinism']}\n")
    return {"scenarios": len(per), "functions": len(layout.functions), "executed": len(totals),
            "nondeterministic": [r["name"] for r in runs if not r["determinism"].startswith(("identical", "no recording"))]}


# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenarios", type=Path, nargs="*")
    ap.add_argument("--run", default="coverage", help="run name: target/coverage/<run>.tsv")
    ap.add_argument("--m5", action="store_true", help="every recorded scenario m5_gate.rs lists")
    ap.add_argument("--scope", nargs="*", metavar="SETUP",
                    help="the out-of-scope CPU setups (all, or those named): savestates made under "
                         "target/coverage/scope, never registered")
    ap.add_argument("--limit", type=int, default=0, help="only the first N selected scenarios")
    ap.add_argument("--every", type=int, default=1, help="every Nth selected scenario (a spread subset)")
    ap.add_argument("--corpus-every", type=int, default=1, metavar="N",
                    help="with --m5: every Nth explorer corpus scene (corpus_v3_*), all the rest")
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--timeout", type=float, default=3600.0)
    ap.add_argument("--keep", action="store_true", help="keep each run's raw trace and branch dump")
    ap.add_argument("--resume", action="store_true", help="skip scenarios this run already has")
    ap.add_argument("--aggregate", action="store_true", help="only rebuild the tables")
    ap.add_argument("--one", type=Path, help=argparse.SUPPRESS)
    ap.add_argument("--run-dir", type=Path, help=argparse.SUPPRESS)
    a = ap.parse_args(argv)
    if a.one:
        run_one(a.one.resolve(), a.run_dir, a.timeout, a.keep)
        return

    run_dir = OUT_ROOT / a.run
    run_dir.mkdir(parents=True, exist_ok=True)
    if not a.aggregate:
        if not coverage_binary().exists():
            sys.exit(f"{coverage_binary()} is missing: build the coverage fork (docs/PORT_NOTES/RETAIL_COVERAGE.md)")
        paths = [p.resolve() for p in a.scenarios]
        if a.m5:
            found, missing = scenario_paths(m5_names(M5_GATE.read_text()))
            if missing:
                print(f"{len(missing)} m5 names without a scenario file or recording: {' '.join(missing[:12])}"
                      f"{' ...' if len(missing) > 12 else ''}")
            corpus = [p for p in found if p.stem.startswith("corpus_v3_")]
            kept = set(corpus[::max(1, a.corpus_every)])
            chosen = [p for p in found if p in kept or not p.stem.startswith("corpus_v3_")]
            # Bridged Slippi replays first: the closest thing to real play.
            paths += sorted(chosen, key=lambda p: not p.stem.startswith("slp_"))
        if a.scope is not None:
            paths += make_scope_scenarios(OUT_ROOT / "scope", a.scope)
        paths = paths[::max(1, a.every)]
        if a.limit:
            paths = paths[:a.limit]
        if a.resume:
            paths = [p for p in paths
                     if not (run_dir / f"{tomllib.loads(p.read_text())['name']}.run.json").exists()]
        t0 = time.monotonic()
        failures = []
        with ThreadPoolExecutor(max_workers=max(1, a.jobs)) as pool:
            for scenario, code, wall in pool.map(lambda p: run_child(p, run_dir, a.timeout, a.keep), paths):
                print(f"{scenario.stem}: {'ok' if code == 0 else f'FAILED ({code})'} in {wall:.0f}s", flush=True)
                if code:
                    failures.append(scenario.stem)
        print(f"{len(paths)} scenarios, {len(failures)} failed, {time.monotonic() - t0:.0f}s wall, {a.jobs} jobs")
        if failures:
            print("failed: " + " ".join(failures))
    result = aggregate(a.run, run_dir, Layout.load())
    print(json.dumps(result))
    print(f"wrote {OUT_ROOT / a.run}.tsv (+ .hits, .units, .runs)")


if __name__ == "__main__":
    with contextlib.suppress(KeyboardInterrupt):
        main()
