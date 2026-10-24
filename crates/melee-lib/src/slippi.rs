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
    /// it. Two parts reach gameplay (`Screen::Widescreen`):
    /// - 0x8036A4A8, CObjLoad: a perspective CObj's aspect is the
    ///   description's times 320 / 219, so every projected x lies nearer the
    ///   centre. Pokémon Stadium's close-up test (grStadium_801D32D0) then
    ///   frames a fighter over a wider range. Until ef8e734 (2019-10-29) the
    ///   factor was 92 / 63, and the tournament consoles of early 2020 ran
    ///   that one with the bounds below (`Screen::Widescreen92By63`);
    /// - 0x80030C7C / 0x80030C88, Camera_80030BBC: the on-screen test's left
    ///   and right bounds are 100 and 540, not the scissor's 0 and 640. The
    ///   magnifier's damage (Fighter_8006A360) follows that test.
    ///
    /// The code's other writes are display only: the screen flash
    /// (0x803BB05C), the bubble's zoom and placement (0x804DDB28..58), the
    /// nametag scale (0x802FCFC4, 0x804DDB84) and 0x80086B24, which draws a
    /// fighter's model whether or not ftLib_80086A8C finds it near the screen.
    pub screen: Screen,
    /// A console code a replay does not record and whose text is not known:
    /// grIzumi_801CBE64 creates both Fountain of Dreams side platforms at
    /// height 24.0 where it would pass `yakumono_param` +0x0 and +0x8 (20.0
    /// and 28.0: the `lfs f1` at 0x801CC02C and 0x801CC084 before the two
    /// calls of grIzumi_801CCBDC). Nothing else of the stage changes: the
    /// platforms then step, sink and resurface by the retail parameters.
    /// Read from the frames of six tournament games of 2020-03-04, whose
    /// consoles also spawned from [`NeutralTable::LevelFountainEntryByOrder`]
    /// (docs/SLIPPI.md, "Consoles with level Fountain of Dreams platforms").
    pub fountain_level: bool,
    /// Slippi's netplay code `Online/Core/BrawlOffscreenDamage.asm`
    /// (0x8006A880, in every online build): the magnifier's damage counts
    /// the ticks a fighter is outside the stage's camera limits instead of
    /// the ticks its bubble was drawn
    /// (`FighterCore::outside_camera_limits`), so no camera, screen code or
    /// display pass enters it.
    pub offscreen_damage_by_camera_limits: bool,
}

pub use melee_cm::Screen;

/// The seed Slippi's netplay build stores before every tick of an online
/// game (slippi-ssbm-asm `Online/Core/InitOnlinePlay.asm`, FN_SyncRNG: a
/// GObj proc on p_link 7, s_link 0, ahead of every fighter proc):
/// `rlwinm r4, r3, 16, 0xFFFFFFFF` on the scene frame counter (0x80479D60,
/// zero on the match's first tick), then `add` the RNG offset the two
/// players agreed on, which is also the seed the match is created from
/// (stored at 0x8016E748) and so Game Start's.
///
/// The simulated match holds nothing of this: whoever drives it sets the
/// seed before each tick (`diagnostics::Simulation::set_rng_seed`).
pub fn online_sync_seed(scene_frame: u32, rng_offset: u32) -> u32 {
    scene_frame.rotate_left(16).wrapping_add(rng_offset)
}

/// The creation height of both side platforms under
/// [`SlippiCodes::fountain_level`]: the mean of retail's 20.0 and 28.0.
const FOUNTAIN_LEVEL_HEIGHT: f32 = 24.0;

