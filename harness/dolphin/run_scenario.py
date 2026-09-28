"""Run one scenario through Dolphin and decode the trace, from a normal shell.

    cd harness
    uv run python dolphin/run_scenario.py scenarios/idle_fd_fox.toml
    uv run python dolphin/run_scenario.py scenarios/idle_fd_fox.toml --speed 1 --keep
    uv run python dolphin/run_scenario.py scenarios/idle_ys_fox.toml --tick-trace --video OGL

Steps: launch the scripting Dolphin with trace_scenario.py (unlimited emulation
speed by default), wait for `<raw>.done`, SIGTERM Dolphin, run decode.py.
Outputs, under harness/traces/ unless --out is given:
    <name>.raw.jsonl        one record per frame, raw Fighter bytes
    <name>.raw.jsonl.done   summary: frames, fps, savestate load sync check
    <name>.expected.jsonl   the canonical melee-diff trace
Once decoded (and validated, with --tick-trace), the raw and expected traces
are replaced by verified `.jsonl.zst` files (harness/trace_io.py) unless
--no-compress is given; every trace reader accepts either form.

--tick-trace selects tick_trace.py, inserts `.tick` after <name>, interprets
scenario.frames as scheduler ticks, and runs validate_ticks.py after decoding.
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
import dolphin_config  # noqa: E402
import trace_io  # noqa: E402

import data_root  # noqa: E402

ISO = data_root.ROMS / "GALE01.iso"


SI_GC_CONTROLLER, SI_NONE = 6, 0   # SerialInterface::SIDevices


def dolphin_command(iso: Path, speed: float, video: str | None, ports: int,
                    tick_trace: bool = False, background_input: bool = False) -> list[str]:
    """Same SI setup as drive.py launch: the savestate was recorded with `ports`
    emulated controllers plugged in and Dolphin should see the same devices."""
    script = "tick_trace.py" if tick_trace else "trace_scenario.py"
    # A live human port needs the windowed app (keyboard focus, speed 1).
    dolphin = dolphin_config.binary(gui=background_input)
    cmd = [str(dolphin), "-e", str(iso), "--script", str(HERE / script),
           "-C", f"Dolphin.Core.EmulationSpeed={speed}", *dolphin_config.launch_flags(dolphin, video)]
    if background_input:
        # A human port: the keyboard reaches the emulated pad even when the render
        # window is not focused.
        cmd += ["-C", "Dolphin.Input.BackgroundInput=True"]
    for i in range(4):
        cmd += ["-C", f"Dolphin.Core.SIDevice{i}={SI_GC_CONTROLLER if i < ports else SI_NONE}"]
    return cmd


def wait_for(path: Path, err: Path, timeout: float, proc: subprocess.Popen | None = None) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            return
        if err.exists():
            sys.exit(f"Dolphin tracer failed:\n{err.read_text()}")
        if proc is not None and proc.poll() is not None:
            sys.exit(f"Dolphin exited with {proc.returncode} before writing {path}")
        time.sleep(0.2)
    sys.exit(f"timed out after {timeout}s waiting for {path}")


def stop(proc: subprocess.Popen) -> None:
    if proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(15)
        except subprocess.TimeoutExpired:
            proc.kill()


def dolphin_pids() -> list[int]:
    out = subprocess.run(["pgrep", "-f", str(dolphin_config.binary(gui=True))], capture_output=True, text=True).stdout
    return [int(x) for x in out.split()]


def stop_pid(pid: int) -> None:
    try:
        os.kill(pid, signal.SIGTERM)
        for _ in range(30):
            os.kill(pid, 0)
            time.sleep(0.5)
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenario", type=Path)
    ap.add_argument("--out", type=Path, default=data_root.TRACES, help="output directory")
    ap.add_argument("--speed", type=float, default=0.0, help="Dolphin EmulationSpeed; 0 = unlimited")
    ap.add_argument("--background-input", action="store_true", help="keyboard input without window focus")
    ap.add_argument("--video", default=None, help="video backend, e.g. Null")
    ap.add_argument("--ports", type=int, default=2, choices=[1, 2, 3, 4],
                    help="emulated GC controllers plugged in (match the savestate)")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--keep", action="store_true", help="leave Dolphin running afterwards")
    ap.add_argument("--tick-trace", action="store_true",
                    help="capture scenario.frames scheduler ticks, then validate the idle trace")
    ap.add_argument("--no-compress", action="store_true",
                    help="leave the raw and expected traces as plain JSONL (default: verified .zst)")
    a = ap.parse_args(argv)

    scenario = tomllib.loads(a.scenario.read_text())
    name = scenario.get("name", a.scenario.stem)
    a.out.mkdir(parents=True, exist_ok=True)
    stem = f"{name}.tick" if a.tick_trace else name
    raw = (a.out / f"{stem}.raw.jsonl").resolve()
    done, err = Path(str(raw) + ".done"), Path(str(raw) + ".err")
    expected = a.out / f"{stem}.expected.jsonl"
    for p in (raw, trace_io.compressed_path(raw), done, err):
        p.unlink(missing_ok=True)

    env = {**os.environ, "MELEE_SCENARIO": str(a.scenario.resolve()), "MELEE_RAW_OUT": str(raw)}
    log = (a.out / f"{stem}.dolphin.out").open("wb")
    t0 = time.monotonic()
    command = dolphin_command(ISO, a.speed, a.video, a.ports, a.tick_trace, a.background_input)
    if a.background_input:
        # A human port reads the keyboard through CGEventSourceKeyState, which macOS gates
        # on Input Monitoring for the *responsible* app. Spawned from a shell, Dolphin's
        # responsible app is the terminal; launched through LaunchServices it is Dolphin
        # itself, so only Dolphin needs the permission.
        log.close()
        app = Path(command[0]).parents[2]
        before = set(dolphin_pids())
        proc = subprocess.Popen(["open", "-n", "-a", str(app), "--stdout", str(log.name),
                                 "--stderr", str(log.name),
                                 "--env", f"MELEE_SCENARIO={env['MELEE_SCENARIO']}",
                                 "--env", f"MELEE_RAW_OUT={env['MELEE_RAW_OUT']}",
                                 *(["--env", f"MELEE_KEYPAD={os.environ['MELEE_KEYPAD']}"]
                                   if os.environ.get("MELEE_KEYPAD") else []),
                                 "--args", *command[1:]])
        proc.wait(30)
        pid = None
        for _ in range(60):
            new = set(dolphin_pids()) - before
            if new:
                pid = max(new)
                break
            time.sleep(0.5)
        if pid is None:
            sys.exit("Dolphin did not start through LaunchServices")
        print(f"dolphin pid {pid} (LaunchServices); waiting for {done}")
        try:
            wait_for(done, err, a.timeout, proc)
        finally:
            if not a.keep:
                stop_pid(pid)
    else:
        proc = subprocess.Popen(command, env=env, stdout=log,
                                stderr=subprocess.STDOUT, start_new_session=True)
        print(f"dolphin pid {proc.pid}; waiting for {done}")
        try:
            wait_for(done, err, a.timeout, proc)
        finally:
            if not a.keep:
                stop(proc)
    wall = time.monotonic() - t0
    summary = json.loads(done.read_text())
    decode.main(raw, expected)
    records = sum(1 for line in expected.open() if line.strip())
    print(json.dumps({**summary, "wall_s_incl_boot": round(wall, 1), "records": records,
                      "raw": str(raw), "expected": str(expected)}, indent=1))
    if a.tick_trace:
        import validate_ticks

        wanted = scenario["frames"]
        if summary.get("ended_early"):
            wanted = summary["ticks"]
            print(f"match ended early: {wanted} ticks recorded of {scenario['frames']} requested")
        if records != wanted or summary.get("ticks") != records:
            sys.exit("tick trace record count does not match the scenario and .done marker")
        flags = ["--max-draws", "1024"]  # two simultaneous shield breaks draw ~400 in a tick
        if any(step.get("buttons") or step.get("raw") for step in scenario.get("inputs", [])) or any(
                f.get("controller") == "human" for f in scenario.get("fighters", [])):
            flags.append("--scripted")  # inputs drive the fighters: animation rates vary
        if validate_ticks.main([str(expected), *flags]):
            sys.exit(1)
    if not a.no_compress:
        trace_io.compress_outputs([raw, expected])


if __name__ == "__main__":
    main()
