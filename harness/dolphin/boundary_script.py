"""Create a match-start boundary from a cold boot: runs INSIDE the scripting fork.

Launched by harness/make_boundary.py (headless, unlimited speed). The config is
a JSON file named by MELEE_BOUNDARY_CONFIG:

    {"savestate": "/abs/roms/<name>.sav", "stkind": 32, "stocks": 4,
     "players": [2, 9], "time_limit_minutes": 0, "transform": [0],
     "costumes": [3, 0]}

`players` are CSS character kinds (ft/forward.h CharacterKind), one per port.
`costumes` (optional) gives each port's costume: after the characters are
picked, that port presses X until the CSS door's costume matches
(mnCharSel_CostumeChange, mncharsel.c:2186: X steps to the next costume).
`transform` (optional) lists ports that hold A from the stage screen until
the fighters exist: fn_8016D8AC (gm_16AE.c:1573-1583) reads that port's
HSD_PadCopyStatus when the match loads and swaps a human Zelda for Sheik
(or back), as a player holding A does on retail.

Every frame `BoundaryDriver.step` reads the game's own scene state and decides
the inputs, so no step depends on timing or screenshots:

  boot     pulse Start until the scene machine reaches the main menu (the
           opening movie and the title both exit on a Start edge);
  menu     apply the RAM-only save-data pokes every boundary uses (characters
           and Battlefield/FD unlocked, stock rules, items off), then walk the
           main menu to VS. Mode > Melee by D-pad, checking the hovered item;
  css      per port, place that port's hand over the character's icon (the
           icon bounds come from the CSS icon table in RAM) and press A with
           that port's controller: the retail CSS code picks the character
           exactly as for a person;
  sss      set the stage screen's forced stage (SSSData.force_stage_id, the
           field retail's ordered/random stage rules use), which starts the
           match on that stage on the next frame;
  match    save the state at the first frame where every fighter exists and
           is in Entry (a stricter `save-when-fighters`), write the sidecar
           and `<savestate>.done`, then park. A transforming character's
           second form (Player_80031AD0's second Fighter_Create) exists too,
           asleep (ftCo_800BFD04, ftCo_MS_Sleep); it is not a player.

Failures write `<savestate>.err`. Nothing outside the savestate path is written;
the memory card belongs to the private Dolphin user folder the caller made.
"""
from __future__ import annotations

import json
import os
import sys
import time
import traceback
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE.parent))  # harness/ -> symbols
sys.path.insert(0, str(HERE))         # harness/dolphin -> remote_proto, walk
import remote_proto as proto  # noqa: E402
import symbols  # noqa: E402
import walk  # noqa: E402

try:
    from dolphin import controller, event, memory, savestate  # type: ignore
except ImportError:  # importable outside Dolphin for tests
    controller = event = memory = savestate = None

# gm_1A3F.c:47 state_machine (routingInfo): curr_mode +0, curr_state_id +3.
SCENE_MACHINE = 0x80479D30
GM_MENU, GM_VS = 1, 2                  # gm/forward.h GameModeKind
VS_STATE_CSS, VS_STATE_SSS = 0, 1      # gmvsmode.h gmVsMode_StateId
# mnmain.h MenuFlow mn_804A04F0: cur_menu +0, hovered_selection u16 +2.
MENU_FLOW = symbols.addr("mn_804A04F0")
MENU_MAIN, MENU_VS = 0, 2              # mn/forward.h MenuKind
SEL_MAIN_VS, SEL_VS_MELEE = 1, 0       # MainMenuSelection, VsMenuSelection

# Save data (gmMainLib_804D3EE0 -> 0x8045A6C0), the pokes of docs/DOLPHIN_RUN.md
# "Unlocks and rules are RAM-only pokes". Only the character bits reach match
# state (music selection); the stage bits only unlock the menu tiles.
CHARACTER_UNLOCKS, CHARACTER_UNLOCK_BITS = 0x8045BF28, 0xFFFF
STAGE_UNLOCKS, STAGE_UNLOCK_BITS = 0x8045BF2A, 0x00C0   # Battlefield, Final Destination
GAME_RULES = 0x8045BF10                # gm/types.h GameRules
RULES_MODE, RULES_STOCKS, RULES_STOCK_TIME = 0x2, 0x4, 0x8
MODE_STOCK = 1
ITEM_FREQUENCY = 0x8045C370            # gmm_x1CB0.item_freq (0x1CB0 into save data)
ITEMS_OFF = 0xFF                       # (s8) -1

