//! Samus: ft/kinds/ftSamus. Common states live in melee-ft.
pub mod attributes;
pub mod common;
pub mod escape;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftSs_MS_SelfCount: rows 341..358, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 18;
const FIRST_ACTION: u16 = 341;

/// ftSs_Init_MotionStateTable (ftsamus.c). Unported rows fail closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    place_all(&mut rows, special_hi::rows());
    place_all(&mut rows, special_n::rows());
    place_all(&mut rows, special_s::rows());
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

/// ftSs_Init_MotionStateTable[i].x4_flags (0x803CE2D0, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    0x00000000, 0x00000000, 0x00340111, 0x00340111, 0x00340111, 0x00340111, 0x00340511, 0x00340511,
    0x00340112, 0x00340912, 0x00340512, 0x00340D12, 0x00340213, 0x00340613, 0x00340114, 0x00340514,
    0x00200000, 0x00C00000,
];

/// ftSs_Init_MotionStateTable move IDs: the bomb-jump and air-catch rows
/// are FtMoveId_Default, the rest their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    [
        None,
        None,
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialDown),
        Some(M::SpecialDown),
        None,
        None,
    ]
};
