//! Ness: ft/kinds/ftNess. Common states live in melee-ft.
pub mod attack_s4;
pub mod attributes;
pub mod common;
pub mod init;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftNs_MS_SelfCount: rows 341..376, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 36;
const FIRST_ACTION: u16 = 341;

/// ftNs_Init_MotionStateTable (ftness.c, 0x803CC650). Unported rows fail
/// closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    place_all(&mut rows, attack_s4::rows());
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

/// ftNs_Init_MotionStateTable move IDs (the table's third word): the three
/// smashes, then each special's rows.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(match i {
            0 => M::SideSmash,
            1..=3 => M::UpSmash,
            4..=6 => M::DownSmash,
            7..=14 => M::SpecialNeutral,
            15..=16 => M::SpecialSide,
            17..=25 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
};

/// ftNs_Init_MotionStateTable[i].x4_flags (0x803CC650, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    // 341: AttackS4, the bat.
    0x00241A09,
    // 342..347: the yo-yo smashes (start, charge, release), up then down.
    0x00240A0A, 0x0024080A, 0x0024080A, 0x00240A0B, 0x0024080B, 0x0024080B,
    // 348..355: PK Flash, ground then air.
    0x00340111, 0x00340111, 0x00340111, 0x00340111, 0x00340511, 0x00340511, 0x00340511, 0x00340511,
    // 356..357: PK Fire.
    0x00340112, 0x00340512,
    // 358..366: PK Thunder, ground, air, and the rebound.
    0x00340113, 0x00340113, 0x00340113, 0x00340113, 0x00340513, 0x00340513, 0x00340513, 0x00340513,
    0x00340113,
    // 367..376: PSI Magnet, ground then air.
    0x00340014, 0x003C0014, 0x00340014, 0x00340014, 0x00340014, 0x00340414, 0x003C0414, 0x00340414,
    0x00340414, 0x00340414,
];
