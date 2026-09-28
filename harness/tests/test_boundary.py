"""make_boundary.py writes loadable scenarios; boundary_script.py's menu driver
reads the game's state and acts on it (fake memory, no Dolphin)."""
from __future__ import annotations

import struct
import tomllib

import pytest

import boundary_script as bs
import make_boundary as mb


class Memory:
    """Big-endian emulated RAM backed by a dict of bytes."""

    def __init__(self) -> None:
        self.bytes: dict[int, int] = {}

    def _read(self, addr: int, n: int) -> bytes:
        return bytes(self.bytes.get(addr + i, 0) for i in range(n))

    def _write(self, addr: int, data: bytes) -> None:
        for i, b in enumerate(data):
            self.bytes[addr + i] = b

    def read_u8(self, a): return self._read(a, 1)[0]
    def read_s8(self, a): return struct.unpack(">b", self._read(a, 1))[0]
    def read_u16(self, a): return struct.unpack(">H", self._read(a, 2))[0]
    def read_u32(self, a): return struct.unpack(">I", self._read(a, 4))[0]
    def read_f32(self, a): return struct.unpack(">f", self._read(a, 4))[0]
    def write_u8(self, a, v): self._write(a, struct.pack(">B", v & 0xFF))
    def write_u16(self, a, v): self._write(a, struct.pack(">H", v))
    def write_u32(self, a, v): self._write(a, struct.pack(">I", v))
    def write_f32(self, a, v): self._write(a, struct.pack(">f", v))


def driver(players=(2, 9), stocks=4) -> tuple[bs.BoundaryDriver, Memory]:
    mem = Memory()
    config = {"savestate": "/x.sav", "stkind": 0x1F, "players": list(players), "stocks": stocks}
    return bs.BoundaryDriver(config, mem, save=lambda path: None), mem


def run_until(d: bs.BoundaryDriver, pred, limit: int = 2000) -> list[dict]:
    frames = []
    while not pred() and len(frames) < limit:
        frames.append(d.step())
    return frames


def test_boot_pulses_start_until_the_main_menu_then_pokes_the_rules():
    d, mem = driver(stocks=3)
    frames = [d.step() for _ in range(90)]
    assert frames[0] == {0: {"Start": True}} and frames[bs.PRESS_FRAMES] == {}
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_MENU)
    d.step()
    assert d.phase == "menu"
    assert mem.read_u16(bs.CHARACTER_UNLOCKS) == bs.CHARACTER_UNLOCK_BITS
    assert mem.read_u8(bs.GAME_RULES + bs.RULES_STOCKS) == 3
    assert mem.read_s8(bs.ITEM_FREQUENCY) == -1


def test_menu_moves_to_vs_melee_by_the_hovered_item():
    d, mem = driver()
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_MENU)
    d.step()
    pressed = [f for f in run_until(d, lambda: False, bs.SETTLE_FRAMES + 2) if f]
    assert pressed == [{0: {"Down": True}}]        # 1P hovered, VS is below
    mem.write_u16(bs.MENU_FLOW + 2, bs.SEL_MAIN_VS)
    pressed = [f for f in run_until(d, lambda: False, bs.PULSE_PERIOD) if f]
    assert pressed == [{0: {"A": True}}]


def test_css_hand_enters_the_portrait_band_then_aims_its_token():
    d, mem = driver(players=(2,))
    d.enter("css")
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_VS)
    css, cursor, token = 0x80E00000, 0x80E01000, 0x80E02000
    mem.write_u32(bs.CSS_DATA_PTR, css)
    mem.write_u32(bs.CSS_CURSORS, cursor)
    mem.write_u32(bs.CSS_TOKENS, token)
    icon = bs.CSS_ICONS + 3 * bs.CSS_ICON_SIZE      # Fox's icon, somewhere in the table
    mem.write_u8(icon + 1, 2)
    mem.write_u8(icon + 2, 2)
    for off, v in ((0xC, 10.0), (0x10, 20.0), (0x14, 8.0), (0x18, 2.0)):
        mem.write_f32(icon + off, v)
    run_until(d, lambda: mem.read_f32(cursor + 0x10) != 0.0)
    assert (mem.read_f32(cursor + 0xC), mem.read_f32(cursor + 0x10)) == pytest.approx((15.0 - 2.7, 5.0 + 2.0))
    # The game hands over the token (x5 = 1) one frame behind the hand.
    mem.write_u8(cursor + 5, bs.CURSOR_HOLDS_TOKEN)
    mem.write_f32(token + 0x8, 15.0)
    mem.write_f32(token + 0xC, 5.0 + 2.0 - 2.0)
    frames = run_until(d, lambda: False, 60)
    first = next(f for f in frames if f)
    assert first == {0: {"A": True}}
    assert bs.inside((10.0, 20.0, 8.0, 2.0), mem.read_f32(cursor + 0xC) + 2.7, mem.read_f32(cursor + 0x10) - 2.0)
    mem.write_u8(css + bs.CSS_PLAYERS, 2)            # the CSS records Fox for port 0
    run_until(d, lambda: d.phase == "start")
    assert d.css == [{"ckind": 2, "slot_type": 0, "stocks": 0, "color": 0}]


def test_scenarios_and_registry_entry_parse():
    name = mb.default_name("Battlefield", ["Marth", "Fox"], 4)
    assert name == "start_bf_marth_fox4"
    start = tomllib.loads(mb.start_scenario(name, "Battlefield", ["Marth", "Fox"], 4))
    assert start["savestate"] == f"harness/roms/{name}.sav" and start["frames"] == 600
    assert [f["kind"] for f in start["fighters"]] == ["Marth", "Fox"]
    cold = tomllib.loads(mb.cold_scenario(name, "Battlefield", ["Marth", "Fox"], 4, 1234, [0, 1]))
    assert (cold["name"], cold["expected"], cold["seed"]) == (f"{name}_cold", name, 1234)
    assert [(f["costume"], f["stocks"]) for f in cold["fighters"]] == [(0, 4), (1, 4)]
    entry = tomllib.loads(mb.boundary_entry(name, "Battlefield", ["Marth", "Fox"], 4, 1234))["boundary"][0]
    assert entry == {"name": name, "cold": f"{name}_cold", "savestate": f"harness/roms/{name}.sav",
                     "stage": "Battlefield", "players": ["Marth", "Fox"], "stocks": 4, "seed": 1234}


def test_every_registered_boundary_names_known_characters_and_stages():
    registry = tomllib.loads(mb.BOUNDARIES.read_text())["boundary"]
    for b in registry:
        assert b["stage"] in mb.STAGES
        assert all(p in mb.CHARACTERS for p in b["players"])
