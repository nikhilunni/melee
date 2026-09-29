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

/// NeutralSpawnTable versions, which differ only in Dream Land's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeutralTable {
    /// Late-2019 console tournament builds (not in the Slippi asm history,
    /// which adds the code in 2020-01): Dream Land (-46.6, 37.0) and
    /// (47.389, 37.0), measured from those builds' replays.
    V2019,
    /// The Slippi asm table from 2020-01 on.
    V2020,
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
