"""replay_vs_trace.py: a retail trace follows a replay when every Pre Frame
seed lies on retail's stream within its tick and the fighters' fields agree."""
from __future__ import annotations

import json

import pytest

import replay_vs_trace as rvt


def draw(seed: int, n: int = 1) -> int:
    for _ in range(n):
        seed = (seed * rvt.MULTIPLIER + rvt.INCREMENT) & rvt.MASK
    return seed


def test_steps_counts_draws_and_gives_up_past_the_limit():
    assert rvt.steps(7, 7) == 0
    assert rvt.steps(7, draw(7, 5)) == 5
    assert rvt.steps(7, draw(7, 5), limit=4) is None


def fighter(prefix: str, slot: int, action: int, x: int) -> dict:
    return {f"{prefix}.player_id": {"t": "u", "v": slot}, f"{prefix}.kind": {"t": "i", "v": 9},
            f"{prefix}.motion_id": {"t": "i", "v": action},
            f"{prefix}.cur_pos.x": {"t": "f32", "v": {"bits": x, "approx": 0.0}},
            f"{prefix}.cur_pos.y": {"t": "f32", "v": {"bits": 0, "approx": 0.0}},
            f"{prefix}.percent": {"t": "f32", "v": {"bits": 0, "approx": 0.0}}}


def write_game(tmp_path, version, console_draws, retail_draws, retail_x=(10, 11, 12), after_map=None):
    """Three replay frames against four retail records. A tick's Pre Frame
    seed is taken one draw into it; `console_draws` and `retail_draws` give
    each side's draws per tick."""
    retail, console = [100], [100]
    for n in retail_draws:
        retail.append(draw(retail[-1], n))
    for n in console_draws:
        console.append(draw(console[-1], n))
    trace = tmp_path / "game.tick.expected.jsonl"
    with trace.open("w") as out:
        for record, seed in enumerate(retail):
            state = {"rng.seed": {"t": "u", "v": seed}, **fighter("p0", 2, 14, retail_x[max(record - 1, 0)])}
            line = {"frame": record, "phase": "frame_end", "state": state}
            if after_map is not None and record:
                line["after_map"] = fighter("p0", 2, 14, after_map[record - 1])
            out.write(json.dumps(line) + "\n")
    frames = tmp_path / "frames.jsonl"
    with frames.open("w") as out:
        out.write(json.dumps({"version": version, "game_start_seed": 1}) + "\n")
        for n in range(3):
            player = {"slot": 2, "follower": False, "kind": 9, "seed": draw(console[n]), "action": 14,
                      "x": 10 + n, "y": 0, "percent": 0}
            out.write(json.dumps({"frame": n, "fighters": [player]}) + "\n")
    return str(frames)


@pytest.fixture
def traces(tmp_path, monkeypatch):
    monkeypatch.setattr(rvt.data_root, "TRACES", tmp_path)
    return tmp_path


def test_a_trace_with_the_consoles_draws_and_fields_follows_the_replay(traces, capsys):
    frames = write_game(traces, [3, 7, 0], [4, 2, 9], [4, 2, 9])
    assert rvt.compare(frames, "game", None) == 0
    assert "seeds, actions, positions and percents on every frame" in capsys.readouterr().out


def test_a_tick_with_another_draw_count_leaves_the_replay_from_the_next_frame(traces, capsys):
    # Retail draws two more values on the second tick: the third frame's Pre
    # Frame seed is behind retail's stream.
    frames = write_game(traces, [3, 7, 0], [4, 2, 9], [4, 4, 9])
    assert rvt.compare(frames, "game", None) == 1
    assert "seed: first differs at replay frame 2 (retail record 3); 1 frames differ" in capsys.readouterr().out
    # Below the frame it differs on, the bridge is the replay's game.
    assert rvt.compare(frames, "game", 2) == 0


def test_a_map_proc_sample_is_compared_only_with_a_trace_that_carries_it(traces, capsys):
    # Slippi 2.0.1 samples Post Frame after the map proc: retail's end of
    # tick has moved the fighter on the second tick.
    moved = (10, 99, 12)
    frames = write_game(traces, [2, 0, 1], [4, 2, 9], [4, 2, 9], retail_x=moved)
    assert rvt.compare(frames, "game", None) == 0
    assert "the seed check decides" in capsys.readouterr().out
    # The same end-of-tick difference decides for a replay sampled there.
    frames = write_game(traces, [3, 7, 0], [4, 2, 9], [4, 2, 9], retail_x=moved)
    assert rvt.compare(frames, "game", None) == 1
    # With the map proc's sample recorded, that is what the replay equals.
    frames = write_game(traces, [2, 0, 1], [4, 2, 9], [4, 2, 9], retail_x=moved, after_map=(10, 11, 12))
    assert rvt.compare(frames, "game", None) == 0
    assert "on every frame" in capsys.readouterr().out
