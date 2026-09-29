//! Popo's aerial Squall Hammer: SpecialAirS1 (345, alone) and SpecialAirS2
//! (346, with Nana), 8011FD9C..80120E68.
use super::{
    attributes, clamp_ground_velocity, install_callbacks, press_due, read_steer, rebound,
    remove_callbacks, scratch, side, tilt, GROUND_AIR_FLAGS, GROUND_LINKED, GROUND_SOLO, SPIN_BOX,
};
use crate::partner as link;
use melee_ft::{
    anim::WaitChoice,
    collision::air,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter,
    },
    physics::{airborne, friction},
};

/// ftPp_SpecialAirS1_Anim (8011FD9C) / ftPp_SpecialAirS2_Anim (8011FE48):
/// at the end (linked, also once Nana has left her rows) Fall, or
/// FallSpecial with attribute x70's landing lag.
pub fn anim<const LINKED: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let over = !f.animation.frames_remaining(&f.skeleton)
        || LINKED && !super::partner::in_rows(link::view(f).action);
    if over {
        if LINKED {
            link::separate(f);
        }
        remove_callbacks(f);
        let lag = attributes(f).landing_lag;
        super::fall(f, lag, p.assets)?;
    }
    Ok(None)
}

/// ftPp_SpecialAirS1_IASA (8011FFE0) / ftPp_SpecialAirS2_IASA (80120030).
pub fn input(f: &mut Fighter, _: InputPhase<'_>) {
    let multiplier = attributes(f).air_steer;
    read_steer(f, multiplier);
}

/// ftPp_SpecialAirS1_Phys (801203E0) / ftPp_SpecialAirS2_Phys (80120520):
/// a due B press rises (fadds); the move's own gravity for attribute x5C
/// frames (the frame count converted to float), then the ordinary one; the
/// stick drives the speed toward attribute x3C (ftCommon_8007D2E8), else
/// air friction (ftCommon_8007CEF4).
pub fn physics<const LINKED: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if press_due(f) {
        let rise = attributes(f).press_rise[side(LINKED)];
        f.physics.self_velocity.y += rise;
    }
    let frames = {
        let s = scratch(f);
        s.frames += 1;
        s.frames
    };
    let (own_gravity, gravity, terminal, cap) = {
        let a = attributes(f);
        (
            a.gravity_frames,
            a.gravity[side(LINKED)],
            a.terminal_velocity[side(LINKED)],
            a.air_speed_max,
        )
    };
    let (gravity, terminal) = if (frames as f32) < own_gravity {
        (gravity, terminal)
    } else {
        // ftCommon_FallBasic.
        let air = &f.core.attributes.air;
        (air.gravity, air.terminal_velocity)
    };
    f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
    let steer = scratch(f).steer;
    let velocity = f.physics.self_velocity.x;
    f.physics.animation_velocity.x = if f.commands.variables[0] == 0 && steer != 0.0 {
        let target = if steer > 0.0 { cap } else { -cap };
        accelerate_toward(velocity, steer, target)
    } else {
        let aerial_friction = f.core.attributes.air.aerial_friction;
        friction::air_friction_acceleration(velocity, aerial_friction)
    };
    super::finish_update(f, &p);
}

/// ftCommon_8007D2E8 (8007D2E8): the air acceleration toward `target`,
/// trimmed so the speed does not pass it.
fn accelerate_toward(velocity: f32, acceleration: f32, target: f32) -> f32 {
    if target == 0.0 {
        return -velocity;
    }
    if (velocity * acceleration).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        if acceleration > 0.0 {
            if velocity + acceleration > target {
                return target - velocity;
            }
        } else if velocity + acceleration < target {
            return target - velocity;
        }
    }
    acceleration
}

/// ftPp_SpecialAirS1_Coll (80120A48) / ftPp_SpecialAirS2_Coll (80120C58):
/// ft_800824A0 with the spin's box, the wall rebound on the horizontal
/// speed, and a landing continues on the ground (inline4); then the tilt
/// and the callbacks. The ceiling test (`(env & Collide_CeilingMask) == 1`,
/// retail 80120A84: rlwinm 17..18, cmpwi 1) never holds.
pub fn collision<const LINKED: bool>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = air::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        SPIN_BOX,
    );
    if let Some(speed) = rebound(f, f.physics.self_velocity.x) {
        f.physics.self_velocity.x = speed;
    }
    if landed {
        land::<LINKED>(f, p.assets.expect("Squall Hammer landing assets"))?;
        scratch(f).on_floor = true;
    } else {
        scratch(f).on_floor = false;
    }
    tilt(f);
    install_callbacks(f);
    Ok(())
}

/// inline4: ftCommon_AirToGroundStateChange into the grounded row at this
/// frame, the callbacks, no vertical speed, the ground speed clamped to
/// attribute x38.
fn land<const LINKED: bool>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    let state = if LINKED { GROUND_LINKED } else { GROUND_SOLO };
    f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    install_callbacks(f);
    f.physics.animation_velocity.y = 0.0;
    f.physics.self_velocity.y = 0.0;
    let cap = attributes(f).ground_speed_max;
    clamp_ground_velocity(f, cap);
    Ok(())
}
