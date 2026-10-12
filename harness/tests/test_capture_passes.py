"""Every capture pass of a recording is its own Dolphin run: each must play the
game the tick trace recorded."""
from __future__ import annotations

import json
import tomllib

import pytest

import record
import tick_trace as tick

BRIDGE = {"name": "slp_ps", "input_clock": "tick", "render_every_tick": True, "boundary_seed": 2553617628,
          "no_display_pass_before": [3265, 4000], "frames": 6900, "savestate": "harness/roms/x.sav",
          "stage": "PokemonStadium", "gecko": ["ucf-0.73"], "inputs": [], "fighters": []}


def test_a_rebuilt_scenario_keeps_the_clock_the_seed_and_the_display_cadence():
    assert tick.playback(BRIDGE) == {"input_clock": "tick", "boundary_seed": 2553617628,
                                     "render_every_tick": True, "no_display_pass_before": [3265, 4000]}
    # As TOML (the bone and camera passes write a scenario file).
    assert tomllib.loads(tick.playback_toml(BRIDGE)) == tick.playback(BRIDGE)
    # A plain scripted scene names none of them.
    assert tick.playback({"name": "walk", "frames": 60}) == {} and tick.playback_toml({"frames": 60}) == ""


def test_every_key_the_tick_tracer_plays_by_is_a_playback_key():
    # TickTracer reads these from its scenario besides frames, inputs and
    # fighters; `after_map` adds a sample and changes nothing played.
    source = open(tick.__file__).read()
    read = {key for key in ("input_clock", "boundary_seed", "render_every_tick", "no_display_pass_before")
            if f'scenario.get("{key}"' in source or f'scenario["{key}"]' in source}
    assert read <= set(tick.PLAYBACK_KEYS)
    assert {"render_every_tick", "no_display_pass_before", "input_clock"} <= read


def write_raw(path, seeds, start=0):
    with path.open("w") as out:
        for n, seed in enumerate(seeds, start):
            out.write(json.dumps({"frame": n, "phase": "frame_end", "seed": seed, "fighters": []}) + "\n")
    return path


def test_a_pass_with_the_tick_traces_seeds_is_the_same_game(tmp_path):
    tick_raw = write_raw(tmp_path / "t.tick.raw.jsonl", [5, 6, 7, 8, 9])
    ledger = write_raw(tmp_path / "t.ledger.raw.jsonl", [5, 6, 7, 8, 9])
    # A capped dump holds the first records only.
    particles = write_raw(tmp_path / "t.particles.jsonl.meta.jsonl", [5, 6, 7])
    record.require_same_game("t", tick_raw, [("RNG ledger", ledger, 5), ("particle dump", particles, 3)])


def test_a_pass_that_plays_another_game_fails_the_recording(tmp_path):
    tick_raw = write_raw(tmp_path / "t.tick.raw.jsonl", [5, 6, 7, 8, 9])
    # The dump ran at another display cadence: its seed leaves at record 2.
    particles = write_raw(tmp_path / "t.particles.jsonl.meta.jsonl", [5, 6, 70, 80, 90])
    with pytest.raises(SystemExit, match="particle dump pass is not the tick trace's game.*record 2"):
        record.require_same_game("t", tick_raw, [("particle dump", particles, 5)])


def test_a_pass_whose_match_ended_early_fails_the_recording(tmp_path):
    tick_raw = write_raw(tmp_path / "t.tick.raw.jsonl", [5, 6, 7, 8, 9])
    particles = write_raw(tmp_path / "t.particles.jsonl.meta.jsonl", [5, 6, 7])
    with pytest.raises(SystemExit, match="particle dump pass wrote 3 records of 5"):
        record.require_same_game("t", tick_raw, [("particle dump", particles, 5)])
