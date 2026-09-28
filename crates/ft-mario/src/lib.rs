//! Mario: ft/kinds/ftMario. Common states live in melee-ft.
pub mod attributes;
pub mod init;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftMr_MS_SelfCount: rows 341..350, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 10;
const FIRST_ACTION: u16 = 341;

/// ftMr_Init_MotionStateTable (ftmario.c). AppealSR/AppealSL (341/342) are
/// Dr. Mario's taunt rows with no callbacks; Mario never enters them.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    rows
}

/// ftMr_Init_MotionStateTable move IDs: the taunt rows are
/// FtMoveId_Default, the rest their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    [
        None,
        None,
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialDown),
        Some(M::SpecialDown),
    ]
};
