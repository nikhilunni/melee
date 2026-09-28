//! Captain Falcon: ft/kinds/ftCaptain. Common movement lives in melee-ft.
pub mod attributes;
pub mod init;
pub mod special_n;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftCa_Init_MotionStateTable rows 341..363 (ftCa_MS_SwordSwing4 through
/// ftCa_MS_SpecialHiThrow1), animations from ftCa_SM_SwordSwing4 (295).
const ROW_COUNT: usize = 23;
const FIRST_ANIMATION: i32 = 295;

/// Character rows are contiguous from ftCo_MS_Count. Only Falcon Punch is
/// ported; the item swings and other specials stay explicit boundaries.
pub const fn special_rows() -> [MotionRow; ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); ROW_COUNT];
    let mut i = 0;
    while i < ROW_COUNT {
        rows[i].action = ActionId(341 + i as u16);
        rows[i].animation = FIRST_ANIMATION + i as i32;
        i += 1;
    }
    rows[6] = MotionRow {
        action: special_n::GROUND,
        id: melee_types::CommonMotionState::None,
        animation: FIRST_ANIMATION + 6,
        anim: special_n::ground_anim,
        iasa: special_n::ground_input,
        physics: special_n::ground_physics,
        collision: special_n::ground_collision,
        camera: state::callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[7] = MotionRow {
        action: special_n::AIR,
        id: melee_types::CommonMotionState::None,
        animation: FIRST_ANIMATION + 7,
        anim: special_n::air_anim,
        iasa: special_n::air_input,
        physics: special_n::air_physics,
        collision: special_n::air_collision,
        camera: state::callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows
}

/// ftCa_Init_MotionStateTable's FtMoveId values. The item swings' move ids
/// (FtMoveId_SwordSwing4..LipstickSwing4) have no StaleMove yet.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; ROW_COUNT] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; ROW_COUNT];
    let mut i = 6;
    while i < ROW_COUNT {
        moves[i] = Some(match i {
            6..=7 => M::SpecialNeutral,
            8..=11 => M::SpecialSide,
            12..=15 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
}
