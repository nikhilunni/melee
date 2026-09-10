#!/usr/bin/env python3
"""Parse perf-gate evidence, append a dated report, and reject regressions."""
import datetime
import json
import math
import os
from pathlib import Path
import platform
import re
import sys

HISTORICAL = {"stripped_bytes": 3866720, "text_bytes": 3604480}
MARKER = r"<!-- perf-gate-v1\n(.*?)\n-->"
LLVM_ROW = re.compile(r"^\s*(\d+)\s+\([^)]*\)\s+(\d+)\s+\([^)]*\)\s+(.+)$")


# C15 deliberately transfers common bodies and static adapters to their owners.
# These ceilings replace the generic-shell attribution, never timing/size limits.
C15_COPY_LIMITS = {
    "ft-captain": 66,
    "ft-falco": 67,
    "ft-fox": 67,
    "ft-fox-family": 0,
    "ft-mario": 0,
    "ft-mars": 66,
    "ft-peach": 67,
    "ft-purin": 66,
    "ft-yoshi": 68,
    "melee-ft": 845,
    "melee-sim": 128
}
C15_STRIPPED_LIMIT = 3_747_632
C15_P1_TIME_LIMITS = {"load_ns": 182_600_000, "ticks_600_ns": 25_947_000}
PAIR_HELPERS = {
    "melee_ft::fighter::grab::capture_pair",
    "melee_ft::fighter::grab_throw::enter_back_throw",
    "melee_ft::fighter::grab_throw::release_back_throw",
    "melee_ft::fighter::damage::detect_hit",
}


def common_shell_label(name):
    """Namespace-owned definitions, excluding generic library helpers mentioning a fighter."""
    return (name.startswith("melee_ft::fighter::") and (
        "impl melee_ft::fighter::Fighter>" in name
        or "impl melee_ft::fighter::Fighter<" in name
        or name.startswith("melee_ft::fighter::Fighter::")
        or re.search(r"\bfor melee_ft::fighter::Fighter(?:<[^>]*>)?>", name) is not None
        or name.startswith("melee_ft::fighter::state::callbacks::")
        or name.startswith("melee_ft::fighter::state::row::unimplemented_")
        or name in PAIR_HELPERS))


def concrete_shell_census(contributions):
    """Charge each common definition across compiling crates, never just within one."""
    labels = {}
    for crate, rows in contributions.items():
        for row in rows:
            name = row["function"]
            if common_shell_label(name):
                label = labels.setdefault(name, {"copies": 0, "lines": 0, "crates": {}})
                label["copies"] += row["copies"]
                label["lines"] += row["lines"]
                label["crates"][crate] = row["copies"]
    failures = []
    for name, label in labels.items():
        if "Fighter<" in name or label["copies"] != 1:
            failures.append(f"common definition {name}: {label['copies']} copies across {label['crates']}")
    # Missing common code is incomplete evidence, not a smaller successful census.
    for name in sorted(PAIR_HELPERS - labels.keys()):
        failures.append(f"missing concrete pair helper: {name}")
    return labels, failures


def concrete_limits(metrics):
    failures = []
    if metrics["stripped_bytes"] > C15_STRIPPED_LIMIT:
        failures.append(f"C15 stripped bytes: {metrics['stripped_bytes']} > {C15_STRIPPED_LIMIT}")
    for name, limit in C15_P1_TIME_LIMITS.items():
        if metrics.get(name, 0) > limit:
            failures.append(f"P1 time ceiling {name}: {metrics[name]:.3f} > {limit}")
    for crate, copies in metrics.get("copies", {}).items():
        limit = C15_COPY_LIMITS.get(crate, 0)
        if copies > limit:
            failures.append(f"C15 {crate} copies: {copies} > {limit}")
    total = sum(metrics.get("copies", {}).values())
    limit = sum(C15_COPY_LIMITS.values())
    if total > limit:
        failures.append(f"C15 total copies: {total} > {limit}")
    return failures


def tolerance(name, default, integer=False):
    value = float(os.environ.get(name, default))
    if not math.isfinite(value) or value < 0 or (integer and not value.is_integer()):
        raise ValueError(f"{name} must be a finite nonnegative {'integer' if integer else 'number'}")
    return int(value) if integer else value


