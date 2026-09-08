"""Unit tests for the Dolphin remote-control protocol (no Dolphin needed)."""
from __future__ import annotations

import json
import struct
from pathlib import Path

import pytest

import remote_proto as proto
from test_walk import ENTITIES_SYM, FIGHTER_A, FakeMemory, build_two_fighter_world


# --- parse_command ------------------------------------------------------------

@pytest.mark.parametrize("text, expected", [
    ("press A", {"op": "input", "inputs": {"A": True}, "frames": proto.DEFAULT_PRESS_FRAMES}),
    ("press Start 3", {"op": "input", "inputs": {"Start": True}, "frames": 3}),
    ("press A+Start 4", {"op": "input", "inputs": {"A": True, "Start": True}, "frames": 4}),
    ("hold StickX 1.0 30", {"op": "input", "inputs": {"StickX": 1.0}, "frames": 30}),
    ("hold StickX -1 5", {"op": "input", "inputs": {"StickX": -1.0}, "frames": 5}),
    ("hold StickX 255 30", {"op": "input", "inputs": {"StickX": 127 / 128}, "frames": 30}),
    ("hold StickY 0 2", {"op": "input", "inputs": {"StickY": 0.0}, "frames": 2}),
    ("hold TriggerLeft 0.5 2", {"op": "input", "inputs": {"TriggerLeft": 0.5}, "frames": 2}),
    ("hold L true 2", {"op": "input", "inputs": {"L": True}, "frames": 2}),
    ("stick 0 -1 5", {"op": "input", "inputs": {"StickX": 0.0, "StickY": -1.0}, "frames": 5}),
    ("wait 30", {"op": "wait", "frames": 30}),
    ("clear", {"op": "clear"}),
    ("save /tmp/x.sav", {"op": "save", "path": "/tmp/x.sav"}),
    ("load /tmp/x.sav", {"op": "load", "path": "/tmp/x.sav"}),
    ("shot /tmp/x.png", {"op": "shot", "path": "/tmp/x.png"}),
    ("save-when-wait /tmp/x.sav", {"op": "save_when_wait", "path": "/tmp/x.sav"}),
    ("watch 0x804D6714 4", {"op": "watch", "addr": 0x804D6714, "size": 4}),
    ("unwatch 0x804D6714", {"op": "unwatch", "addr": 0x804D6714}),
    ("unwatch all", {"op": "unwatch", "addr": None}),
    ("pause", {"op": "pause"}),
    ("resume", {"op": "resume"}),
    ("osd hello there", {"op": "osd", "text": "hello there"}),
    ("status", {"op": "status"}),
    ("stop", {"op": "stop"}),
    ("  PRESS  b  ", {"op": "input", "inputs": {"B": True}, "frames": proto.DEFAULT_PRESS_FRAMES}),
    ("press A 3 @1", {"op": "input", "inputs": {"A": True}, "frames": 3, "port": 1}),
    ("press A @3", {"op": "input", "inputs": {"A": True}, "frames": proto.DEFAULT_PRESS_FRAMES, "port": 3}),
    ("hold StickX 1 4 @2", {"op": "input", "inputs": {"StickX": 1.0}, "frames": 4, "port": 2}),
    ("stick 1 0 2 @1", {"op": "input", "inputs": {"StickX": 1.0, "StickY": 0.0}, "frames": 2, "port": 1}),
])
def test_parse_command(text, expected):
    got = proto.parse_command(text)
    if got["op"] == "input":
        assert got["inputs"].keys() == expected["inputs"].keys()
        for k, v in expected["inputs"].items():
            assert got["inputs"][k] == pytest.approx(v)
        assert got["frames"] == expected["frames"]
        assert got["port"] == expected.get("port", 0)
    else:
        assert got == expected


@pytest.mark.parametrize("text", [
    "", "press", "press Q", "press StickX", "press A 0", "press A -1", "press A 3 4",
    "hold StickX 1.5 3", "hold StickX 300 3", "hold StickX -2 3", "hold StickX 1.0", "hold A 1.0 3",
    "hold TriggerLeft -0.5 3", "stick 1 1", "wait", "wait x", "save rel/path.sav",
    "load", "osd", "clear now", "explode", "shot", "shot rel.png", "watch", "watch 0x804D6714", "watch 0x1000 4", "watch zz 4",
    "unwatch", "unwatch 0x804D6714 4", "press A @4", "press A @x", "wait 3 @1",
])
def test_parse_command_rejects(text):
    with pytest.raises(proto.CommandError):
        proto.parse_command(text)


