"""CPU callback lifecycle with fake memory; never launches Dolphin."""
import json

import pytest

import decode
import remote_proto
import run_scenario
import tick_trace as tick
import trace_common
from test_walk import (_FakeController, _FakeStates, build_two_fighter_world)


class Events:
    def on_memorybreakpoint(self, callback):
        self.memory_callback = callback


@pytest.fixture
def capture(tmp_path):
    mem = build_two_fighter_world()
    for addr, word in tick.BOUNDARY_CODE.items():
        mem.write_u32(addr, word)
    mem.write_u32(tick.WATCH_ADDR, 100)
    events, ctl = Events(), _FakeController()
    states = _FakeStates(mem, 123)
    calls = []
    mem.add_memcheck = lambda addr: calls.append(("add", addr, len(states.loaded)))
    mem.remove_memcheck = lambda addr: calls.append(("remove", addr))
    raw = tmp_path / "ticks.raw.jsonl"
    done = tmp_path / "ticks.raw.jsonl.done"
    tracer = tick.TickTracer({"frames": 3}, raw.open("w"), tmp_path / "match.sav",
                             {"seed": 123}, done, mem, ctl, states, events)
    return tracer, mem, raw, done, calls


def store(tracer, mem, value):
    tracer.on_memory(True, tick.WATCH_ADDR, value)
    # Dolphin calls Python BEFORE the guest store.
    mem.write_u32(tick.WATCH_ADDR, value)


def test_samples_each_tick_even_when_two_ticks_share_a_vi(capture, tmp_path):
    tracer, mem, raw, done, calls = capture
    assert not calls and not tracer.states.loaded
    tracer.on_frame()
    assert calls == [("add", tick.WATCH_ADDR, 1)]
    assert raw.read_text() == ""  # loaded VI state is not a completed tick
    store(tracer, mem, 101)
    store(tracer, mem, 102)
    tracer.on_frame()
    tracer.on_frame()  # a VI with no tick must not produce a record
    store(tracer, mem, 103)
    assert not done.exists() and len(calls) == 1  # no removal inside Action
    tracer.on_frame()
    rows = [json.loads(line) for line in raw.read_text().splitlines()]
    assert [row["frame"] for row in rows] == [0, 1, 2]
    assert [row["tick"] for row in rows] == [101, 102, 103]
    assert [row["watch_value"] for row in rows] == [100, 101, 102]
    assert [row["vi_frame"] for row in rows] == [0, 0, 2]
    assert all(row["phase"] == "frame_end" and len(row["fighters"]) == 2 for row in rows)
    assert tracer.ctl.log == [(0, remote_proto.neutral_inputs())] * 3
    summary = json.loads(done.read_text())
    assert summary["synced"] and summary["ticks"] == 3 and summary["initial_tick"] == 100
    assert calls[-1] == ("remove", tick.WATCH_ADDR)
    tracer.on_frame()
    tracer.on_memory(True, tick.WATCH_ADDR, 104)
    assert len(calls) == 2 and len(raw.read_text().splitlines()) == 3
    decoded = tmp_path / "decoded.jsonl"
    decode.main(raw, decoded)
    result = json.loads(decoded.read_text().splitlines()[0])
    assert result["tick"] == 101 and result["watch_value"] == 100
    assert len(result["state"]) == 49  # diagnostics do not contaminate game state


def test_reads_unrelated_writes_duplicates_and_reentrancy_do_not_dump(capture):
    tracer, mem, raw, _, _ = capture
    tracer.on_frame()
    tracer.on_memory(False, tick.WATCH_ADDR, 100)
    tracer.on_memory(True, tick.WATCH_ADDR + 4, 101)
    original_record = tracer.record

    def reentrant_record(phase):
        tracer.on_memory(True, tick.WATCH_ADDR, 101)
        return original_record(phase)

    tracer.record = reentrant_record
    store(tracer, mem, 101)
    tracer.on_memory(True, tick.WATCH_ADDR, 101)
    assert len(raw.read_text().splitlines()) == 1
    assert tracer.duplicates == tracer.reentrant == 1
    tracer.fail("test cleanup")


def test_counter_reset_to_zero_on_first_tick_is_a_scene_start(capture):
    # A match scene resets the scheduler tick counter to 0 on its first tick.
    tracer, mem, raw, done, calls = capture
    tracer.on_frame()
    tracer.on_memory(True, tick.WATCH_ADDR, 0)
    assert not raw.with_suffix(".jsonl.err").exists()
    assert tracer.counter_reset_at_start and tracer.last_tick == 0
    assert len(raw.read_text().splitlines()) == 1
    tracer.fail("test cleanup")


@pytest.mark.parametrize("bad_value", [100, 102])
def test_counter_reset_repeated_write_or_gap_fail(capture, bad_value):
    tracer, mem, raw, done, calls = capture
    tracer.on_frame()
    tracer.on_memory(True, tick.WATCH_ADDR, bad_value)
    assert "discontinuity" in raw.with_suffix(".jsonl.err").read_text()
    assert not done.exists() and tracer.done and len(calls) == 1
    tracer.on_frame()
    assert calls[-1] == ("remove", tick.WATCH_ADDR)


def test_post_store_callback_is_rejected(capture):
    tracer, mem, raw, done, _ = capture
    tracer.on_frame()
    mem.write_u32(tick.WATCH_ADDR, 101)
    tracer.on_memory(True, tick.WATCH_ADDR, 101)
    assert "discontinuity" in raw.with_suffix(".jsonl.err").read_text()
    assert not done.exists()
    tracer.on_frame()


