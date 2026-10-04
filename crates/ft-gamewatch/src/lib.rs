//! Mr. Game & Watch: ft/kinds/ftGameWatch. Common states live in melee-ft.
pub mod attributes;
pub mod init;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftGw_MS_SelfCount: rows 341..380, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 40;
pub(crate) const FIRST_ACTION: u16 = 341;

/// ftGw_Init_MotionStateTable (ftgamewatch.c). Unported rows fail closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    rows
}

#[allow(dead_code)]
pub(crate) const fn place_all<const N: usize>(
    rows: &mut [MotionRow; SPECIAL_ROW_COUNT],
    ported: [MotionRow; N],
) {
    let mut i = 0;
    while i < N {
        rows[(ported[i].action.0 - FIRST_ACTION) as usize] = ported[i];
        i += 1;
    }
}