def test_parse_commands_splits_on_semicolon():
    cmds = proto.parse_commands(["press A 3; wait 10", "status"])
    assert [c["op"] for c in cmds] == ["input", "wait", "status"]


# --- InputQueue ----------------------------------------------------------------

def test_queue_reissues_every_frame_then_releases_once():
    q = proto.InputQueue()
    q.enqueue({"A": True}, 2)
    (p1, f1), (p2, f2) = q.next_frame(), q.next_frame()
    assert p1 == p2 == 0
    assert f1["A"] is True and f2["A"] is True
    assert f1["StickX"] == 0.0 and f1["Start"] is False   # unspecified keys are neutral
    assert q.next_frame() == (0, proto.neutral_inputs())  # explicit release
    assert q.next_frame() is None      # nothing queued: leave the controller alone
    assert q.idle()


def test_queue_runs_steps_in_order_and_counts_remaining():
    q = proto.InputQueue()
    q.enqueue({"Start": True}, 1)
    q.enqueue_wait(2)
    q.enqueue({"StickX": 1.0}, 1, port=1)
    assert q.remaining_frames == 4
    assert q.next_frame()[1]["Start"] is True
    assert q.next_frame()[1]["Start"] is False      # wait frame (neutral)
    assert q.next_frame() == (0, proto.neutral_inputs())
    assert q.next_frame() == (1, {**proto.neutral_inputs(), "StickX": 1.0})
    assert q.remaining_frames == 0 and not q.idle()   # release still pending
    assert q.next_frame() == (1, proto.neutral_inputs())   # released on the step's own port
    assert q.idle()


def test_queue_clear_releases():
    q = proto.InputQueue()
    q.enqueue({"A": True}, 50, port=2)
    q.next_frame()
    q.clear()
    assert q.remaining_frames == 0
    assert q.next_frame() == (2, proto.neutral_inputs())
    assert q.next_frame() is None
    q.clear()                                # nothing was held: nothing to release
    assert q.idle() and q.next_frame() is None


def test_queue_rejects_bad_steps():
    q = proto.InputQueue()
    with pytest.raises(ValueError):
        q.enqueue({"A": True}, 0)
    with pytest.raises(ValueError):
        q.enqueue({"A": True}, 1, port=4)
    with pytest.raises(proto.CommandError):
        q.enqueue({"Q": True}, 1)


# --- CommandFile ---------------------------------------------------------------

def test_command_file_yields_each_seq_once(tmp_path: Path):
    path = tmp_path / "cmd.json"
    cf = proto.CommandFile(path)
    assert cf.poll() is None                     # no file yet
    assert proto.next_seq(path) == 0
    proto.write_json_atomic(path, {"seq": 0, "commands": [{"op": "wait", "frames": 1}]})
    assert cf.poll() == (0, [{"op": "wait", "frames": 1}])
    assert cf.poll() is None                     # unchanged mtime
    assert proto.next_seq(path) == 1
    # Rewrite with the same seq (drive.py never does this, but a stale copy could): ignored.
    proto.write_json_atomic(path, {"seq": 0, "commands": [{"op": "stop"}]})
    cf.last_mtime = None
    assert cf.poll() is None
    proto.write_json_atomic(path, {"seq": 5, "commands": []})
    cf.last_mtime = None
    assert cf.poll() == (5, [])
    assert not (tmp_path / "cmd.json.tmp").exists()


# --- apply_command -------------------------------------------------------------

class FakeHost:
    def __init__(self):
        self.calls = []

    def save_state(self, path): self.calls.append(("save", path))
    def load_state(self, path): self.calls.append(("load", path))
    def save_when_wait(self, path): self.calls.append(("save_when_wait", path))
    def pause(self): self.calls.append(("pause",))
    def resume(self): self.calls.append(("resume",))
    def osd(self, text): self.calls.append(("osd", text))
    def shot(self, path): self.calls.append(("shot", path))
    def watch(self, addr, size): self.calls.append(("watch", addr, size))
    def unwatch(self, addr): self.calls.append(("unwatch", addr))
    def stop(self): self.calls.append(("stop",))
    def write_status(self): self.calls.append(("status",))


