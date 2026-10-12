"""pad_schedule_check.py: a tick-clock recording must have consumed, tick for
tick, the raw pads its scenario scheduled."""
from __future__ import annotations

import json

import pad_schedule_check as psc


def status(button=0, triggers=(0, 0)) -> dict:
    return {"button": {"t": "u", "v": button}, "analogL": {"t": "u", "v": triggers[0]},
            "analogR": {"t": "u", "v": triggers[1]}}


SCENARIO = {"frames": 4, "input_clock": "tick", "inputs": [
    {"frame": 1, "port": 1, "buttons": {}, "raw": {"button": 0x100, "stickX": 61, "stickY": 50, "triggerR": 200}},
    {"frame": 2, "port": 1, "buttons": {}, "raw": {"err": -3}},
    {"frame": 3, "port": 1, "buttons": {}, "raw": {"stickX": 106, "substickY": 27}},
    {"frame": 3, "port": 3, "buttons": {}, "raw": {"button": 0x200}}]}
#: Per record: port 2's queue sticks and status, port 4's status. The A
#: press carries HSD's right bit; a trigger past 140 reads 140; the failed
#: poll keeps tick 1's status over a zeroed entry.
CONSUMED = [
    ([0, 0, 0, 0], status(), status()),
    ([61, 50, 0, 0], status(0x80100, (0, 140)), status()),
    ([0, 0, 0, 0], status(0x80100, (0, 140)), status()),
    ([106, 0, 0, 27], status(0x80000), status(0x200)),
    ([106, 0, 0, 27], status(0x80000), status(0x200)),
]


def write_trace(path, consumed):
    with path.open("w") as out:
        for frame, (queue, port2, port4) in enumerate(consumed):
            out.write(json.dumps({
                "frame": frame, "phase": "frame_end", "state": {},
                "pad_queue_sticks": [[0, 0, 0, 0], queue, [0, 0, 0, 0], [0, 0, 0, 0]],
                "inputs": {"p0": status(), "p1": port2, "p2": status(), "p3": port4}}) + "\n")
    return path


def test_a_recording_that_consumed_its_schedule_passes(tmp_path):
    assert psc.check(SCENARIO, write_trace(tmp_path / "t.jsonl", CONSUMED)) is None


def test_the_first_record_that_consumed_another_pad_is_named(tmp_path):
    # Record 3 dequeued Dolphin's own poll of the tick before's pad (the
    # one-entry queue's stale qwrite): raw X 82 for the scheduled 106.
    consumed = list(CONSUMED)
    consumed[3] = ([82, 0, 0, 27], *CONSUMED[3][1:])
    found = psc.check(SCENARIO, write_trace(tmp_path / "t.jsonl", consumed))
    assert found == "record 3, port 2: queue sticks [82, 0, 0, 27] for the scheduled [106, 0, 0, 27]"
    # A button or a trigger the status does not hold.
    consumed = list(CONSUMED)
    consumed[1] = (CONSUMED[1][0], status(0x80000, (0, 139)), status())
    found = psc.check(SCENARIO, write_trace(tmp_path / "t.jsonl", consumed))
    assert found == ("record 1, port 2: buttons 0x0 for the scheduled 0x100; "
                     "triggers [0, 139] for the scheduled [0, 140]")
    # A failed poll that did not keep the status.
    consumed = list(CONSUMED)
    consumed[2] = (CONSUMED[2][0], status(), status())
    found = psc.check(SCENARIO, write_trace(tmp_path / "t.jsonl", consumed))
    assert found == "record 2, port 2: a failed poll did not keep the last status"


def test_scenarios_without_raw_pads_are_not_checked(tmp_path):
    trace = write_trace(tmp_path / "t.jsonl", [([9, 9, 9, 9], status(), status())] * 5)
    vi_clock = {**SCENARIO, "input_clock": "vi"}
    no_raw = {"frames": 4, "input_clock": "tick", "inputs": [{"frame": 1, "port": 1, "buttons": {"A": True}}]}
    assert psc.check(vi_clock, trace) is None and psc.check(no_raw, trace) is None
    # A recording that ended early is checked as far as it goes.
    assert psc.check(SCENARIO, write_trace(tmp_path / "short.jsonl", CONSUMED[:2])) is None
