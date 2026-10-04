//! The yo-yo up smash, ftnessattackhi4.c (80115BB0..80116544): rows 342
//! (start), 343 (charge) and 344 (release).
use super::{
    anchored_physics, charge_anim, charge_input, collision, enter_start, release_anim,
    release_physics, start_anim, start_input, ANIMATIONS, UP_CHARGE, UP_RELEASE, UP_START,
};
use crate::common::{self, row};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, InputPhase, MotionRow, PhysicsPhase},
        Fighter,
    },
};

/// ftNs_AttackHi4Release_Phys: the yo-yo eases to the anchor by 0.1 a
/// frame until frame 24.
const EASE_UNTIL: i32 = 24;
const EASE_RATE: f32 = 0.1;

pub const fn rows() -> [MotionRow; 3] {
    [
        row(UP_START, ANIMATIONS[0], anim, input, anchored_physics, collision),
        row(
            UP_CHARGE,
            ANIMATIONS[1],
            charge_animation,
            charge_iasa,
            common::ground_friction,
            collision,
        ),
        row(
            UP_RELEASE,
            ANIMATIONS[2],
            release_animation,
            release_iasa,
            physics_release,
            collision,
        ),
    ]
}

/// ftNs_AttackHi4_Enter (80115BB0).
pub fn enter(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    enter_start(f, UP_START, assets)
}

/// ftNs_AttackHi4_Anim (80115C9C).
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    start_anim(f, UP_CHARGE, p.assets, p.map)?;
    Ok(None)
}

/// ftNs_AttackHi4_IASA (80115E74).
fn input(f: &mut Fighter, p: InputPhase<'_>) {
    start_input(f, p);
}

/// ftNs_AttackHi4Charge_Anim (80115F88).
fn charge_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    charge_anim(f, UP_RELEASE, p.assets)?;
    Ok(None)
}

/// ftNs_AttackHi4Charge_IASA (801160B4).
fn charge_iasa(f: &mut Fighter, p: InputPhase<'_>) {
    charge_input(f, UP_RELEASE, p.assets);
}

/// ftNs_AttackHi4Release_Anim (8011620C).
fn release_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    release_anim(f, p.assets, p.map)?;
    Ok(None)
}

/// ftNs_AttackHi4Release_IASA (801162B0): Wait's interrupts once allowed.
fn release_iasa(f: &mut Fighter, p: InputPhase<'_>) {
    melee_ft::fighter::state::callbacks::input::tilt(f, p);
}

/// ftNs_AttackHi4Release_Phys (801162E0).
fn physics_release(f: &mut Fighter, p: PhysicsPhase<'_>) {
    release_physics(f, p, EASE_UNTIL, EASE_RATE);
}
