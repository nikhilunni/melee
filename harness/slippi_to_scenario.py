"""Turn a Slippi replay's inputs into a retail scenario, and its cold twin.

    cargo run -q --release -p melee-sim -- replay <game.slp> --retail-inputs inputs.jsonl
    cd harness && uv run python slippi_to_scenario.py inputs.jsonl --name <scenario> \
        --boundary <start scene> [--ticks N]

A replay records no retail memory, so a divergence the port shows against one
cannot be triaged. This feeds the replay's pads to retail instead: the same
match from a start boundary with the replay's layout (stage, characters,
costumes, ports, timer, the Slippi build's Gecko codes), so the ordinary gate
and triage compare the port with a full retail trace of the same game.

`inputs.jsonl` is `melee-sim replay --retail-inputs`: a header (setup, the
Game Start seed and the pre-music boundary seed it reaches) and one raw pad
per player and tick. The boundary is a `make_boundary.py` start scene made for
that setup, for example

    make_boundary.py --stage Battlefield --players Marth Peach --costumes 1 1 --ports 2 4 \
        --time-limit 8 --gecko ucf-0.8 neutral-spawn --spawn neutral-2020 --no-register

On Final Destination, Fountain of Dreams and a transforming Pokemon Stadium
the boundary must be made for the replay, with `--game-start-seed <header
game_start_seed> --name <its own>`: those stages keep what they drew at
creation (the background's accelerations, the platforms' first waits, the
base form's first duration), which `boundary_seed` cannot change afterwards.
Dream Land keeps the first fly-by's delay (3000 to 4000 ticks), so a bridge
past tick 3000 there needs the replay's boundary too.
`--foreign-creation` accepts another boundary there; retail then plays the
replay's pads but leaves its random stream at the stage's first draw (on
Stadium, at the first transformation; on Dream Land, at the first fly-by).

Two scenarios are written:

- `<name>.toml`, which record_many.py records: the boundary's savestate, the
  pads on the tick input clock, and `boundary_seed`, which the tracers write
  over the saved seed before the first tick. Retail then draws what the
  replay's game drew, so it follows the replay through random outcomes too.
- `<name>_cold.toml`, which the port gates: the boundary built from parameters
  (its own `seed`), then `boundary_seed`, driven by the pads the recording
  consumed.

The pads are the header's records, which `--retail-inputs` has already
checked to read as the replay's Pre Frame (melee-sim replay_retail_pads.rs).
A console's failed poll (`err`: PAD_ERR_TRANSFER, where HSD keeps the last
status) is scheduled as one. After recording, record.py checks that every
tick consumed its scheduled pad (pad_schedule_check.py), and
replay_vs_trace.py compares the recording with the replay itself: inputs,
seeds and fighter fields.

The retail scenario carries `render_every_tick = true` (tick_trace.py): the
recorder then renders after every tick, as the console that made the replay
did. What follows the display pass (a tether's rope and so its wall catch,
the magnifier's damage, a screen KO's position) is otherwise another game's.
`--dolphin-cadence` leaves it out. `--no-display-pass-before N` plays a
console's lag frame: records N-1 and N run in one frame, so the display pass
between them does not happen (`no_display_pass_before`, tick_trace.py).

The controller fix is the boundary's (its codes live in the savestate), which
may be a later UCF than the replay's build ran. The Stadium codes are the
boundary's too (`stadium_preload`, `stadium_frozen`, from its Gecko list) and
must be the replay's, because they move the stage's draws.
"""
from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

import data_root
import gecko
import replay_to_scenario as pads

#: Scenario.validate's limit (melee-sim scenario.rs).
MAX_TICKS = 30_000
#: The Start button's pad bit. A recorded frame with Start held is a console
#: with Pause off (a paused game records no frames, and the press that ends
#: a pause is never recorded): the boundary's match pauses, so Start is not
#: fed to it. The port has no pause either.
START = 1 << 12


def load_boundary(name: str) -> tuple[dict, dict]:
    start, cold = (data_root.SCENARIOS / f"{name}{suffix}.toml" for suffix in ("", "_cold"))
    if not start.exists() or not cold.exists():
        sys.exit(f"no boundary {name}: make it with make_boundary.py (see this file's header)")
    return tomllib.loads(start.read_text()), tomllib.loads(cold.read_text())


