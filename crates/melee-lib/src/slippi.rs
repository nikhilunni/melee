//! Gameplay-affecting codes that Slippi builds install on top of retail.
//!
//! Slippi recordings come from a patched game: its tournament and netplay
//! code sets move spawns, fix controllers and more. A replay only matches
//! when the simulator runs the same codes. Each is ported from the Slippi
//! asm (project-slippi/slippi-ssbm-asm) and cited by file and injection
//! address; all default off, which is retail.

/// Which Slippi codes a match runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlippiCodes {
    pub spawn: SpawnRule,
    /// Common/Preload Stadium Transformations (0x801D14C8, 0x801D45EC,
    /// 0x801D460C, 0x801D4610, 0x801D4724, 0x801D4F14; in every recording
    /// build from Slippi 1.3.0): Pokémon Stadium chooses and reads the next
    /// form when the base form starts waiting, and announces it on the tick
    /// the wait ends (`melee_gr::stadium::transform::Preload`). The form's
    /// draw moves a whole base duration earlier and no disc read is polled.
    pub stadium_preload: bool,
    /// External/Frozen PS/Core/FreezePokemon.asm (0x801D45FC; Game Start's
    /// "frozen PS" flag, Slippi 2.0.0): Pokémon Stadium never transforms.
    /// With the preload code the first form is still drawn and read.
    pub stadium_frozen: bool,
    /// "Frozen Stages" [UnclePunch, Fizzi] (External/Frozen All/Core/1-4.asm,
    /// `Output/Console/g_stages_all`; in the Slippi tree from 2021-06, on
    /// tournament consoles a year earlier). A replay does not record it.
    /// Four writes, three of them older standalone codes (Dolphin's
    /// GALE01r2.ini, by Zauron: "Disable Yoshi's Story Shyguys", "Disable
    /// Pokemon Stadium Transformations"; Slippi's netplay list: "Disable FD
    /// Background Transitions" by Achilles and Dan Salvato):
    /// - 0x801E3348 `bl grStory_801E3418` is a nop: no Shy Guys;
    /// - 0x8021AAE4 `bl grLast_8021B2E8` is a nop: Final Destination's
    ///   background never leaves its first phase;
    /// - 0x801D1548 `bl grStadium_801D4548` is a nop: Pokémon Stadium's
    ///   transformation controller never runs (nor the preload code's draw
    ///   inside it);
    /// - 0x803E67E0, grOp_803E67D8[2], is 0: Whispy's cycle waits where it
    ///   would blow.
    ///
    /// 20XX TE's Frozen Mode (`frozen_mode.mgc`) has the three nops but
    /// stops Whispy at 0x802115B4 instead, which is not ported.
    pub frozen_stages: bool,
    /// "Widescreen 16:9" [Dan Salvato, mirrorbender, Achilles1515,
    /// UnclePunch] (External/Widescreen, `Output/Console/g_screen_wide`; the
    /// off-screen bounds from e9457ee, 2019-11-03). A replay does not record
    /// it. Two parts reach gameplay (`melee_cm::Screen::Widescreen`):
    /// - 0x8036A4A8, CObjLoad: a perspective CObj's aspect is the
    ///   description's times 320 / 219, so every projected x lies nearer the
    ///   centre. Pokémon Stadium's close-up test (grStadium_801D32D0) then
    ///   frames a fighter over a wider range;
    /// - 0x80030C7C / 0x80030C88, Camera_80030BBC: the on-screen test's left
    ///   and right bounds are 100 and 540, not the scissor's 0 and 640. The
    ///   magnifier's damage (Fighter_8006A360) follows that test.
    ///
    /// The code's other writes are display only: the screen flash
    /// (0x803BB05C), the bubble's zoom and placement (0x804DDB28..58), the
    /// nametag scale (0x802FCFC4, 0x804DDB84) and 0x80086B24, which draws a
    /// fighter's model whether or not ftLib_80086A8C finds it near the screen.
    pub widescreen: bool,
}

impl SlippiCodes {
    /// The screen code these codes install on the game camera.
    pub fn screen(&self) -> melee_cm::Screen {
        if self.widescreen {
            melee_cm::Screen::Widescreen
        } else {
            melee_cm::Screen::Standard
        }
    }
}

