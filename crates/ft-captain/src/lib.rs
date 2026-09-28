//! Captain Falcon: ft/kinds/ftCaptain. Common movement lives in melee-ft.
pub mod attributes;
pub mod init;
pub mod special_hi;
pub mod special_hi_catch;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{
    state::{self, callbacks},
    ActionId, MotionRow,
};

/// ftCa_Init_MotionStateTable rows 341..363 (ftCa_MS_SwordSwing4 through
/// ftCa_MS_SpecialHiThrow1), animations from ftCa_SM_SwordSwing4 (295).
const ROW_COUNT: usize = 23;
const FIRST_ACTION: u16 = 341;
const FIRST_ANIMATION: i32 = 295;

/// ftCa_SM_* for a motion: the table's order, except that the animation
/// enum lists SpecialLwEndAir before SpecialAirLwEndAir while the motion
/// enum lists them the other way round (ftCaptain/forward.h:59-60, 87-88).
const fn animation(action: ActionId) -> i32 {
    let index = match action.0 - FIRST_ACTION {
        20 => 21,
        21 => 20,
        other => other,
    };
    FIRST_ANIMATION + index as i32
}

/// A ported special row.
const fn row(
    action: ActionId,
    anim: state::AnimFn,
    iasa: state::InputFn,
    physics: state::PhysicsFn,
    collision: state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: melee_types::CommonMotionState::None,
        animation: animation(action),
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

const fn place(rows: &mut [MotionRow; ROW_COUNT], row: MotionRow) {
    rows[(row.action.0 - FIRST_ACTION) as usize] = row;
}

/// Raptor Boost's IASA callbacks are empty; the Falcon Kick rows have none
/// (ftcaptain.c:199-273).
fn no_input(_: &mut melee_ft::fighter::Fighter, _: state::InputPhase<'_>) {}

/// Character rows are contiguous from ftCo_MS_Count. The item swings stay
/// explicit boundaries.
pub const fn special_rows() -> [MotionRow; ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); ROW_COUNT];
    let mut i = 0;
    while i < ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        rows[i].animation = animation(rows[i].action);
        i += 1;
    }
    use special_hi as hi;
    use special_lw as lw;
    place(
        &mut rows,
        row(hi::GROUND, hi::anim, hi::input, hi::physics, hi::collision),
    );
    place(
        &mut rows,
        row(hi::AIR, hi::anim, hi::input, hi::physics, hi::collision),
    );
    use special_hi_catch as catch;
    place(
        &mut rows,
        row(
            hi::CATCH,
            catch::catch_anim,
            catch::no_input,
            catch::catch_physics,
            catch::catch_collision,
        ),
    );
    place(
        &mut rows,
        row(
            hi::THROW,
            catch::throw_anim,
            catch::no_input,
            catch::throw_physics,
            catch::throw_collision,
        ),
    );
    use special_s as s;
    let no_iasa = no_input as state::InputFn;
    place(
        &mut rows,
        row(
            s::GROUND_START,
            s::ground_start_anim,
            no_iasa,
            s::ground_physics,
            s::ground_start_collision,
        ),
    );
    place(
        &mut rows,
        row(
            s::GROUND,
            s::ground_anim,
            no_iasa,
            s::ground_physics,
            s::ground_collision,
        ),
    );
    place(
        &mut rows,
        row(
            s::AIR_START,
            s::air_start_anim,
            no_iasa,
            s::air_start_physics,
            s::air_start_collision,
        ),
    );
    place(
        &mut rows,
        row(
            s::AIR,
            s::air_anim,
            no_iasa,
            s::air_physics,
            s::air_collision,
        ),
    );
    place(
        &mut rows,
        row(
            special_n::GROUND,
            special_n::ground_anim,
            special_n::ground_input,
            special_n::ground_physics,
            special_n::ground_collision,
        ),
    );
    place(
        &mut rows,
        row(
            special_n::AIR,
            special_n::air_anim,
            special_n::air_input,
            special_n::air_physics,
            special_n::air_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::GROUND,
            lw::ground_anim,
            no_input,
            lw::ground_physics,
            lw::ground_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::GROUND_END,
            lw::end_anim,
            no_input,
            lw::ground_end_physics,
            lw::end_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::AIR,
            lw::air_anim,
            no_input,
            lw::air_physics,
            lw::air_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::AIR_LANDING,
            lw::landing_anim,
            no_input,
            lw::landing_physics,
            lw::landing_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::AIR_END,
            lw::fall_anim,
            no_input,
            lw::air_end_physics,
            lw::air_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::GROUND_END_AIR,
            lw::end_anim,
            no_input,
            lw::ground_end_air_physics,
            lw::end_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::REBOUND,
            lw::fall_anim,
            no_input,
            lw::rebound_physics,
            callbacks::collision::air_catch_hit,
        ),
    );
    rows
}

/// ftCa_Init_MotionStateTable's FtMoveId values. The item swings' move ids
/// (FtMoveId_SwordSwing4..LipstickSwing4) have no StaleMove yet.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; ROW_COUNT] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; ROW_COUNT];
    let mut i = 6;
    while i < ROW_COUNT {
        moves[i] = Some(match i {
            6..=7 => M::SpecialNeutral,
            8..=11 => M::SpecialSide,
            12..=15 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
}
