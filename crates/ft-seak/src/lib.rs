//! Sheik: ft/kinds/ftSeak. Common states live in melee-ft; Zelda, her
//! transformation partner, lives in ft-zelda.
pub mod attributes;
mod common;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use melee_ft::fighter::{state, state::callbacks, ActionId, MotionRow};

/// ftSk_MS_SelfCount: rows 341..364, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 24;
const FIRST_ACTION: u16 = 341;

/// ftSk_Init_MotionStateTable (ftseak.c). Unported rows fail closed.
pub const fn special_rows() -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    use special_n as n;
    let needles = [
        (
            n::GROUND_START,
            n::start_anim as melee_ft::fighter::state::AnimFn,
            common::no_input as melee_ft::fighter::state::InputFn,
            false,
            n::charge_ground_collision as melee_ft::fighter::state::CollisionFn,
        ),
        (
            n::GROUND_LOOP,
            n::loop_anim,
            n::loop_input,
            false,
            n::charge_ground_collision,
        ),
        (
            n::GROUND_CANCEL,
            n::cancel_anim,
            common::no_input,
            false,
            n::release_ground_collision,
        ),
        (
            n::GROUND_END,
            n::end_anim,
            common::no_input,
            false,
            n::release_ground_collision,
        ),
        (
            n::AIR_START,
            n::start_anim,
            common::no_input,
            true,
            n::charge_air_collision,
        ),
        (
            n::AIR_LOOP,
            n::loop_anim,
            n::loop_input,
            true,
            n::charge_air_collision,
        ),
        (
            n::AIR_CANCEL,
            n::cancel_anim,
            common::no_input,
            true,
            n::release_air_collision,
        ),
        (
            n::AIR_END,
            n::end_anim,
            common::no_input,
            true,
            n::release_air_collision,
        ),
    ];
    let mut i = 0;
    while i < needles.len() {
        let (action, anim, iasa, air, collision) = needles[i];
        let physics = if air {
            n::air_physics as melee_ft::fighter::state::PhysicsFn
        } else {
            n::ground_physics
        };
        place(
            &mut rows,
            common::row(action, n::ANIMATIONS[i], anim, iasa, physics, collision),
        );
        i += 1;
    }
    use special_hi as hi;
    let vanish = [
        (
            hi::GROUND_START,
            hi::start_anim as melee_ft::fighter::state::AnimFn,
            callbacks::physics::guard_on as melee_ft::fighter::state::PhysicsFn,
            hi::start_ground_collision as melee_ft::fighter::state::CollisionFn,
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
    while i < vanish.len() {
        let (action, anim, physics, collision) = vanish[i];
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
    use melee_ft::fighter::state::{AnimFn, CollisionFn, InputFn, PhysicsFn};
    use special_s as s;
    let chain: [(ActionId, AnimFn, InputFn, PhysicsFn, CollisionFn); 6] = [
        (
            s::GROUND_START,
            s::start_anim,
            common::no_input,
            s::ground_physics,
            s::start_ground_collision,
        ),
        (
            s::GROUND_LOOP,
            s::loop_anim,
            s::loop_input,
            s::ground_physics,
            s::loop_ground_collision,
        ),
        (
            s::GROUND_END,
            s::end_anim,
            common::no_input,
            s::ground_physics,
            s::end_ground_collision,
        ),
        (
            s::AIR_START,
            s::start_anim,
            common::no_input,
            s::start_air_physics,
            s::start_air_collision,
        ),
        (
            s::AIR_LOOP,
            s::loop_anim,
            s::loop_input,
            s::air_physics,
            s::loop_air_collision,
        ),
        (
            s::AIR_END,
            s::end_anim,
            common::no_input,
            s::air_physics,
            s::end_air_collision,
        ),
    ];
    let mut i = 0;
    while i < chain.len() {
        let (action, anim, iasa, physics, collision) = chain[i];
        place(
            &mut rows,
            common::row(action, s::ANIMATIONS[i], anim, iasa, physics, collision),
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

/// ftSk_Init_MotionStateTable move IDs: needles (341..348), chain
/// (349..354), vanish (355..360), transform (361..364).
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [Some(M::SpecialNeutral); SPECIAL_ROW_COUNT];
    let mut i = 8;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(if i < 14 {
            M::SpecialSide
        } else if i < 20 {
            M::SpecialUp
        } else {
            M::SpecialDown
        });
        i += 1;
    }
    moves
};