impl SlippiCodes {
    /// The heights grIzumi_801CBE64 passes to grIzumi_801CCBDC for the left
    /// and right platform (0x801CC02C, 0x801CC084), given the stage data's.
    pub fn fountain_creation_heights(&self, data: [f32; 2]) -> [f32; 2] {
        if self.fountain_level {
            [FOUNTAIN_LEVEL_HEIGHT; 2]
        } else {
            data
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
    /// The consoles of [`SlippiCodes::fountain_level`] (2020-03-04): the
    /// 2020 table with Fountain of Dreams' first two singles rows at
    /// y = 24.0 (the table's 21.0 and 27.0 are each 4.875 above the platform
    /// top retail creates, as 24.0 is above the level ones), and the entries
    /// restaggered as in `V2019EntryByOrder`.
    LevelFountainEntryByOrder,
}

/// SetSpawn_AdjustEntryFrames: `mulli r4, REG_SpawnID, 5`.
const ENTRY_FRAMES_PER_ORDER: i32 = 5;

impl NeutralTable {
    /// The entry delay the code stores for the `order`th present player
    /// (Player +0x4C, read by ftCo_800C61B0), replacing fn_8016D8AC's count;
    /// `None` when this version leaves retail's delay alone.
    pub fn entry_delay(self, order: usize) -> Option<i32> {
        match self {
            Self::V2019EntryByOrder | Self::LevelFountainEntryByOrder => {
                Some(ENTRY_FRAMES_PER_ORDER * order as i32)
            }
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
/// NeutralSpawnTable's teams rows (every version's): a team's two players
/// on one side, by team order (`teams_order`).
const TEAMS_SPAWNS: [(i32, [[f32; 2]; 4]); 6] = [
    (
        0x20,
        [[-60.0, 10.0], [-20.0, 10.0], [60.0, 10.0], [20.0, 10.0]],
    ),
    (0x1F, [[-38.8, 35.2], [-38.8, 5.0], [38.8, 35.2], [38.8, 5.0]]),
    (0x08, [[-42.0, 26.6], [-42.0, 5.0], [42.0, 28.0], [42.0, 5.0]]),
    (0x1C, [[-46.6, 37.2], [-46.6, 5.0], [47.4, 37.3], [47.4, 5.0]]),
    (
        0x02,
        [[-41.25, 21.0], [-41.25, 5.0], [41.25, 27.0], [41.25, 5.0]],
    ),
    (0x03, [[-40.0, 32.0], [-40.0, 5.0], [40.0, 32.0], [40.0, 5.0]]),
];

/// NeutralSpawn.asm in a team battle (0x8016E5C4-0x8016E6C8): only when
/// every team has none or two players; then each player's order is its
/// place among the players sorted by team, then slot. `None` leaves every
/// player at retail's spawn. `teams` are the players' teams in slot order.
pub fn teams_order(teams: &[u8]) -> Option<Vec<usize>> {
    for team in 0..3u8 {
        let members = teams.iter().filter(|&&t| t == team).count();
        if members == 1 || members > 2 {
            return None;
        }
    }
    let mut sorted: Vec<usize> = (0..teams.len()).collect();
    sorted.sort_by_key(|&player| (teams[player], player));
    Some(
        (0..teams.len())
            .map(|player| sorted.iter().position(|&p| p == player).expect("listed"))
            .collect(),
    )
}

/// NeutralSpawn.asm's teams row for the `order`th player of
/// [`teams_order`], with the facing SetSpawn stores; `None` for a stage
/// without a row.
pub fn neutral_spawn_teams(stage_id: i32, order: usize) -> Option<(hsd_types::Vec3, f32)> {
    let (_, spawns) = TEAMS_SPAWNS.iter().find(|(id, _)| *id == stage_id)?;
    let [x, y] = *spawns.get(order)?;
    let facing = if x > 0.0 { -1.0 } else { 1.0 };
    Some((hsd_types::Vec3::new(x, y, 0.0), facing))
}

/// V2019's Dream Land singles spawns for orders 0 and 1 (f32 bits
/// 0xC23A6666/0x42140000 and 0x423D8E70/0x42140000).
const DREAM_LAND_2019: [[f32; 2]; 2] = [[-46.6, 37.0], [f32::from_bits(0x423D_8E70), 37.0]];

/// `LevelFountainEntryByOrder`'s Fountain of Dreams singles spawns for
/// orders 0 and 1, read from those consoles' first frames.
const FOUNTAIN_LEVEL: [[f32; 2]; 2] = [[-41.25, 24.0], [41.25, 24.0]];
/// Fountain of Dreams' external stage id.
const FOUNTAIN_OF_DREAMS: i32 = 0x02;

/// NeutralSpawn.asm, singles: the spawn of the `order`th present player
/// (ascending slots) and the facing SetSpawn stores. `None` for a stage
/// without a row, or an order of a measured row the replays have not shown.
pub fn neutral_spawn(
    table: NeutralTable,
    stage_id: i32,
    order: usize,
) -> Option<(hsd_types::Vec3, f32)> {
    let [x, y] = if table == NeutralTable::V2019 && stage_id == 0x1C {
        *DREAM_LAND_2019.get(order)?
    } else if table == NeutralTable::LevelFountainEntryByOrder && stage_id == FOUNTAIN_OF_DREAMS {
        *FOUNTAIN_LEVEL.get(order)?
    } else {
        let (_, spawns) = SINGLES_SPAWNS.iter().find(|(id, _)| *id == stage_id)?;
        spawns[order]
    };
    // SetSpawn_UpdateFacingDirection: fcmpo x, 0.0; ble faces right.
    let facing = if x > 0.0 { -1.0 } else { 1.0 };
    Some((hsd_types::Vec3::new(x, y, 0.0), facing))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(table: NeutralTable, stage_id: i32, order: usize) -> Option<[f32; 2]> {
        neutral_spawn(table, stage_id, order).map(|(spawn, _)| [spawn.x, spawn.y])
    }

    #[test]
    fn teams_spawn_by_team_then_slot_only_two_a_side() {
        assert_eq!(teams_order(&[0, 1, 0, 1]), Some(vec![0, 2, 1, 3]));
        assert_eq!(teams_order(&[1, 1, 0, 0]), Some(vec![2, 3, 0, 1]));
        assert_eq!(teams_order(&[0, 0, 0, 1]), None);
        assert_eq!(teams_order(&[0, 1, 2]), None);
        let (spawn, facing) = neutral_spawn_teams(0x1F, 1).unwrap();
        assert_eq!([spawn.x.to_bits(), spawn.y.to_bits()], [0xC21B_3333, 0x40A0_0000]);
        assert_eq!(facing, 1.0);
    }

    #[test]
    fn the_online_sync_seed_turns_the_frame_counter_sixteen_bits_and_adds_the_offset() {
        // The first seeds of an online game with offset 0x3AAE
        // (`crates/slp/tests/data/v3.16.slp`).
        for (scene_frame, seed) in [(0, 0x0000_3AAE), (1, 0x0001_3AAE), (4, 0x0004_3AAE)] {
            assert_eq!(online_sync_seed(scene_frame, 0x3AAE), seed);
        }
        // The counter's high half comes round to the low one, and the add wraps.
        assert_eq!(online_sync_seed(0x0001_0002, 0xFFFF), 0x0003_0000);
        assert_eq!(online_sync_seed(0xFFFF, 0xFFFF_FFFF), 0xFFFE_FFFF);
    }

    #[test]
    fn level_fountain_table_moves_only_the_first_two_fountain_rows() {
        use NeutralTable::{LevelFountainEntryByOrder as Level, V2020};
        let cases = [
            (FOUNTAIN_OF_DREAMS, 0, Some([-41.25, 24.0])),
            (FOUNTAIN_OF_DREAMS, 1, Some([41.25, 24.0])),
            // Orders the consoles' games did not show stay unported.
            (FOUNTAIN_OF_DREAMS, 2, None),
            (FOUNTAIN_OF_DREAMS, 3, None),
        ];
        for (stage_id, order, expected) in cases {
            assert_eq!(row(Level, stage_id, order), expected, "order {order}");
        }
        assert_eq!(row(V2020, FOUNTAIN_OF_DREAMS, 0), Some([-41.25, 21.0]));
        assert_eq!(row(V2020, FOUNTAIN_OF_DREAMS, 1), Some([41.25, 27.0]));
        for (stage_id, _) in SINGLES_SPAWNS {
            if stage_id != FOUNTAIN_OF_DREAMS {
                for order in 0..4 {
                    assert_eq!(row(Level, stage_id, order), row(V2020, stage_id, order));
                }
            }
        }
    }

    #[test]
    fn level_fountain_table_enters_by_spawn_order() {
        let delays =
            [0, 1, 2, 3].map(|order| NeutralTable::LevelFountainEntryByOrder.entry_delay(order));
        assert_eq!(delays, [Some(0), Some(5), Some(10), Some(15)]);
    }

    #[test]
    fn fountain_platforms_are_created_level_only_under_the_code() {
        let data = [20.0, 28.0];
        let retail = SlippiCodes::default();
        let level = SlippiCodes {
            fountain_level: true,
            ..retail
        };
        assert_eq!(retail.fountain_creation_heights(data), data);
        assert_eq!(level.fountain_creation_heights(data), [24.0, 24.0]);
    }
}
