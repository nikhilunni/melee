"""Does a retail recording play the console's game? Compare a bridged
scenario's retail tick trace with the Slippi replay it was bridged from.

    cargo run -q --release -p slp --example frames -- <game.slp> > frames.jsonl
    cd harness && uv run python replay_vs_trace.py frames.jsonl <scenario name>

Replay frame n (0 is Slippi's -123) is trace record n + 1. Per frame and
recorded fighter this checks what a replay holds of retail's state:

- Pre Frame's seed lies on retail's random stream between the end of the
  previous record and the end of this one (the port's own replay check,
  `compare_input_seeds`): the console and retail in Dolphin have drawn the
  same number of values;
- Post Frame's action, position and percent equal the record's, bit for bit;
- Pre Frame's inputs are the pad retail consumed on that tick: the physical
  buttons, HSD's stick-direction bits, the dead-zoned sticks, the analog
  triggers (the record's `inputs`, HSD_PadGameStatus) and the raw stick bytes
  the replay's version records (`pad_queue_sticks`, the queue entry UCF
  reads).

The inputs decide whether the bridge did its work: a difference from the
countdown's GO on (frame 84) means retail was fed another pad than the
console's, and nothing the recording shows afterwards is about the replay.
Before GO no fighter reads its pad and Pre Frame's stick need not be a pad's;
differences there are counted and do not fail. A recorded Start press is not
fed to retail (slippi_to_scenario.py) and is not compared.

Slippi before 3.4.0 samples Post Frame at the end of the fighter's map proc,
not of the tick: a scenario recorded with `after_map = true` carries that
sample and it is compared; without it the fields of such a replay differ on
any tick a later proc moves the fighter, and only the seed check decides.

A Slippi Online recording (`--online-rng-offset`, the scenario's
`online_rng_offset`): each tick starts from the netplay code's seed, not from
the record before, so the stream searched for a Pre Frame seed starts there.

It prints the first frame each check fails on and the count of failing
frames, and for the inputs the first differing pad with both sides' values.
A bridge that fails is retail, but not that console's game from the first
failing frame on: nothing after it says anything about the replay.
"""
from __future__ import annotations

import argparse
import json
import struct
import sys

import data_root
import trace_io
import validate_ticks

MULTIPLIER, INCREMENT, MASK = 214013, 2531011, 0xFFFFFFFF
#: The first Slippi version whose Post Frame is taken at the end of the tick.
POST_FRAME_AT_TICK_END = (3, 4, 0)
#: The most draws searched for in one tick.
MAX_TICK_DRAWS = 1 << 16
#: The first replay frame whose pad the fighters read: the countdown's GO
#: (melee-sim replay_retail_pads.rs, INPUT_RELEASE_FRAME).
INPUT_RELEASE_FRAME = 84
#: HSD_PadADConvert's stick-direction bits of the button word.
DIRECTIONS = 0x00FF0000
#: The physical buttons, without Start (not fed to retail).
PHYSICAL = 0xFFFF & ~(1 << 12)


def f32(bits: int) -> float:
    return struct.unpack("<f", struct.pack("<I", bits))[0]


#: PlCo's stick dead zone as the f32 the fighter compares with.
STICK_DEAD_ZONE = f32(0x3E8CCCCD)


def dead_zoned(status: dict, x: str, y: str) -> list[float]:
    """The fighter's reading of an HSD stick: zero at or inside the dead zone."""
    return [0.0 if abs(v) <= STICK_DEAD_ZONE else v
            for v in (f32(status[x]["v"]["bits"]), f32(status[y]["v"]["bits"]))]


def pad_differences(pad: dict, status: dict, queue: list[int] | None) -> dict[str, tuple]:
    """Pre Frame's pad fields that retail's HSD status and queue entry do not
    give: field -> (the replay's value, retail's)."""
    button = status["button"]["v"]
    pairs = {
        "buttons": (pad["physical"] & PHYSICAL, button & PHYSICAL),
        "directions": (pad["held"] & DIRECTIONS, button & DIRECTIONS),
        "stick": ([f32(v) for v in pad["stick"]], dead_zoned(status, "nml_stickX", "nml_stickY")),
        "cstick": ([f32(v) for v in pad["cstick"]], dead_zoned(status, "nml_subStickX", "nml_subStickY")),
        "triggers": ([f32(v) for v in pad["triggers"]],
                     [f32(status[key]["v"]["bits"]) for key in ("nml_analogL", "nml_analogR")]),
    }
    if queue is not None:
        recorded = [*pad["raw_stick"], *pad["raw_cstick"]]
        pairs["raw"] = (recorded, [have if want is not None else None
                                   for want, have in zip(recorded, queue)])
    return {field: pair for field, pair in pairs.items() if pair[0] != pair[1]}


def show(field: str, value) -> str:
    if field in ("buttons", "directions"):
        return f"{value:#x}"
    return "(" + ", ".join("-" if v is None else f"{v:g}" for v in value) + ")"


def steps(start: int, goal: int, limit: int = MAX_TICK_DRAWS) -> int | None:
    """Draws from seed `start` to seed `goal`, None beyond `limit`."""
    seed = start
    for n in range(limit + 1):
        if seed == goal:
            return n
        seed = (seed * MULTIPLIER + INCREMENT) & MASK
    return None


