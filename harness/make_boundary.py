"""Create, record, gate and register a match-start boundary in one command.

    cd harness && uv run python make_boundary.py --stage Battlefield --players Fox Marth [--stocks 4]
                                                 [--name start_bf_fox_marth4] [--no-register]
                                                 [--transform PORT ...]
                                                 [--gecko ucf-0.8 neutral-spawn --spawn neutral-2020]
                                                 [--ports 2 4] [--time-limit 8]

Sheik is a player name too: the port picks Zelda on the CSS and holds A while
the match loads (--transform on that port), as a person does.

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

A Slippi layout: `--gecko ucf-0.8 neutral-spawn --spawn neutral-2020` boots
with those Gecko codes (gecko.py; text from MELEE_GECKO_DIR), so the match
starts where Slippi's tournament builds place the players; the cold scenario
then names the port's spawn rule (melee_lib::slippi::SpawnRule) and each
fighter's controller fix. The savestate keeps the installed code list in RAM
and Dolphin does not reinstall over it: a scenario recorded from the boundary
must list exactly the boundary's codes. `--ports 2 4` seats the players on
those ports (only they hold a controller), as a tournament station does; the
start scene then has fighter slots 1 and 3, which only its cold twin can gate.
`--time-limit 8` plays stock with an eight-minute timer, the tournament rule.
`--game-start-seed N` creates the match from a Slippi replay's Game Start seed
(written where Slippi reads it, before the Ground and Players exist), so what
the stage draws at creation is the replay's: Final Destination's background
accelerations and Fountain of Dreams' first platform waits outlive the
boundary, and a replay bridged from any other boundary leaves the console's
random stream there. Such a boundary serves that one replay; the cold twin
names the seed, and its gate checks the port's setup draws reach the saved one.
A code the port names with a scenario flag (`ps-preload`: `stadium_preload`,
`ps-frozen`: `stadium_frozen`, `frozen-stages`: `frozen_stages`; gecko.SCENARIO_FLAGS)
sets it in both scenarios.

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
import data_root  # noqa: E402
import dolphin_config  # noqa: E402
import gecko  # noqa: E402
from record import dolphin_flags  # noqa: E402

ISO = data_root.ROMS / "GALE01.iso"
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
# -> CSS CharacterKind (ft/forward.h). Sheik is Zelda's CSS icon with A held
# while the match loads (TRANSFORMED).
CHARACTERS = {
    "CaptainFalcon": 0x00, "DonkeyKong": 0x01, "Fox": 0x02, "GameAndWatch": 0x03, "Kirby": 0x04,
    "Bowser": 0x05, "Link": 0x06, "Luigi": 0x07, "Mario": 0x08, "Marth": 0x09,
    "Mewtwo": 0x0A, "Ness": 0x0B, "Peach": 0x0C, "Pikachu": 0x0D, "IceClimbers": 0x0E,
    "Jigglypuff": 0x0F, "Samus": 0x10, "Yoshi": 0x11, "Zelda": 0x12, "Falco": 0x14,
    "YoungLink": 0x15, "DrMario": 0x16, "Roy": 0x17, "Pichu": 0x18, "Ganondorf": 0x19,
    "Sheik": 0x12,
}
# fn_8016D8AC (gm_16AE.c:1573-1583): a human port holding A when the match
# loads swaps CKIND_ZELDA for CKIND_SEAK (and back). The scenario names the
# character the player starts as; the CSS pick is the other form.
TRANSFORMED = {"Sheik": "Zelda"}
TIMEOUT = 300.0


def default_name(stage: str, players: list[str], stocks: int) -> str:
    return f"start_{STAGES[stage][1]}_{'_'.join(p.lower() for p in players)}{stocks}"


def gecko_line(codes: list[str] | None) -> str:
    return "gecko = [" + ", ".join(f'"{c}"' for c in codes) + "]\n" if codes else ""


def fix_line(codes: list[str] | None) -> str:
    """A controller-fix code (ucf-*) is also each fighter's `controller_fix`,
    which is what the port reads."""
    fixes = [c for c in codes or [] if c.startswith("ucf-")]
    return f'controller_fix = "{fixes[0]}"\n' if fixes else ""


def creation_line(game_start_seed: int | None) -> str:
    """A boundary whose match was created from a replay's Game Start seed."""
    if game_start_seed is None:
        return ""
    return ("# The match was created from this seed, written at Slippi's Game Start read\n"
            "# (0x8016E74C, before fn_8016E730 creates the Ground and Players).\n"
            f"game_start_seed = {game_start_seed}\n")


