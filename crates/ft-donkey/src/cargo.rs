//! The cargo carry's rows, ftDk_MS_ThrowFWait0 (351) .. ftDk_MS_ThrowAirFLw
//! (368). Their callbacks are the common ftCo_Cargo* ones
//! (`melee_ft::fighter::cargo`); the table and its animations are Donkey
//! Kong's (ftDk_Init_MotionStateTable, 0x803CB838).
use crate::common::row;
use melee_ft::fighter::{
    cargo,
    state::{callbacks, MotionRow},
    ActionId,
};

/// ftDk_MS_ThrowFWait0, the first cargo row (ftDonkeyAttributes
/// x4_motion_state).
const FIRST: u16 = 351;
/// ftDk_SM_ThrowFWait0: row 351's anim_id.
const FIRST_ANIMATION: i32 = 0x131;

/// Rows 351..359 and 361..368. ftDk_MS_ThrowFWait2 (360,
/// ftCo_CargoWait2_Anim) is left out: no retail code enters it.
pub const fn rows() -> [MotionRow; 17] {
    use callbacks::physics;
    const fn at(offset: u16) -> (ActionId, i32) {
        (ActionId(FIRST + offset), FIRST_ANIMATION + offset as i32)
    }
    let ground = cargo::ground_collision;
    let friction = physics::guard_on;
    let (wait, wait_animation) = at(cargo::row::WAIT);
    let (walk_slow, walk_slow_animation) = at(cargo::row::WALK);
    let (walk_middle, walk_middle_animation) = at(cargo::row::WALK + 1);
    let (walk_fast, walk_fast_animation) = at(cargo::row::WALK + 2);
    let (turn, turn_animation) = at(cargo::row::TURN);
    let (knee_bend, knee_bend_animation) = at(cargo::row::KNEE_BEND);
    let (fall, fall_animation) = at(cargo::row::FALL);
    let (jump, jump_animation) = at(cargo::row::JUMP);
    let (landing, landing_animation) = at(cargo::row::LANDING);
    let mut rows = [
        row(wait, wait_animation, cargo::wait_anim, cargo::wait_input, friction, ground),
        row(walk_slow, walk_slow_animation, cargo::walk_anim, cargo::walk_input, physics::walk, ground),
        row(walk_middle, walk_middle_animation, cargo::walk_anim, cargo::walk_input, physics::walk, ground),
        row(walk_fast, walk_fast_animation, cargo::walk_anim, cargo::walk_input, physics::walk, ground),
        row(turn, turn_animation, cargo::turn_anim, cargo::turn_input, friction, ground),
        row(knee_bend, knee_bend_animation, cargo::knee_bend_anim, cargo::knee_bend_input, friction, ground),
        row(fall, fall_animation, cargo::air_anim, cargo::air_input, physics::pass, cargo::air_collision),
        row(jump, jump_animation, cargo::air_anim, cargo::air_input, cargo::jump_physics, cargo::air_collision),
        row(landing, landing_animation, cargo::landing_anim, cargo::no_input, friction, ground),
        // The eight throws, filled below.
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
        row(wait, 0, cargo::throw_anim, cargo::no_input, friction, ground),
    ];
    // ftCo_CargoThrowF/B/Hi/Lw: the common throws' IASA (empty) and Phys
    // (ft_80085004's root motion on the ground). The aerial rows play the
    // same four animations (ftCo_CargoThrowAir: ft_80084DB0).
    let mut direction = 0;
    while direction < 4 {
        let (state, animation) = at(cargo::row::THROW + direction);
        rows[9 + direction as usize] = row(
            state,
            animation,
            cargo::throw_anim,
            cargo::no_input,
            physics::jab,
            cargo::throw_collision,
        );
        rows[13 + direction as usize] = row(
            ActionId(state.0 + cargo::row::AIR_THROW_OFFSET),
            animation,
            cargo::throw_anim,
            cargo::no_input,
            physics::pass,
            cargo::air_throw_collision,
        );
        direction += 1;
    }
    rows
}
