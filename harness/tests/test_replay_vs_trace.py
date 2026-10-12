"""replay_vs_trace.py: a retail trace follows a replay when every Pre Frame
seed lies on retail's stream within its tick and the fighters' fields agree."""
from __future__ import annotations

import json
import struct

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


def bits(value: float) -> int:
    return struct.unpack("<I", struct.pack("<f", value))[0]


def single(value: float) -> float:
    return rvt.f32(bits(value))


def hsd_status(button=0, stick=(0, 0), cstick=(0, 0), triggers=(0, 0)) -> dict:
    """An HSD_PadStatus as decode.py writes it, from clamped stick bytes."""
    def f32(value):
        return {"t": "f32", "v": {"bits": bits(value), "approx": value}}
    return {"button": {"t": "u", "v": button},
            "nml_stickX": f32(stick[0] / 80), "nml_stickY": f32(stick[1] / 80),
            "nml_subStickX": f32(cstick[0] / 80), "nml_subStickY": f32(cstick[1] / 80),
            "nml_analogL": f32(triggers[0] / 140), "nml_analogR": f32(triggers[1] / 140)}


def pre_frame(physical=0, held=0, stick=(0.0, 0.0), cstick=(0.0, 0.0), triggers=(0, 0), raw_x=None) -> dict:
    return {"stick": [bits(v) for v in stick], "cstick": [bits(v) for v in cstick], "trigger": 0,
            "held": held, "physical": physical, "triggers": [bits(t / 140) for t in triggers],
            "raw_stick": [raw_x, None], "raw_cstick": [None, None]}


@pytest.mark.parametrize("pad,status,queue,differing", [
    # (59, -53) with A and the right bit; the fighter's own bit 31 and the
    # unrecorded raw bytes are not the pad's.
    (pre_frame(0x100, 0x80080100, (0.7375, -0.6625), triggers=(0, 3), raw_x=79),
     hsd_status(0x80100, (59, -53), triggers=(0, 3)), [79, -70, 0, 0], {}),
    # A dead-zoned stick is zero on both sides, whatever the raw bytes.
    (pre_frame(raw_x=12), hsd_status(0, (12, -22)), [12, -22, 0, 0], {}),
    # `PEACH/15_52_33`, tick 3011: the C-stick's up bit, which raw (0, 27) lacks.
    (pre_frame(held=0x100000, cstick=(0.0, 0.3375), raw_x=0), hsd_status(0, cstick=(0, 27)), [0, 0, 0, 27],
     {"directions": (0x100000, 0)}),
    # The queue entry UCF reads held another X than the console's.
    (pre_frame(held=0x80000, stick=(0.9875, 0.0), raw_x=106), hsd_status(0x80000, (79, 0)), [82, -1, 0, 0],
     {"raw": ([106, None, None, None], [82, None, None, None])}),
    # A recorded Start press is not fed to retail and not compared.
    (pre_frame(0x1100, 0x1100), hsd_status(0x100), [0, 0, 0, 0], {}),
    (pre_frame(0x100, 0x100, triggers=(40, 0)), hsd_status(0x200, triggers=(41, 0)), None,
     {"buttons": (0x100, 0x200), "triggers": ([single(40 / 140), 0.0], [single(41 / 140), 0.0])}),
])
def test_pad_differences_compare_what_pre_frame_holds_of_the_pad(pad, status, queue, differing):
    found = rvt.pad_differences(pad, status, queue)
    assert found == differing


def write_pads(traces, frames_path, retail_sticks):
    """Give the three-frame game of `write_game` a pad per frame: the replay
    holds raw X 30, 40, 50; retail consumed `retail_sticks`."""
    trace = traces / "game.tick.expected.jsonl"
    records = [json.loads(line) for line in trace.read_text().splitlines()]
    for record, x in zip(records[1:], retail_sticks):
        record["inputs"] = {f"p{port}": hsd_status(0x80000 if port == 2 else 0, (x if port == 2 else 0, 0))
                            for port in range(4)}
        record["pad_queue_sticks"] = [[0, 0, 0, 0], [0, 0, 0, 0], [x, 0, 0, 0], [0, 0, 0, 0]]
    trace.write_text("".join(json.dumps(record) + "\n" for record in records))
    lines = [json.loads(line) for line in open(frames_path)]
    for line, x in zip(lines[1:], (30, 40, 50)):
        line["fighters"][0]["pad"] = pre_frame(held=0x80000, stick=(x / 80, 0.0), raw_x=x)
    open(frames_path, "w").write("".join(json.dumps(line) + "\n" for line in lines))


def test_a_pad_retail_did_not_consume_fails_the_bridge_from_go_on(traces, capsys, monkeypatch):
    frames = write_game(traces, [3, 7, 0], [4, 2, 9], [4, 2, 9])
    write_pads(traces, frames, (30, 40, 50))
    assert rvt.compare(frames, "game", None) == 0
    assert "was fed its pads on every frame" in capsys.readouterr().out
    # Retail consumed the tick before's pad on the last tick. Before GO no
    # fighter reads its pad: counted, not failed.
    write_pads(traces, frames, (30, 40, 40))
    assert rvt.compare(frames, "game", None) == 0
    assert "1 pads differ before GO" in capsys.readouterr().out
    monkeypatch.setattr(rvt, "INPUT_RELEASE_FRAME", 2)
    assert rvt.compare(frames, "game", None) == 1
    out = capsys.readouterr().out
    assert "inputs: first differs at replay frame 2 (retail record 3); 1 frames differ" in out
    assert ("retail was not fed the replay's pads: at frame 2 the replay has port 3 stick (0.625, 0), "
            "retail (0.5, 0); port 3 raw (50, -, -, -), retail (40, -, -, -)") in out


def test_frames_without_pads_are_said_not_to_be_compared(traces, capsys):
    frames = write_game(traces, [3, 7, 0], [4, 2, 9], [4, 2, 9])
    assert rvt.compare(frames, "game", None) == 0
    assert "inputs not compared" in capsys.readouterr().out