def start_scenario(name: str, stage: str, players: list[str], stocks: int,
                   codes: list[str] | None = None, slots: list[int] | None = None,
                   game_start_seed: int | None = None) -> str:
    # The savestate importer covers slots 0..n: other ports gate the cold twin.
    gate = f'gate = "{name}_cold"\n' if slots and slots != list(range(len(players))) else ""
    slots = slots or list(range(len(players)))
    transformed = "".join(
        f"# {p} is {TRANSFORMED[p]}'s CSS icon with A held on port {i + 1} while the match loads\n"
        f"# (fn_8016D8AC); {TRANSFORMED[p]} sleeps beside {p} as the transformation partner.\n"
        for i, p in enumerate(players) if p in TRANSFORMED)
    fighters = "\n".join(
        f'[[fighters]]\nslot = {slots[i]}\nkind = "{kind}"\ncontroller = "{"scripted" if i == 0 else "idle"}"\n'
        f'{fix_line(codes)}'
        for i, kind in enumerate(players))
    return f'''# {" vs ".join(players)} on {stage}, {stocks} stocks, start boundary. Savestate
# made by make_boundary.py (headless, RAM-only save-data pokes: characters and
# Battlefield/FD unlocked, stock rules, items off; the memory card is a
# discarded copy). Every fighter in Entry at the first initialised frame.
{transformed}name = "{name}"
{gate}savestate = "harness/roms/{name}.sav"
frames = 600
seed = 1
{creation_line(game_start_seed)}stage = "{stage}"
{gecko.scenario_flag_lines(gecko.scenario_flags(codes))}{gecko_line(codes)}
inputs = []

{fighters}'''


def cold_scenario(name: str, stage: str, players: list[str], stocks: int, seed: int,
                  costumes: list[int], spawn: str | None = None,
                  codes: list[str] | None = None, slots: list[int] | None = None,
                  time_limit: int = 0, game_start_seed: int | None = None) -> str:
    slots = slots or list(range(len(players)))
    timer_line = f"time_limit = {time_limit * 60}\n" if time_limit else ""
    spawn_line = f'spawn = "{spawn}"\n' if spawn else ""
    fighters = "\n".join(
        f'[[fighters]]\nslot = {slots[i]}\nkind = "{kind}"\ncostume = {costumes[i]}\nstocks = {stocks}\n'
        f'controller = "idle"\n{fix_line(codes)}' for i, kind in enumerate(players))
    return f'''# Parameters only; seed is after stage/fighter creation, before music.
# Stock {stocks}, items off, normal Versus Entry, human ports, neutral pads.
name = "{name}_cold"
expected = "{name}"
frames = 600
seed = {seed}
{creation_line(game_start_seed)}stage = "{stage}"
{gecko.scenario_flag_lines(gecko.scenario_flags(codes))}all_characters_unlocked = true
{timer_line}{spawn_line}inputs = []

{fighters}'''


def boundary_entry(name: str, stage: str, players: list[str], stocks: int, seed: int,
                   costumes: list[int] | None = None) -> str:
    quoted = ", ".join(f'"{p}"' for p in players)
    # Costumes other than each port's first are written out (boundary.rs).
    worn = f"costumes = {list(costumes)}\n" if costumes and any(costumes) else ""
    return (f'\n[[boundary]]\nname = "{name}"\ncold = "{name}_cold"\n'
            f'savestate = "harness/roms/{name}.sav"\nstage = "{stage}"\nplayers = [{quoted}]\n'
            f'stocks = {stocks}\nseed = {seed}\n{worn}')


