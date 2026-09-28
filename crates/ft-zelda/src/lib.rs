//! Zelda: ft/kinds/ftZelda. Common states live in melee-ft; Sheik, her
//! transformation partner, lives in ft-seak.
pub mod attributes;
mod common;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

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
    use melee_ft::fighter::state::{callbacks, AnimFn, CollisionFn, PhysicsFn};
    use special_n as n;
    place(
        &mut rows,
        common::row(
            n::GROUND,
            n::ANIMATIONS[0],
            n::anim,
            common::no_input,
            n::ground_physics,
            n::ground_collision,
        ),
    );
    place(
        &mut rows,
        common::row(
            n::AIR,
            n::ANIMATIONS[1],
            n::anim,
            common::no_input,
            n::air_physics,
            n::air_collision,
        ),
    );
    use melee_ft::fighter::state::InputFn;
    use special_s as s;
    let dins_fire: [(ActionId, AnimFn, InputFn, PhysicsFn, CollisionFn); 6] = [
        (s::GROUND_START, s::start_anim, common::no_input, s::start_ground_physics, s::ground_collision),
        (s::GROUND_LOOP, s::loop_anim, s::loop_input, callbacks::physics::guard_on, s::ground_collision),
        (s::GROUND_END, s::end_anim, common::no_input, callbacks::physics::guard_on, s::ground_collision),
        (s::AIR_START, s::start_anim, common::no_input, s::air_physics, s::air_collision),
        (s::AIR_LOOP, s::loop_anim, s::loop_input, s::air_physics, s::air_collision),
        (s::AIR_END, s::end_anim, common::no_input, s::air_physics, s::air_collision),
    ];
    let mut i = 0;
    while i < dins_fire.len() {
        let (action, anim, input, physics, collision) = dins_fire[i];
        place(
            &mut rows,
            common::row(action, s::ANIMATIONS[i], anim, input, physics, collision),
        );
        i += 1;
    }
    use special_hi as hi;
    let farores_wind: [(ActionId, AnimFn, PhysicsFn, CollisionFn); 6] = [
        (
            hi::GROUND_START,
            hi::start_anim,
            callbacks::physics::guard_on,
            hi::start_ground_collision,
        ),
        (
            hi::GROUND_TRAVEL,
            hi::travel_anim,
            hi::travel_ground_physics,
            hi::travel_ground_collision,
        ),
        (
            hi::GROUND_END,
            hi::end_anim,
            hi::end_ground_physics,
            hi::end_ground_collision,
        ),
        (
            hi::AIR_START,
            hi::start_anim,
            hi::start_air_physics,
            hi::start_air_collision,
        ),
        (
            hi::AIR_TRAVEL,
            hi::travel_anim,
            hi::travel_air_physics,
            hi::travel_air_collision,
        ),
        (
            hi::AIR_END,
            hi::end_air_anim,
            hi::end_air_physics,
            hi::end_air_collision,
        ),
    ];
    let mut i = 0;
    while i < farores_wind.len() {
        let (action, anim, physics, collision) = farores_wind[i];
        place(
            &mut rows,
            common::row(
                action,
                hi::ANIMATIONS[i],
                anim,
                common::no_input,
                physics,
                collision,
            ),
        );
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
