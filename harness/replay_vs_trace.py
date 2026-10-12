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
- Post Frame's action, position and percent equal the record's, bit for bit.

Slippi before 3.4.0 samples Post Frame at the end of the fighter's map proc,
not of the tick: a scenario recorded with `after_map = true` carries that
sample and it is compared; without it the fields of such a replay differ on
any tick a later proc moves the fighter, and only the seed check decides.

It prints the first frame each check fails on and the count of failing
frames. A bridge that fails is retail, but not that console's game from the
first failing frame on: nothing after it says anything about the replay.
"""
from __future__ import annotations

import argparse
import json
import sys

import data_root
import trace_io

MULTIPLIER, INCREMENT, MASK = 214013, 2531011, 0xFFFFFFFF
#: The first Slippi version whose Post Frame is taken at the end of the tick.
POST_FRAME_AT_TICK_END = (3, 4, 0)
#: The most draws searched for in one tick.
MAX_TICK_DRAWS = 1 << 16


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


def compare(frames_path: str, scenario: str, limit: int | None) -> int:
    trace = data_root.TRACES / f"{scenario}.tick.expected.jsonl"
    records = {}
    sampled = False
    with trace_io.open_text(trace) as lines:
        for line in lines:
            record = json.loads(line)
            if record.get("phase") == "frame_end":
                sampled |= "after_map" in record
                records[record["frame"]] = record["state"] | record.get("after_map", {})
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
            total = steps(start, end)
            by_kind = fighters_of(state)
            failed = set()
            for fighter in frame["fighters"]:
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
    if "seed" in first or (fields_decide and first):
        return 1
    print("  retail follows the replay" + ("" if first else
          ": seeds, actions, positions and percents on every frame"))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("frames", help="the replay's frames (slp example `frames`)")
    ap.add_argument("scenario", help="the bridged retail scenario's name")
    ap.add_argument("--frames-limit", type=int, metavar="N",
                    help="compare replay frames below N only")
    a = ap.parse_args()
    return compare(a.frames, a.scenario, a.frames_limit)


if __name__ == "__main__":
    sys.exit(main())
