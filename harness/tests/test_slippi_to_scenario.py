"""slippi_to_scenario.py: a replay's raw pads become tick-clock steps, and the
boundary must be the replay's match."""
from __future__ import annotations

import json
import tomllib

import pytest

import make_boundary as mb
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


def write_boundary(scenarios, name, stage, codes, seed=7):
    """A make_boundary.py start scene and cold twin, as that command writes them."""
    players, slots = ["Falco", "Marth"], [0, 3]
    (scenarios / f"{name}.toml").write_text(mb.start_scenario(name, stage, players, 4, codes, slots))
    (scenarios / f"{name}_cold.toml").write_text(
        mb.cold_scenario(name, stage, players, 4, seed, [3, 4], "retail", codes, slots, 8))


def write_inputs(path, stage, **header):
    players = [{"slot": 0, "kind": "Falco", "costume": 3, "stocks": 4, "controller_fix": "ucf-0.8"},
               {"slot": 3, "kind": "Marth", "costume": 4, "stocks": 4, "controller_fix": "ucf-0.8"}]
    lines = [{"boundary_seed": 99, "all_characters_unlocked": True, "time_limit": 480, "stage": stage,
              "ticks": 3, "players": players, **header},
             record(1, 0, button=0x100), record(1, 3), record(2, 0), record(2, 3)]
    path.write_text("\n".join(json.dumps(line) for line in lines) + "\n")


@pytest.fixture
def scenarios(tmp_path, monkeypatch):
    monkeypatch.setattr(s2s.data_root, "SCENARIOS", tmp_path)
    return tmp_path


@pytest.mark.parametrize("codes,flags", [
    (["ucf-0.8", "ps-preload"], {"stadium_preload": True}),
    (["ucf-0.8", "ps-preload", "ps-frozen"], {"stadium_preload": True, "stadium_frozen": True}),
    (["ucf-0.8"], {}),
])
def test_stadium_codes_reach_the_scenario_and_its_cold_twin(scenarios, codes, flags):
    write_boundary(scenarios, "start_ps", "PokemonStadium", codes)
    header = {"stadium_preload": False, "stadium_frozen": False, **flags}
    write_inputs(scenarios / "inputs.jsonl", "PokemonStadium", **header)
    s2s.main([str(scenarios / "inputs.jsonl"), "--name", "slp_ps", "--boundary", "start_ps"])
    retail = tomllib.loads((scenarios / "slp_ps.toml").read_text())
    twin = tomllib.loads((scenarios / "slp_ps_cold.toml").read_text())
    for scenario in (retail, twin):
        assert {key: scenario[key] for key in scenario if key.startswith("stadium_")} == flags
    assert retail["gecko"] == codes and "gecko" not in twin
    assert (retail["boundary_seed"], twin["boundary_seed"], twin["seed"]) == (99, 99, 7)


def test_a_boundary_without_the_replays_stadium_codes_is_refused(scenarios):
    write_boundary(scenarios, "start_ps", "PokemonStadium", ["ucf-0.8"])
    write_inputs(scenarios / "inputs.jsonl", "PokemonStadium", stadium_preload=True, stadium_frozen=False)
    with pytest.raises(SystemExit, match="--gecko ps-preload"):
        s2s.main([str(scenarios / "inputs.jsonl"), "--name", "slp_ps", "--boundary", "start_ps"])
    assert not (scenarios / "slp_ps.toml").exists()
    # A boundary with a code the console did not run is not its match either.
    write_boundary(scenarios, "start_frozen", "PokemonStadium", ["ucf-0.8", "ps-preload", "ps-frozen"])
    with pytest.raises(SystemExit, match="without --gecko ps-frozen"):
        s2s.main([str(scenarios / "inputs.jsonl"), "--name", "slp_ps", "--boundary", "start_frozen"])
