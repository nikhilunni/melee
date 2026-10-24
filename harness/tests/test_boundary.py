"""make_boundary.py writes loadable scenarios; boundary_script.py's menu driver
reads the game's state and acts on it (fake memory, no Dolphin)."""
from __future__ import annotations

import struct
import tomllib
import inspect
from unittest.mock import create_autospec

import pytest

import boundary_script as bs
import make_boundary as mb


def test_boundary_command_keeps_item_rules_teams_and_extra_in_their_own_arguments(monkeypatch, tmp_path):
    monkeypatch.setattr(mb.data_root, "ROMS", tmp_path)
    monkeypatch.setattr(mb.data_root, "SCENARIOS", tmp_path)
    registry = tmp_path / "boundaries.toml"
    registry.write_text("boundary = []\n")
    monkeypatch.setattr(mb, "BOUNDARIES", registry)
    signature = inspect.signature(mb.make_savestate)
    capture = create_autospec(mb.make_savestate, side_effect=RuntimeError("before Dolphin"))
    monkeypatch.setattr(mb, "make_savestate", capture)
    with pytest.raises(RuntimeError, match="before Dolphin"):
        mb.main(["--stage", "FinalDestination", "--players", "Fox", "Marth", "--name", "args",
                 "--teams", "0", "1", "--friendly-fire", "--item-frequency", "very-high",
                 "--items", "Sword", "--no-containers"])
    arguments = signature.bind(*capture.call_args.args, **capture.call_args.kwargs)
    arguments.apply_defaults()
    assert arguments.arguments["teams"] == [0, 1]
    assert arguments.arguments["friendly_fire"] is True
    assert arguments.arguments["item_frequency"] == 4
    assert arguments.arguments["item_switch"] == [mb.SWITCHABLE["Sword"]]
    assert arguments.arguments["rules_item_mask"] & 0xF == 0
    assert arguments.arguments["extra"] is None


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


def test_css_presses_x_until_each_port_wears_its_costume():
    d, mem = driver(players=(15, 2))
    d.config["costumes"] = [2, 0]
    d.enter("css")
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_VS)
    css = 0x80E00000
    mem.write_u32(bs.CSS_DATA_PTR, css)
    for port, ckind in enumerate((15, 2)):
        mem.write_u8(css + bs.CSS_PLAYERS + port * bs.PLAYER_SIZE, ckind)
    d.player = 2                                     # both characters picked
    color = css + bs.CSS_PLAYERS + 3                 # port 0's PlayerInitData.color
    presses = []
    while d.phase != "start" and len(presses) < 10:
        frame = d.step()
        if frame:
            presses.append(frame)
            mem.write_u8(color, mem.read_u8(color) + 1)   # mnCharSel_CostumeChange: X steps up
    assert presses == [{0: {"X": True}}, {0: {"X": True}}]
    assert [p["color"] for p in d.css] == [2, 0]


def test_a_costume_boundary_entry_names_its_costumes():
    entry = tomllib.loads(mb.boundary_entry("b", "FinalDestination", ["Jigglypuff", "Fox"], 4, 7,
                                            [3, 0]))["boundary"][0]
    assert entry["costumes"] == [3, 0]
    plain = tomllib.loads(mb.boundary_entry("b", "FinalDestination", ["Jigglypuff", "Fox"], 4, 7,
                                            [0, 0]))["boundary"][0]
    assert "costumes" not in plain


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


def test_items_are_poked_into_save_data_as_the_item_switch_holds_them():
    d, mem = driver()
    d.config |= {"item_frequency": 4, "item_switch": [mb.SWITCHABLE["Sword"]]}
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_MENU)
    d.step()
    assert mem.read_s8(bs.ITEM_FREQUENCY) == 4
    assert (mem.read_u32(bs.ITEM_SWITCH), mem.read_u32(bs.ITEM_SWITCH + 4)) == (0, 1 << 27)
    # No item switch named: every entry on.
    d, mem = driver()
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_MENU)
    d.step()
    assert mem.read_u32(bs.ITEM_SWITCH + 4) == bs.ITEM_SWITCH_ALL