def test_apply_command_dispatch():
    q, h = proto.InputQueue(), FakeHost()
    for text in ["press A 3", "wait 2", "save /s.sav", "load /s.sav", "save-when-wait /w.sav", "pause", "resume",
                 "osd hi", "shot /s.png", "watch 0x80479CF0 48", "unwatch all", "status", "stop",
                 "clear"]:
        proto.apply_command(proto.parse_command(text), q, h)
    assert h.calls == [("save", "/s.sav"), ("load", "/s.sav"), ("save_when_wait", "/w.sav"), ("pause",), ("resume",),
                       ("osd", "hi"), ("shot", "/s.png"), ("watch", 0x80479CF0, 48),
                       ("unwatch", None), ("status",), ("stop",)]
    assert q.remaining_frames == 0   # `clear` emptied the 5 queued frames
    with pytest.raises(ValueError):
        proto.apply_command({"op": "nope"}, q, h)


# --- status helpers -------------------------------------------------------------

class TypedFakeMemory(FakeMemory):
    def read_s32(self, addr):
        return struct.unpack(">i", struct.pack(">I", self.read_u32(addr)))[0]

    def read_s8(self, addr):
        return struct.unpack(">b", bytes([self.read_u8(addr)]))[0]

    def read_f32(self, addr):
        return struct.unpack(">f", struct.pack(">I", self.read_u32(addr)))[0]


def test_fighter_summary_reads_schema_offsets():
    m = TypedFakeMemory()
    m.write_u32(FIGHTER_A + 0x004, 1)                      # kind = Fox
    m.write_bytes(FIGHTER_A + 0x00C, b"\x02")              # player_id
    m.write_u32(FIGHTER_A + 0x010, proto.MS_WAIT)          # motion_id
    m.write_bytes(FIGHTER_A + 0x0B0, struct.pack(">fff", -12.5, 0.0001, 0.0))
    s = proto.fighter_summary(m, FIGHTER_A)
    assert s == {"base": "0x80453080", "kind": 1, "kind_name": "Fox", "player_id": 2,
                 "motion_id": 14, "pos": [pytest.approx(-12.5), pytest.approx(0.0001), 0.0]}
    assert proto.fighter_looks_valid(s)
    m.write_u32(FIGHTER_A + 0x004, 0x7FFFFFFF)
    assert not proto.fighter_looks_valid(proto.fighter_summary(m, FIGHTER_A))


def test_kind_names_match_decomp_enum():
    assert proto.FIGHTER_KIND_NAMES[0] == "Mario"
    assert proto.FIGHTER_KIND_NAMES[1] == "Fox"
    assert proto.FIGHTER_KIND_NAMES[2] == "Captain"
    assert len(proto.FIGHTER_KIND_NAMES) == 33


def test_pad_summary_layout():
    m = TypedFakeMemory()
    base = 0x804C1FAC + proto.PAD_STATUS_SIZE      # port 1
    m.write_u32(base, 0x00001000)                  # Start bit
    m.write_bytes(base + 0x18, struct.pack(">bb", 127, -80))
    assert proto.pad_summary(m, 0x804C1FAC, 1) == {"button": "0x00001000", "stick_x": 127, "stick_y": -80}


def test_fps_meter():
    f = proto.FpsMeter(now=100.0)
    assert f.sample(60, now=101.0) == pytest.approx(60.0)
    assert f.sample(90, now=101.5) == pytest.approx(60.0)
    assert f.sample(90, now=101.501) == pytest.approx(60.0)   # same-frame resample: keep the value
    assert f.sample(120, now=102.0) == pytest.approx(60.0)    # window still measured from 101.5


def test_build_status_shape():
    q = proto.InputQueue()
    q.enqueue({"A": True}, 3)
    s = proto.build_status(pid=1, frame=10, fps=59.9, seed=0x1234, seed_changed=True, last_seq=2,
                           queue=q, fighters=[], pad=None, errors=0, held=None, note="n")
    assert s["seed_hex"] == "0x00001234" and s["queue_frames"] == 3 and s["idle"] is False
    json.dumps(s)   # serialisable


# --- remote.Remote with fakes -------------------------------------------------------