def llvm_rows(text):
    rows = []
    for line in text.splitlines():
        match = LLVM_ROW.match(line)
        if match:
            lines, copies, name = match.groups()
            if "melee_ft::" in name:
                rows.append({"lines": int(lines), "copies": int(copies), "function": name})
    if not re.search(r"^\s*\d+\s+\d+\s+\(TOTAL\)\s*$", text, re.MULTILINE):
        raise ValueError("unrecognized cargo llvm-lines output (TOTAL missing)")
    return sorted(rows, key=lambda row: row["lines"], reverse=True)


def read_text_size(text):
    rows = [line.split() for line in text.splitlines() if line.strip()]
    if len(rows) != 2 or rows[0][0] not in ("text", "__TEXT"):
        raise ValueError("expected size output for one native Mach-O or ELF executable")
    return int(rows[1][0])


def estimate(path):
    value = json.loads(path.read_text())["mean"]
    numbers = [value["point_estimate"], value["confidence_interval"]["lower_bound"],
               value["confidence_interval"]["upper_bound"]]
    if any(not math.isfinite(n) or n <= 0 for n in numbers):
        raise ValueError(f"invalid Criterion estimate: {path}")
    return {"ns": numbers[0], "lower_ns": numbers[1], "upper_ns": numbers[2]}


def compare(current, previous, time_percent, size_percent, copies_allowed):
    failures = []
    for name in ("stripped_bytes", "text_bytes", "load_ns", "ticks_600_ns"):
        if name not in previous or name not in current:
            continue
        percent = time_percent if name.endswith("_ns") else size_percent
        limit = previous[name] * (1 + percent / 100)
        if current[name] > limit:
            failures.append(f"{name}: {current[name]:.3f} > {limit:.3f} (previous {previous[name]:.3f}, +{percent:g}%)")
    for crate in current.get("copies", {}).keys() | previous.get("copies", {}).keys():
        old = previous.get("copies", {}).get(crate, 0)
        new = current.get("copies", {}).get(crate, 0)
        if new > old + copies_allowed:
            failures.append(f"{crate} melee-ft copies: {new} > {old} + {copies_allowed}")
    return failures