def test_the_menus_item_mask_keeps_containers_and_clears_what_the_switch_turns_off():
    sword = mb.menu_item_mask([mb.SWITCHABLE["Sword"]])
    assert sword & 0xF == 0xF                  # Capsule, Box, Barrel, Egg
    assert sword >> 12 & 1 and not sword >> 6 & 1    # Sword on, Bob-omb off
    assert sword >> 0x23 & 1                   # the unused bit keeps the menus' 1
    assert mb.menu_item_mask(None) == (1 << 64) - 1


def test_an_items_boundary_names_its_rules_in_both_scenarios_and_the_registry():
    items = (4, 0xFFFFFFF800001000)
    start = tomllib.loads(mb.start_scenario("b", "FinalDestination", ["Fox", "Marth"], 4, items=items))
    cold = tomllib.loads(mb.cold_scenario("b", "FinalDestination", ["Fox", "Marth"], 4, 7, [0, 0],
                                          items=items))
    entry = tomllib.loads(mb.boundary_entry("b", "FinalDestination", ["Fox", "Marth"], 4, 7,
                                            items=items))["boundary"][0]
    for table in (start, cold, entry):
        assert (table["item_frequency"], int(table["item_mask"], 16)) == items
    plain = tomllib.loads(mb.cold_scenario("b", "FinalDestination", ["Fox", "Marth"], 4, 7, [0, 0]))
    assert "item_frequency" not in plain
    with pytest.raises(SystemExit, match="needs its own --name"):
        mb.main(["--stage", "FinalDestination", "--players", "Fox", "Marth",
                 "--item-frequency", "high", "--no-containers"])
    with pytest.raises(SystemExit, match="need --item-frequency"):
        mb.main(["--stage", "FinalDestination", "--players", "Fox", "Marth", "--items", "Sword"])


def test_the_item_mask_is_written_into_the_start_data_when_the_match_is_created():
    d, mem, watched = creation_driver(seed=None, rules_item_mask=0xFFF0)
    at = bs.VS_START_RULES + bs.RULES_ITEM_MASK
    mem.write_u32(at, 0xFFFFFFFF)
    mem.write_u32(at + 4, 0xFFFFFFFF)
    d.step()
    d.on_memory(True, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)
    assert (mem.read_u32(at), mem.read_u32(at + 4)) == (0, 0xFFF0)
    assert d.creation_seed() == {"rules_item_mask_replaced": (1 << 64) - 1}


def test_every_registered_boundary_names_known_characters_and_stages():
    registry = tomllib.loads(mb.BOUNDARIES.read_text())["boundary"]
    for b in registry:
        assert b["stage"] in mb.STAGES
        assert all(p in mb.CHARACTERS for p in b["players"])


def test_a_slippi_layout_seats_players_on_their_ports_with_codes_and_timer():
    name = "start_bf_slippi8_p24_marth1_peach1_4"
    codes = ["ucf-0.8", "neutral-spawn"]
    start = tomllib.loads(mb.start_scenario(name, "Battlefield", ["Marth", "Peach"], 4, codes, [1, 3]))
    assert start["gecko"] == codes and start["gate"] == f"{name}_cold"
    assert [(f["slot"], f["controller_fix"]) for f in start["fighters"]] == [(1, "ucf-0.8"), (3, "ucf-0.8")]
    cold = tomllib.loads(mb.cold_scenario(name, "Battlefield", ["Marth", "Peach"], 4, 7, [1, 1],
                                          "neutral-2020", codes, [1, 3], 8))
    assert (cold["spawn"], cold["time_limit"]) == ("neutral-2020", 480)
    assert [(f["slot"], f["controller_fix"]) for f in cold["fighters"]] == [(1, "ucf-0.8"), (3, "ucf-0.8")]
    # The first ports need no cold twin to gate, and retail has no codes.
    plain = tomllib.loads(mb.start_scenario("b", "Battlefield", ["Marth", "Fox"], 4))
    assert "gate" not in plain and "gecko" not in plain


