//! The yo-yo down smash, ftnessattacklw4.c (8011659C..80116B70): rows 345
//! (start), 346 (charge) and 347 (release). Its article faces backward.
use super::{
    anchored_physics, charge_anim, charge_input, collision, enter_start, release_anim,
    release_physics, start_anim, start_input, ANIMATIONS, DOWN_CHARGE, DOWN_RELEASE, DOWN_START,
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

/// ftNs_AttackLw4Release_Phys: the yo-yo eases to the anchor by 0.2 a
/// frame until frame 19 (ftNs_AttackHi4_YoyoSetHitPosUnk).
const EASE_UNTIL: i32 = 19;
const EASE_RATE: f32 = 0.2;

pub const fn rows() -> [MotionRow; 3] {
    [
        row(DOWN_START, ANIMATIONS[3], anim, input, anchored_physics, collision),
        row(
            DOWN_CHARGE,
            ANIMATIONS[4],
            charge_animation,
            charge_iasa,
            common::ground_friction,
            collision,
        ),
        row(
            DOWN_RELEASE,
            ANIMATIONS[5],
            release_animation,
            release_iasa,
            physics_release,
            collision,
        ),
    ]
}

/// ftNs_AttackLw4_Enter (8011659C).
pub fn enter(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    enter_start(f, DOWN_START, assets)
}

/// ftNs_AttackLw4_Anim (80116638).
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    start_anim(f, DOWN_CHARGE, p.assets, p.map)?;
    Ok(None)
}

/// ftNs_AttackLw4_IASA (801166D4).
fn input(f: &mut Fighter, p: InputPhase<'_>) {
    start_input(f, p);
}

/// ftNs_AttackLw4Charge_Anim (80116798).
fn charge_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    charge_anim(f, DOWN_RELEASE, p.assets)?;
    Ok(None)
}

/// ftNs_AttackLw4Charge_IASA (80116828).
fn charge_iasa(f: &mut Fighter, p: InputPhase<'_>) {
    charge_input(f, DOWN_RELEASE, p.assets);
}

/// ftNs_AttackLw4Release_Anim (80116958).
fn release_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    release_anim(f, p.assets, p.map)?;
    Ok(None)
}

/// ftNs_AttackLw4Release_IASA (801169BC): Wait's interrupts once allowed.
fn release_iasa(f: &mut Fighter, p: InputPhase<'_>) {
    melee_ft::fighter::state::callbacks::input::tilt(f, p);
}

/// ftNs_AttackLw4Release_Phys (801169EC).
fn physics_release(f: &mut Fighter, p: PhysicsPhase<'_>) {
    release_physics(f, p, EASE_UNTIL, EASE_RATE);
}
