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
    # write_scenario writes under the data root, not this checkout.
    monkeypatch.setattr(bridge.data_root, "SCENARIOS", tmp_path / "scenarios")
    (tmp_path / "scenarios").mkdir()
    calibration = tmp_path / "pad_calibration.json"
    calibration.write_text(json.dumps(TABLE))
    monkeypatch.setattr(bridge, "CALIBRATION", calibration)
    hold_a = [sample(0x100), sample(), sample(), sample()]
    path = bridge.write_scenario(recording([NEUTRAL, hold_a, hold_a, NEUTRAL]), "bridged", None)
    scenario = tomllib.loads(path.read_text())
    assert scenario["input_clock"] == "tick" and scenario["frames"] == 5
    assert scenario["savestate"] == "harness/roms/start_fd_fox4.sav"
    port0 = [(s["frame"], s["raw"]) for s in scenario["inputs"] if s["port"] == 0]
    assert port0 == [(1, {}), (2, {"button": 0x100}), (4, {})]
    assert [f["kind"] for f in scenario["fighters"]] == ["Fox", "Marth"]
    # The port's Match renders after every tick; --dolphin-cadence leaves the key out.
    assert scenario["render_every_tick"] is True
    dolphin = bridge.write_scenario(recording([NEUTRAL]), "dolphin", None, render_every_tick=False)
    assert "render_every_tick" not in tomllib.loads(dolphin.read_text())


def test_first_sample_drives_the_tick_after_the_boundary(monkeypatch, tmp_path):
    # Match::new completes the oracle's tick 0, so no sample is lost to it.
    # write_scenario writes under the data root, not this checkout.
    monkeypatch.setattr(bridge.data_root, "SCENARIOS", tmp_path / "scenarios")
    (tmp_path / "scenarios").mkdir()
    calibration = tmp_path / "pad_calibration.json"
    calibration.write_text(json.dumps(TABLE))
    monkeypatch.setattr(bridge, "CALIBRATION", calibration)
    press = [sample(0x100), sample(), sample(), sample()]
    scenario = tomllib.loads(bridge.write_scenario(recording([press]), "x", None).read_text())
    assert scenario["frames"] == 2
    assert [(s["frame"], s["raw"]) for s in scenario["inputs"] if s["port"] == 0] \
        == [(1, {"button": 0x100})]


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


def poll(mem, status: bytes) -> None:
    """HSD_PadRenewRawStatus (controller.c:77-105) with qtype 2: a full queue
    drops the poll; otherwise it is stored at queue[qwrite]."""
    lib = tick.PAD_LIB_ADDR
    qnum, qwrite, qcount = mem.read_u8(lib), mem.read_u8(lib + 2), mem.read_u8(lib + 3)
    if qcount == qnum:
        return
    for i, byte in enumerate(status):
        mem.write_u8(mem.queue + qwrite * tick.PAD_ENTRY_BYTES + i, byte)
    mem.write_u8(lib + 2, (qwrite + 1) % qnum)
    mem.write_u8(lib + 3, qcount + 1)


@pytest.mark.parametrize("stale_qwrite", [0, 2, 3, 4])
def test_a_poll_during_a_tick_leaves_the_entries_ucf_reads(tmp_path, stale_qwrite):
    # The one-entry queue is entry 4, its history entries 3 and 2. A poll
    # that arrives after the tick's dequeue is stored at queue[qwrite]: with
    # the five-entry queue's qwrite left in place the first one overwrote
    # one of them (record 302 of slp_ps_fox_drmario_wide9263_r1_t9800 read
    # raw X 78, the tick before's, for the scheduled 16).
    scenario = {"frames": 4, "input_clock": "tick", "render_every_tick": True, "inputs": [
        {"frame": 0, "port": 0, "buttons": {}, "raw": {"stickX": 95}},
        {"frame": 1, "port": 0, "buttons": {}, "raw": {"stickX": 78}},
        {"frame": 2, "port": 0, "buttons": {}, "raw": {"stickX": 16}}]}
    mem = QueueMemory(qnum=5, qwrite=stale_qwrite, qcount=0)
    tracer = tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=mem)
    tracer.gecko_hooks = True
    lib = tick.PAD_LIB_ADDR
    for record in range(3):
        tracer.inject_tick_pads(record)
        poll(mem, bytes(tick.PAD_STATUS_BYTES))          # full: dropped
        assert mem.read_u8(lib + 1) == 4                 # the tick dequeues entry 4
        mem.write_u8(lib + 1, 0)
        mem.write_u8(lib + 3, 0)
        poll(mem, bytes([0, 0, 82] + [0] * 9))           # Dolphin's own poll, mid-tick
    assert [mem.status(entry, 0)[2] for entry in (4, 3, 2)] == [16, 78, 95]
    assert mem.status(0, 0)[2] == 82


