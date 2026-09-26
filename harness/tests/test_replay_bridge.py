"""Corpus bridge: port recordings -> tick-clock Dolphin scenarios."""
import json
import struct
import tomllib

import pytest

import replay_to_scenario as bridge
import tick_trace as tick


def f32_bits(value: float) -> int:
    return struct.unpack("<I", struct.pack("<f", value))[0]


def sample(buttons=0, stick=(0.0, 0.0), cstick=(0.0, 0.0), triggers=(0.0, 0.0)) -> list[int]:
    return [buttons, *(f32_bits(v) for v in (*stick, *cstick, *triggers))]


NEUTRAL = [sample(), sample(), sample(), sample()]
TABLE = {"stick": {str(v): v / 128 for v in range(-80, 81)},
         "trigger": {str(v): v / 256 for v in range(0, 141)}}


@pytest.mark.parametrize("value,scale,raw", [(0.7, 80, 56), (-0.5, 80, -40), (1.0, 80, 80),
                                             (0.3, 140, 42), (1.0, 140, 140)])
def test_integral_recovers_raw_from_f32_quotient(value, scale, raw):
    as_f32 = struct.unpack("<f", struct.pack("<f", value))[0]
    assert bridge.integral(as_f32, scale, -scale, scale, "x") == raw


@pytest.mark.parametrize("value", [0.33, 1.5, 0.7000001])
def test_integral_rejects_values_retail_cannot_produce(value):
    with pytest.raises(bridge.Unreachable):
        bridge.integral(value, 80, -80, 80, "x")


def test_stick_outside_hsd_clamp_radius_is_unreachable():
    with pytest.raises(bridge.Unreachable):
        bridge.stick_raw(1.0, 1.0, "stick")
    seventy = bridge.as_f32(0.7)
    assert bridge.stick_raw(seventy, seventy, "stick") == (56, 56)


def test_dolphin_pad_carries_buttons_sticks_and_triggers():
    pad, raw = bridge.dolphin_pad(sample(0x140, (0.0, -1.0), (0.5, 0.0), (1.0, 0.0)), TABLE)
    assert pad == {"L": True, "A": True, "StickY": -80 / 128, "CStickX": 40 / 128,
                   "TriggerLeft": 140 / 256}
    assert raw == {"button": 0x140, "stickY": -80, "substickX": 40, "triggerL": 140}


def test_dolphin_pad_rejects_non_controller_bits():
    with pytest.raises(bridge.Unreachable):
        bridge.dolphin_pad(sample(1 << 31), TABLE)


def recording(samples, seed=2477457595, players=(("Fox"), ("Marth"))) -> dict:
    return {"version": 1, "config": {"stage": "FinalDestination",
                                      "players": [[0, players[0], 0], [1, players[1], 0]],
                                      "stocks": 4, "all_characters_unlocked": True, "seed": seed},
            "fault": {"attempt": 3, "message": "boom"}, "samples": samples}


def test_boundary_selection_requires_the_boundary_seed():
    assert bridge.boundary_for(recording([])["config"])["name"] == "start_fd_fox4"
    assert bridge.boundary_for(recording([], 629775590, ("Marth", "Fox"))["config"])["name"] \
        == "start_fd_marth4"
    with pytest.raises(SystemExit, match="boundary seed"):
        bridge.boundary_for(recording([], seed=1)["config"])


def test_scenario_uses_tick_clock_and_only_emits_changes(monkeypatch, tmp_path):
    monkeypatch.setattr(bridge, "HERE", tmp_path)
    (tmp_path / "scenarios").mkdir()
    calibration = tmp_path / "pad_calibration.json"
    calibration.write_text(json.dumps(TABLE))
    monkeypatch.setattr(bridge, "CALIBRATION", calibration)
    hold_a = [sample(0x100), sample(), sample(), sample()]
    path = bridge.write_scenario(recording([NEUTRAL, hold_a, hold_a, NEUTRAL]), "bridged", None)
    scenario = tomllib.loads(path.read_text())
    assert scenario["input_clock"] == "tick" and scenario["frames"] == 4
    assert scenario["savestate"] == "harness/roms/start_fd_fox4.sav"
    port0 = [(s["frame"], s["raw"]) for s in scenario["inputs"] if s["port"] == 0]
    assert port0 == [(0, {}), (1, {"button": 0x100}), (3, {})]
    assert [f["kind"] for f in scenario["fighters"]] == ["Fox", "Marth"]


def test_first_tick_must_be_neutral(monkeypatch, tmp_path):
    monkeypatch.setattr(bridge, "HERE", tmp_path)
    with pytest.raises(bridge.Unreachable, match="first recorded tick"):
        bridge.write_scenario(recording([[sample(0x100), sample(), sample(), sample()]]), "x", None)


class QueueMemory:
    """HSD_PadLibData plus a four-entry raw queue in a byte dict."""

    def __init__(self, qnum=4, qwrite=2, qcount=0):
        self.bytes: dict[int, int] = {}
        self.queue = 0x80500000
        base = tick.PAD_LIB_ADDR
        for offset, value in enumerate([qnum, 0, qwrite, qcount]):
            self.bytes[base + offset] = value
        for offset, value in enumerate(struct.pack(">I", self.queue)):
            self.bytes[base + 8 + offset] = value

    def read_u8(self, addr):
        return self.bytes.get(addr, 0)

    def read_u32(self, addr):
        return struct.unpack(">I", bytes(self.read_u8(addr + i) for i in range(4)))[0]

    def write_u8(self, addr, value):
        self.bytes[addr] = value

    def status(self, slot, port) -> bytes:
        base = self.queue + slot * tick.PAD_ENTRY_BYTES + port * tick.PAD_STATUS_BYTES
        return bytes(self.read_u8(base + i) for i in range(tick.PAD_STATUS_BYTES))


def test_raw_pad_bytes_follow_padstatus_layout():
    raw = tick.raw_pad_bytes({"button": 0x300, "stickX": -80, "substickY": 40, "triggerR": 140})
    assert raw == bytes([0x03, 0x00, 0xB0, 0, 0, 40, 0, 140, 0xFF, 0xFF])


def test_tick_clock_rewrites_queue_to_the_next_ticks_pad(tmp_path):
    scenario = {"frames": 3, "input_clock": "tick", "inputs": [
        {"frame": 0, "port": 0, "buttons": {}, "raw": {}},
        {"frame": 1, "port": 0, "buttons": {"A": True}, "raw": {"button": 0x100}},
        {"frame": 2, "port": 1, "buttons": {"StickX": 0.5}, "raw": {"stickX": 64}}]}
    mem = QueueMemory(qnum=4, qwrite=2, qcount=0)
    tracer = tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=mem)
    tracer.inject_tick_pads(0)
    assert (mem.read_u8(tick.PAD_LIB_ADDR + 1), mem.read_u8(tick.PAD_LIB_ADDR + 3)) == (1, 1)
    assert mem.status(1, 0)[:2] == b"\x00\x00"
    tracer.inject_tick_pads(1)
    assert mem.status(1, 0)[:2] == b"\x01\x00" and mem.status(1, 0)[8] == 0xFF
    mem.write_u8(tick.PAD_LIB_ADDR + 2, 3)  # a VI poll advanced qwrite
    tracer.inject_tick_pads(2)
    assert mem.read_u8(tick.PAD_LIB_ADDR + 1) == 2
    assert mem.status(2, 0)[:2] == b"\x01\x00"  # port 0 keeps its held step
    assert mem.status(2, 1)[2] == 64
    assert tracer.injected_ticks == 3
