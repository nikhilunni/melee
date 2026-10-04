//! Mewtwo: ft/kinds/ftMewtwo. Common states live in melee-ft.
pub mod attributes;
pub mod init;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftMt_MS_SelfCount: rows 341..360, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 20;
const FIRST_ACTION: u16 = 341;

/// ftMt_Init_MotionStateTable (ftmewtwo.c, 0x803D0B00). Unported rows fail
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

/// ftMt_Init_MotionStateTable move IDs (the table's third word): Shadow
/// Ball's ten rows, Confusion's two, Teleport's six and Disable's two.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(match i {
            0..=9 => M::SpecialNeutral,
            10..=11 => M::SpecialSide,
            12..=17 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
};

/// ftMt_Init_MotionStateTable[i].x4_flags (0x803D0B00, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    // 341..350: Shadow Ball, ground then air (start, loop, full loop,
    // cancel, end).
    0x00340111, 0x003C0111, 0x003C0111, 0x00340111, 0x00340111, 0x00340511, 0x003C0511, 0x003C0511,
    0x00340511, 0x00340511, // 351..352: Confusion.
    0x00341012, 0x00341412, // 353..358: Teleport (start, lost, travel), ground then air.
    0x00340013, 0x00340013, 0x00340013, 0x00340413, 0x00340413, 0x00340413,
    // 359..360: Disable.
    0x00340114, 0x00340514,
];