/// Where each player starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpawnRule {
    /// Retail: the stage marker of the player's slot.
    #[default]
    Retail,
    /// External/NeutralSpawn/NeutralSpawn.asm (0x8016E510): the table row
    /// of the player's order among present players, facing by sign of x.
    NeutralTable(NeutralTable),
}

/// NeutralSpawn versions: the table (which differs only in Dream Land's
/// row) and whether the code also restaggers the entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeutralTable {
    /// Late-2019 console tournament builds: Dream Land (-46.6, 37.0) and
    /// (47.389, 37.0), measured from those builds' replays.
    V2019,
    /// External/NeutralSpawn.asm from 2019-10-21 (1a01aec) until 2020-01-14
    /// (4dc7447, "remove feature causing neutral spawn desync"): the 2020
    /// table, then SetSpawn_AdjustEntryFrames calls Player_SetUnk4C
    /// (0x80035FDC) with five times the spawn order, so the first player
    /// enters at once. Consoles still ran this build in March 2020.
    V2019EntryByOrder,
    /// The Slippi asm code from 2020-01-14 on.
    V2020,
}

/// SetSpawn_AdjustEntryFrames: `mulli r4, REG_SpawnID, 5`.
const ENTRY_FRAMES_PER_ORDER: i32 = 5;

impl NeutralTable {
    /// The entry delay the code stores for the `order`th present player
    /// (Player +0x4C, read by ftCo_800C61B0), replacing fn_8016D8AC's count;
    /// `None` when this version leaves retail's delay alone.
    pub fn entry_delay(self, order: usize) -> Option<i32> {
        match self {
            Self::V2019EntryByOrder => Some(ENTRY_FRAMES_PER_ORDER * order as i32),
            Self::V2019 | Self::V2020 => None,
        }
    }
}

/// NeutralSpawnTable's singles rows: (external stage id, spawns by order).
const SINGLES_SPAWNS: [(i32, [[f32; 2]; 4]); 6] = [
    (
        0x20,
        [[-60.0, 10.0], [60.0, 10.0], [-20.0, 10.0], [20.0, 10.0]],
    ),
    (0x1F, [[-38.8, 35.2], [38.8, 35.2], [0.0, 8.0], [0.0, 62.4]]),
    (0x08, [[-42.0, 26.6], [42.0, 28.0], [0.0, 46.9], [0.0, 4.9]]),
    (0x1C, [[-46.6, 37.2], [47.4, 37.3], [0.0, 7.0], [0.0, 58.5]]),
    (
        0x02,
        [[-41.25, 21.0], [41.25, 27.0], [0.0, 5.25], [0.0, 48.0]],
    ),
    (
        0x03,
        [[-40.0, 32.0], [40.0, 32.0], [70.0, 7.0], [-70.0, 7.0]],
    ),
];
/// V2019's Dream Land singles spawns for orders 0 and 1 (f32 bits
/// 0xC23A6666/0x42140000 and 0x423D8E70/0x42140000).
const DREAM_LAND_2019: [[f32; 2]; 2] = [[-46.6, 37.0], [f32::from_bits(0x423D_8E70), 37.0]];

/// NeutralSpawn.asm, singles: the spawn of the `order`th present player
/// (ascending slots) and the facing SetSpawn stores. `None` for a stage
/// without a row, or a V2019 order the replays have not shown.
pub fn neutral_spawn(
    table: NeutralTable,
    stage_id: i32,
    order: usize,
) -> Option<(hsd_types::Vec3, f32)> {
    let [x, y] = if table == NeutralTable::V2019 && stage_id == 0x1C {
        *DREAM_LAND_2019.get(order)?
    } else {
        let (_, spawns) = SINGLES_SPAWNS.iter().find(|(id, _)| *id == stage_id)?;
        spawns[order]
    };
    // SetSpawn_UpdateFacingDirection: fcmpo x, 0.0; ble faces right.
    let facing = if x > 0.0 { -1.0 } else { 1.0 };
    Some((hsd_types::Vec3::new(x, y, 0.0), facing))
}
