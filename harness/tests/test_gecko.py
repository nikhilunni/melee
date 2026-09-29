"""Gecko code installation and the tracer's pad-queue view; no Dolphin."""
import json

import pytest

import dolphin_config
import gecko
import tick_trace as tick
import trace_common
from test_walk import _FakeController, _FakeStates, build_two_fighter_world

# A made-up two-code list in Slippi's text layout (not real UCF code).
TEXT = """$Example [someone]
C20C9A44 00000002 #a/b.asm
60000000 60000000
60000000 00000000
C20998A4 00000001 #c.asm
60000000 00000000
"""


def test_text_and_binary_codes_give_the_same_lines(tmp_path):
    (tmp_path / "a.txt").write_text(TEXT)
    lines = gecko.code_lines("a", tmp_path)
    assert lines[0] == "C20C9A44 00000002" and len(lines) == 5
    (tmp_path / "b.bin").write_bytes(b"".join(bytes.fromhex(line.replace(" ", ""))
                                              for line in lines))
    assert gecko.code_lines("b", tmp_path) == lines
    assert gecko.injection_addresses(lines) == [0x800C9A44, 0x800998A4]


def test_missing_code_or_directory_is_a_clear_error(tmp_path, monkeypatch):
    with pytest.raises(SystemExit, match="missing"):
        gecko.code_lines("ucf-0.8", tmp_path)
    monkeypatch.delenv(gecko.DIR_ENV, raising=False)
    with pytest.raises(SystemExit, match=gecko.DIR_ENV):
        gecko.code_lines("ucf-0.8")


def test_install_enables_each_code_in_the_user_folder(tmp_path):
    (tmp_path / "codes").mkdir()
    (tmp_path / "codes" / "ucf-0.8.txt").write_text(TEXT)
    hooks = gecko.install(tmp_path / "user", ["ucf-0.8"], tmp_path / "codes")
    ini = (tmp_path / "user" / gecko.GAME_INI).read_text().splitlines()
    assert ini[:2] == ["[Gecko]", "$ucf-0.8"]
    assert "[Gecko_Enabled]" in ini and ini[ini.index("[Gecko_Enabled]") + 1] == "$ucf-0.8"
    assert hooks == [0x800C9A44, 0x800998A4]


def test_cheats_flag_follows_the_environment(tmp_path, monkeypatch):
    binary = tmp_path / "dolphin-emu-nogui"
    monkeypatch.delenv("DOLPHIN_CHEATS", raising=False)
    assert "Dolphin.Core.EnableCheats=True" not in dolphin_config.launch_flags(binary, None)
    monkeypatch.setenv("DOLPHIN_CHEATS", "1")
    assert "Dolphin.Core.EnableCheats=True" in dolphin_config.launch_flags(binary, None)


class Events:
    def on_memorybreakpoint(self, callback):
        self.memory_callback = callback


QUEUE = 0x80460000


def tracer_with_queue(tmp_path, monkeypatch, hooks: str, steps):
    monkeypatch.setenv(gecko.HOOKS_ENV, hooks)
    mem = build_two_fighter_world()
    mem.write_u8 = lambda addr, value: mem.mem.__setitem__(addr, value & 0xFF)
    for addr, word in tick.BOUNDARY_CODE.items():
        mem.write_u32(addr, word)
    mem.write_u32(tick.WATCH_ADDR, 100)
    mem.write_u8(tick.PAD_LIB_ADDR, 5)          # qnum
    mem.write_u8(tick.PAD_LIB_ADDR + 2, 3)      # qwrite
    mem.write_u32(tick.PAD_LIB_ADDR + 8, QUEUE)
    mem.add_memcheck = lambda addr: None
    mem.remove_memcheck = lambda addr: None
    raw = tmp_path / "ticks.raw.jsonl"
    scenario = {"frames": 3, "input_clock": "tick", "inputs": steps}
    tracer = tick.TickTracer(scenario, raw.open("w"), tmp_path / "match.sav", {"seed": 123},
                             tmp_path / "ticks.raw.jsonl.done", mem, _FakeController(),
                             _FakeStates(mem, 123), Events())
    return tracer, mem, raw


def stick_x(mem, entry, port=0):
    byte = mem.read_u8(QUEUE + entry * tick.PAD_ENTRY_BYTES + port * tick.PAD_STATUS_BYTES + 2)
    return byte - 0x100 if byte & 0x80 else byte


