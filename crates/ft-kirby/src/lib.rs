//! Kirby: ft/kinds/ftKirby. Common states live in melee-ft.
pub mod attack_dash;
pub mod attributes;
pub mod bump_star;
pub mod common;
pub mod init;
pub mod mouthful;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;
pub mod swallow;

use melee_ft::fighter::{
    state::{self, callbacks},
    ActionId, MotionRow,
};
use melee_types::CommonMotionState as S;

/// ftKb_MS_SelfCount: rows 341..543, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 203;
pub(crate) const FIRST_ACTION: u16 = 341;
/// ftKb_SM_JumpAerialF1 = ftCo_SM_Count: the table's first submotion.
const FIRST_SUBMOTION: i32 = 295;
/// JumpAerialF1..F5 and JumpAerialF1Met..F5Met (341..350).
const MULTI_JUMP_ROWS: usize = 10;

/// ftKb_Init_MotionStateTable (ftkirby.c, 0x803C8368). Unported rows fail
/// closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    // Both multijump families (bare and helmeted) run the common callbacks:
    // ftCo_JumpAerialF1_Anim/IASA/Phys/Coll (800D7590..800D767C).
    i = 0;
    while i < MULTI_JUMP_ROWS {
        rows[i] = MotionRow {
            action: ActionId(FIRST_ACTION + i as u16),
            animation: FIRST_SUBMOTION + i as i32,
            anim: callbacks::animation::multi_jump,
            physics: callbacks::physics::multi_jump,
            collision: callbacks::collision::pass,
            ..state::COMMON[S::JumpAerialF as usize]
        };
        i += 1;
    }
    place_all(&mut rows, attack_dash::rows());
    place_all(&mut rows, special_n::rows());
    place_all(&mut rows, mouthful::rows());
    place_all(&mut rows, swallow::rows());
    place_all(&mut rows, special_s::rows());
    place_all(&mut rows, special_hi::rows());
    place_all(&mut rows, special_lw::rows());
    rows
}

const fn place_all<const N: usize>(
    rows: &mut [MotionRow; SPECIAL_ROW_COUNT],
    ported: [MotionRow; N],
) {
    let mut i = 0;
    while i < N {
        rows[(ported[i].action.0 - FIRST_ACTION) as usize] = ported[i];
        i += 1;
    }
}

/// ftKb_Init_MotionStateTable move IDs (the table's third word): the
/// multijumps are FtMoveId_Default, the dash attack's two rows its own, then
/// each special's rows. The copied specials' ids (FtMoveId_KbSpecialNMr and
/// on, 0x16..0x2F) arrive with their rows.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    let mut i = MULTI_JUMP_ROWS;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = match i {
            10..=11 => Some(M::Dash),
            12..=41 | 75..=76 => Some(M::SpecialNeutral),
            42..=43 => Some(M::SpecialSide),
            44..=51 => Some(M::SpecialUp),
            52..=57 => Some(M::SpecialDown),
            _ => None,
        };
        i += 1;
    }
    moves
};

/// ftKb_Init_MotionStateTable[i].x4_flags (0x803C8368, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    0x0000006A, 0x0000006A, 0x0000006A, 0x0000006A, 0x0000006A, 0x0000006A, 0x0000006A, 0x0000006A,
    0x0000006A, 0x0000006A, 0x00240005, 0x00240405, 0x00340011, 0x003C0011, 0x00340011, 0x00340011,
    0x00340011, 0x00340011, 0x00340011, 0x00344011, 0x00344011, 0x00344011, 0x00342011, 0x00348011,
    0x00340011, 0x00340011, 0x00340011, 0x00340011, 0x00340011, 0x00340011, 0x00340411, 0x003C0411,
    0x00340411, 0x00340411, 0x00340411, 0x00340411, 0x00340411, 0x00340411, 0x00340411, 0x00340411,
    0x00340411, 0x00342411, 0x00340012, 0x00340412, 0x00340013, 0x00340013, 0x00340013, 0x00340013,
    0x00340413, 0x00340413, 0x00340413, 0x00340413, 0x00340214, 0x00340214, 0x00340214, 0x00340614,
    0x00340614, 0x00340614, 0x00340015, 0x00340415, 0x0034011A, 0x003C011A, 0x0034011A, 0x0034051A,
    0x003C051A, 0x0034051A, 0x00340120, 0x00340120, 0x00340120, 0x00340120, 0x00340520, 0x00340520,
    0x00340021, 0x00340021, 0x00340021, 0x00340021, 0x00340021, 0x00340421, 0x00340421, 0x00340421,
    0x00340421, 0x00340421, 0x00340116, 0x003C0116, 0x00340116, 0x00340516, 0x003C0516, 0x00340516,
    0x0034011F, 0x0034051F, 0x00340024, 0x00340424, 0x00340217, 0x00340617, 0x0034011C, 0x0034011C,
    0x0034011C, 0x0034011C, 0x0034051C, 0x0034051C, 0x0034051C, 0x0034051C, 0x00340019, 0x003C0019,
    0x00340019, 0x00340419, 0x003C0419, 0x00340419, 0x0034001D, 0x0034001D, 0x0034041D, 0x0034041D,
    0x0034011E, 0x0034051E, 0x00340218, 0x00340218, 0x00340218, 0x00340218, 0x00340218, 0x00340618,
    0x00340618, 0x00340618, 0x00340618, 0x00340618, 0x00340026, 0x00340426, 0x0034011B, 0x003C011B,
    0x0034011B, 0x0034011B, 0x0034051B, 0x003C051B, 0x0034051B, 0x0034051B, 0x00340222, 0x00340222,
    0x003C0222, 0x003C0222, 0x00340222, 0x00340222, 0x00340222, 0x00340222, 0x00340622, 0x00340622,
    0x003C0622, 0x003C0622, 0x00340622, 0x00340222, 0x00340622, 0x00340622, 0x00340622, 0x00340225,
    0x00340225, 0x00340225, 0x00340225, 0x00340625, 0x00340625, 0x00340625, 0x00340625, 0x00340123,
    0x003C0123, 0x003C0123, 0x00340123, 0x00340123, 0x00340523, 0x003C0523, 0x003C0523, 0x00340523,
    0x00340523, 0x0034012B, 0x0034052B, 0x00340028, 0x00340428, 0x00340127, 0x003C0127, 0x00340127,
    0x00340527, 0x003C0527, 0x00340527, 0x00340129, 0x003C0129, 0x00340129, 0x00340529, 0x003C0529,
    0x00340529, 0x0034012A, 0x0034052A, 0x0034022C, 0x0034062C, 0x0034022D, 0x0034022D, 0x0034022D,
    0x0034022D, 0x0034062D, 0x0034062D, 0x0034062D, 0x0034062D, 0x0034002E, 0x003C002E, 0x0034002E,
    0x0034042E, 0x003C042E, 0x0034042E,
];