def test_a_failed_poll_writes_err_and_every_other_entry_clears_it(tmp_path):
    scenario = {"frames": 4, "input_clock": "tick", "render_every_tick": True, "inputs": [
        {"frame": 0, "port": 0, "buttons": {}, "raw": {"button": 0x100, "stickX": 61}},
        {"frame": 1, "port": 0, "buttons": {}, "raw": {"err": -3}},
        {"frame": 2, "port": 0, "buttons": {}, "raw": {"button": 0x100, "stickX": 61}}]}
    mem = QueueMemory(qnum=5, qwrite=0, qcount=0)
    tracer = tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=mem)
    err = tick.PAD_ERR_OFFSET
    tracer.inject_tick_pads(0)
    assert mem.status(4, 0)[err] == 0
    tracer.inject_tick_pads(1)
    assert mem.status(4, 0)[:3] == b"\x00\x00\x00" and mem.status(4, 0)[err] == 0xFD
    tracer.inject_tick_pads(2)
    assert mem.status(4, 0)[:3] == b"\x01\x00\x3d" and mem.status(4, 0)[err] == 0
    # A schedule without one leaves err as Dolphin polled it.
    plain = {"frames": 2, "input_clock": "tick", "inputs": [
        {"frame": 0, "port": 0, "buttons": {}, "raw": {"stickX": 61}}]}
    mem = QueueMemory(qnum=4, qwrite=2, qcount=0)
    mem.write_u8(mem.queue + 1 * tick.PAD_ENTRY_BYTES + err, 0xFF)
    tick.TickTracer(plain, None, tmp_path / "unused.sav", mem=mem).inject_tick_pads(0)
    assert mem.status(1, 0)[err] == 0xFF


def test_an_unconsumed_poll_moves_the_queue_history_one_entry_back(tmp_path):
    # Raw X 1, 45, 100, 100, 100 over records 1..5, and a poll nothing
    # dequeued before record 3: UCF's qread-3 (entry 2) is 45 there, not 1.
    scenario = {"frames": 6, "input_clock": "tick", "render_every_tick": True,
                "unconsumed_poll_before": [3], "inputs": [
                    {"frame": 1, "port": 0, "buttons": {}, "raw": {"stickX": 1}},
                    {"frame": 2, "port": 0, "buttons": {}, "raw": {"stickX": 45}},
                    {"frame": 3, "port": 0, "buttons": {}, "raw": {"stickX": 100}}]}
    mem = QueueMemory(qnum=5, qwrite=2, qcount=0)
    tracer = tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=mem)
    tracer.gecko_hooks = True
    read = []
    for record in range(6):
        tracer.inject_tick_pads(record)
        read.append((mem.status(4, 0)[2], mem.status(2, 0)[2]))  # qread-1, qread-3
    assert read == [(0, 0), (1, 0), (45, 0), (100, 45), (100, 100), (100, 100)]
    # Without it: (100, 1) at record 3 and (100, 45) at record 4.
    plain = tick.TickTracer({**scenario, "unconsumed_poll_before": []}, None, tmp_path / "unused.sav",
                            mem=QueueMemory(qnum=5, qwrite=2, qcount=0))
    plain.gecko_hooks = True
    for record in range(4):
        plain.inject_tick_pads(record)
    assert (plain.mem.status(4, 0)[2], plain.mem.status(2, 0)[2]) == (100, 1)


@pytest.mark.parametrize("keys,message", [
    ({"input_clock": "vi", "unconsumed_poll_before": [3]}, "needs the tick input clock"),
    ({"unconsumed_poll_before": [1]}, "records from 2 on"),
    ({"unconsumed_poll_before": [4, 5]}, "no two in a row"),
])
def test_an_unconsumed_poll_is_refused_where_the_queue_cannot_hold_it(tmp_path, keys, message):
    scenario = {"frames": 6, "input_clock": "tick", "inputs": [], **keys}
    with pytest.raises(ValueError, match=message):
        tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=QueueMemory())


def test_a_lag_frame_queues_two_ticks_and_the_next_injection_restores_one(tmp_path):
    # Records 2 and 3 run in one frame: no display pass before record 3.
    scenario = {"frames": 5, "input_clock": "tick", "render_every_tick": True,
                "no_display_pass_before": [3], "inputs": [
                    {"frame": 0, "port": 0, "buttons": {}, "raw": {}},
                    {"frame": 2, "port": 0, "buttons": {"A": True}, "raw": {"button": 0x100}},
                    {"frame": 3, "port": 0, "buttons": {}, "raw": {"stickX": 64}}]}
    mem = QueueMemory(qnum=5, qwrite=2, qcount=0)
    tracer = tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=mem)
    lib = tick.PAD_LIB_ADDR

    def queue():  # qnum, qread, qcount
        return mem.read_u8(lib), mem.read_u8(lib + 1), mem.read_u8(lib + 3)

    for record in (0, 1):
        tracer.inject_tick_pads(record)
        assert queue() == (1, 4, 1)
    # Record 2 starts the lag frame: gm_801A4D34 finds two queued polls. The
    # tick dequeues entry 1, a copy of the injected entry 4, which UCF reads.
    tracer.inject_tick_pads(2)
    assert queue() == (2, 1, 2)
    assert mem.status(1, 0)[:2] == mem.status(4, 0)[:2] == b"\x01\x00"
    # Record 3's injection, inside the same frame, is a one-entry queue again.
    tracer.inject_tick_pads(3)
    assert queue() == (1, 4, 1) and mem.status(4, 0)[2] == 64
    tracer.inject_tick_pads(4)
    assert queue() == (1, 4, 1)


@pytest.mark.parametrize("keys, message", [
    ({"no_display_pass_before": [3]}, "needs render_every_tick"),
    ({"render_every_tick": True, "no_display_pass_before": [1]}, "records from 2 on"),
    ({"render_every_tick": True, "no_display_pass_before": [5, 6]}, "no two in a row"),
])
def test_a_lag_frame_is_refused_where_the_queue_cannot_hold_it(tmp_path, keys, message):
    scenario = {"frames": 9, "input_clock": "tick", "inputs": [], **keys}
    with pytest.raises(ValueError, match=message):
        tick.TickTracer(scenario, None, tmp_path / "unused.sav", mem=QueueMemory(qnum=5))