def stage_flags(start: dict, cold: dict) -> dict[str, bool]:
    """The boundary's stage-code flags: what its savestate runs (the start
    scene's Gecko list), or what a boundary written by hand names."""
    flags = gecko.scenario_flags(start.get("gecko"))
    for flag in gecko.SCENARIO_FLAGS.values():
        if start.get(flag) or cold.get(flag):
            flags[flag] = True
    return flags


def check_stage_codes(header: dict, flags: dict[str, bool], boundary: str) -> None:
    """The replay's console and the boundary must run the same stage codes
    (a header written before it carried the flags is not checked)."""
    for code, flag in gecko.SCENARIO_FLAGS.items():
        if flag in header and bool(header[flag]) != bool(flags.get(flag)):
            sys.exit(f"{boundary} is not the replay's match: the replay has {flag} = "
                     f"{str(bool(header[flag])).lower()}; make the boundary "
                     f"{'with' if header[flag] else 'without'} --gecko {code}")


#: Stages whose creation draws set state that outlives the boundary:
#: grLast_8021AC30's two background accelerations (four draws),
#: grIzumi_801CC358's first platform waits and grStadium_801D13E0's first
#: base-form duration (one draw: when the first transformation begins).
CREATION_DRAW_STAGES = ("FinalDestination", "FountainOfDreams", "PokemonStadium")
#: Header flags of codes under which Stadium's duration is never read: Frozen
#: Stadium (0x801D45FC, the wait never ends) and Frozen Stages (0x801D1548,
#: the controller never runs).
STADIUM_NEVER_TRANSFORMS = ("stadium_frozen", "frozen_stages")
#: Dream Land's creation draw is the first fly-by's delay (grOldPupupu_80210C7C,
#: 3000..4000 ticks). When it runs out grOldPupupu_80210D10 draws the fly-by
#: (four values) on another tick than the console's; nothing shows before.
DREAM_LAND_FIRST_FLY_BY = 3000


def keeps_creation_draws(header: dict, ticks: int | None = None) -> bool:
    """Whether what the replay's stage drew at creation shapes its match
    within `ticks` (the whole replay when None)."""
    if header["stage"] == "PokemonStadium" and any(header.get(flag) for flag in STADIUM_NEVER_TRANSFORMS):
        return False
    if header["stage"] == "DreamLand":
        return ticks is None or ticks > DREAM_LAND_FIRST_FLY_BY
    return header["stage"] in CREATION_DRAW_STAGES


def check_creation(header: dict, cold: dict, boundary: str, foreign: bool = False,
                   ticks: int | None = None) -> int | None:
    """The Game Start seed the boundary's match was created from, when it is
    the replay's; None for a boundary whose creation is its own."""
    want, made = header.get("game_start_seed"), cold.get("game_start_seed")
    if made is not None and made == want:
        if cold["seed"] != header["boundary_seed"]:
            sys.exit(f"{boundary} was created from the replay's Game Start seed {want} but retail "
                     f"reached seed {cold['seed']} where the port's setup draws reach "
                     f"{header['boundary_seed']}")
        return made
    if foreign:
        return None
    if made is not None:
        sys.exit(f"{boundary} was created from Game Start seed {made}, the replay's is {want}: "
                 "it is another replay's boundary (--foreign-creation to use it anyway)")
    if keeps_creation_draws(header, ticks):
        sys.exit(f"{header['stage']} keeps its creation draws: make the boundary for this replay "
                 f"with make_boundary.py --game-start-seed {want} --name <name> "
                 "(--foreign-creation to use this one; retail will not follow the replay's draws)")
    return None


