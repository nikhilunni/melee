//! Donkey Kong: ft/kinds/ftDonkey. Common states live in melee-ft.
pub mod attributes;
mod cargo;
pub mod common;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

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
    place_all(&mut rows, cargo::rows());
    place_all(&mut rows, special_n::rows());
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

/// ftDk_Init_MotionStateTable move IDs (the table's third word, 0x803CB838):
/// both carries are FtMoveId_ThrowF, the cargo throws their own four, the
/// rest their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(match i {
            0..=19 => M::ThrowForward,
            20 | 24 => M::CargoThrowForward,
            21 | 25 => M::CargoThrowBack,
            22 | 26 => M::CargoThrowUp,
            23 | 27 => M::CargoThrowDown,
            28..=37 => M::SpecialNeutral,
            38..=39 => M::SpecialSide,
            40..=41 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
};

/// ftDk_Init_MotionStateTable[i].x4_flags (0x803CB838, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    // 341..350: the heavy-item carry.
    0x00480000, 0x00484066, 0x00484066, 0x00484066, 0x00482064, 0x00480000, 0x00480000, 0x00488069,
    0x00480000, 0x004A0000, // 351..360: the cargo carry.
    0x00A80035, 0x00A84035, 0x00A84035, 0x00A84035, 0x00A82035, 0x00A80035, 0x00A80035, 0x00A88035,
    0x00A80035, 0x00AA0035, // 361..368: the cargo throws, ground then air.
    0x00A40039, 0x00A4003A, 0x00A4003B, 0x00A4003C, 0x00A40039, 0x00A4003A, 0x00A4003B, 0x00A4003C,
    // 369..378: Giant Punch, ground then air.
    0x00340211, 0x00340211, 0x00340211, 0x00340211, 0x00340211, 0x00340611, 0x00340611, 0x00340611,
    0x00340611, 0x00340611, // 379..382: Headbutt and Spinning Kong.
    0x00340212, 0x00340612, 0x00340213, 0x00340613, // 383..386: Hand Slap.
    0x00340214, 0x003C0214, 0x00340214, 0x00340614,
];