@pytest.mark.parametrize("codes,flags", [
    (["ucf-0.8", "ps-preload"], {"stadium_preload"}),
    (["ucf-0.8", "ps-preload", "ps-frozen"], {"stadium_preload", "stadium_frozen"}),
    (["ucf-0.8"], set()),
    (None, set()),
])
def test_stadium_codes_set_their_flags_in_the_start_scene_and_its_cold_twin(codes, flags):
    start = tomllib.loads(mb.start_scenario("b", "PokemonStadium", ["Falco", "Marth"], 4, codes, [0, 3]))
    cold = tomllib.loads(mb.cold_scenario("b", "PokemonStadium", ["Falco", "Marth"], 4, 7, [3, 4],
                                          "retail", codes, [0, 3], 8))
    for scenario in (start, cold):
        assert {key for key in scenario if key.startswith("stadium_")} == flags
        assert all(scenario[flag] is True for flag in flags)
        # Top-level keys: none may fall under a [[fighters]] table.
        assert not any(key.startswith("stadium_") for f in scenario["fighters"] for key in f)
    assert "gecko" not in cold


def test_frozen_stages_sets_its_flag_in_the_start_scene_and_its_cold_twin():
    codes = ["ucf-0.8", "frozen-stages"]
    start = tomllib.loads(mb.start_scenario("b", "YoshisStory", ["Falco", "Marth"], 4, codes, [0, 3]))
    cold = tomllib.loads(mb.cold_scenario("b", "YoshisStory", ["Falco", "Marth"], 4, 7, [3, 4],
                                          "retail", codes, [0, 3], 8))
    assert start["frozen_stages"] is True and cold["frozen_stages"] is True
    assert "gecko" not in cold


@pytest.mark.parametrize("code, flag, other", [("widescreen", "widescreen", "widescreen_92_63"),
                                               ("widescreen-92-63", "widescreen_92_63", "widescreen")])
def test_widescreen_sets_its_aspects_flag_in_the_start_scene_and_its_cold_twin(code, flag, other):
    codes = [code]
    start = tomllib.loads(mb.start_scenario("b", "PokemonStadium", ["Fox", "Marth"], 4, codes, [0, 1]))
    cold = tomllib.loads(mb.cold_scenario("b", "PokemonStadium", ["Fox", "Marth"], 4, 7, [0, 0],
                                          "retail", codes, [0, 1], None))
    assert start[flag] is True and cold[flag] is True
    assert other not in start and other not in cold
    assert start["gecko"] == [code] and "gecko" not in cold


def creation_driver(seed=0x1234_5678, **named):
    """A driver on the stage screen whose config names a Game Start seed."""
    mem = Memory()
    mem.add_memcheck = lambda addr: watched.append(addr)
    mem.remove_memcheck = lambda addr: watched.remove(addr)
    watched: list[int] = []
    config = {"savestate": "/x.sav", "stkind": 0x20, "players": [9, 9], "stocks": 4,
              **({"game_start_seed": seed} if seed is not None else {}), **named}
    d = bs.BoundaryDriver(config, mem, save=lambda path: None)
    d.phase = "start"
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_VS)
    mem.write_u8(bs.SCENE_MACHINE + 3, bs.VS_STATE_SSS)
    mem.write_u32(bs.SSS_DATA_PTR, 0x80BD0000)
    mem.write_u32(bs.SEED_ADDR, 99)
    return d, mem, watched


