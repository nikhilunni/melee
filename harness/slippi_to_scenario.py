"""Turn a Slippi replay's inputs into a retail scenario, and its cold twin.

    cargo run -q --release -p melee-sim -- replay <game.slp> --retail-inputs inputs.jsonl
    cd harness && uv run python slippi_to_scenario.py inputs.jsonl --name <scenario> \
        --boundary <start scene> [--ticks N]

A replay records no retail memory, so a divergence the port shows against one
cannot be triaged. This feeds the replay's pads to retail instead: the same
match from a start boundary with the replay's layout (stage, characters,
costumes, ports, timer, the Slippi build's Gecko codes), so the ordinary gate
and triage compare the port with a full retail trace of the same game.

`inputs.jsonl` is `melee-sim replay --retail-inputs`: a header (setup and the
pre-music boundary seed) and one raw pad per player and tick. The boundary is
a `make_boundary.py` start scene made for that setup, for example

    make_boundary.py --stage Battlefield --players Marth Peach --costumes 1 1 --ports 2 4 \
        --time-limit 8 --gecko ucf-0.8 neutral-spawn --spawn neutral-2020 --no-register

Two scenarios are written:

- `<name>.toml`, which record_many.py records: the boundary's savestate, the
  pads on the tick input clock, and `boundary_seed`, which the tracers write
  over the saved seed before the first tick. Retail then draws what the
  replay's game drew, so it follows the replay through random outcomes too.
- `<name>_cold.toml`, which the port gates: the boundary built from parameters
  (its own `seed`), then `boundary_seed`, driven by the pads the recording
  consumed.

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


def check_setup(header: dict, cold: dict, boundary: str) -> None:
    """The boundary must be the replay's match."""
    want = [(p["slot"], p["kind"], p["costume"], p["stocks"]) for p in header["players"]]
    have = [(f["slot"], f["kind"], f.get("costume", 0), f.get("stocks", 4)) for f in cold["fighters"]]
    replay = (header["stage"], header.get("time_limit"), want)
    made = (cold["stage"], cold.get("time_limit"), have)
    if replay != made:
        sys.exit(f"{boundary} is not the replay's match:\n  replay   {replay}\n  boundary {made}")


def steps(records: list[dict], ticks: int, table: dict) -> list[str]:
    """One step per change of a player's raw pad (the tick clock holds it)."""
    out, previous = [], {}
    for record in records:
        if record["tick"] >= ticks:
            continue
        slot = record["slot"]
        buttons, raw = {}, {}
        held = record["button"] & sum(pads.BUTTONS)
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
    a = ap.parse_args(argv)

    lines = a.inputs.read_text().splitlines()
    header, records = json.loads(lines[0]), [json.loads(line) for line in lines[1:]]
    start, cold = load_boundary(a.boundary)
    check_setup(header, cold, a.boundary)
    flags = stage_flags(start, cold)
    check_stage_codes(header, flags, a.boundary)
    stage_codes = gecko.scenario_flag_lines(flags)
    ticks = min(a.ticks or header["ticks"], header["ticks"], MAX_TICKS)
    schedule = steps(records, ticks, json.loads(pads.CALIBRATION.read_text()))
    seed = header["boundary_seed"]
    codes = start.get("gecko", [])
    gecko_list = "gecko = [" + ", ".join(f'"{c}"' for c in codes) + "]\n" if codes else ""
    comment = "".join(f"# {line}\n" for line in a.comment.splitlines())

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
savestate = "{start["savestate"]}"
frames = {ticks}
seed = 1
boundary_seed = {seed}
stage = "{cold["stage"]}"
{stage_codes}{gecko_list}inputs = [
{chr(10).join(schedule)}
]

{fighters}''')
    timer = f'time_limit = {cold["time_limit"]}\n' if "time_limit" in cold else ""
    spawn = f'spawn = "{cold["spawn"]}"\n' if "spawn" in cold else ""
    cold_fighters = "\n".join(
        f'[[fighters]]\nslot = {f["slot"]}\nkind = "{f["kind"]}"\ncostume = {f.get("costume", 0)}\n'
        f'stocks = {f.get("stocks", 4)}\ncontroller = "scripted"\n{fix(f)}' for f in cold["fighters"])
    twin = data_root.SCENARIOS / f"{a.name}_cold.toml"
    twin.write_text(f'''{comment}# The port's side of {a.name}: boundary {a.boundary} built from
# parameters, the replay's seed in place of its own, and the pads that
# recording consumed.
name = "{a.name}_cold"
expected = "{a.name}"
frames = {ticks}
seed = {cold["seed"]}
boundary_seed = {seed}
stage = "{cold["stage"]}"
{stage_codes}all_characters_unlocked = true
{timer}{spawn}inputs = []

{cold_fighters}''')
    print(f"{retail.name}: {len(schedule)} steps over {ticks} ticks; {twin.name} gates it")


if __name__ == "__main__":
    main()
