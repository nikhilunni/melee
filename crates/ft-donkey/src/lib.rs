//! Donkey Kong: ft/kinds/ftDonkey. Common states live in melee-ft.
pub mod attributes;
pub mod init;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftDk_MS_SelfCount: rows 341..386, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 46;
const FIRST_ACTION: u16 = 341;

/// ftDk_Init_MotionStateTable (ftdonkey.c, 0x803CB838). Unported rows fail
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

/// ftDk_Init_MotionStateTable[i].x4_flags (0x803CB838, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    // 341..350: the heavy-item carry.
    0x00480000, 0x00484066, 0x00484066, 0x00484066, 0x00482064, 0x00480000, 0x00480000, 0x00488069,
    0x00480000, 0x004A0000,
    // 351..360: the cargo carry.
    0x00A80035, 0x00A84035, 0x00A84035, 0x00A84035, 0x00A82035, 0x00A80035, 0x00A80035, 0x00A88035,
    0x00A80035, 0x00AA0035,
    // 361..368: the cargo throws, ground then air.
    0x00A40039, 0x00A4003A, 0x00A4003B, 0x00A4003C, 0x00A40039, 0x00A4003A, 0x00A4003B, 0x00A4003C,
    // 369..378: Giant Punch, ground then air.
    0x00340211, 0x00340211, 0x00340211, 0x00340211, 0x00340211, 0x00340611, 0x00340611, 0x00340611,
    0x00340611, 0x00340611,
    // 379..382: Headbutt and Spinning Kong.
    0x00340212, 0x00340612, 0x00340213, 0x00340613,
    // 383..386: Hand Slap.
    0x00340214, 0x003C0214, 0x00340214, 0x00340614,
];
