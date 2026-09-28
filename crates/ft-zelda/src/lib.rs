//! Zelda: ft/kinds/ftZelda. Common states live in melee-ft; Sheik, her
//! transformation partner, lives in ft-seak.
pub mod attributes;
mod common;
pub mod init;
pub mod special_lw;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftZd_MS_SelfCount: rows 341..358, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 18;
const FIRST_ACTION: u16 = 341;

/// ftZd_Init_MotionStateTable (ftzelda.c). Unported rows fail closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    use special_lw as lw;
    let transform = [
        (
            lw::GROUND,
            lw::anim as melee_ft::fighter::state::AnimFn,
            false,
        ),
        (lw::GROUND_ARRIVAL, lw::arrival_anim, false),
        (lw::AIR, lw::anim, true),
        (lw::AIR_ARRIVAL, lw::arrival_anim, true),
    ];
    let mut i = 0;
    while i < transform.len() {
        let (action, anim, air) = transform[i];
        place(
            &mut rows,
            common::row(
                action,
                lw::ANIMATIONS[i],
                anim,
                common::no_input,
                if air {
                    lw::air_physics
                } else {
                    lw::ground_physics
                },
                if air {
                    lw::air_collision
                } else {
                    lw::ground_collision
                },
            ),
        );
        i += 1;
    }
    rows
}

const fn place(rows: &mut [MotionRow; SPECIAL_ROW_COUNT], row: MotionRow) {
    rows[(row.action.0 - FIRST_ACTION) as usize] = row;
}

/// ftZd_Init_MotionStateTable move IDs: Nayru's Love (341, 342), Din's
/// Fire (343..348), Farore's Wind (349..354), transform (355..358).
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [Some(M::SpecialNeutral); SPECIAL_ROW_COUNT];
    let mut i = 2;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(if i < 8 {
            M::SpecialSide
        } else if i < 14 {
            M::SpecialUp
        } else {
            M::SpecialDown
        });
        i += 1;
    }
    moves
};
