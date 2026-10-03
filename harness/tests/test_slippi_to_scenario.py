"""slippi_to_scenario.py: a replay's raw pads become tick-clock steps, and the
boundary must be the replay's match."""
from __future__ import annotations

import json

import pytest

import replay_to_scenario as pads
import slippi_to_scenario as s2s

TABLE = json.loads(pads.CALIBRATION.read_text())


def record(tick, slot, button=0, stick=(0, 0), cstick=(0, 0), triggers=(0, 0)):
    return {"tick": tick, "slot": slot, "button": button, "stick": list(stick), "cstick": list(cstick),
            "triggers": list(triggers)}


def test_steps_hold_a_pad_until_it_changes_and_stop_at_the_tick_limit():
    records = [record(1, 1), record(1, 3, button=0x100),
               record(2, 1, stick=(94, -17)), record(2, 3, button=0x100),
               record(3, 1, stick=(94, -17), triggers=(0, 140)), record(3, 3),
               record(4, 1), record(4, 3)]
    steps = s2s.steps(records, 4, TABLE)
    assert [line.split(", buttons")[0].strip() for line in steps] == [
        "{ frame = 1, port = 1", "{ frame = 1, port = 3", "{ frame = 2, port = 1", "{ frame = 3, port = 1",
        "{ frame = 3, port = 3"]
    # A raw beyond HSD's clamp circle is injected as recorded (UCF reads it).
    assert "raw = { stickX = 94, stickY = -17 }" in steps[2]
    assert "raw = { stickX = 94, stickY = -17, triggerR = 140 }" in steps[3]
    assert "raw = { button = 256 }" in steps[1] and "A = true" in steps[1]
    assert steps[4].endswith("buttons = {  }, raw = {  } },")


def test_the_boundary_must_be_the_replays_match():
    header = {"stage": "Battlefield", "time_limit": 480,
              "players": [{"slot": 1, "kind": "Marth", "costume": 1, "stocks": 4},
                          {"slot": 3, "kind": "Peach", "costume": 1, "stocks": 4}]}
    cold = {"stage": "Battlefield", "time_limit": 480,
            "fighters": [{"slot": 1, "kind": "Marth", "costume": 1, "stocks": 4},
                         {"slot": 3, "kind": "Peach", "costume": 1, "stocks": 4}]}
    s2s.check_setup(header, cold, "b")
    for change in ({"time_limit": 0}, {"stage": "FinalDestination"}):
        with pytest.raises(SystemExit):
            s2s.check_setup(header, {**cold, **change}, "b")
    other_ports = [{**cold["fighters"][0], "slot": 0}, {**cold["fighters"][1], "slot": 1}]
    with pytest.raises(SystemExit):
        s2s.check_setup(header, {**cold, "fighters": other_ports}, "b")
