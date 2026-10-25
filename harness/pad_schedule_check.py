"""Did a tick-clock recording consume the pads its scenario scheduled?

    cd harness && uv run python pad_schedule_check.py <scenario name> [...]

A scenario on the tick input clock (`input_clock = "tick"`) gives each step
the raw PADStatus the tick must consume (`raw`), and the tracer writes it
into the game's pad queue (dolphin/tick_trace.py). Each tick record carries
the queue entry the tick dequeued (`pad_queue_sticks`: the four raw stick
bytes) and HSD's reading of it (`inputs`). This compares both with the
schedule, tick for tick, and prints the first record that consumed another
pad: the queue entry's stick bytes, and the status's buttons and triggers.

A bridged replay rests on it: `melee-sim replay --retail-inputs` writes raw
pads that read as the replay's Pre Frame, so a recording that consumed them
was fed the console's pads. record.py runs the check after every tick-clock
recording and fails the recording when it does not hold.
"""
from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

import data_root
import trace_io

STICK_KEYS = ("stickX", "stickY", "substickX", "substickY")
#: PAD_ERR_TRANSFER: HSD keeps the port's last status (controller.c:386).
PAD_ERR_TRANSFER = -3
#: HSD's analog trigger ceiling (gmmain.c: HSD_PadScale's 140).
TRIGGER_MAX = 140
#: The pad's own button bits; the rest of the status word is HSD's.
PHYSICAL = 0xFFFF


def held_pads(steps: list[dict], ticks: int) -> list[dict[int, dict]]:
    """Per tick record, each scheduled port's raw pad (a step holds until the
    port's next one)."""
    by_tick: dict[int, list[dict]] = {}
    for step in steps:
        by_tick.setdefault(step["frame"], []).append(step)
    held: dict[int, dict] = {}
    out = []
    for tick in range(ticks):
        for step in by_tick.get(tick, []):
            held[step["port"]] = step.get("raw", {})
        out.append(dict(held))
    return out


def differences(raw: dict, queue: list[int], status: dict | None, before: dict | None) -> list[str]:
    """What one port's dequeued entry and HSD status hold that the scheduled
    raw pad does not give. `before` is the status of the record before, which
    a failed poll keeps."""
    found = []
    want = [raw.get(key, 0) for key in STICK_KEYS]
    if want != list(queue):
        found.append(f"queue sticks {list(queue)} for the scheduled {want}")
    if status is None:
        return found
    if raw.get("err") == PAD_ERR_TRANSFER:
        if before is not None and any(status[key] != before[key] for key in ("button", "analogL", "analogR")):
            found.append("a failed poll did not keep the last status")
        return found
    button = status["button"]["v"] & PHYSICAL
    if button != raw.get("button", 0):
        found.append(f"buttons {button:#x} for the scheduled {raw.get('button', 0):#x}")
    triggers = [status[key]["v"] for key in ("analogL", "analogR")]
    scheduled = [min(raw.get(key, 0), TRIGGER_MAX) for key in ("triggerL", "triggerR")]
    if triggers != scheduled:
        found.append(f"triggers {triggers} for the scheduled {scheduled}")
    return found


def mismatches(scenario: dict, trace: Path):
    """Each record of `trace` (a decoded tick trace) that did not consume its
    scheduled pad, as text. Nothing for a scenario that schedules no raw pads."""
    steps = scenario.get("inputs", [])
    if scenario.get("input_clock") != "tick" or not any("raw" in step for step in steps):
        return
    schedule = held_pads(steps, scenario["frames"] + 1)
    before = None
    previous_state = None
    with trace_io.open_text(trace) as lines:
        for line in lines:
            record = json.loads(line)
            if record.get("phase") != "frame_end":
                continue
            n = record["frame"]
            queues, statuses = record.get("pad_queue_sticks"), record.get("inputs")
            # A slowed game's pass whose game timer is not due dequeues the
            # pad (HSD_PadRenewMasterStatus) but neither renews the game's
            # status (lb_80019900) nor changes the match: only the queue holds.
            state = record.get("state")
            paused = bool(state) and state == previous_state
            previous_state = state
            # Record 0 is the boundary: the savestate's pad, not a scheduled one.
            if 0 < n < len(schedule) and queues is not None:
                for port, raw in sorted(schedule[n].items()):
                    status = None if paused else statuses and statuses[f"p{port}"]
                    found = differences(raw, queues[port], status, before and before[f"p{port}"])
                    if found:
                        yield f"record {n}, port {port + 1}: " + "; ".join(found)
            if not paused:
                before = statuses


def check(scenario: dict, trace: Path) -> str | None:
    """The first record that did not consume its scheduled pad; None when
    every record did."""
    return next(mismatches(scenario, trace), None)


def main(argv: list[str]) -> int:
    failed = 0
    for name in argv:
        scenario = tomllib.loads((data_root.SCENARIOS / f"{name}.toml").read_text())
        found = check(scenario, data_root.TRACES / f"{name}.tick.expected.jsonl")
        print(f"{name}: " + (found or "every record consumed its scheduled pad"))
        failed |= found is not None
    return failed


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