def fighters_of(state: dict) -> dict[tuple[int, int], str]:
    """(player slot, internal kind) -> the record's `pN` prefix."""
    found = {}
    for key, value in state.items():
        if key.endswith(".player_id"):
            prefix = key[: -len(".player_id")]
            found[(value["v"], state[f"{prefix}.kind"]["v"])] = prefix
    return found


def compare(frames_path: str, scenario: str, limit: int | None,
            online_rng_offset: int | None = None) -> int:
    trace = data_root.TRACES / f"{scenario}.tick.expected.jsonl"
    records, pads = {}, {}
    sampled = False
    # The inputs' first difference after GO, as text.
    first_pad: str | None = None
    frozen_pads, with_pads = 0, True
    with trace_io.open_text(trace) as lines:
        for line in lines:
            record = json.loads(line)
            if record.get("phase") == "frame_end":
                sampled |= "after_map" in record
                records[record["frame"]] = record["state"] | record.get("after_map", {})
                pads[record["frame"]] = (record.get("inputs"), record.get("pad_queue_sticks"))
    first: dict[str, int] = {}
    count: dict[str, int] = {}
    compared = 0
    with open(frames_path) as lines:
        header = json.loads(next(lines))
        end_of_tick = tuple(header["version"]) >= POST_FRAME_AT_TICK_END
        for line in lines:
            frame = json.loads(line)
            n = frame["frame"]
            if limit is not None and n >= limit:
                break
            before, state = records.get(n), records.get(n + 1)
            if before is None or state is None:
                break
            compared += 1
            start, end = before["rng.seed"]["v"], state["rng.seed"]["v"]
            if online_rng_offset is not None:
                start = validate_ticks.online_seed(n + 1, online_rng_offset)
            total = steps(start, end)
            by_kind = fighters_of(state)
            failed = set()
            statuses, queues = pads[n + 1]
            for fighter in frame["fighters"]:
                with_pads &= "pad" in fighter
                if "pad" in fighter and statuses is not None and not fighter["follower"]:
                    differing = pad_differences(fighter["pad"], statuses[f"p{fighter['slot']}"],
                                                queues[fighter["slot"]] if queues else None)
                    if differing and n < INPUT_RELEASE_FRAME:
                        frozen_pads += 1
                    elif differing:
                        failed.add("inputs")
                        first_pad = first_pad or "; ".join(
                            f"port {fighter['slot'] + 1} {field} {show(field, want)}, retail {show(field, have)}"
                            for field, (want, have) in differing.items())
                drawn = steps(start, fighter["seed"])
                if total is None or drawn is None or drawn > total:
                    failed.add("seed")
                prefix = by_kind.get((fighter["slot"], fighter["kind"]))
                if prefix is None:
                    failed.add("fighter")
                    continue
                if state[f"{prefix}.motion_id"]["v"] != fighter["action"]:
                    failed.add("action")
                position = (state[f"{prefix}.cur_pos.x"]["v"]["bits"],
                            state[f"{prefix}.cur_pos.y"]["v"]["bits"])
                if position != (fighter["x"], fighter["y"]):
                    failed.add("position")
                if state[f"{prefix}.percent"]["v"]["bits"] != fighter["percent"]:
                    failed.add("percent")
            for check in failed:
                first.setdefault(check, n)
                count[check] = count.get(check, 0) + 1
    print(f"{scenario}: {compared} replay frames compared with retail records 1..{compared}")
    fields_decide = end_of_tick or sampled
    if not fields_decide:
        print("  Post Frame is the map proc's sample and the trace has none (after_map): "
              "field differences are expected on some ticks; the seed check decides")
    for check in sorted(first, key=first.get):
        print(f"  {check}: first differs at replay frame {first[check]} "
              f"(retail record {first[check] + 1}); {count[check]} frames differ")
    if not with_pads:
        print("  inputs not compared: the frames file has no Pre Frame pads (an older `frames` example)")
    if frozen_pads:
        print(f"  {frozen_pads} pads differ before GO (frame {INPUT_RELEASE_FRAME}), where no fighter reads one")
    if first_pad:
        print(f"  retail was not fed the replay's pads: at frame {first['inputs']} the replay has {first_pad}")
        return 1
    if "seed" in first or (fields_decide and first):
        return 1
    print("  retail follows the replay" + ("" if first else
          ": seeds, actions, positions and percents on every frame")
          + (", and was fed its pads on every frame" if with_pads else ""))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("frames", help="the replay's frames (slp example `frames`)")
    ap.add_argument("scenario", help="the bridged retail scenario's name")
    ap.add_argument("--frames-limit", type=int, metavar="N",
                    help="compare replay frames below N only")
    ap.add_argument("--online-rng-offset", type=int, metavar="OFFSET",
                    help="the scenario's online_rng_offset (a Slippi Online game)")
    a = ap.parse_args()
    return compare(a.frames, a.scenario, a.frames_limit, a.online_rng_offset)


if __name__ == "__main__":
    sys.exit(main())
