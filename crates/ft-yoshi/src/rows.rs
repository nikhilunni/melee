//! ftYs_Init_MotionStateTable (ftyoshi.c), actions 341..368.
use melee_ft::fighter::{state, ActionId, MotionRow};
use melee_types::combat::StaleMove;

/// ftYs_MS_SelfCount.
pub const COUNT: usize = 28;
/// Table index of each move's first row.
const SPECIAL_N: usize = 5;
const SPECIAL_S: usize = 15;
const SPECIAL_HI: usize = 23;
const SPECIAL_LW: usize = 25;

pub const fn rows() -> [MotionRow; COUNT] {
    use melee_types::CommonMotionState as S;
    let mut rows = [state::unimplemented_row(); COUNT];
    let mut i = 0;
    while i < COUNT {
        rows[i].action = ActionId(341 + i as u16);
        i += 1;
    }
    // ftYs_MS_GuardOn_0..GuardOn_1 (341..345): the egg shield.
    let guard = [
        S::GuardOn,
        S::Guard,
        S::GuardOff,
        S::GuardSetOff,
        S::GuardReflect,
    ];
    i = 0;
    while i < guard.len() {
        rows[i] = MotionRow {
            action: ActionId(341 + i as u16),
            ..state::COMMON[guard[i] as usize]
        };
        i += 1;
    }
    let n = crate::special_n::rows();
    i = 0;
    while i < n.len() {
        rows[SPECIAL_N + i] = n[i];
        i += 1;
    }
    let s = crate::special_s::rows();
    i = 0;
    while i < s.len() {
        rows[SPECIAL_S + i] = s[i];
        i += 1;
    }
    let hi = crate::special_hi::rows();
    i = 0;
    while i < hi.len() {
        rows[SPECIAL_HI + i] = hi[i];
        i += 1;
    }
    let lw = crate::special_lw::rows();
    i = 0;
    while i < lw.len() {
        rows[SPECIAL_LW + i] = lw[i];
        i += 1;
    }
    rows
}

/// The table's FtMoveId column: Default for the shield rows.
pub const fn moves() -> [Option<StaleMove>; COUNT] {
    let mut moves = [None; COUNT];
    let mut i = SPECIAL_N;
    while i < COUNT {
        moves[i] = Some(if i < SPECIAL_S {
            StaleMove::SpecialNeutral
        } else if i < SPECIAL_HI {
            StaleMove::SpecialSide
        } else if i < SPECIAL_LW {
            StaleMove::SpecialUp
        } else {
            StaleMove::SpecialDown
        });
        i += 1;
    }
    moves
}