def test_the_game_start_seed_is_written_when_the_match_is_created():
    d, mem, watched = creation_driver()
    d.step()
    assert d.phase == "match" and watched == [bs.MATCH_START_TEST]
    # Other stores to the watched word, and reads, leave the seed alone.
    d.on_memory(True, bs.MATCH_START_TEST, 0x801A46F4)
    d.on_memory(False, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)
    assert mem.read_u32(bs.SEED_ADDR) == 99 and d.seed_replaced is None
    # fn_8016E730's gm_801A4B08 call: the seed creation draws from.
    d.on_memory(True, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)
    assert mem.read_u32(bs.SEED_ADDR) == 0x1234_5678 and d.seed_replaced == 99
    # Only once (a later scene would pass the same call), and the breakpoint
    # is removed on the next frame, outside its own callback.
    mem.write_u32(bs.SEED_ADDR, 7)
    d.on_memory(True, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)
    assert mem.read_u32(bs.SEED_ADDR) == 7 and watched == [bs.MATCH_START_TEST]
    d.step()
    assert watched == []
    assert d.creation_seed() == {"game_start_seed": 0x1234_5678, "seed_replaced": 99}


def test_a_match_created_without_the_seed_written_is_not_saved(monkeypatch):
    d, mem, watched = creation_driver()
    d.step()
    fighters = [{"motion_id": bs.MS_ENTRY}, {"motion_id": bs.MS_ENTRY}]
    monkeypatch.setattr(d, "read_fighters", lambda: fighters)
    with pytest.raises(bs.BoundaryError, match="Game Start seed was not written"):
        d.step()
    assert not d.done


def start_player(port: int) -> int:
    return bs.VS_START_PLAYERS + port * bs.START_PLAYER_SIZE


def test_spawn_points_are_written_into_the_start_data_when_the_match_is_created():
    # Ice Climbers and Samus on ports 1 and 4, at the stage's markers 0 and 1.
    d, mem, watched = creation_driver(seed=None, players=[14, 16], ports=[0, 3], spawn_points=[0, 1])
    for port, kind in ((0, 14), (3, 16)):
        mem.write_u8(start_player(port), kind)
        mem.write_u8(start_player(port) + bs.START_PLAYER_SPAWN, 0xFF)   # the menus' -1
    d.step()
    assert watched == [bs.MATCH_START_TEST]
    d.on_memory(True, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)
    assert [mem.read_s8(start_player(port) + bs.START_PLAYER_SPAWN) for port in (0, 3)] == [0, 1]
    # The seed is the menus' own: no Game Start seed was named.
    assert mem.read_u32(bs.SEED_ADDR) == 99
    assert d.creation_seed() == {"spawn_points": [0, 1], "spawn_points_replaced": [-1, -1]}
    d.step()
    assert watched == []


def test_spawn_points_are_refused_when_the_start_data_holds_another_character():
    d, mem, watched = creation_driver(players=[14, 16], ports=[0, 3], spawn_points=[0, 1])
    mem.write_u8(start_player(0), 14)
    mem.write_u8(start_player(3), 9)
    d.step()
    with pytest.raises(bs.BoundaryError, match="player 3 is character 9, not 16"):
        d.on_memory(True, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)


def test_a_match_created_without_the_spawn_points_written_is_not_saved(monkeypatch):
    d, mem, watched = creation_driver(seed=None, spawn_points=[0, 1])
    d.step()
    fighters = [{"motion_id": bs.MS_ENTRY}, {"motion_id": bs.MS_ENTRY}]
    monkeypatch.setattr(d, "read_fighters", lambda: fighters)
    with pytest.raises(bs.BoundaryError, match="the spawn points were not written"):
        d.step()
    assert not d.done