def check_setup(header: dict, cold: dict, boundary: str) -> None:
    """The boundary must be the replay's match."""
    want = [(p["slot"], p["kind"], p["costume"], p["stocks"]) for p in header["players"]]
    have = [(f["slot"], f["kind"], f.get("costume", 0), f.get("stocks", 4)) for f in cold["fighters"]]
    # A console that named spawn points (Game Start, per player) placed its
    # players at those markers whatever their ports.
    points = [p.get("spawn_point", -1) for p in header["players"]]
    made_points = [f.get("spawn_point", -1) for f in cold["fighters"]]
    if points != made_points:
        sys.exit(f"{boundary} is not the replay's match: the replay's players spawn at points "
                 f"{points}, the boundary's at {made_points}; make the boundary with "
                 f"--spawn-points {' '.join(str(p) for p in points)}")
    replay = (header["stage"], header.get("time_limit"), want)
    made = (cold["stage"], cold.get("time_limit"), have)
    if replay != made:
        sys.exit(f"{boundary} is not the replay's match:\n  replay   {replay}\n  boundary {made}")
    spawn = cold.get("spawn", "retail")
    if header.get("spawn", spawn) != spawn:
        # The 2019 and 2020 NeutralSpawn tables share every row but Dream Land's.
        print(f"note: the replay spawned by {header['spawn']}, {boundary} by {spawn}", file=sys.stderr)