# mncharsel.c: CSSData* mnCharSel_804D6CB0; cursors mnCharSel_804A0BC0[4]
# (CSSCursorData: x5 state, xC/x10 position); held tokens mnCharSel_804A0BD0[4]
# (CSSCharModel: x8/xC position); icons[25] at 0x803F0B24, 0x1C bytes each
# (char_kind +1, state +2, bounds l/r/u/d at +0xC/+0x10/+0x14/+0x18).
CSS_DATA_PTR = symbols.addr("mnCharSel_804D6CB0")
CSS_CURSORS = symbols.addr("mnCharSel_804A0BC0")
CSS_TOKENS = symbols.addr("mnCharSel_804A0BD0")
CSS_ICONS, CSS_ICON_SIZE, CSS_ICON_COUNT = 0x803F0B24, 0x1C, 25
CURSOR_HOLDS_TOKEN = 1
TOKEN_OFFSET = (2.7, -2.0)   # held token minus hand position
# CSSData.vs (+8) .start (+8): rules at +0x10 (0x60 bytes), players at +0x70
# (PlayerInitData, 0x24 each: ckind +0, slot_type +1, stocks +2, color +3).
CSS_PLAYERS, PLAYER_SIZE = 0x70, 0x24
PKIND_HUMAN = 0

# mnstagesel.c: SSSData* sss_data (.sbss 0x804D6C90; a .bss symbol shares the
# name), force_stage_id s8 at +3 (mnStageSel_Scene_OnFrame:766).
SSS_DATA_PTR = 0x804D6C90
SSS_FORCE_STAGE = 0x3

SEED_ADDR = symbols.addr("seed")
ENTITIES_ADDR = symbols.addr("HSD_GObj_Entities")
MS_ENTRY = 322   # ftCo_MS_Entry, every fighter's first match state
# pl/player.c:58 ftMapping_list: CKIND_POPONANA creates a second fighter
# (Nana, player_entity[1]) for its port (Player_80031AD0).
CKIND_ICE_CLIMBERS = 0x0E


def fighter_count(players: list[int]) -> int:
    """Fighter GObjs a match creates for these CSS kinds (Nana is her own)."""
    return sum(2 if kind == CKIND_ICE_CLIMBERS else 1 for kind in players)
MS_SLEEP = 11    # ftCo_MS_Sleep: a transforming character's inactive form

PULSE_PERIOD = 40   # menus ignore input for ~40 frames after a transition
PRESS_FRAMES = 3
SETTLE_FRAMES = 60  # after the CSS appears, before its hands are moved
TIMEOUT_FRAMES = 60 * 60 * 5


class BoundaryError(RuntimeError):
    pass


def icon_center(bounds: tuple[float, float, float, float]) -> tuple[float, float]:
    left, right, up, down = bounds
    return (left + right) / 2, (up + down) / 2


def inside(bounds: tuple[float, float, float, float], x: float, y: float) -> bool:
    """The CSS drop test (mncharsel.c:2647): strict bounds, y grows upward."""
    left, right, up, down = bounds
    return left < x < right and down < y < up


