//! Mr. Game & Watch: ft/kinds/ftGameWatch. Common states live in melee-ft.
pub mod articles;
pub mod attack;
pub mod attack_air;
pub mod attributes;
mod common;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

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
    place_all(&mut rows, attack::rows());
    place_all(&mut rows, attack_air::rows());
    place_all(&mut rows, special_n::rows());
    place_all(&mut rows, special_s::rows());
    place_all(&mut rows, special_hi::rows());
    place_all(&mut rows, special_lw::rows());
    rows
}

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

/// ftGw_Init_MotionStateTable[i].x4_flags (0x803D23E8, read from the retail
/// DOL): the x2070 word each row sets (ft_800895E0).
pub static MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    0x002C0201, 0x002C0204, 0x002C0204, 0x002C0204, 0x00240208, 0x00240A09, 0x0024060C, 0x0024060E,
    0x0024060F, 0x0000060C, 0x0000060E, 0x0000060F, 0x00340111, 0x00340511, 0x00340012, 0x00340012,
    0x00340012, 0x00340012, 0x00340012, 0x00340012, 0x00340012, 0x00340012, 0x00340012, 0x00340412,
    0x00340412, 0x00340412, 0x00340412, 0x00340412, 0x00340412, 0x00340412, 0x00340412, 0x00340412,
    0x00340013, 0x00340413, 0x003C0014, 0x00340014, 0x00340014, 0x003C0414, 0x00340414, 0x00340414,
];

/// ftGw_Init_MotionStateTable move IDs (the packed word's top byte): the
/// attack rows and their landings carry their attack's, the special rows
/// their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    moves[0] = Some(M::Jab1);
    moves[1] = Some(M::RapidJab);
    moves[2] = Some(M::RapidJab);
    moves[3] = Some(M::RapidJab);
    moves[4] = Some(M::DownTilt);
    moves[5] = Some(M::SideSmash);
    moves[6] = Some(M::NeutralAir);
    moves[7] = Some(M::BackAir);
    moves[8] = Some(M::UpAir);
    moves[9] = Some(M::NeutralAir);
    moves[10] = Some(M::BackAir);
    moves[11] = Some(M::UpAir);
    moves[12] = Some(M::SpecialNeutral);
    moves[13] = Some(M::SpecialNeutral);
    let mut i = 14;
    while i < 32 {
        moves[i] = Some(M::SpecialSide);
        i += 1;
    }
    moves[32] = Some(M::SpecialUp);
    moves[33] = Some(M::SpecialUp);
    i = 34;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(M::SpecialDown);
        i += 1;
    }
    moves
};

/// MotionState.x9_b0 (bit 23 of the packed move word): set on the twelve
/// attack and landing rows, clear on the specials.
pub const SPECIAL_PARTNER_SYNC: [bool; SPECIAL_ROW_COUNT] = {
    let mut sync = [false; SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < 12 {
        sync[i] = true;
        i += 1;
    }
    sync
};
