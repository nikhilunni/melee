//! Sudden Death's Bob-omb rain, the tail of Ground_801C0C2C (ground.c): once
//! the match clock passes twenty seconds, every thirty-one frames a Bob-omb
//! falls near a random player or at one of the stage's item markers.
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_types::GrKind;

/// `current_frame > 0x4B0`: the rain starts after twenty seconds.
const FIRST_DROP_AFTER_FRAME: u32 = 0x4B0;
/// `current_frame - stage_info.x9C > 0x1E`.
const DROP_INTERVAL_FRAMES: u32 = 0x1E;
/// Stage_80224FDC: item-drop markers 0x7F..0x93 (Ground_801C2D24 ids).
pub const ITEM_MARKER_FIRST: i16 = 0x7F;
pub const ITEM_MARKER_COUNT: usize = 0x15;
/// Stage_80224FDC's fallback: the player spawn markers 0..3.
pub const SPAWN_MARKER_COUNT: usize = 4;
/// Ground_801C0A70: drops over a player start this far below the blast top.
const PLAYER_DROP_BELOW_BLAST_TOP: f32 = -5.0;
/// `HSD_Randi(0x64) - 0x32`: horizontal spread around the player.
const PLAYER_DROP_SPREAD: i32 = 0x64;
const PLAYER_DROP_HALF_SPREAD: i32 = 0x32;

/// Ground_801C2D24 results the rain can read: `None` where the stage binds no
/// joint to the marker.
#[derive(Clone, Debug, Default)]
pub struct DropMarkers {
    pub items: [Option<Vec3>; ITEM_MARKER_COUNT],
    pub spawns: [Option<Vec3>; SPAWN_MARKER_COUNT],
}

/// stage_info.x9C, the clock frame of the last drop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BombRain {
    pub last_drop_frame: u32,
}

impl BombRain {
    /// Ground_801C0C2C (801C0C2C), the gm_8016B238 branch: where a Bob-omb
    /// falls this frame, if one does. `players` holds each player slot's
    /// position (Player_LoadPlayerCoords) when the slot has an entity.
    pub fn drop_position(
        &mut self,
        frame: u32,
        stage: GrKind,
        players: &[Option<Vec3>; 4],
        blast_top: f32,
        markers: &DropMarkers,
        rng: &mut HsdRng,
    ) -> Option<Vec3> {
        if frame <= FIRST_DROP_AFTER_FRAME
            || frame.wrapping_sub(self.last_drop_frame) <= DROP_INTERVAL_FRAMES
        {
            return None;
        }
        self.last_drop_frame = frame;
        choose_drop_position(stage, players, blast_top, markers, rng)
    }
}

/// Ground_801C0A70 (801C0A70): half the time, over a random player on the
/// listed stages; otherwise (or with an empty slot) a random item marker.
fn choose_drop_position(
    stage: GrKind,
    players: &[Option<Vec3>; 4],
    blast_top: f32,
    markers: &DropMarkers,
    rng: &mut HsdRng,
) -> Option<Vec3> {
    if rng.randi(2) != 0 && drops_over_players(stage) {
        if let Some(player) = players[rng.randi(4) as usize] {
            let spread = rng.randi(PLAYER_DROP_SPREAD) - PLAYER_DROP_HALF_SPREAD;
            return Some(Vec3::new(
                player.x + spread as f32,
                PLAYER_DROP_BELOW_BLAST_TOP + blast_top,
                player.z,
            ));
        }
    }
    random_item_marker(markers, rng)
}

/// Ground_801C0A70's enabled_stages: Icicle Mountain and Flat Zone are left out.
fn drops_over_players(stage: GrKind) -> bool {
    use GrKind::*;
    matches!(
        stage,
        Castle
            | RCruise
            | Kongo
            | Garden
            | GreatBay
            | Shrine
            | Zebes
            | Kraid
            | Story
            | Yorster
            | Izumi
            | Greens
            | Corneria
            | Venom
            | PStadium
            | Pura
            | MuteCity
            | BigBlue
            | Onett
            | Fourside
            | Inishie1
            | Inishie2
            | OldPupupu
            | OldYoshi
            | OldKongo
            | Battle
            | Last
    )
}

/// Stage_80224FDC (80224FDC): draw item markers from a shrinking range until
/// one exists, then spawn markers the same way. Retail's shrinking range is
/// not a shuffle: a missing marker can be drawn again.
fn random_item_marker(markers: &DropMarkers, rng: &mut HsdRng) -> Option<Vec3> {
    for range in (1..=ITEM_MARKER_COUNT as i32).rev() {
        if let Some(position) = markers.items[rng.randi(range) as usize] {
            return Some(position);
        }
    }
    // The second loop's range becomes the last draw when it finds nothing.
    let mut range = SPAWN_MARKER_COUNT as i32;
    while range != 0 {
        let index = rng.randi(range);
        if let Some(position) = markers.spawns[index as usize] {
            return Some(position);
        }
        range = index;
    }
    None
}