def test_remote_frame_loop_with_fakes(tmp_path: Path, monkeypatch):
    import remote

    monkeypatch.setattr(remote, "REMOTE_DIR", tmp_path)
    monkeypatch.setattr(remote, "CMD_FILE", tmp_path / "cmd.json")
    monkeypatch.setattr(remote, "STATUS_FILE", tmp_path / "status.json")
    monkeypatch.setattr(remote, "LOG_FILE", tmp_path / "log.txt")

    m = TypedFakeMemory()
    for k, v in build_two_fighter_world().mem.items():
        m.mem[k] = v
    m.write_u32(remote.SEED_ADDR, 0xABCD)
    m.write_u32(FIGHTER_A + 0x004, 1)

    class Ctl:
        log = []

        def set_gc_buttons(self, port, inputs):
            self.log.append((port, inputs))

    class States:
        saved = []

        def save_to_file(self, p): self.saved.append(p)
        def load_from_file(self, p): pass

    r = remote.Remote(mem=m, ctl=Ctl(), states=States(), emu=None, osd=None)
    proto.write_json_atomic(tmp_path / "cmd.json",
                            {"seq": 0, "commands": [{"op": "input", "inputs": {"Start": True}, "frames": 1, "port": 1},
                                                    {"op": "save", "path": str(tmp_path / "s.sav")},
                                                    {"op": "bogus"}]})
    r.on_frame()
    assert Ctl.log == [(1, {**proto.neutral_inputs(), "Start": True})]
    assert r.held == {"port": 1, **proto.neutral_inputs(), "Start": True}
    assert States.saved == [str(tmp_path / "s.sav")]
    sidecar = json.loads((tmp_path / "s.sav.json").read_text())
    assert sidecar["seed"] == 0xABCD and sidecar["frame"] == 1
    assert [f["kind_name"] for f in sidecar["fighters"]] == ["Fox", "Mario"]
    status = json.loads((tmp_path / "status.json").read_text())     # written because a batch arrived
    assert status["last_seq"] == 0 and status["errors"] == 1        # the bogus op was logged
    r.watch(FIGHTER_A, 8)
    r.write_status()
    status = json.loads((tmp_path / "status.json").read_text())
    assert status["watch"] == {"0x80453080": "0000000000000001"}    # +0 zeros, +4 kind = Fox
    r.unwatch(None)
    r.write_status()
    assert json.loads((tmp_path / "status.json").read_text())["watch"] == {}
    assert "bogus" in (tmp_path / "log.txt").read_text()
    r.on_frame()                                                    # release frame
    assert Ctl.log[-1] == (1, proto.neutral_inputs())
    r.on_frame()
    assert len(Ctl.log) == 2                                        # idle: controller untouched
    assert ENTITIES_SYM == remote.ENTITIES_ADDR


# --- screenshots -------------------------------------------------------------------

def _png_chunks(data: bytes) -> list[tuple[bytes, bytes]]:
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    out, i = [], 8
    while i < len(data):
        (n,) = struct.unpack(">I", data[i:i + 4])
        kind, body, crc = data[i + 4:i + 8], data[i + 8:i + 8 + n], data[i + 8 + n:i + 12 + n]
        import zlib
        assert struct.unpack(">I", crc)[0] == zlib.crc32(kind + body) & 0xFFFFFFFF
        out.append((kind, body))
        i += 12 + n
    return out


def test_write_png_roundtrip(tmp_path: Path):
    import zlib
    w, h = 3, 2
    rgb = bytes(range(w * h * 3))
    p = tmp_path / "f.png"
    proto.write_png(p, w, h, rgb)
    chunks = _png_chunks(p.read_bytes())
    assert [k for k, _ in chunks] == [b"IHDR", b"IDAT", b"IEND"]
    assert struct.unpack(">IIBBBBB", chunks[0][1]) == (w, h, 8, 2, 0, 0, 0)   # 8-bit RGB
    raw = zlib.decompress(chunks[1][1])
    assert raw == b"\x00" + rgb[:9] + b"\x00" + rgb[9:]                   # filter byte per row
    assert not (tmp_path / "f.png.tmp").exists()
    with pytest.raises(ValueError):
        proto.write_png(p, w, h, rgb[:-1])


