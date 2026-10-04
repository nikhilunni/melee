//! Bowser: ft/kinds/ftKoopa. Common states live in melee-ft.
pub mod attributes;
pub mod init;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftKp_MS_SelfCount: rows 341..363, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 23;
const FIRST_ACTION: u16 = 341;

/// ftKp_Init_MotionStateTable (ftkoopa.c, 0x803CEDC0). Unported rows fail
/// closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    rows
}

/// ftKp_Init_MotionStateTable[i].x4_flags (0x803CEDC0, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    0x00340011, 0x003C0011, 0x00340011, 0x00340411, 0x003C0411, 0x00340411, 0x00340012, 0x00340012,
    0x00340012, 0x00340012, 0x00340012, 0x00340012, 0x00340412, 0x00340412, 0x00340412, 0x00340012,
    0x00340412, 0x00340412, 0x00340213, 0x00340613, 0x00340214, 0x00340614, 0x00340214,
];

/// Table index of each move's first row.
const SPECIAL_S: usize = 6;
const SPECIAL_HI: usize = 18;
const SPECIAL_LW: usize = 20;

/// ftKp_Init_MotionStateTable's move-id column: six Fire Breath rows,
/// twelve Koopa Klaw rows, two Whirling Fortress rows, three Bowser Bomb
/// rows.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(if i < SPECIAL_S {
            M::SpecialNeutral
        } else if i < SPECIAL_HI {
            M::SpecialSide
        } else if i < SPECIAL_LW {
            M::SpecialUp
        } else {
            M::SpecialDown
        });
        i += 1;
    }
    moves
};
