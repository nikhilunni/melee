"""Create a match-start boundary from a cold boot: runs INSIDE the scripting fork.

Launched by harness/make_boundary.py (headless, unlimited speed). The config is
a JSON file named by MELEE_BOUNDARY_CONFIG:

    {"savestate": "/abs/roms/<name>.sav", "stkind": 32, "stocks": 4,
     "players": [2, 9], "time_limit_minutes": 0, "transform": [0],
     "costumes": [3, 0]}

`players` are CSS character kinds (ft/forward.h CharacterKind), one per port.
`ports` (optional) names each player's port, 0-based and ascending (default:
the first ports); only those hold a controller, so the other doors stay N/A.
`costumes` (optional) gives each port's costume: after the characters are
picked, that port presses X until the CSS door's costume matches
(mnCharSel_CostumeChange, mncharsel.c:2186: X steps to the next costume).
`transform` (optional) lists ports that hold A from the stage screen until
the fighters exist: fn_8016D8AC (gm_16AE.c:1573-1583) reads that port's
HSD_PadCopyStatus when the match loads and swaps a human Zelda for Sheik
(or back), as a player holding A does on retail.
`game_start_seed` (optional) is written over the RNG seed when the match is
created, where Slippi's Game Start reads it (0x8016E74C, after fn_8016E730's
db_Setup call and before it creates the Ground and Players), so the stage's
creation draws are those of the console that recorded the seed. The scripting
API has memory breakpoints only, so the hook is the first store after that
address: gm_801A4B08 (called at 0x8016E75C) stores
gm_AnyControllerPressedStart at gm_80479D58 +0x14, and nothing between the
two addresses draws.
`item_frequency` (optional, default -1: items off) is the Item Switch
screen's frequency, written to save data (gmm_x1CB0.item_freq: -1 off,
0 Very Low .. 4 Very High); `item_switch` (optional, default every item) lists
the Item Switch screen's entries that are on, as save-data mask bits
(gmm_x1CB0.item_mask; gm_1601.c:3648 maps bit i to item kind
lbl_803B7844[i] when the menus build the match rules). Containers (Capsule,
Box, Barrel, Egg) have no entry: the VS rules start with every bit set
(gm_1601.c:3534), so they spawn whenever items are on. `rules_item_mask`
(optional, a u64) replaces the match's item mask at the creation hook below
(StartMeleeRules.x20 of gmVsMelee_StartData, which fn_8016DCC0 copies into
the match's rules afterwards), for a match no menu makes (no containers).
`cpu_players` (optional,
coverage runs only) lists players that the match makes CPUs at `cpu_level`:
written at the creation hook into the start data's PlayerInitData (slot_type
+1 = Gm_PKind_Cpu, cpu_level +0xF); the CSS still picks them as humans.
`cpu_levels` (optional) gives each player's CPU level, 0 for a human: a
nonzero level makes that player a CPU at that level, written at the same hook
into the match's start data (PlayerInitData slot_type +1 = Gm_PKind_Cpu,
cpu_level +0xF, nametag +0xA = GM_NAMETAG_NONE: what the CSS writes when a
hand clicks a door to CPU, mncharsel.c:2476, and sets its slider,
mncharsel.c:2341; cpu_kind +0xE stays the CSS's 4). fn_8016D8AC copies them
into the match info afterwards (gm_16AE.c:1597-1598), so Fighter_Create runs
ftCo_800A101C with CPU type 4 and the level.
`spawn_points` (optional) gives each player's spawn point, written at the
same hook into the match's start data (gmVsMelee_StartData's PlayerInitData
+5, which the menus leave at -1: the port's own marker). fn_8016D8AC copies
it into the match info afterwards (0x8016D9E4, gm_16AE.c:1586) and fn_8016DEEC places the
player at that marker. A console whose replay names spawn points (Game
Start's per-player block) ran a code that set them.

Every frame `BoundaryDriver.step` reads the game's own scene state and decides
the inputs, so no step depends on timing or screenshots:

  boot     pulse Start until the scene machine reaches the main menu (the
           opening movie and the title both exit on a Start edge);
  menu     apply the RAM-only save-data pokes every boundary uses (characters
           and every stage unlocked, stock rules, items off), then walk the
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
# VS. Mode > Special Melee (MENU_KIND_SPECIAL, SEL_VS_SPECIAL): each mode's
# SpecialVsMenuSelection and the GameModeKind its CSS runs in.
MENU_SPECIAL, SEL_VS_SPECIAL = 12, 2
SPECIAL_MODES = {
    "camera": (0, 0x2A), "stamina": (1, 0x1F), "super-sudden-death": (2, 0x10),
    "giant": (3, 0x1E), "tiny": (4, 0x1D), "invisible": (5, 0x11),
    "single-button": (7, 0x2C), "lightning": (8, 0x13), "slomo": (9, 0x12),
}

# Save data (gmMainLib_804D3EE0 -> 0x8045A6C0), the pokes of docs/DOLPHIN_RUN.md
# "Unlocks and rules are RAM-only pokes". The unlock bits reach match state
# only through music selection (Ground_801C24F8: rule 6 asks whether every
# character is unlocked, gm_80164ABC; rule 7 whether every stage is,
# gm_80164600). Every stage is unlocked, as gm_8016468C (0x8016468C) does:
# all eleven bits. Savestates made before 2026-10-24 have only Battlefield
# and Final Destination (0x00C0), which no ported stage's music reads.
CHARACTER_UNLOCKS, CHARACTER_UNLOCK_BITS = 0x8045BF28, 0xFFFF
STAGE_UNLOCKS, STAGE_UNLOCK_BITS = 0x8045BF2A, 0x07FF
GAME_RULES = 0x8045BF10                # gm/types.h GameRules
RULES_MODE, RULES_STOCKS, RULES_STOCK_TIME = 0x2, 0x4, 0x8
# GameRules time_limit (+3, minutes: the Time, Coin and Bonus timer),
# handicap (+5: 0 off, 1 auto, 2 on) and damage_ratio (+6, tenths); the VS
# rules build StartMeleeRules from them (gm_80167BC8, gm_1601.c:3570-3643).
RULES_TIME, RULES_HANDICAP, RULES_DAMAGE_RATIO = 0x3, 0x5, 0x6
MODE_TIME, MODE_STOCK, MODE_COIN = 0, 1, 2   # mn/forward.h Mode
ITEM_FREQUENCY = 0x8045C370            # gmm_x1CB0.item_freq (0x1CB0 into save data)
ITEMS_OFF = 0xFF                       # (s8) -1
ITEM_SWITCH = 0x8045C378               # gmm_x1CB0.item_mask (u64, bit i: lbl_803B7844[i])
ITEM_SWITCH_ALL = (1 << 32) - 1
# gmVsMelee_StartData.rules (StartMeleeRules): xB item frequency, x20 item mask.
RULES_ITEM_FREQUENCY, RULES_ITEM_MASK = 0xB, 0x20
# The match's own rules (gm_GetRules: lbl_8046B6A0.x24C8).
MATCH_RULES = 0x8046B6A0 + 0x24C8

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
# A team battle (make_boundary.py --teams): the CSS start rules' is_teams
# (+8; gm_8016B168 reads lbl_8046B6A0 +24D0) and friendly_fire (+1 bit 0;
# gm_8016B0D4), and each door's team (mnCharSel_803F0DFC.doors[4], CSSDoor
# 0x24 bytes, team +0xA), which the CSS copies into the start data's
# PlayerInitData team (+9) every frame (mncharsel.c:1518).
CSS_RULES, RULES_IS_TEAMS, RULES_FRIENDLY_FIRE = 0x10, 0x8, 0x1
PLAYER_TEAM = 0x9
PLAYER_HANDICAP = 0x8
CSS_DOORS, DOOR_SIZE, DOOR_TEAM = symbols.addr("mnCharSel_803F0DFC"), 0x24, 0xA
# The match's StartMeleeRules (lbl_8046B6A0 +24C8), read back for the summary.
MATCH_RULES = symbols.addr("lbl_8046B6A0") + 0x24C8

# mnstagesel.c: SSSData* sss_data (.sbss 0x804D6C90; a .bss symbol shares the
# name), force_stage_id s8 at +3 (mnStageSel_Scene_OnFrame:766).
SSS_DATA_PTR = 0x804D6C90
SSS_FORCE_STAGE = 0x3

SEED_ADDR = symbols.addr("seed")
# fn_8016E730 (gm_16AE.c:1982, 0x8016E75C): gm_801A4B08 stores the match's
# Start-press test at gm_80479D58 +0x14 (0x801A4B10), the first store after
# 0x8016E74C, where Slippi reads the Game Start seed.
MATCH_START_TEST = symbols.addr("gm_80479D58") + 0x14
MATCH_START_TEST_FN = symbols.addr("gm_AnyControllerPressedStart")
# gmVsMelee_StartData (StartMeleeData): rules (0x60), then one PlayerInitData
# (0x24) per port; +0 is the character kind, +5 the spawn point.
VS_START_RULES = symbols.addr("gmVsMelee_StartData")
VS_START_PLAYERS = VS_START_RULES + 0x60
START_PLAYER_SIZE = 0x24
START_PLAYER_SPAWN = 5
START_PLAYER_SLOT_TYPE, START_PLAYER_NAMETAG, START_PLAYER_CPU_LEVEL = 1, 0xA, 0xF
PKIND_CPU = 1
NAMETAG_NONE = 0x78
ENTITIES_ADDR = symbols.addr("HSD_GObj_Entities")
MS_ENTRY = 322   # ftCo_MS_Entry, every fighter's first match state
# pl/player.c:58 ftMapping_list: CKIND_POPONANA creates a second fighter
# (Nana, player_entity[1]) for its port (Player_80031AD0).
CKIND_ICE_CLIMBERS = 0x0E


def fighter_count(players: list[int]) -> int:
    """Fighter GObjs a match creates for these CSS kinds (Nana is her own)."""
    return sum(2 if kind == CKIND_ICE_CLIMBERS else 1 for kind in players)
MS_SLEEP = 11    # ftCo_MS_Sleep: a transforming character's inactive form
# CKIND_ZELDA (and CKIND_SEAK once A is held): the port's other form is a
# third fighter, created after the awake ones and asleep from the start.
CKIND_ZELDA = 0x12


def sleeper_count(players: list[int]) -> int:
    """Sleeping fighter GObjs a match creates for these CSS kinds."""
    return sum(kind == CKIND_ZELDA for kind in players)

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
        self.ports = list(config.get("ports") or range(len(config["players"])))
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
        # The seed the match is created from, and the seed it replaced once
        # the creation hook has written it.
        self.game_start_seed: int | None = config.get("game_start_seed")
        self.seed_replaced: int | None = None
        # Each player's spawn point, and what the hook found there.
        self.spawn_points: list[int] | None = config.get("spawn_points")
        self.spawn_points_replaced: list[int] | None = None
        self.cpu_players: list[int] = list(config.get("cpu_players") or [])
        # The match's item mask, replaced at the same hook.
        self.rules_item_mask: int | None = config.get("rules_item_mask")
        self.rules_item_mask_replaced: int | None = None
        # Each player's CPU level (0: human), and what the hook found there.
        self.cpu_levels: list[int] | None = config.get("cpu_levels")
        self.cpu_replaced: list[dict] | None = None
        self.creation_hooked = False
        self.watching_creation = False
        # `special` (optional): a Special Melee mode instead of Melee.
        special = config.get("special")
        self.special = SPECIAL_MODES[special] if special else None
        self.major = self.special[1] if self.special else GM_VS

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

    def pulse(self, button: str) -> dict:
        """Press `button` for PRESS_FRAMES every PULSE_PERIOD frames of this phase
        (the first player's controller drives the menus)."""
        if (self.frame - self.phase_frame) % PULSE_PERIOD < PRESS_FRAMES:
            return {self.ports[0]: {button: True}}
        return {}

    def step(self) -> dict:
        self.frame += 1
        if self.frame > TIMEOUT_FRAMES:
            raise BoundaryError(f"no boundary after {TIMEOUT_FRAMES} frames (phase {self.phase})")
        if self.watching_creation and self.creation_hooked:
            # Not from inside the breakpoint's own callback.
            self.mem.remove_memcheck(MATCH_START_TEST)
            self.watching_creation = False
        inputs = self.phase_inputs(*self.scene())
        for port, pad in self.press_due().items():
            inputs.setdefault(port, {}).update(pad)
        if self.phase == "match":
            for player in self.config.get("transform", []):
                inputs.setdefault(self.ports[player], {})["A"] = True
        return inputs

    def phase_inputs(self, mode: int, state: int) -> dict:
        if self.phase == "boot":
            if mode == GM_MENU:
                self.poke_save_data()
                self.enter("menu")
                return {}
            return self.pulse("Start")
        if self.phase == "menu":
            if mode == self.major:
                self.enter("css")
                return {}
            return self.navigate_menu()
        if self.phase == "css":
            return self.select_characters(mode, state)
        if self.phase == "start":
            if mode == self.major and state == VS_STATE_SSS and self.ptr(SSS_DATA_PTR):
                self.mem.write_u8(self.ptr(SSS_DATA_PTR) + SSS_FORCE_STAGE, self.config["stkind"])
                if self.hooks_creation():
                    self.mem.add_memcheck(MATCH_START_TEST)
                    self.watching_creation = True
                self.enter("match")
                return {}
            return self.pulse("Start")
        if self.phase == "match":
            # Every fighter created and in Entry: a fighter still inside
            # Fighter_Create reads as motion 0 at the origin, before its CPU
            # setup draws, and would leave the seed short of post-creation.
            # A transforming character's other form sleeps and makes no entry;
            # it is created last, and the frame callback can fall before it
            # (a Sheik start on Final Destination saved two draws early).
            created = self.read_fighters()
            fighters = [f for f in created if f["motion_id"] != MS_SLEEP]
            asleep = len(created) - len(fighters)
            if len(fighters) == fighter_count(self.config["players"]) and all(
                    f["motion_id"] == MS_ENTRY for f in fighters) and (
                    asleep == sleeper_count(self.config["players"])):
                if self.hooks_creation() and not self.creation_hooked:
                    missed = " and ".join(
                        what for what, named in (("the Game Start seed was", self.game_start_seed),
                                                 ("the spawn points were", self.spawn_points),
                                                 ("the item mask was", self.rules_item_mask),
                                                 ("the CPU levels were", self.cpu_levels))
                        if named is not None)
                    raise BoundaryError("the match was created without passing fn_8016E730's "
                                        f"gm_801A4B08 call: {missed} not written")
                self.save(self.config["savestate"])
                self.done = True
                self.enter("saved")
            return {}
        return {}

    def on_memory(self, is_write: bool, addr: int, value: int) -> None:
        """The creation hook (a memory breakpoint; the CPU is stopped inside
        gm_801A4B08): replace the seed fn_8016E730 creates the match from,
        and the spawn points fn_8016D8AC is about to copy."""
        if not self.watching_creation or self.creation_hooked:
            return
        if not is_write or addr != MATCH_START_TEST or value != MATCH_START_TEST_FN:
            return
        self.creation_hooked = True
        if self.game_start_seed is not None:
            self.seed_replaced = self.mem.read_u32(SEED_ADDR)
            self.mem.write_u32(SEED_ADDR, self.game_start_seed)
        if self.spawn_points is not None:
            self.spawn_points_replaced = []
            for player, point in enumerate(self.spawn_points):
                base = VS_START_PLAYERS + self.ports[player] * START_PLAYER_SIZE
                if self.mem.read_s8(base) != self.config["players"][player]:
                    raise BoundaryError(f"the start data's player {self.ports[player]} is character "
                                        f"{self.mem.read_s8(base)}, not {self.config['players'][player]}")
                self.spawn_points_replaced.append(self.mem.read_s8(base + START_PLAYER_SPAWN))
                self.mem.write_u8(base + START_PLAYER_SPAWN, point & 0xFF)
        if self.cpu_levels is None:
            for player in self.cpu_players:
                base = VS_START_PLAYERS + self.ports[player] * START_PLAYER_SIZE
                self.mem.write_u8(base + START_PLAYER_SLOT_TYPE, PKIND_CPU)
                self.mem.write_u8(base + START_PLAYER_CPU_LEVEL, int(self.config.get("cpu_level", 9)))
        if self.cpu_levels is not None:
            self.cpu_replaced = []
            for player, level in enumerate(self.cpu_levels):
                base = VS_START_PLAYERS + self.ports[player] * START_PLAYER_SIZE
                if self.mem.read_s8(base) != self.config["players"][player]:
                    raise BoundaryError(f"the start data's player {self.ports[player]} is character "
                                        f"{self.mem.read_s8(base)}, not {self.config['players'][player]}")
                self.cpu_replaced.append({
                    "slot_type": self.mem.read_u8(base + START_PLAYER_SLOT_TYPE),
                    "nametag": self.mem.read_u8(base + START_PLAYER_NAMETAG),
                    "cpu_kind": self.mem.read_u8(base + START_PLAYER_CPU_LEVEL - 1),
                    "cpu_level": self.mem.read_u8(base + START_PLAYER_CPU_LEVEL)})
                if level:
                    self.mem.write_u8(base + START_PLAYER_SLOT_TYPE, PKIND_CPU)
                    self.mem.write_u8(base + START_PLAYER_NAMETAG, NAMETAG_NONE)
                    self.mem.write_u8(base + START_PLAYER_CPU_LEVEL, level)
        if self.rules_item_mask is not None:
            at = VS_START_RULES + RULES_ITEM_MASK
            self.rules_item_mask_replaced = (self.mem.read_u32(at) << 32) | self.mem.read_u32(at + 4)
            self.mem.write_u32(at, self.rules_item_mask >> 32)
            self.mem.write_u32(at + 4, self.rules_item_mask & 0xFFFFFFFF)

    def hooks_creation(self) -> bool:
        return (self.game_start_seed is not None or self.spawn_points is not None
                or self.cpu_levels is not None or bool(self.cpu_players) or self.rules_item_mask is not None)

    def poke_save_data(self) -> None:
        mem = self.mem
        mem.write_u16(CHARACTER_UNLOCKS, mem.read_u16(CHARACTER_UNLOCKS) | CHARACTER_UNLOCK_BITS)
        mem.write_u16(STAGE_UNLOCKS, mem.read_u16(STAGE_UNLOCKS) | STAGE_UNLOCK_BITS)
        mode = self.config.get("mode", MODE_STOCK)
        mem.write_u8(GAME_RULES + RULES_MODE, mode)
        mem.write_u8(GAME_RULES + RULES_STOCKS, self.config["stocks"])
        # Stock reads its own timer (stock_time_limit); Time and Coin read
        # time_limit (gm_1601.c:3578-3600).
        minutes = self.config.get("time_limit_minutes", 0)
        mem.write_u8(GAME_RULES + RULES_STOCK_TIME, minutes if mode == MODE_STOCK else 0)
        if mode != MODE_STOCK:
            mem.write_u8(GAME_RULES + RULES_TIME, minutes)
        if "handicap_rule" in self.config:
            mem.write_u8(GAME_RULES + RULES_HANDICAP, self.config["handicap_rule"])
        if "damage_ratio_tenths" in self.config:
            mem.write_u8(GAME_RULES + RULES_DAMAGE_RATIO, self.config["damage_ratio_tenths"])
        if self.config.get("teams"):
            # GameRules +9 friendly_fire: the match's rules take it from here.
            mem.write_u8(GAME_RULES + 9, 1 if self.config.get("friendly_fire") else 0)
        mem.write_u8(ITEM_FREQUENCY, self.config.get("item_frequency", ITEMS_OFF) & 0xFF)
        switch = self.config.get("item_switch")
        mask = ITEM_SWITCH_ALL if switch is None else sum(1 << bit for bit in set(switch))
        mem.write_u32(ITEM_SWITCH, 0)
        mem.write_u32(ITEM_SWITCH + 4, mask)

    def navigate_menu(self) -> dict:
        if self.frame < self.wait_until or self.frame - self.phase_frame < SETTLE_FRAMES:
            return {}
        menu = self.mem.read_u8(MENU_FLOW)
        hovered = self.mem.read_u16(MENU_FLOW + 2)
        targets = {MENU_MAIN: SEL_MAIN_VS, MENU_VS: SEL_VS_MELEE}
        if self.special:
            targets = {MENU_MAIN: SEL_MAIN_VS, MENU_VS: SEL_VS_SPECIAL, MENU_SPECIAL: self.special[0]}
        target = targets.get(menu)
        self.wait_until = self.frame + PULSE_PERIOD
        first = self.ports[0]
        if target is None:   # a submenu this path never opens: back out
            return {first: {"B": True}}
        if hovered == target:
            return {first: {"A": True}}
        return {first: {"Down" if hovered < target else "Up": True}}

    def select_characters(self, mode: int, state: int) -> dict:
        if (mode, state) != (self.major, VS_STATE_CSS):
            raise BoundaryError(f"left the CSS early (mode {mode}, state {state})")
        players = self.config["players"]
        if self.player == len(players) and not self.costumes_done():
            return self.change_costume()
        if self.player == len(players) and not self.teams_set():
            return {}
        if self.player == len(players) and not self.handicaps_set():
            return {}
        if self.player == len(players):
            self.css = [self.css_player(p) for p in self.ports]
            if any(p["slot_type"] != PKIND_HUMAN for p in self.css):
                raise BoundaryError(f"every port must be human: {self.css}")
            self.enter("start")
            return {}
        if not self.ptr(CSS_DATA_PTR) or self.frame - self.phase_frame < SETTLE_FRAMES:
            return {}
        if self.frame < self.wait_until:
            return {}
        port, want = self.ports[self.player], players[self.player]
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

    def teams_set(self) -> bool:
        """Poke the team battle into the CSS (idempotent); True once the start
        data reads it back, or at once without --teams."""
        teams = self.config.get("teams")
        if not teams:
            return True
        css = self.ptr(CSS_DATA_PTR)
        rules = css + CSS_RULES
        self.mem.write_u8(rules + RULES_IS_TEAMS, 1)
        flags = self.mem.read_u8(rules + RULES_FRIENDLY_FIRE)
        friendly = 1 if self.config.get("friendly_fire") else 0
        self.mem.write_u8(rules + RULES_FRIENDLY_FIRE, (flags & ~1) | friendly)
        for port, team in zip(self.ports, teams):
            self.mem.write_u8(CSS_DOORS + port * DOOR_SIZE + DOOR_TEAM, team)
            self.mem.write_u8(css + CSS_PLAYERS + port * PLAYER_SIZE + PLAYER_TEAM, team)
        return all(self.mem.read_u8(css + CSS_PLAYERS + port * PLAYER_SIZE + PLAYER_TEAM) == team
                   for port, team in zip(self.ports, teams))

    def handicaps_set(self) -> bool:
        """Poke each door's handicap into the CSS start data (PlayerInitData
        +8, what the CSS's handicap slider writes with the rule on), as
        teams_set does; True once it reads back, or at once without
        `handicaps`. gm_80167BC8 (gm_1601.c:3632-3637) turns them into the
        attack and defense ratios of lbl_803B7930 when the match starts."""
        handicaps = self.config.get("handicaps")
        if not handicaps:
            return True
        css = self.ptr(CSS_DATA_PTR)
        for port, handicap in zip(self.ports, handicaps):
            self.mem.write_u8(css + CSS_PLAYERS + port * PLAYER_SIZE + PLAYER_HANDICAP, handicap)
        return all(self.mem.read_u8(css + CSS_PLAYERS + port * PLAYER_SIZE + PLAYER_HANDICAP) == h
                   for port, h in zip(self.ports, handicaps))

    def costumes_done(self) -> bool:
        wanted = self.config.get("costumes")
        return wanted is None or all(
            self.css_player(port)["color"] == costume for port, costume in zip(self.ports, wanted))

    def change_costume(self) -> dict:
        """X on the first port whose costume differs, one press per PULSE_PERIOD."""
        if self.frame < self.wait_until:
            return {}
        for port, costume in zip(self.ports, self.config["costumes"]):
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
                "item_frequency": self.mem.read_s8(ITEM_FREQUENCY),
                "is_teams": self.mem.read_u8(MATCH_RULES + RULES_IS_TEAMS),
                "friendly_fire": self.mem.read_u8(MATCH_RULES + RULES_FRIENDLY_FIRE) & 1,
                **self.match_rules(), **self.match_items(), **self.creation_seed()}

    def match_rules(self) -> dict:
        """The match's mode, timer and ratios as StartMeleeRules holds them:
        match_kind (byte 0 bits 7-5), timer_enabled (bit 1), time_limit
        (+0x10, seconds), the SD penalty xC, x30 the damage ratio, and each
        player's attack and defense ratio (StaticPlayer +0x50/+0x54 is not
        read: the start data's PlayerInitData +0x18/+0x1C, which
        fn_8016D8AC copies)."""
        flags = self.mem.read_u8(MATCH_RULES)
        players = [{"handicap": self.mem.read_s8(VS_START_PLAYERS + p * START_PLAYER_SIZE + 8),
                    "attack_ratio": self.mem.read_f32(VS_START_PLAYERS + p * START_PLAYER_SIZE + 0x18),
                    "defense_ratio": self.mem.read_f32(VS_START_PLAYERS + p * START_PLAYER_SIZE + 0x1C)}
                   for p in self.ports]
        return {"match_kind": flags >> 5, "timer_enabled": (flags >> 1) & 1,
                "time_limit": self.mem.read_u32(MATCH_RULES + 0x10),
                "sd_penalty": self.mem.read_s8(MATCH_RULES + 0xC),
                "score_display": self.mem.read_u8(MATCH_RULES + 3) >> 7,
                "damage_ratio": self.mem.read_f32(MATCH_RULES + 0x30),
                "rule_players": players}

    def match_items(self) -> dict:
        """The item rules the match runs: frequency and mask (gm_8016AE80,
        gm_8016AEA4)."""
        mask = (self.mem.read_u32(MATCH_RULES + RULES_ITEM_MASK) << 32) | self.mem.read_u32(
            MATCH_RULES + RULES_ITEM_MASK + 4)
        return {"rules_item_frequency": self.mem.read_s8(MATCH_RULES + RULES_ITEM_FREQUENCY),
                "rules_item_mask": mask}

    def creation_seed(self) -> dict:
        """The seed the match was created from and the spawn points it was
        given, when the config named them."""
        out = {}
        if self.game_start_seed is not None:
            out |= {"game_start_seed": self.game_start_seed, "seed_replaced": self.seed_replaced}
        if self.spawn_points is not None:
            out |= {"spawn_points": self.spawn_points,
                    "spawn_points_replaced": self.spawn_points_replaced}
        if self.rules_item_mask is not None:
            out |= {"rules_item_mask_replaced": self.rules_item_mask_replaced}
        if self.cpu_levels is not None:
            out |= {"cpu_levels": self.cpu_levels, "cpu_replaced": self.cpu_replaced}
        return out


def main() -> None:
    config = json.loads(Path(os.environ["MELEE_BOUNDARY_CONFIG"]).read_text())
    sav = Path(config["savestate"])
    done, err = Path(str(sav) + ".done"), Path(str(sav) + ".err")

    def save(path: str) -> None:
        savestate.save_to_file(path)
        sidecar = {"savestate": path, "saved_at": time.strftime("%Y-%m-%d %H:%M:%S"),
                   "frame": driver.frame, "seed": memory.read_u32(SEED_ADDR),
                   "fighters": driver.read_fighters(), **driver.match_rules(), **driver.match_items(),
                   **driver.creation_seed()}
        proto.write_json_atomic(Path(path + ".json"), sidecar)

    driver = BoundaryDriver(config, memory, save)
    finished = False

    def on_memory(is_write, addr, value):
        try:
            driver.on_memory(is_write, addr, value)
        except Exception:  # noqa: BLE001
            err.write_text(f"frame {driver.frame} creation hook\n{traceback.format_exc()}")

    if driver.hooks_creation():
        event.on_memorybreakpoint(on_memory)

    def on_frame():
        nonlocal finished
        if finished:
            return None
        try:
            inputs = driver.step()
            for port in driver.ports:
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