class BoundaryDriver:
    """One frame at a time: `step()` returns {port: inputs} to hold this frame."""

    def __init__(self, config: dict, mem, save) -> None:
        self.config = config
        self.mem = mem
        self.save = save          # save(path) -> None, writes the savestate
        self.frame = 0
        self.phase = "boot"
        self.phase_frame = 0      # frame the current phase started
        self.player = 0           # CSS: port being selected
        self.attempts = 0
        self.wait_until = 0
        self.css: list[dict] = []
        self.pending_press: tuple[int, int] | None = None   # (port, first frame)
        self.done = False

    # --- memory ----------------------------------------------------------
    def scene(self) -> tuple[int, int]:
        return self.mem.read_u8(SCENE_MACHINE), self.mem.read_u8(SCENE_MACHINE + 3)

    def ptr(self, addr: int) -> int:
        value = self.mem.read_u32(addr)
        return value if 0x80000000 <= value < 0x81800000 else 0

    def icon(self, ckind: int) -> tuple[float, float, float, float]:
        for i in range(CSS_ICON_COUNT):
            base = CSS_ICONS + i * CSS_ICON_SIZE
            if self.mem.read_u8(base + 1) == ckind:
                if self.mem.read_u8(base + 2) == 0:
                    raise BoundaryError(f"CSS icon for character kind {ckind} is locked")
                return tuple(self.mem.read_f32(base + off) for off in (0xC, 0x10, 0x14, 0x18))
        raise BoundaryError(f"no CSS icon has character kind {ckind}")

    def css_player(self, port: int) -> dict:
        base = self.ptr(CSS_DATA_PTR) + CSS_PLAYERS + port * PLAYER_SIZE
        return {"ckind": self.mem.read_s8(base), "slot_type": self.mem.read_u8(base + 1),
                "stocks": self.mem.read_s8(base + 2), "color": self.mem.read_u8(base + 3)}

    # --- phases ----------------------------------------------------------
    def enter(self, phase: str) -> None:
        self.phase, self.phase_frame = phase, self.frame

    def pulse(self, button: str, port: int = 0) -> dict:
        """Press `button` for PRESS_FRAMES every PULSE_PERIOD frames of this phase."""
        if (self.frame - self.phase_frame) % PULSE_PERIOD < PRESS_FRAMES:
            return {port: {button: True}}
        return {}

    def step(self) -> dict:
        self.frame += 1
        if self.frame > TIMEOUT_FRAMES:
            raise BoundaryError(f"no boundary after {TIMEOUT_FRAMES} frames (phase {self.phase})")
        inputs = self.phase_inputs(*self.scene())
        for port, pad in self.press_due().items():
            inputs.setdefault(port, {}).update(pad)
        if self.phase == "match":
            for port in self.config.get("transform", []):
                inputs.setdefault(port, {})["A"] = True
        return inputs

    def phase_inputs(self, mode: int, state: int) -> dict:
        if self.phase == "boot":
            if mode == GM_MENU:
                self.poke_save_data()
                self.enter("menu")
                return {}
            return self.pulse("Start")
        if self.phase == "menu":
            if mode == GM_VS:
                self.enter("css")
                return {}
            return self.navigate_menu()
        if self.phase == "css":
            return self.select_characters(mode, state)
        if self.phase == "start":
            if mode == GM_VS and state == VS_STATE_SSS and self.ptr(SSS_DATA_PTR):
                self.mem.write_u8(self.ptr(SSS_DATA_PTR) + SSS_FORCE_STAGE, self.config["stkind"])
                self.enter("match")
                return {}
            return self.pulse("Start")
        if self.phase == "match":
            # Every fighter created and in Entry: a fighter still inside
            # Fighter_Create reads as motion 0 at the origin, before its CPU
            # setup draws, and would leave the seed short of post-creation.
            # A transforming character's other form sleeps and makes no entry.
            fighters = [f for f in self.read_fighters() if f["motion_id"] != MS_SLEEP]
            if len(fighters) == fighter_count(self.config["players"]) and all(
                    f["motion_id"] == MS_ENTRY for f in fighters):
                self.save(self.config["savestate"])
                self.done = True
                self.enter("saved")
            return {}
        return {}

    def poke_save_data(self) -> None:
        mem = self.mem
        mem.write_u16(CHARACTER_UNLOCKS, mem.read_u16(CHARACTER_UNLOCKS) | CHARACTER_UNLOCK_BITS)
        mem.write_u16(STAGE_UNLOCKS, mem.read_u16(STAGE_UNLOCKS) | STAGE_UNLOCK_BITS)
        mem.write_u8(GAME_RULES + RULES_MODE, MODE_STOCK)
        mem.write_u8(GAME_RULES + RULES_STOCKS, self.config["stocks"])
        mem.write_u8(GAME_RULES + RULES_STOCK_TIME, self.config.get("time_limit_minutes", 0))
        mem.write_u8(ITEM_FREQUENCY, ITEMS_OFF)

    def navigate_menu(self) -> dict:
        if self.frame < self.wait_until or self.frame - self.phase_frame < SETTLE_FRAMES:
            return {}
        menu = self.mem.read_u8(MENU_FLOW)
        hovered = self.mem.read_u16(MENU_FLOW + 2)
        target = {MENU_MAIN: SEL_MAIN_VS, MENU_VS: SEL_VS_MELEE}.get(menu)
        self.wait_until = self.frame + PULSE_PERIOD
        if target is None:   # a submenu this path never opens: back out
            return {0: {"B": True}}
        if hovered == target:
            return {0: {"A": True}}
        return {0: {"Down" if hovered < target else "Up": True}}

    def select_characters(self, mode: int, state: int) -> dict:
        if (mode, state) != (GM_VS, VS_STATE_CSS):
            raise BoundaryError(f"left the CSS early (mode {mode}, state {state})")
        players = self.config["players"]
        if self.player == len(players) and not self.costumes_done():
            return self.change_costume()
        if self.player == len(players):
            self.css = [self.css_player(p) for p in range(len(players))]
            if any(p["slot_type"] != PKIND_HUMAN for p in self.css):
                raise BoundaryError(f"every port must be human: {self.css}")
            self.enter("start")
            return {}
        if not self.ptr(CSS_DATA_PTR) or self.frame - self.phase_frame < SETTLE_FRAMES:
            return {}
        if self.frame < self.wait_until:
            return {}
        port, want = self.player, players[self.player]
        if self.css_player(port)["ckind"] == want:
            self.player += 1
            self.attempts = 0
            return {}
        self.attempts += 1
        if self.attempts > 5:
            raise BoundaryError(f"port {port} did not pick character kind {want}: {self.css_player(port)}")
        cursor, token = self.ptr(CSS_CURSORS + 4 * port), self.ptr(CSS_TOKENS + 4 * port)
        if not cursor or not token:
            if self.frame - self.phase_frame > 10 * SETTLE_FRAMES:
                words = [f"{self.mem.read_u32(a + 4 * i):08X}" for a in (CSS_CURSORS, CSS_TOKENS)
                         for i in range(4)]
                raise BoundaryError(f"port {port} has no CSS hand or token: {words}")
            self.attempts -= 1
            return {}
        bounds = self.icon(want)
        cx, cy = icon_center(bounds)
        if self.mem.read_u8(cursor + 5) != CURSOR_HOLDS_TOKEN:
            # A hand entering the portrait band takes its token (and makes an
            # N/A door human): mncharsel.c:3296-3340. The token then rides at
            # (+2.7, -2.0) from the hand.
            self.mem.write_f32(cursor + 0xC, cx - TOKEN_OFFSET[0])
            self.mem.write_f32(cursor + 0x10, cy - TOKEN_OFFSET[1])
            self.wait_until = self.frame + PRESS_FRAMES
            return {}
        # Aim the held token (measured offset: it follows a frame behind).
        dx = self.mem.read_f32(token + 0x8) - self.mem.read_f32(cursor + 0xC)
        dy = self.mem.read_f32(token + 0xC) - self.mem.read_f32(cursor + 0x10)
        self.mem.write_f32(cursor + 0xC, cx - dx)
        self.mem.write_f32(cursor + 0x10, cy - dy)
        self.wait_until = self.frame + PULSE_PERIOD
        self.pending_press = (port, self.frame + 4)
        return {}

    def costumes_done(self) -> bool:
        wanted = self.config.get("costumes")
        return wanted is None or all(
            self.css_player(port)["color"] == costume for port, costume in enumerate(wanted))

    def change_costume(self) -> dict:
        """X on the first port whose costume differs, one press per PULSE_PERIOD."""
        if self.frame < self.wait_until:
            return {}
        for port, costume in enumerate(self.config["costumes"]):
            if self.css_player(port)["color"] != costume:
                self.attempts += 1
                if self.attempts > 12:
                    raise BoundaryError(f"port {port} did not reach costume {costume}: "
                                        f"{self.css_player(port)}")
                self.wait_until = self.frame + PULSE_PERIOD
                return {port: {"X": True}}
        return {}

    def press_due(self) -> dict:
        """The A press for a hand moved a few frames ago, once the token followed."""
        if self.pending_press is None:
            return {}
        port, at = self.pending_press
        if self.frame < at:
            return {}
        if self.frame < at + PRESS_FRAMES:
            return {port: {"A": True}}
        self.pending_press = None
        return {}

    def read_fighters(self) -> list[dict]:
        try:
            bases = walk.fighter_bases(self.mem, ENTITIES_ADDR)
            return [f for f in (proto.fighter_summary(self.mem, b) for b in bases)
                    if proto.fighter_looks_valid(f)]
        except Exception:  # noqa: BLE001  the entity list is garbage before a match
            return []

    def summary(self) -> dict:
        return {"frame": self.frame, "seed": self.mem.read_u32(SEED_ADDR),
                "css_players": self.css, "fighters": self.read_fighters(),
                "item_frequency": self.mem.read_s8(ITEM_FREQUENCY)}


def main() -> None:
    config = json.loads(Path(os.environ["MELEE_BOUNDARY_CONFIG"]).read_text())
    sav = Path(config["savestate"])
    done, err = Path(str(sav) + ".done"), Path(str(sav) + ".err")

    def save(path: str) -> None:
        savestate.save_to_file(path)
        sidecar = {"savestate": path, "saved_at": time.strftime("%Y-%m-%d %H:%M:%S"),
                   "frame": driver.frame, "seed": memory.read_u32(SEED_ADDR),
                   "fighters": driver.read_fighters()}
        proto.write_json_atomic(Path(path + ".json"), sidecar)

    driver = BoundaryDriver(config, memory, save)
    finished = False

    def on_frame():
        nonlocal finished
        if finished:
            return None
        try:
            inputs = driver.step()
            for port in range(len(config["players"])):
                controller.set_gc_buttons(port, inputs.get(port, {}))
            if driver.done:
                proto.write_json_atomic(done, driver.summary())
                finished = True
        except Exception:  # noqa: BLE001
            err.write_text(f"frame {driver.frame} phase {driver.phase}\n{traceback.format_exc()}")
            finished = True
        return None

    event.on_frameadvance(on_frame)


if event is not None:
    main()
