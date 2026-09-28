//! Mario: ft/kinds/ftMario. Common states live in melee-ft.
pub mod attributes;
mod common;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{state, ActionId, MotionRow};

/// ftMr_MS_SelfCount: rows 341..350, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 10;
const FIRST_ACTION: u16 = 341;

/// ftMr_Init_MotionStateTable (ftmario.c). AppealSR/AppealSL (341/342) are
/// Dr. Mario's taunt rows with no callbacks; Mario never enters them.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    place(
        &mut rows,
        common::row(
            special_n::GROUND,
            special_n::anim,
            special_n::ground_input,
            special_n::ground_physics,
            special_n::ground_collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_n::AIR,
            special_n::anim,
            special_n::air_input,
            special_n::air_physics,
            special_n::air_collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_hi::GROUND,
            special_hi::anim,
            special_hi::input,
            special_hi::ground_physics,
            special_hi::collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_hi::AIR,
            special_hi::anim,
            special_hi::input,
            special_hi::air_physics,
            special_hi::collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_lw::GROUND,
            special_lw::ground_anim,
            special_lw::input,
            special_lw::ground_physics,
            special_lw::ground_collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_lw::AIR,
            special_lw::air_anim,
            special_lw::input,
            special_lw::air_physics,
            special_lw::air_collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_s::GROUND,
            special_s::anim,
            special_s::input,
            special_s::ground_physics,
            special_s::ground_collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_s::AIR,
            special_s::anim,
            special_s::input,
            special_s::air_physics,
            special_s::air_collision,
        ),
    );
    rows
}

const fn place(rows: &mut [MotionRow; SPECIAL_ROW_COUNT], row: MotionRow) {
    rows[(row.action.0 - FIRST_ACTION) as usize] = row;
}

/// ftMr_Init_MotionStateTable move IDs: the taunt rows are
/// FtMoveId_Default, the rest their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    [
        None,
        None,
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialDown),
        Some(M::SpecialDown),
    ]
};
