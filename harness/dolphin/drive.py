"""Shell CLI for remote.py: queue GC inputs, save/load states, read status.

    cd harness
    uv run python dolphin/drive.py launch                 # start Dolphin + remote.py
    uv run python dolphin/drive.py status                 # pretty-print status.json
    uv run python dolphin/drive.py "press Start 3" "wait 60"
    uv run python dolphin/drive.py --sync "stick 1 0 20; press A"   # wait until applied+idle
    uv run python dolphin/drive.py save /abs/path/idle_fd_fox.sav
    uv run python dolphin/drive.py shot [/abs/path.png]   # emulator frame -> PNG, waits for it
    uv run python dolphin/drive.py wait-idle
    uv run python dolphin/drive.py wait-watch 0x804D6718 25 500   # block until watched u32 in range
    uv run python dolphin/drive.py kill                   # SIGTERM the Dolphin pid

Command grammar is in remote_proto.parse_command. Anything that is not a local
verb (launch, status, wait-idle, kill) is parsed and written to cmd.json as one
batch, which remote.py applies on its next frame.
"""
from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import remote_proto as proto  # noqa: E402

HARNESS = HERE.parent
sys.path.insert(0, str(HARNESS))
import dolphin_config  # noqa: E402
DEFAULT_DIR = HARNESS / "roms" / ".remote"
ISO = HARNESS / "roms" / "GALE01.iso"
LOCAL_VERBS = ("launch", "status", "wait-idle", "kill")   # plus shot / wait-watch, which take args


SI_GC_CONTROLLER = 6   # SerialInterface::SIDEVICE_GC_CONTROLLER
SI_NONE = 0


def launch_command(script: Path, iso: Path = ISO, speed: float | None = None,
                   video: str | None = None, ports: int = 1,
                   extra: list[str] | None = None) -> list[str]:
    """Dolphin command line. `ports` emulated GC controllers are plugged in
    (the game sees them as connected pads with no input until the script
    overrides them); the rest are empty."""
    # Menu driving takes screenshots (`shot`), so it keeps the windowed app.
    dolphin = dolphin_config.binary(gui=True)
    cmd = [str(dolphin), "-e", str(iso), "--script", str(script),
           *dolphin_config.launch_flags(dolphin, video)]
    if speed is not None:
        cmd += ["-C", f"Dolphin.Core.EmulationSpeed={speed}"]   # 0 = unlimited
    for i in range(4):
        dev = SI_GC_CONTROLLER if i < ports else SI_NONE
        cmd += ["-C", f"Dolphin.Core.SIDevice{i}={dev}"]
    return cmd + list(extra or [])


def launch(remote_dir: Path, speed: float | None, video: str | None, ports: int,
           extra: list[str]) -> int:
    remote_dir.mkdir(parents=True, exist_ok=True)
    out = (remote_dir / "dolphin.out").open("ab")
    cmd = launch_command(HERE / "remote.py", speed=speed, video=video, ports=ports, extra=extra)
    proc = subprocess.Popen(cmd, stdout=out, stderr=subprocess.STDOUT, start_new_session=True,
                            env={**os.environ, "MELEE_REMOTE_DIR": str(remote_dir)})
    (remote_dir / "dolphin.pid").write_text(str(proc.pid))
    print(f"launched pid {proc.pid}: {' '.join(cmd)}")
    return proc.pid


def read_status(remote_dir: Path) -> dict | None:
    try:
        return proto.read_json(remote_dir / "status.json")
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def print_status(remote_dir: Path) -> None:
    s = read_status(remote_dir)
    if s is None:
        sys.exit("no status.json yet (is Dolphin running remote.py and a game booted?)")
    age = time.time() - s["wall_time"]
    print(f"frame={s['frame']} fps={s['fps']} seed={s['seed_hex']} changed={s['seed_changed']} "
          f"age={age:.1f}s seq={s['last_seq']} queue={s['queue_frames']} idle={s['idle']} "
          f"errors={s['errors']} note={s['note']!r}")
    if s.get("pad0"):
        print(f"pad0: {s['pad0']}")
    if s.get("held"):
        held = {k: v for k, v in s["held"].items() if v}
        print(f"held: {held}")
    for addr, data in s.get("watch", {}).items():
        print(f"watch {addr}: {data}")
    for i, f in enumerate(s["fighters"]):
        x, y, z = f["pos"]
        print(f"p{i}: {f['base']} {f['kind_name']}(kind={f['kind']}) player={f['player_id']} "
              f"motion={f['motion_id']} pos=({x:.4f}, {y:.4f}, {z:.4f})")
    if not s["fighters"]:
        print("fighters: none")