def test_a_spawn_point_boundary_names_them_in_its_cold_twin_and_is_never_registered():
    cold = tomllib.loads(mb.cold_scenario("b", "PokemonStadium", ["IceClimbers", "Samus"], 4, 7, [1, 4],
                                          "retail", ["ucf-0.73"], [0, 3], 8, None, [0, 1]))
    assert [(f["slot"], f["spawn_point"]) for f in cold["fighters"]] == [(0, 0), (3, 1)]
    plain = tomllib.loads(mb.cold_scenario("b", "PokemonStadium", ["IceClimbers", "Samus"], 4, 7, [1, 4]))
    assert all("spawn_point" not in f for f in plain["fighters"])
    with pytest.raises(SystemExit, match="--no-register"):
        mb.main(["--stage", "PokemonStadium", "--players", "IceClimbers", "Samus", "--spawn-points", "0", "1"])
    with pytest.raises(SystemExit, match="one spawn point per player"):
        mb.main(["--stage", "PokemonStadium", "--players", "IceClimbers", "Samus", "--spawn-points", "0",
                 "--no-register"])


def test_a_boundary_without_a_game_start_seed_sets_no_breakpoint():
    d, mem = driver()
    assert d.game_start_seed is None and d.creation_seed() == {}
    d.on_memory(True, bs.MATCH_START_TEST, bs.MATCH_START_TEST_FN)
    assert mem.read_u32(bs.SEED_ADDR) == 0


def test_a_replays_boundary_names_its_game_start_seed_and_is_never_registered():
    start = tomllib.loads(mb.start_scenario("b", "FinalDestination", ["Marth", "Marth"], 4,
                                            ["ucf-0.73"], [0, 1], 2534789673))
    cold = tomllib.loads(mb.cold_scenario("b", "FinalDestination", ["Marth", "Marth"], 4, 2462485393,
                                          [4, 1], "neutral-2020", ["ucf-0.73"], [0, 1], 8, 2534789673))
    assert start["game_start_seed"] == cold["game_start_seed"] == 2534789673
    assert cold["seed"] == 2462485393
    assert "game_start_seed" not in tomllib.loads(mb.start_scenario("b", "FinalDestination", ["Marth", "Fox"], 4))
    with pytest.raises(SystemExit, match="--no-register"):
        mb.main(["--stage", "FinalDestination", "--players", "Marth", "Marth", "--game-start-seed", "5"])
    with pytest.raises(SystemExit, match="--name"):
        mb.main(["--stage", "FinalDestination", "--players", "Marth", "Marth", "--game-start-seed", "5",
                 "--no-register"])


def test_the_menu_driver_uses_the_players_ports():
    d, mem = driver(players=(9, 12))
    d.ports = [1, 3]
    assert d.step() == {1: {"Start": True}}          # the first player's pad drives the menus
    d.config["costumes"] = [1, 0]
    d.enter("css")
    mem.write_u8(bs.SCENE_MACHINE, bs.GM_VS)
    css = 0x80E00000
    mem.write_u32(bs.CSS_DATA_PTR, css)
    for port, ckind in zip(d.ports, (9, 12)):
        mem.write_u8(css + bs.CSS_PLAYERS + port * bs.PLAYER_SIZE, ckind)
    d.player = 2                                     # both characters picked
    assert d.step() == {1: {"X": True}}              # port 2's costume, not port 1's
    mem.write_u8(css + bs.CSS_PLAYERS + 1 * bs.PLAYER_SIZE + 3, 1)
    while d.phase != "start":
        d.step()
    assert [p["ckind"] for p in d.css] == [9, 12]


def test_si_flags_plug_the_named_ports(monkeypatch):
    import dolphin_config
    monkeypatch.delenv(dolphin_config.SI_PORTS_ENV, raising=False)
    assert dolphin_config.si_flags(2)[1::2] == [f"Dolphin.Core.SIDevice{i}={6 if i < 2 else 0}"
                                                for i in range(4)]
    monkeypatch.setenv(dolphin_config.SI_PORTS_ENV, "1,3")
    assert dolphin_config.si_flags(2)[1::2] == [f"Dolphin.Core.SIDevice{i}={6 if i in (1, 3) else 0}"
                                                for i in range(4)]