def consume(tracer, mem, value):
    """The game dequeues the injected entry, then the tick ends."""
    mem.write_u8(tick.PAD_LIB_ADDR + 1, (mem.read_u8(tick.PAD_LIB_ADDR + 1) + 1) % 5)
    tracer.on_memory(True, tick.WATCH_ADDR, value)
    mem.write_u32(tick.WATCH_ADDR, value)


def test_gecko_runs_keep_one_tick_per_queue_entry_and_record_what_ucf_reads(tmp_path, monkeypatch):
    steps = [{"frame": 0, "buttons": {}, "raw": {"stickX": -80}},
             {"frame": 1, "buttons": {}, "raw": {"stickX": 10}},
             {"frame": 2, "buttons": {}, "raw": {"stickX": 127}}]
    tracer, mem, raw = tracer_with_queue(tmp_path, monkeypatch, "800C9A44", steps)
    mem.write_u32(0x800C9A44, 0x48000010)  # the installed branch
    tracer.on_frame()
    # Tick 0's pad in the newest poll slot (2); slots 1 and 0 hold neutral history.
    assert [stick_x(mem, e) for e in (2, 1, 0)] == [-80, 0, 0]
    # Dolphin polled twice during tick 0 (entries 3 and 4): qwrite jumped by two.
    mem.write_u8(tick.PAD_LIB_ADDR + 2, 0)
    for entry in (3, 4):
        mem.write_u8(QUEUE + entry * tick.PAD_ENTRY_BYTES + 2, 0x33)
    consume(tracer, mem, 101)  # the tick end injects tick 1's pad
    # Newest poll slot 4 holds tick 1, then tick 0, then the neutral tick before.
    assert [stick_x(mem, e) for e in (4, 3, 2)] == [10, -80, 0]
    consume(tracer, mem, 102)
    rows = [json.loads(line) for line in raw.read_text().splitlines()]
    # UCF's FETCH_INPUT: entries qread-1 and qread-3 (index-1, +5 when negative).
    assert rows[0]["pad_queue_x"][0] == [-80, 0]
    assert rows[1]["pad_queue_x"][0] == [10, 0]
    assert rows[1]["pad_queue_sticks"][0] == [10, 0, 0, 0]
    assert len(rows[1]["pad_queue_x"]) == 4


def test_missing_gecko_hook_fails_the_first_record(tmp_path, monkeypatch):
    tracer, mem, raw = tracer_with_queue(tmp_path, monkeypatch, "800C9A44", [])
    mem.write_u32(0x800C9A44, 0xD01F002C)  # retail stfs: the code never ran
    tracer.on_frame()
    consume(tracer, mem, 101)
    assert tracer.done and "not installed" in (tmp_path / "ticks.raw.jsonl.err").read_text()


def test_without_codes_the_queue_history_is_left_alone(tmp_path, monkeypatch):
    steps = [{"frame": 0, "buttons": {}, "raw": {"stickX": -80}}]
    tracer, mem, raw = tracer_with_queue(tmp_path, monkeypatch, "", steps)
    mem.write_u8(QUEUE + 1 * tick.PAD_ENTRY_BYTES + 2, 0x33)
    tracer.on_frame()
    assert [stick_x(mem, e) for e in (2, 1)] == [-80, 0x33]
    consume(tracer, mem, 101)
    row = json.loads(raw.read_text().splitlines()[0])
    assert row["pad_queue_x"][0] == [-80, 0]
    assert len(bytes.fromhex(row["pad_game"])) == 4 * 0x44  # still beside it
    assert trace_common.PAD_GAME_ADDR


def test_recorder_installs_codes_for_every_pass_and_restores_the_environment(tmp_path, monkeypatch):
    import os

    import record
    (tmp_path / "codes").mkdir()
    (tmp_path / "codes" / "ucf-0.8.txt").write_text(TEXT)
    monkeypatch.setenv(gecko.DIR_ENV, str(tmp_path / "codes"))
    monkeypatch.setenv("DOLPHIN_USER_DIR", str(tmp_path / "user"))
    monkeypatch.delenv("DOLPHIN_CHEATS", raising=False)
    monkeypatch.delenv(gecko.HOOKS_ENV, raising=False)
    with record.gecko_codes(["ucf-0.8"]):
        assert os.environ["DOLPHIN_CHEATS"] == "1"
        assert gecko.hooks_from_env() == [0x800C9A44, 0x800998A4]
        assert (tmp_path / "user" / gecko.GAME_INI).exists()
    assert "DOLPHIN_CHEATS" not in os.environ and gecko.HOOKS_ENV not in os.environ
    with record.gecko_codes([]):
        assert "DOLPHIN_CHEATS" not in os.environ