def main(run, report):
    time_percent = tolerance("PERF_TIME_TOLERANCE", 10)
    size_percent = tolerance("PERF_SIZE_TOLERANCE", 5)
    copies_allowed = tolerance("PERF_COPIES_TOLERANCE", 0, integer=True)
    missing = []
    estimates = {}
    for name in ("load", "ticks_600"):
        try:
            if (run / "bench.failed").exists():
                raise ValueError("benchmark command failed")
            estimates[name] = estimate(run / f"criterion/start_fd_fox/{name}/new/estimates.json")
        except (OSError, ValueError, KeyError) as error:
            missing.append(f"Criterion {name}: {error}")
    metrics = {
        "stripped_bytes": (run / "melee-sim.stripped").stat().st_size,
        "text_bytes": read_text_size((run / "size.txt").read_text()),

    }
    metrics.update({f"{name}_ns": estimate["ns"] for name, estimate in estimates.items()})
    contributions = {}
    if (run / "llvm-version.txt").exists():
        for package in ["melee-ft", "melee-sim", *(run / "characters.txt").read_text().splitlines()]:
            try:
                if (run / f"llvm-{package}.failed").exists():
                    raise ValueError("command failed")
                contributions[package] = llvm_rows((run / f"llvm-{package}.txt").read_text())
            except (OSError, ValueError) as error:
                missing.append(f"llvm-lines {package}: {error}")
        if not contributions.get("melee-sim"):
            missing.append("llvm-lines: no melee-ft functions found in the sim library")
        if not contributions.get("melee-ft"):
            missing.append("llvm-lines: concrete melee-ft core census is empty")
    else:
        missing.append("cargo-llvm-lines unavailable; installation failed")
    if not (run / "bloat.txt").exists() or (run / "bloat.failed").exists():
        missing.append("cargo-bloat unavailable or failed")
    else:
        if not re.search(r"\bmelee_sim\b", (run / "bloat.txt").read_text()):
            missing.append("cargo-bloat output did not contain melee_sim")
    metrics["copies"] = {crate: sum(row["copies"] for row in rows)
                         for crate, rows in contributions.items()}
    existing = report.read_text() if report.exists() else "# Performance regression history\n"
    history = [json.loads(match) for match in re.findall(MARKER, existing, re.DOTALL)]
    # Failed/incomplete measurements never ratchet the regression baseline upward.
    previous = next((block for block in reversed(history) if block["status"] == "PASS"), None)
    baseline = previous["metrics"] if previous else HISTORICAL
    if previous and previous.get("architecture") != "concrete-shell-v1":
        # Explicit C15 accounting migration: core + static adapter definitions
        # replace sim monomorphizations. Fixed aggregate/common-label caps below
        # prevent accepting duplicates merely moved into another crate.
        baseline = {**baseline, "copies": C15_COPY_LIMITS}
    common_labels, common_failures = concrete_shell_census(contributions)
    (run / "common-definitions.json").write_text(json.dumps(common_labels, indent=2, sort_keys=True))
    regressions = compare(metrics, baseline, time_percent, size_percent, copies_allowed)
    regressions += concrete_limits(metrics) + common_failures
    # There was no historical instantiation census to compare the first run to.
    if previous is None:
        regressions = [item for item in regressions if "melee-ft copies:" not in item]
    status = "INCOMPLETE" if missing else "REGRESSION" if regressions else "PASS"
    now = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")
    block = {"date": now, "status": status, "metrics": metrics,
             "architecture": "concrete-shell-v1",
             "rustc": (run / "rustc.txt").read_text().strip(),
             "platform": platform.platform(),
             "revision": (run / "revision.txt").read_text().strip()}
    heading = status if missing else f"COMPLETE — {status}"
    lines = [f"\n## {now} — {heading}\n", f"Evidence: `{run}`. Revision `{block['revision']}` (working tree included).",
             f"\n{block['rustc']}; {block['platform']}.",
             f"\nTolerance: time +{time_percent:g}%, size +{size_percent:g}%, copies +{copies_allowed} per compiling crate.",
             f"Baseline: {previous['date'] if previous else '2026-09-09 main size measurements; timing/copies not yet baselined'}.",
             f"C15 fixed ceilings: {C15_STRIPPED_LIMIT:,} stripped bytes; {sum(C15_COPY_LIMITS.values()):,} total copies; one definition per common label.",
             "\n| Measurement | Value |", "|---|---:|",
             f"| Stripped binary | {metrics['stripped_bytes']:,} bytes |",
             f"| Text (`size`) | {metrics['text_bytes']:,} bytes |",
             ]
    for name, value in estimates.items():
        lines.append(f"| {name} mean | {value['ns']/1e6:.3f} ms (95% CI {value['lower_ns']/1e6:.3f}..{value['upper_ns']/1e6:.3f} ms) |")
    if "ticks_600" in estimates:
        lines.append(f"| Headless throughput | {600e9/estimates['ticks_600']['ns']:,.0f} ticks/s |")
    lines += [f"| Total melee-ft copies | {sum(metrics['copies'].values()):,} |",
              f"| Common definition labels audited across crates | {len(common_labels)} |"]
    lines += [f"\n- {message}" for message in missing + regressions]
    for name in ("bloat-version", "llvm-version"):
        if (run / f"{name}.txt").exists():
            lines.append(f"\nTool: `{(run / f'{name}.txt').read_text().strip()}`.")
    if (run / "bloat.txt").exists():
        lines += ["\nPer-crate text contribution (cargo-bloat estimates):\n```text",
                  (run / "bloat.txt").read_text().strip(), "```"]
    for crate, rows in contributions.items():
        lines += [f"\n### {crate}: {metrics['copies'][crate]} melee-ft copies",
                  "\n| IR lines | Copies | Function (top 20 by IR lines) |", "|---:|---:|---|"]
        lines += [f"| {r['lines']} | {r['copies']} | `{r['function'].replace('|', '&#124;')}` |" for r in rows[:20]]
    lines += ["\n<!-- perf-gate-v1", json.dumps(block, sort_keys=True), "-->\n"]
    report.write_text(existing + "\n".join(lines))
    timings = "; ".join(f"{name} {value['ns']/1e6:.3f} ms" for name, value in estimates.items())
    print(f"[{status}] perf-gate: {metrics['stripped_bytes']} stripped bytes, {metrics['text_bytes']} text bytes; {timings or 'timing unavailable'}")
    for message in missing + regressions:
        print(message, file=sys.stderr)
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    try:
        sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2])))
    except (OSError, ValueError, KeyError, IndexError) as error:
        sys.exit(f"perf-gate: invalid or missing evidence: {error}")
