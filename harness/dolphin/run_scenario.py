"""Run one scenario through Dolphin and decode the trace, from a normal shell.

    cd harness
    uv run python dolphin/run_scenario.py scenarios/idle_fd_fox.toml
    uv run python dolphin/run_scenario.py scenarios/idle_fd_fox.toml --speed 1 --keep

Steps: launch the scripting Dolphin with trace_scenario.py (unlimited emulation
speed by default), wait for `<raw>.done`, SIGTERM Dolphin, run decode.py.
Outputs, under harness/traces/ unless --out is given:
    <name>.raw.jsonl        one record per frame, raw Fighter bytes
    <name>.raw.jsonl.done   summary: frames, fps, savestate load sync check
    <name>.expected.jsonl   the canonical melee-diff trace
"""
from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
HARNESS = HERE.parent
sys.path.insert(0, str(HARNESS))
import decode  # noqa: E402

DOLPHIN = Path(os.environ.get(
    "DOLPHIN_BIN",
    Path.home() / "Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin"))
ISO = HARNESS / "roms" / "GALE01.iso"


SI_GC_CONTROLLER, SI_NONE = 6, 0   # SerialInterface::SIDevices


def dolphin_command(iso: Path, speed: float, video: str | None, ports: int) -> list[str]:
    """Same SI setup as drive.py launch: the savestate was recorded with `ports`
    emulated controllers plugged in and Dolphin should see the same devices."""
    cmd = [str(DOLPHIN), "-e", str(iso), "--script", str(HERE / "trace_scenario.py"),
           "-C", f"Dolphin.Core.EmulationSpeed={speed}"]
    if video:
        cmd += ["-v", video]
    for i in range(4):
        cmd += ["-C", f"Dolphin.Core.SIDevice{i}={SI_GC_CONTROLLER if i < ports else SI_NONE}"]
    return cmd


def wait_for(path: Path, err: Path, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            return
        if err.exists():
            sys.exit(f"trace_scenario.py failed:\n{err.read_text()}")
        time.sleep(0.2)
    sys.exit(f"timed out after {timeout}s waiting for {path}")


def stop(proc: subprocess.Popen) -> None:
    if proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(15)
        except subprocess.TimeoutExpired:
            proc.kill()


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenario", type=Path)
    ap.add_argument("--out", type=Path, default=HARNESS / "traces", help="output directory")
    ap.add_argument("--speed", type=float, default=0.0, help="Dolphin EmulationSpeed; 0 = unlimited")
    ap.add_argument("--video", default=None, help="video backend, e.g. Null")
    ap.add_argument("--ports", type=int, default=2, choices=[1, 2, 3, 4],
                    help="emulated GC controllers plugged in (match the savestate)")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--keep", action="store_true", help="leave Dolphin running afterwards")
    a = ap.parse_args(argv)

    scenario = tomllib.loads(a.scenario.read_text())
    name = scenario.get("name", a.scenario.stem)
    a.out.mkdir(parents=True, exist_ok=True)
    raw = a.out / f"{name}.raw.jsonl"
    done, err = Path(str(raw) + ".done"), Path(str(raw) + ".err")
    expected = a.out / f"{name}.expected.jsonl"
    for p in (raw, done, err):
        p.unlink(missing_ok=True)

    env = {**os.environ, "MELEE_SCENARIO": str(a.scenario.resolve()), "MELEE_RAW_OUT": str(raw)}
    log = (a.out / f"{name}.dolphin.out").open("wb")
    t0 = time.monotonic()
    proc = subprocess.Popen(dolphin_command(ISO, a.speed, a.video, a.ports), env=env, stdout=log,
                            stderr=subprocess.STDOUT, start_new_session=True)
    print(f"dolphin pid {proc.pid}; waiting for {done}")
    try:
        wait_for(done, err, a.timeout)
    finally:
        if not a.keep:
            stop(proc)
    wall = time.monotonic() - t0
    summary = json.loads(done.read_text())
    decode.main(raw, expected)
    records = sum(1 for line in expected.open() if line.strip())
    print(json.dumps({**summary, "wall_s_incl_boot": round(wall, 1), "records": records,
                      "raw": str(raw), "expected": str(expected)}, indent=1))


if __name__ == "__main__":
    main()