def make_savestate(sav: Path, stkind: int, players: list[int], stocks: int, timeout: float,
                   transform: list[int] | None = None, costumes: list[int] | None = None,
                   codes: list[str] | None = None, slots: list[int] | None = None,
                   time_limit: int = 0, game_start_seed: int | None = None) -> dict:
    """Run boundary_script.py in a private headless Dolphin; return its summary."""
    done, err = Path(str(sav) + ".done"), Path(str(sav) + ".err")
    for p in (done, err):
        p.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix="melee-boundary-") as scratch, \
            dolphin_config.isolated_user_dir() as user_dir:
        config = Path(scratch) / "config.json"
        config.write_text(json.dumps({"savestate": str(sav), "stkind": stkind,
                                      "players": players, "stocks": stocks,
                                      "transform": transform or [],
                                      "time_limit_minutes": time_limit,
                                      **({"ports": slots} if slots else {}),
                                      **({"game_start_seed": game_start_seed}
                                         if game_start_seed is not None else {}),
                                      **({"costumes": costumes} if costumes else {})}))
        dolphin = dolphin_config.binary()
        flags = dolphin_flags(dolphin, ports=len(players))
        if codes:
            # The codes run from boot, so match setup (spawns) sees them.
            gecko.install(Path(user_dir), codes)
            flags += ["-C", "Dolphin.Core.EnableCheats=True"]
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
    ap.add_argument("--transform", type=int, nargs="*", default=[],
                    help="ports that hold A while the match loads (Zelda <-> Sheik); implied for Sheik")
    ap.add_argument("--costumes", type=int, nargs="+",
                    help="one costume per port (CSS X presses; default: each port's first free costume)")
    ap.add_argument("--gecko", nargs="+", default=[],
                    help="Gecko codes installed from boot (gecko.py names, e.g. neutral-spawn)")
    ap.add_argument("--spawn", choices=["retail", "neutral-2019", "neutral-2019-entry", "neutral-2020"],
                    help="the cold scenario's spawn rule, matching --gecko")
    ap.add_argument("--ports", type=int, nargs="+", choices=[1, 2, 3, 4],
                    help="each player's controller port, ascending (default: the first ports)")
    ap.add_argument("--time-limit", type=int, default=0, metavar="MINUTES",
                    help="stock with a countdown timer (default: none)")
    ap.add_argument("--game-start-seed", type=int, metavar="SEED",
                    help="create the match from this seed (a replay's Game Start seed: "
                         "`game_start_seed` in melee-sim replay --retail-inputs)")
    ap.add_argument("--timeout", type=float, default=TIMEOUT)
    a = ap.parse_args(argv)
    if (a.gecko or a.spawn or a.ports or a.time_limit or a.game_start_seed is not None) \
            and not a.no_register:
        # boundaries.toml describes explorer matches: retail spawns, first
        # ports, no timer, no codes, the seed the menus left.
        sys.exit("a boundary with codes, ports, a timer or a Game Start seed is not an explorer "
                 "boundary: pass --no-register")
    if a.game_start_seed is not None and not 0 <= a.game_start_seed < 1 << 32:
        sys.exit("--game-start-seed is a 32-bit seed")
    if a.game_start_seed is not None and not a.name:
        sys.exit("a boundary made for one replay needs its own --name")
    slots = None
    if a.ports is not None:
        if len(a.ports) != len(a.players) or sorted(set(a.ports)) != a.ports:
            sys.exit("--ports needs one ascending port per player")
        slots = [p - 1 for p in a.ports]
        # Every Dolphin this command starts plugs controllers into those ports.
        os.environ[dolphin_config.SI_PORTS_ENV] = ",".join(str(s) for s in slots)
    if a.costumes is not None and len(a.costumes) != len(a.players):
        sys.exit("--costumes needs one costume per player")
    transform = sorted(set(a.transform) | {i for i, p in enumerate(a.players) if p in TRANSFORMED})
    if not 2 <= len(a.players) <= 4:
        sys.exit("a boundary has 2 to 4 players")
    name = a.name or default_name(a.stage, a.players, a.stocks)
    sav = data_root.ROMS / f"{name}.sav"
    scenario, cold = data_root.SCENARIOS / f"{name}.toml", data_root.SCENARIOS / f"{name}_cold.toml"
    existing = [p for p in (sav, Path(str(sav) + ".json"), scenario, cold) if p.exists()]
    if existing:
        sys.exit(f"refusing to overwrite: {', '.join(str(p) for p in existing)}")
    if any(b["name"] == name for b in tomllib.loads(BOUNDARIES.read_text())["boundary"]):
        sys.exit(f"{name} is already in {BOUNDARIES.name}")

    print(f"== {name}: driving the menus", flush=True)
    t0 = time.monotonic()
    summary = make_savestate(sav, STAGES[a.stage][0], [CHARACTERS[p] for p in a.players],
                             a.stocks, a.timeout, transform, a.costumes, a.gecko, slots, a.time_limit,
                             a.game_start_seed)
    sidecar = json.loads(Path(str(sav) + ".json").read_text())
    seed = sidecar["seed"]
    costumes = [p["color"] for p in summary["css_players"]]
    print(f"   saved {sav.name} at frame {summary['frame']} in {time.monotonic() - t0:.0f}s: seed {seed}, "
          f"costumes {costumes}, item frequency {summary['item_frequency']}", flush=True)
    if a.game_start_seed is not None:
        print(f"   created from Game Start seed {a.game_start_seed} (it replaced {summary['seed_replaced']})",
              flush=True)

    scenario.write_text(start_scenario(name, a.stage, a.players, a.stocks, a.gecko, slots,
                                       a.game_start_seed))
    cold.write_text(cold_scenario(name, a.stage, a.players, a.stocks, seed, costumes, a.spawn, a.gecko,
                                  slots, a.time_limit, a.game_start_seed))
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
              f"{boundary_entry(name, a.stage, a.players, a.stocks, seed, costumes)}")
        sys.exit(1)
    if a.no_register:
        print(f"== {name} passes; not registered (--no-register)")
        return
    with BOUNDARIES.open("a") as f:
        f.write(boundary_entry(name, a.stage, a.players, a.stocks, seed, costumes))
    if BACKUP.is_dir():
        for src in (sav, Path(str(sav) + ".json")):
            dst = BACKUP / src.name
            if not dst.exists():
                shutil.copy2(src, dst)
    print(f"== {name} registered in {BOUNDARIES.name}")


if __name__ == "__main__":
    main()
