//! The Ice Climbers: Popo (ft/kinds/ftPopo) and Nana (ft/kinds/ftNana).
//! One player's character creates both fighters (Player_80031AD0); Nana is
//! the player's partner fighter (x221F_b4) and runs the same motion table
//! (ftNn_Init_MotionStateTable repeats ftPp_Init_MotionStateTable). Common
//! states live in melee-ft; Nana's follow logic lives in melee-cpu.
pub mod attributes;
pub mod climber;
pub mod init;
pub mod partner;
pub mod special;
pub mod special_hi;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftPp_MS_SelfCount: rows 341..366, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 26;
const FIRST_ACTION: u16 = 341;

/// ftPp_Init_MotionStateTable (ftpopo.c), shared by Nana. A row not yet
/// ported fails closed with its retail index.
pub const SPECIAL_ROWS: [MotionRow; SPECIAL_ROW_COUNT] = {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    let ice_shot = special_n::ROWS;
    rows[0] = ice_shot[0];
    rows[1] = ice_shot[1];
    let squall = special_s::rows();
    let mut i = 0;
    while i < squall.len() {
        rows[2 + i] = squall[i];
        i += 1;
    }
    let partner_squall = special_s::partner::rows();
    rows[18] = partner_squall[0];
    rows[19] = partner_squall[1];
    let belay = special_hi::ROWS;
    let mut i = 0;
    while i < belay.len() {
        rows[(belay[i].action.0 - FIRST_ACTION) as usize] = belay[i];
        i += 1;
    }
    let partner = special_hi::partner::ROWS;
    let mut i = 0;
    while i < partner.len() {
        rows[(partner[i].action.0 - FIRST_ACTION) as usize] = partner[i];
        i += 1;
    }
    rows
};

/// ftPp_Init_MotionStateTable's x4_flags column (read from the DOL at
/// 0x803CD2D4, every 0x20 bytes): each row's class nibble is 3.
pub const SPECIAL_MOTION_FLAGS: [u32; SPECIAL_ROW_COUNT] = [
    0x0034_0111, // SpecialN
    0x0034_0511, // SpecialAirN
    0x0034_0212, // SpecialS1
    0x0034_0212, // SpecialS2
    0x0034_0612, // SpecialAirS1
    0x0034_0612, // SpecialAirS2
    0x0034_0213, // SpecialHiStart_0
    0x0034_0213, // SpecialHiThrow_0
    0x0034_0213, // SpecialHiThrow2
    0x0034_0213, // SpecialHiStart_1
    0x0034_0213, // SpecialHiThrow_1
    0x0034_0613, // SpecialAirHiStart_0
    0x0034_0613, // SpecialAirHiThrow_0
    0x0034_0613, // SpecialAirHiThrow2
    0x0034_0613, // SpecialAirHiStart_1
    0x0034_0613, // SpecialAirHiThrow_1
    0x0034_0014, // SpecialLw
    0x0034_0414, // SpecialAirLw
    0x0034_0212, // SpecialS_0
    0x0034_0612, // SpecialS_1
    0x0034_0213, // SpecialHi_0
    0x0034_0213, // SpecialHi_1
    0x0034_0213, // SpecialHi_2
    0x0034_0613, // SpecialHi_3
    0x0034_0613, // SpecialHi_4
    0x0034_0613, // SpecialHi_5
];

/// ftPp_Init_MotionStateTable's x9_b0 (bit 23 of the packed word): the
/// neutral, side and down specials (0x803CD2D0, read from the DOL).
pub const SPECIAL_PARTNER_SYNC: [bool; SPECIAL_ROW_COUNT] = {
    let mut rows = [false; SPECIAL_ROW_COUNT];
    let synced = [341, 342, 343, 344, 345, 346, 357, 358, 359, 360];
    let mut i = 0;
    while i < synced.len() {
        rows[synced[i] - FIRST_ACTION as usize] = true;
        i += 1;
    }
    rows
};

/// ftPp_Init_MotionStateTable move IDs (the packed word's top byte).
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    [
        Some(M::SpecialNeutral), // SpecialN
        Some(M::SpecialNeutral), // SpecialAirN
        Some(M::SpecialSide),    // SpecialS1
        Some(M::SpecialSide),    // SpecialS2
        Some(M::SpecialSide),    // SpecialAirS1
        Some(M::SpecialSide),    // SpecialAirS2
        Some(M::SpecialUp),      // SpecialHiStart_0
        Some(M::SpecialUp),      // SpecialHiThrow_0
        Some(M::SpecialUp),      // SpecialHiThrow2
        Some(M::SpecialUp),      // SpecialHiStart_1
        Some(M::SpecialUp),      // SpecialHiThrow_1
        Some(M::SpecialUp),      // SpecialAirHiStart_0
        Some(M::SpecialUp),      // SpecialAirHiThrow_0
        Some(M::SpecialUp),      // SpecialAirHiThrow2
        Some(M::SpecialUp),      // SpecialAirHiStart_1
        Some(M::SpecialUp),      // SpecialAirHiThrow_1
        Some(M::SpecialDown),    // SpecialLw
        Some(M::SpecialDown),    // SpecialAirLw
        Some(M::SpecialSide),    // SpecialS_0
        Some(M::SpecialSide),    // SpecialS_1
        Some(M::SpecialUp),      // SpecialHi_0
        Some(M::SpecialUp),      // SpecialHi_1
        Some(M::SpecialUp),      // SpecialHi_2
        Some(M::SpecialUp),      // SpecialHi_3
        Some(M::SpecialUp),      // SpecialHi_4
        Some(M::SpecialUp),      // SpecialHi_5
    ]
};