@pytest.mark.parametrize("problem", ["code", "sidecar", "saturated", "api"])
def test_installation_fails_with_useful_error_marker(capture, problem):
    tracer, mem, raw, done, _ = capture
    expected = {"code": "retail code mismatch", "sidecar": "sidecar",
                "saturated": "saturated", "api": "add_memcheck"}[problem]
    if problem == "code":
        mem.write_u32(tick.STORE_PC, 0)
    elif problem == "sidecar":
        tracer.sidecar = {"seed": 999}
    elif problem == "saturated":
        mem.write_u32(tick.WATCH_ADDR, tick.SATURATED)
    else:
        del mem.add_memcheck
    tracer.on_frame()
    assert expected in raw.with_suffix(".jsonl.err").read_text()
    assert not done.exists()


def test_watchdog_rejects_missing_ticks(capture):
    tracer, _, raw, done, calls = capture
    for _ in range(tick.MAX_VI_WITHOUT_TICK + 1):
        tracer.on_frame()
    assert "no scheduler tick" in raw.with_suffix(".jsonl.err").read_text()
    assert calls[-1][0] == "remove" and not done.exists()


def test_runner_selects_tick_script_and_required_dolphin_flags(tmp_path):
    command = run_scenario.dolphin_command(tmp_path / "g.iso", 0, "OGL", 2, True)
    assert command[command.index("--script") + 1].endswith("/tick_trace.py")
    assert command[command.index("-v") + 1] == "OGL"
    assert "Dolphin.Core.SIDevice1=6" in command
    legacy = run_scenario.dolphin_command(tmp_path / "g.iso", 0, "OGL", 2)
    assert legacy[legacy.index("--script") + 1].endswith("/trace_scenario.py")


@pytest.mark.parametrize("scenario", [{"frames": 0}, {"frames": 1.5},
                                      {"frames": 3, "inputs": [{"frame": 1, "buttons": {"Q": True}}]},
                                      {"frames": 3, "inputs": [{"frame": -1, "buttons": {"A": True}}]},
                                      {"frames": 3, "fighters": [{"slot": 0, "inputs": []}]}])
def test_invalid_tick_scenario_rejected_before_installation(tmp_path, scenario):
    with pytest.raises(ValueError):
        tick.TickTracer(scenario, None, tmp_path / "match.sav")


def test_startup_error_replaces_stale_done_marker(tmp_path, monkeypatch):
    import trace_common

    raw = tmp_path / "trace.jsonl"
    done = tmp_path / "trace.jsonl.done"
    err = tmp_path / "trace.jsonl.err"
    done.write_text("old success")
    err.write_text("old error")
    monkeypatch.setenv("MELEE_RAW_OUT", str(raw))
    monkeypatch.setenv("MELEE_SCENARIO", str(tmp_path / "missing.toml"))
    trace_common.run(tick.TickTracer)
    assert not done.exists() and "missing.toml" in err.read_text()
    assert raw.read_text() == ""


def test_scripted_steps_hold_per_port_and_each_tick_records_the_game_pads(tmp_path):
    mem = build_two_fighter_world()
    for addr, word in tick.BOUNDARY_CODE.items():
        mem.write_u32(addr, word)
    mem.write_u32(tick.WATCH_ADDR, 100)
    events, ctl = Events(), _FakeController()
    states = _FakeStates(mem, 123)
    mem.add_memcheck = lambda addr: None
    mem.remove_memcheck = lambda addr: None
    mem.write_u32(trace_common.PAD_GAME_ADDR + 0x44 + 0x20, 0x3F800000)  # p1 nml_stickX = 1.0
    raw = tmp_path / "ticks.raw.jsonl"
    scenario = {"frames": 3, "inputs": [{"frame": 1, "buttons": {"StickX": 0.5}},
                                        {"frame": 1, "port": 1, "buttons": {"A": True}},
                                        {"frame": 2, "buttons": {}}]}
    tracer = tick.TickTracer(scenario, raw.open("w"), tmp_path / "match.sav", {"seed": 123},
                             tmp_path / "ticks.raw.jsonl.done", mem, ctl, states, events)
    neutral = remote_proto.neutral_inputs()
    tracer.on_frame()
    store(tracer, mem, 101)
    tracer.on_frame()
    store(tracer, mem, 102)
    tracer.on_frame()
    tracer.on_frame()
    assert ctl.log[:1] == [(0, neutral)]
    assert ctl.log[1:3] == [(0, {**neutral, "StickX": 0.5}), (1, {**neutral, "A": True})]
    assert ctl.log[3:5] == [(0, neutral), (1, {**neutral, "A": True})]  # p1 holds until its next step
    store(tracer, mem, 103)
    tracer.on_frame()
    rows = [json.loads(line) for line in raw.read_text().splitlines()]
    assert len(rows) == 3 and all(len(bytes.fromhex(r["pad_game"])) == 4 * 0x44 for r in rows)
    decoded = tmp_path / "decoded.jsonl"
    decode.main(raw, decoded)
    result = json.loads(decoded.read_text().splitlines()[0])
    assert len(result["state"]) == 49  # inputs live beside the compared state
    assert result["inputs"]["p1"]["nml_stickX"] == {"t": "f32", "v": {"bits": 0x3F800000, "approx": 1.0}}
    assert result["inputs"]["p0"]["button"] == {"t": "u", "v": 0}
    assert set(result["inputs"]) == {"p0", "p1", "p2", "p3"}