def wait_until(remote_dir: Path, pred, timeout: float) -> dict:
    deadline = time.monotonic() + timeout
    while True:
        s = read_status(remote_dir)
        if s is not None and pred(s):
            return s
        if time.monotonic() > deadline:
            sys.exit(f"timed out after {timeout}s waiting on status.json")
        time.sleep(0.05)


def send(remote_dir: Path, commands: list[dict]) -> int:
    remote_dir.mkdir(parents=True, exist_ok=True)
    path = remote_dir / "cmd.json"
    seq = proto.next_seq(path)
    proto.write_json_atomic(path, {"seq": seq, "commands": commands})
    return seq


def shot(remote_dir: Path, path: Path, timeout: float) -> Path:
    """Ask remote.py for a screenshot and block until the PNG lands."""
    path = path.resolve()
    before = path.stat().st_mtime_ns if path.exists() else None
    send(remote_dir, [proto.parse_command(f"shot {path}")])
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists() and path.stat().st_mtime_ns != before:
            print(path)
            return path
        time.sleep(0.05)
    sys.exit(f"no screenshot at {path} after {timeout}s (is a game running?)")


def watched_u32(status: dict, addr: int) -> int | None:
    """Big-endian u32 at the start of a watched region, or None if not watched yet."""
    data = status.get("watch", {}).get(f"0x{addr:08X}")
    if not data or data.startswith("error") or len(data) < 8:
        return None
    return int(data[:8], 16)


def wait_watch(remote_dir: Path, addr: int, lo: int, hi: int, timeout: float) -> int:
    """Ensure `addr` is watched, then block until its u32 value is within [lo, hi]."""
    s = read_status(remote_dir)
    if s is None or watched_u32(s, addr) is None:
        send(remote_dir, [proto.parse_command(f"watch 0x{addr:08X} 4")])
    s = wait_until(remote_dir, lambda st: (v := watched_u32(st, addr)) is not None and lo <= v <= hi,
                   timeout)
    v = watched_u32(s, addr)
    print(f"0x{addr:08X} = {v} (0x{v:X}) at frame {s['frame']}")
    return v


def kill(remote_dir: Path) -> None:
    pid = None
    s = read_status(remote_dir)
    if s:
        pid = s.get("pid")
    pidfile = remote_dir / "dolphin.pid"
    if pid is None and pidfile.exists():
        pid = int(pidfile.read_text())
    if pid is None:
        sys.exit("no pid known")
    os.kill(pid, signal.SIGTERM)
    print(f"sent SIGTERM to {pid}")


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dir", type=Path, default=DEFAULT_DIR, help="remote dir (cmd/status/log)")
    ap.add_argument("--sync", action="store_true", help="wait until the batch is applied and the queue is idle")
    ap.add_argument("--timeout", type=float, default=60.0)
    ap.add_argument("--speed", type=float, default=None, help="launch: EmulationSpeed (0 = unlimited)")
    ap.add_argument("--video", default=None, help="launch: video backend, e.g. Null")
    ap.add_argument("--ports", type=int, default=1, choices=[1, 2, 3, 4],
                    help="launch: how many emulated GC controllers are plugged in")
    ap.add_argument("--extra", nargs="*", default=[], help="launch: extra Dolphin args")
    ap.add_argument("words", nargs="+", help="local verb or commands (';'-separated allowed)")
    a = ap.parse_args(argv)

    verb = a.words[0]
    if verb in LOCAL_VERBS and len(a.words) == 1:
        if verb == "launch":
            launch(a.dir, a.speed, a.video, a.ports, a.extra)
        elif verb == "status":
            print_status(a.dir)
        elif verb == "wait-idle":
            wait_until(a.dir, lambda s: s["idle"], a.timeout)
            print_status(a.dir)
        elif verb == "kill":
            kill(a.dir)
        return

    if verb == "shot" and len(a.words) <= 2:
        shot_path = Path(a.words[1]) if len(a.words) == 2 else a.dir / "shot.png"
        shot(a.dir, shot_path, a.timeout)
        return

    if verb == "wait-watch":
        if len(a.words) != 4:
            sys.exit("usage: wait-watch <addr> <lo> <hi>")
        wait_watch(a.dir, int(a.words[1], 0), int(a.words[2], 0), int(a.words[3], 0), a.timeout)
        return

    try:
        commands = proto.parse_commands(a.words)
    except proto.CommandError as e:
        sys.exit(f"bad command: {e}")
    seq = send(a.dir, commands)
    print(f"sent seq {seq}: {len(commands)} command(s)")
    if a.sync:
        wait_until(a.dir, lambda s: s["last_seq"] >= seq and s["idle"], a.timeout)
        print_status(a.dir)


if __name__ == "__main__":
    main()
