"""Create, record, gate and register a match-start boundary in one command.

    cd harness && uv run python make_boundary.py --stage Battlefield --players Fox Marth [--stocks 4]
                                                 [--name start_bf_fox_marth4] [--no-register]

A boundary (boundaries.toml) is a retail savestate at a match's first
initialised frame whose cold construction the port reproduces exactly; the
explorer starts its cases there and replay_to_scenario.py replays them in
Dolphin. This command makes one with no human steps:

1. Boots the disc in headless Dolphin, in a private user folder (the memory
   card copy is discarded), and runs dolphin/boundary_script.py: it drives the
   menus from the game's own state (VS. Mode > Melee, each port's character,
   the stage) and saves roms/<name>.sav with its sidecar.
2. Writes scenarios/<name>.toml (the retail start scene, 600 ticks) and
   scenarios/<name>_cold.toml (the same match built from parameters: stage,
   characters, costumes, stocks and the sidecar seed).
3. Records <name> with record_many.py (a private Dolphin user folder, so
   several make_boundary runs can proceed at once; --ledger-suffix ledger600, as for every
   start scene) and gates <name>_cold with melee-sim.
4. When the gate passes, appends the boundary to boundaries.toml (and copies
   the savestate to ~/melee-data/roms when that backup exists). A failing gate
   leaves everything unregistered: the port does not yet support the layout.

It never overwrites: an existing savestate or scenario of the same name stops
it before Dolphin starts.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import dolphin_config  # noqa: E402
from record import dolphin_flags  # noqa: E402

ISO = HERE / "roms/GALE01.iso"
BOUNDARIES = HERE / "boundaries.toml"
BACKUP = Path.home() / "melee-data/roms"
SCRIPT = HERE / "dolphin/boundary_script.py"

# Scenario stage name -> (StKind, short name). gr/forward.h StKind.
STAGES = {
    "FinalDestination": (0x20, "fd"),
    "Battlefield": (0x1F, "bf"),
    "YoshisStory": (0x08, "ys"),
    "DreamLand": (0x1C, "dl"),
    "FountainOfDreams": (0x02, "fod"),
    "PokemonStadium": (0x03, "ps"),
}
# Scenario character name (the port's spelling, melee-lib scene_characters!)
# -> CSS CharacterKind (ft/forward.h). Sheik is chosen in-match from Zelda.
CHARACTERS = {
    "CaptainFalcon": 0x00, "DonkeyKong": 0x01, "Fox": 0x02, "GameAndWatch": 0x03, "Kirby": 0x04,
    "Bowser": 0x05, "Link": 0x06, "Luigi": 0x07, "Mario": 0x08, "Marth": 0x09,
    "Mewtwo": 0x0A, "Ness": 0x0B, "Peach": 0x0C, "Pikachu": 0x0D, "IceClimbers": 0x0E,
    "Jigglypuff": 0x0F, "Samus": 0x10, "Yoshi": 0x11, "Zelda": 0x12, "Falco": 0x14,
    "YoungLink": 0x15, "DrMario": 0x16, "Roy": 0x17, "Pichu": 0x18, "Ganondorf": 0x19,
}
TIMEOUT = 300.0


def default_name(stage: str, players: list[str], stocks: int) -> str:
    return f"start_{STAGES[stage][1]}_{'_'.join(p.lower() for p in players)}{stocks}"


def start_scenario(name: str, stage: str, players: list[str], stocks: int) -> str:
    fighters = "\n".join(
        f'[[fighters]]\nslot = {i}\nkind = "{kind}"\ncontroller = "{"scripted" if i == 0 else "idle"}"\n'
        for i, kind in enumerate(players))
    return f'''# {" vs ".join(players)} on {stage}, {stocks} stocks, start boundary. Savestate
# made by make_boundary.py (headless, RAM-only save-data pokes: characters and
# Battlefield/FD unlocked, stock rules, items off; the memory card is a
# discarded copy). Every fighter in Entry at the first initialised frame.
name = "{name}"
savestate = "harness/roms/{name}.sav"
frames = 600
seed = 1
stage = "{stage}"

inputs = []

{fighters}'''


def cold_scenario(name: str, stage: str, players: list[str], stocks: int, seed: int,
                  costumes: list[int]) -> str:
    fighters = "\n".join(
        f'[[fighters]]\nslot = {i}\nkind = "{kind}"\ncostume = {costumes[i]}\nstocks = {stocks}\n'
        f'controller = "idle"\n' for i, kind in enumerate(players))
    return f'''# Parameters only; seed is after stage/fighter creation, before music.
# Stock {stocks}, items off, normal Versus Entry, human ports, neutral pads.
name = "{name}_cold"
expected = "{name}"
frames = 600
seed = {seed}
stage = "{stage}"
all_characters_unlocked = true
inputs = []

{fighters}'''


def boundary_entry(name: str, stage: str, players: list[str], stocks: int, seed: int) -> str:
    quoted = ", ".join(f'"{p}"' for p in players)
    return (f'\n[[boundary]]\nname = "{name}"\ncold = "{name}_cold"\n'
            f'savestate = "harness/roms/{name}.sav"\nstage = "{stage}"\nplayers = [{quoted}]\n'
            f'stocks = {stocks}\nseed = {seed}\n')


def make_savestate(sav: Path, stkind: int, players: list[int], stocks: int, timeout: float) -> dict:
    """Run boundary_script.py in a private headless Dolphin; return its summary."""
    done, err = Path(str(sav) + ".done"), Path(str(sav) + ".err")
    for p in (done, err):
        p.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix="melee-boundary-") as scratch, \
            dolphin_config.isolated_user_dir() as user_dir:
        config = Path(scratch) / "config.json"
        config.write_text(json.dumps({"savestate": str(sav), "stkind": stkind,
                                      "players": players, "stocks": stocks}))
        dolphin = dolphin_config.binary()
        flags = dolphin_flags(dolphin, ports=len(players))
        env = {**os.environ, "MELEE_BOUNDARY_CONFIG": str(config)}
        log = sav.with_suffix(".boundary.log")
        with log.open("wb") as out:
            proc = subprocess.Popen([str(dolphin), "-e", str(ISO), "--script", str(SCRIPT), *flags,
                                     "-u", str(user_dir)],
                                    env=env, stdout=out, stderr=subprocess.STDOUT, start_new_session=True)
            t0 = time.monotonic()
            try:
                while not done.exists() and not err.exists():
                    if proc.poll() is not None:
                        sys.exit(f"Dolphin exited with {proc.returncode} before the boundary; see {log}")
                    if time.monotonic() - t0 > timeout:
                        sys.exit(f"no boundary after {timeout}s; see {log}")
                    time.sleep(0.5)
            finally:
                proc.terminate()
                try:
                    proc.wait(10)
                except subprocess.TimeoutExpired:
                    proc.kill()
    if err.exists():
        sys.exit(f"boundary_script.py failed:\n{err.read_text()[-2000:]}")
    summary = json.loads(done.read_text())
    done.unlink()
    log.unlink(missing_ok=True)
    return summary


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--stage", required=True, choices=sorted(STAGES))
    ap.add_argument("--players", required=True, nargs="+", choices=sorted(CHARACTERS),
                    help="one character per port, P1 first")
    ap.add_argument("--stocks", type=int, default=4)
    ap.add_argument("--name", help="default: start_<stage>_<p1>_<p2><stocks>")
    ap.add_argument("--no-register", action="store_true", help="record and gate, but leave boundaries.toml alone")
    ap.add_argument("--timeout", type=float, default=TIMEOUT)
    a = ap.parse_args(argv)
    if not 2 <= len(a.players) <= 4:
        sys.exit("a boundary has 2 to 4 players")
    name = a.name or default_name(a.stage, a.players, a.stocks)
    sav = HERE / "roms" / f"{name}.sav"
    scenario, cold = HERE / "scenarios" / f"{name}.toml", HERE / "scenarios" / f"{name}_cold.toml"
    existing = [p for p in (sav, Path(str(sav) + ".json"), scenario, cold) if p.exists()]
    if existing:
        sys.exit(f"refusing to overwrite: {', '.join(str(p) for p in existing)}")
    if any(b["name"] == name for b in tomllib.loads(BOUNDARIES.read_text())["boundary"]):
        sys.exit(f"{name} is already in {BOUNDARIES.name}")

    print(f"== {name}: driving the menus", flush=True)
    t0 = time.monotonic()
    summary = make_savestate(sav, STAGES[a.stage][0], [CHARACTERS[p] for p in a.players],
                             a.stocks, a.timeout)
    sidecar = json.loads(Path(str(sav) + ".json").read_text())
    seed = sidecar["seed"]
    costumes = [p["color"] for p in summary["css_players"]]
    print(f"   saved {sav.name} at frame {summary['frame']} in {time.monotonic() - t0:.0f}s: seed {seed}, "
          f"costumes {costumes}, item frequency {summary['item_frequency']}", flush=True)

    scenario.write_text(start_scenario(name, a.stage, a.players, a.stocks))
    cold.write_text(cold_scenario(name, a.stage, a.players, a.stocks, seed, costumes))
    print(f"== recording {scenario.name}", flush=True)
    # record_many gives the run a private Dolphin user folder, so parallel
    # make_boundary runs (one per agent) never share config or card writes.
    subprocess.run([sys.executable, str(HERE / "record_many.py"), str(scenario), "--jobs", "1",
                    "--", "--ledger-suffix", "ledger600"],
                   cwd=HERE, check=True, stdout=subprocess.DEVNULL)
    print(f"== gating {cold.name}", flush=True)
    gate = subprocess.run(["cargo", "run", "-q", "--release", "-p", "melee-sim", "--", "gate", str(cold)],
                          cwd=ROOT, text=True, capture_output=True)
    print("   " + "\n   ".join((gate.stdout + gate.stderr).strip().splitlines()[-6:]), flush=True)
    if gate.returncode:
        print(f"== {name} is not registered: the port does not reproduce it yet. Its savestate, scenarios "
              f"and traces stay for that work; register it by rerunning the gate and adding\n"
              f"{boundary_entry(name, a.stage, a.players, a.stocks, seed)}")
        sys.exit(1)
    if a.no_register:
        print(f"== {name} passes; not registered (--no-register)")
        return
    with BOUNDARIES.open("a") as f:
        f.write(boundary_entry(name, a.stage, a.players, a.stocks, seed))
    if BACKUP.is_dir():
        for src in (sav, Path(str(sav) + ".json")):
            dst = BACKUP / src.name
            if not dst.exists():
                shutil.copy2(src, dst)
    print(f"== {name} registered in {BOUNDARIES.name}")


if __name__ == "__main__":
    main()
