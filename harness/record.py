"""Record every capture a scenario needs, in one command.

    cd harness && uv run python record.py scenarios/walk_fd_fox.toml [--bones N] [--no-ledger] [--no-particles]

Runs, one Dolphin at a time:
  1. the tick trace (run_scenario.py --tick-trace): <name>.tick.{raw,expected}.jsonl
  2. the RNG ledger (dolphin/rng_ledger.py):         <name>.<ledger-suffix>.raw.jsonl
  3. the particle dump (dolphin_particle_snippet.py): <name>.particles.jsonl (+initial/meta)
  4. optionally the tick-aligned bone dump (dolphin_bones_tick_snippet.py, --bones N ticks)
and prints P1's motion transitions and the RNG sites beyond the idle set.
All outputs are machine-local under harness/traces (gitignored).
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DOLPHIN = Path.home() / "Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin"
ISO = HERE / "roms/GALE01.iso"
IDLE_SITES = {"hsd_8039EE24+0xDC", "hsd_8039DAD4+0x10A0", "hsd_8039DAD4+0x10F8",
              "hsd_8039930C+0x1D7C", "hsd_8039930C+0x1DE8", "hsd_8039930C+0x1E54",
              "hsd_8039930C+0x1EC0", "ftCo_8008A7A8+0x114"}


def dolphin_flags(ports: int = 2) -> list[str]:
    flags = ["-v", "OGL", "-C", "Dolphin.Core.EmulationSpeed=0"]
    for i in range(4):
        flags += ["-C", f"Dolphin.Core.SIDevice{i}={6 if i < ports else 0}"]
    return flags


def run_dolphin_until(script: Path, env: dict, done: Path, err: Path, log: Path, timeout: float) -> None:
    """Launch Dolphin with `script`, wait for `done` or `err`, then kill it."""
    for p in (done, err):
        p.unlink(missing_ok=True)
    with log.open("wb") as out:
        proc = subprocess.Popen([str(DOLPHIN), "-e", str(ISO), "--script", str(script), *dolphin_flags()],
                                env={**os.environ, **env}, stdout=out, stderr=subprocess.STDOUT,
                                start_new_session=True)
        t0 = time.monotonic()
        try:
            while not done.exists() and not err.exists():
                if time.monotonic() - t0 > timeout:
                    raise TimeoutError(f"{script.name}: no completion marker after {timeout}s")
                time.sleep(0.5)
        finally:
            proc.terminate()
            try:
                proc.wait(10)
            except subprocess.TimeoutExpired:
                proc.kill()
    if err.exists():
        sys.exit(f"{script.name} failed:\n{err.read_text()[-2000:]}")


def motions(expected: Path, player: int = 0) -> list[tuple[int, int, float, float]]:
    out, last = [], None
    for line in expected.open():
        if not line.strip():
            continue
        r = json.loads(line)
        s = r["state"]
        m = s[f"p{player}.motion_id"]["v"]
        if m != last:
            out.append((r["frame"], m, round(s[f"p{player}.cur_pos.x"]["v"]["approx"], 1),
                        round(s[f"p{player}.cur_pos.y"]["v"]["approx"], 1)))
            last = m
    return out


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenario", type=Path)
    ap.add_argument("--bones", type=int, default=0, metavar="N", help="also dump N ticks of bones")
    ap.add_argument("--no-ledger", action="store_true")
    ap.add_argument("--no-particles", action="store_true")
    ap.add_argument("--ledger-suffix", default="ledger", help="ledger file suffix (the 600-tick idle/start scenes use ledger600)")
    ap.add_argument("--timeout", type=float, default=600.0)
    a = ap.parse_args(argv)

    scenario_path = a.scenario.resolve()
    scenario = tomllib.loads(scenario_path.read_text())
    name = scenario["name"]
    frames = scenario["frames"]
    savestate = ROOT / scenario["savestate"]
    traces = HERE / "traces"
    traces.mkdir(exist_ok=True)

    print(f"== {name}: tick trace ({frames} ticks)")
    subprocess.run([sys.executable, str(HERE / "dolphin/run_scenario.py"), str(scenario_path),
                    "--tick-trace", "--video", "OGL", "--ports", "2"], check=True,
                   stdout=subprocess.DEVNULL)
    expected = traces / f"{name}.tick.expected.jsonl"
    print("   p0 (tick, motion, x, y):", motions(expected))

    if not a.no_ledger:
        out = traces / f"{name}.{a.ledger_suffix}.raw.jsonl"
        print(f"== {name}: RNG ledger -> {out.name}")
        run_dolphin_until(HERE / "dolphin/rng_ledger.py",
                          {"MELEE_SCENARIO": str(scenario_path), "MELEE_RAW_OUT": str(out)},
                          Path(str(out) + ".done"), Path(str(out) + ".err"),
                          traces / f"{name}.ledger.dolphin.out", a.timeout)
        report = subprocess.run([sys.executable, str(HERE / "rng_ledger_report.py"), str(out), "--ticks", "1"],
                                capture_output=True, text=True).stdout
        extra = []
        for line in report.splitlines():
            parts = line.split()
            if len(parts) == 2 and parts[0].isdigit() and parts[1] not in IDLE_SITES \
                    and not parts[1].startswith("grLast"):
                extra.append(f"{parts[1]} x{parts[0]}")
        print("   RNG sites beyond the idle set:", extra or "none")

    if not a.no_particles:
        out = traces / f"{name}.particles.jsonl"
        print(f"== {name}: particle dump ({frames} ticks)")
        run_dolphin_until(HERE / "dolphin_particle_snippet.py",
                          {"MELEE_PARTICLES_SCENARIO": str(scenario_path),
                           "MELEE_PARTICLES_SAVESTATE": str(savestate),
                           "MELEE_PARTICLES_OUT": str(out), "MELEE_PARTICLES_TICKS": str(frames)},
                          Path(str(out) + ".done"), Path(str(out) + ".err"),
                          traces / f"{name}.particles.dolphin.out", a.timeout)

    if a.bones:
        out = traces / f"{name}.bones.jsonl"
        print(f"== {name}: bone dump ({a.bones} ticks)")
        run_dolphin_until(HERE / "dolphin_bones_tick_snippet.py",
                          {"MELEE_BONES_ANY_ANIM": "1", "MELEE_BONES_SAVESTATE": str(savestate),
                           "MELEE_BONES_SCENARIO": str(scenario_path),
                           "MELEE_BONES_OUT": str(out), "MELEE_BONES_TICKS": str(a.bones)},
                          # the snippet writes <name>.bones.raw.jsonl(.done/.err) beside the dump
                          Path(str(out.with_suffix(".raw.jsonl")) + ".done"),
                          Path(str(out.with_suffix(".raw.jsonl")) + ".err"),
                          traces / f"{name}.bones.dolphin.out", a.timeout)
    print(f"== {name}: done")


if __name__ == "__main__":
    main()