def test_remote_shot_is_a_one_shot_coroutine(tmp_path: Path, monkeypatch):
    import asyncio
    import zlib
    import remote

    monkeypatch.setattr(remote, "LOG_FILE", tmp_path / "log.txt")
    monkeypatch.setattr(remote, "STATUS_FILE", tmp_path / "status.json")
    monkeypatch.setattr(remote, "CMD_FILE", tmp_path / "cmd.json")

    class Events:
        frameadvance = "unset"
        frames = [(2, 1, bytes(6)), (2, 1, bytes(6)), (2, 1, bytes([9] * 6))]   # last one is fresh

        async def framedrawn(self):
            return self.frames.pop(0)

        def on_frameadvance(self, cb): self.frameadvance = cb

    class Ctl:
        def set_gc_buttons(self, port, inputs): pass

    ev = Events()
    r = remote.Remote(mem=TypedFakeMemory(), ctl=Ctl(), states=None, emu=None, osd=None, events=ev)
    assert r.on_frame() is None                    # nothing pending: plain callback
    r.shot(str(tmp_path / "a.png"))
    coro = r.on_frame()                            # Dolphin schedules a returned coroutine
    assert asyncio.iscoroutine(coro) and r.shot_path is None
    asyncio.run(coro)                              # stands in for the fork resuming it on framedrawn
    assert ev.frames == []                         # warm-up frames consumed, last one kept
    assert zlib.decompress(_png_chunks((tmp_path / "a.png").read_bytes())[1][1]) == b"\x00" + bytes([9] * 6)
    assert json.loads((tmp_path / "status.json").read_text())["note"].startswith("shot ")
    assert r.on_frame() is None
    r.stop()
    assert callable(ev.frameadvance) and ev.frameadvance is not r.on_frame   # None is rejected by the fork


def test_drive_watched_u32():
    import drive

    st = {"watch": {"0x804D6718": "00000259", "0x80479CF0": "error: boom"}}
    assert drive.watched_u32(st, 0x804D6718) == 0x259
    assert drive.watched_u32(st, 0x80479CF0) is None
    assert drive.watched_u32(st, 0x80000000) is None
    assert drive.watched_u32({}, 0x804D6718) is None


def test_launch_command_plugs_requested_ports():
    import drive

    cmd = drive.launch_command(Path("/s.py"), iso=Path("/g.iso"), speed=0, video="OGL", ports=2)
    assert cmd[:6] == [str(drive.DOLPHIN), "-e", "/g.iso", "--script", "/s.py", "-C"]
    assert "Dolphin.Core.EmulationSpeed=0" in cmd and "OGL" in cmd
    assert "Dolphin.Core.SIDevice0=6" in cmd and "Dolphin.Core.SIDevice1=6" in cmd
    assert "Dolphin.Core.SIDevice2=0" in cmd and "Dolphin.Core.SIDevice3=0" in cmd


def test_all_waiting():
    assert not proto.all_waiting([])
    assert proto.all_waiting([{"motion_id": 14}, {"motion_id": 14}])
    assert not proto.all_waiting([{"motion_id": 14}, {"motion_id": 0x142}])


def test_remote_save_when_wait_fires_once_when_all_fighters_wait(tmp_path: Path, monkeypatch):
    import remote

    monkeypatch.setattr(remote, "LOG_FILE", tmp_path / "log.txt")
    monkeypatch.setattr(remote, "STATUS_FILE", tmp_path / "status.json")
    monkeypatch.setattr(remote, "CMD_FILE", tmp_path / "cmd.json")

    m = TypedFakeMemory()
    for k, v in build_two_fighter_world().mem.items():
        m.mem[k] = v
    m.write_u32(FIGHTER_A + 0x010, 0x142)      # Entry
    m.write_u32(0x80455500 + 0x010, proto.MS_WAIT)

    class Ctl:
        def set_gc_buttons(self, port, inputs): pass

    class States:
        saved = []

        def save_to_file(self, p): self.saved.append(p)

    r = remote.Remote(mem=m, ctl=Ctl(), states=States(), emu=None, osd=None, events=None)
    r.save_when_wait(str(tmp_path / "w.sav"))
    r.on_frame()
    assert States.saved == []                  # fighter A still in Entry
    m.write_u32(FIGHTER_A + 0x010, proto.MS_WAIT)
    r.on_frame()
    r.on_frame()
    assert States.saved == [str(tmp_path / "w.sav")]   # exactly once
    sidecar = json.loads((tmp_path / "w.sav.json").read_text())
    assert sidecar["frame"] == 2 and [f["motion_id"] for f in sidecar["fighters"]] == [14, 14]
    assert "fired at frame 2" in json.loads((tmp_path / "status.json").read_text())["note"]