def steps(records: list[dict], ticks: int, table: dict) -> list[str]:
    """One step per change of a player's raw pad (the tick clock holds it)."""
    out, previous = [], {}
    for record in records:
        if record["tick"] >= ticks:
            continue
        slot = record["slot"]
        buttons, raw = {}, {}
        held = record["button"] & sum(pads.BUTTONS) & ~START
        for bit, key in pads.BUTTONS.items():
            if held & bit:
                buttons[key] = True
        if held:
            raw["button"] = held
        sticks = (("StickX", "stickX", record["stick"][0]), ("StickY", "stickY", record["stick"][1]),
                  ("CStickX", "substickX", record["cstick"][0]), ("CStickY", "substickY", record["cstick"][1]))
        for key, raw_key, value in sticks:
            if value:
                # The calibration covers HSD's clamp circle; a raw beyond it
                # takes the rim's entry (the tick clock injects `raw`).
                buttons[key] = table["stick"][str(max(-pads.STICK_MAX, min(pads.STICK_MAX, value)))]
                raw[raw_key] = value
        triggers = (("TriggerLeft", "triggerL", record["triggers"][0]),
                    ("TriggerRight", "triggerR", record["triggers"][1]))
        for key, raw_key, value in triggers:
            if value:
                buttons[key] = table["trigger"][str(value)]
                raw[raw_key] = value
        if "err" in record:
            # A failed poll: the queue entry as the console's stood, and no
            # button for Dolphin's own poll.
            buttons, raw = {}, {**raw, "err": record["err"]}
        if previous.get(slot) != raw:
            keys = ", ".join(f"{k} = {pads.toml_value(v)}" for k, v in buttons.items())
            raws = ", ".join(f"{k} = {v}" for k, v in raw.items())
            out.append(f"  {{ frame = {record['tick']}, port = {slot}, buttons = {{ {keys} }}, raw = {{ {raws} }} }},")
            previous[slot] = raw
    return out


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("inputs", type=Path, help="melee-sim replay --retail-inputs output")
    ap.add_argument("--name", required=True)
    ap.add_argument("--boundary", required=True, help="a make_boundary.py start scene with the replay's setup")
    ap.add_argument("--ticks", type=int, help="scheduler ticks to play (default: the whole replay)")
    ap.add_argument("--comment", default="", help="what the scenario witnesses (its header)")
    ap.add_argument("--foreign-creation", action="store_true",
                    help="accept a boundary not created from the replay's Game Start seed on a stage "
                         "that keeps its creation draws")
    ap.add_argument("--dolphin-cadence", action="store_true",
                    help="leave out render_every_tick: the recorder then runs two ticks per render, "
                         "which a console does not (a tether's rope, the magnifier and a screen KO "
                         "follow the display pass)")
    ap.add_argument("--no-display-pass-before", type=int, nargs="+", default=[], metavar="RECORD",
                    help="a console's lag frames: each record runs in one frame with the record before "
                         "it, with no display pass between (tick_trace.py, lag_frame)")
    ap.add_argument("--overwrite", action="store_true",
                    help="replace existing scenario files of this name (their recordings no longer "
                         "belong to them)")
    a = ap.parse_args(argv)
    existing = [p for p in (data_root.SCENARIOS / f"{a.name}.toml", data_root.SCENARIOS / f"{a.name}_cold.toml")
                if p.exists()]
    if existing and not a.overwrite:
        sys.exit(f"{', '.join(p.name for p in existing)} already exist: choose another --name "
                 "(or --overwrite, if the scenario is yours to replace)")

    lines = a.inputs.read_text().splitlines()
    header, records = json.loads(lines[0]), [json.loads(line) for line in lines[1:]]
    start, cold = load_boundary(a.boundary)
    check_setup(header, cold, a.boundary)
    flags = stage_flags(start, cold)
    check_stage_codes(header, flags, a.boundary)
    stage_codes = gecko.scenario_flag_lines(flags)
    if "pads_end" in header and (a.ticks or MAX_TICKS) > header["ticks"]:
        # --retail-inputs stopped at a pad retail cannot be fed.
        if a.ticks:
            sys.exit(f"the replay's pads can be fed to retail for {header['ticks']} ticks only, not "
                     f"{a.ticks}: {header['pads_end']}")
        print(f"note: the scenario ends at tick {header['ticks']}: {header['pads_end']}", file=sys.stderr)
    ticks = min(a.ticks or header["ticks"], header["ticks"], MAX_TICKS)
    created = check_creation(header, cold, a.boundary, a.foreign_creation, ticks)
    creation = f"game_start_seed = {created}\n" if created is not None else ""
    schedule = steps(records, ticks, json.loads(pads.CALIBRATION.read_text()))
    seed = header["boundary_seed"]
    codes = start.get("gecko", [])
    gecko_list = "gecko = [" + ", ".join(f'"{c}"' for c in codes) + "]\n" if codes else ""
    comment = "".join(f"# {line}\n" for line in a.comment.splitlines())
    cadence = "" if a.dolphin_cadence else "render_every_tick = true\n"
    if a.no_display_pass_before:
        if a.dolphin_cadence:
            sys.exit("--no-display-pass-before drops a display pass of the console's cadence")
        cadence += f"no_display_pass_before = {sorted(a.no_display_pass_before)}\n"

    def fix(fighter: dict) -> str:
        return f'controller_fix = "{fighter["controller_fix"]}"\n' if "controller_fix" in fighter else ""

    fighters = "\n".join(
        f'[[fighters]]\nslot = {f["slot"]}\nkind = "{f["kind"]}"\ncontroller = "scripted"\n{fix(f)}'
        for f in cold["fighters"])
    retail = data_root.SCENARIOS / f"{a.name}.toml"
    retail.write_text(f'''{comment}# Generated by slippi_to_scenario.py from a Slippi replay's inputs. Boundary
# {a.boundary}; step frames count tick records (input_clock = "tick"). The
# recorder writes boundary_seed over the saved seed; the port gates
# {a.name}_cold.
name = "{a.name}"
gate = "{a.name}_cold"
input_clock = "tick"
{cadence}savestate = "{start["savestate"]}"
frames = {ticks}
seed = 1
{creation}boundary_seed = {seed}
stage = "{cold["stage"]}"
{stage_codes}{gecko_list}inputs = [
{chr(10).join(schedule)}
]

{fighters}''')
    timer = f'time_limit = {cold["time_limit"]}\n' if "time_limit" in cold else ""
    spawn = f'spawn = "{cold["spawn"]}"\n' if "spawn" in cold else ""
    def point(fighter: dict) -> str:
        return f'spawn_point = {fighter["spawn_point"]}\n' if "spawn_point" in fighter else ""

    cold_fighters = "\n".join(
        f'[[fighters]]\nslot = {f["slot"]}\nkind = "{f["kind"]}"\ncostume = {f.get("costume", 0)}\n'
        f'stocks = {f.get("stocks", 4)}\n{point(f)}controller = "scripted"\n{fix(f)}'
        for f in cold["fighters"])
    twin = data_root.SCENARIOS / f"{a.name}_cold.toml"
    twin.write_text(f'''{comment}# The port's side of {a.name}: boundary {a.boundary} built from
# parameters, the replay's seed in place of its own, and the pads that
# recording consumed.
name = "{a.name}_cold"
expected = "{a.name}"
frames = {ticks}
seed = {cold["seed"]}
{creation}boundary_seed = {seed}
stage = "{cold["stage"]}"
{stage_codes}all_characters_unlocked = true
{timer}{spawn}inputs = []

{cold_fighters}''')
    print(f"{retail.name}: {len(schedule)} steps over {ticks} ticks; {twin.name} gates it"
          + ("; the match was created from the replay's Game Start seed" if created is not None else ""))


if __name__ == "__main__":
    main()
