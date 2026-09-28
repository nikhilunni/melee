//! Luigi: ft/kinds/ftLuigi. Common states live in melee-ft. Retail shares no
//! code with Mario: every special is its own ftLg_* function.
pub mod attributes;
mod common;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftLg_MS_SelfCount: rows 341..358, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 18;
const FIRST_ACTION: u16 = common::FIRST_ACTION;

/// ftLg_Init_MotionStateTable (ftluigi.c).
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    let n = special_n::rows();
    let s = special_s::rows();
    let hi = special_hi::rows();
    let lw = special_lw::rows();
    let mut i = 0;
    while i < n.len() {
        place(&mut rows, n[i]);
        i += 1;
    }
    let mut i = 0;
    while i < s.len() {
        place(&mut rows, s[i]);
        i += 1;
    }
    let mut i = 0;
    while i < 2 {
        place(&mut rows, hi[i]);
        place(&mut rows, lw[i]);
        i += 1;
    }
    rows
}

const fn place(rows: &mut [MotionRow; SPECIAL_ROW_COUNT], row: MotionRow) {
    rows[(row.action.0 - FIRST_ACTION) as usize] = row;
}

/// ftLg_Init_MotionStateTable move IDs.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let n = Some(M::SpecialNeutral);
    let s = Some(M::SpecialSide);
    let hi = Some(M::SpecialUp);
    let lw = Some(M::SpecialDown);
    [n, n, s, s, s, s, s, s, s, s, s, s, s, s, hi, hi, lw, lw]
};
