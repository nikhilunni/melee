//! Motion helpers Donkey Kong's specials share (ft_081B.c, ft_084E.c,
//! ftcommon.c); the ones the Giant Punch needs live in
//! `ft_donkey_family::motion`.
use crate::init::DonkeyKong;
use melee_ef::request::EffectRequest;
use melee_ft::{
    fighter::{state::PhysicsPhase, Fighter},
    physics::{airborne, friction, grounded},
};

pub use ft_donkey_family::motion::{
    air_friction_fall, air_to_ground, change, fall, ground_friction, ground_to_air, lands,
    no_input, part, row, seal_graphics, stays_grounded, stays_on_edge, wait, GROUND_AIR, KEEP_GFX,
    SKIP_HIT,
};

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
pub fn fall_at(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_ApplyFrictionAir (8007CE94) with `friction`.
pub fn air_friction(f: &mut Fighter, friction: f32) {
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, friction);
}

/// ftCommon_8007CADC (8007CADC) -> ftCommon_8007CA80: walk toward the
/// stick's target with no friction.
pub fn walk_toward_stick(f: &mut Fighter, threshold: f32, acceleration: f32, target: f32) {
    let stick = f.input.current.stick.x;
    let (mut acceleration, target) = if stick.abs() >= threshold {
        (stick * acceleration, stick * target)
    } else {
        (0.0, 0.0)
    };
    let velocity = f.physics.ground_velocity;
    if target == 0.0 {
        acceleration = -velocity;
    } else if (velocity * acceleration).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        if acceleration > 0.0 {
            if velocity + acceleration > target {
                acceleration = target - velocity;
            }
        } else if velocity + acceleration < target {
            acceleration = target - velocity;
        }
    }
    f.physics.ground_acceleration = acceleration;
}

/// ftCommon_ApplyGroundMovement (8007CB74), then Fighter_procUpdate's
/// grounded tail.
pub fn move_on_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        p.map.floor_speed_scale(&core.collision.data),
    );
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}

/// ftCommon_8007D344 (8007D344) -> ftCommon_8007D140: drift toward the
/// stick's target with the fighter's aerial friction.
pub fn drift_with_friction(f: &mut Fighter, threshold: f32, acceleration: f32, target: f32) {
    let stick = f.input.current.stick.x;
    let (acceleration, target) = if stick.abs() >= threshold {
        (stick * acceleration, stick * target)
    } else {
        (0.0, 0.0)
    };
    let air = &f.core.attributes.air;
    f.core.physics.animation_velocity.x =
        airborne::drift_acceleration(f.core.physics.self_velocity.x, acceleration, target, air);
}

/// ftCommon_ClampSelfVelX (8007D440).
pub fn clamp_self_velocity_x(f: &mut Fighter, maximum: f32) {
    let velocity = f.physics.self_velocity.x;
    if velocity < -maximum {
        f.physics.self_velocity.x = -maximum;
    } else if velocity > maximum {
        f.physics.self_velocity.x = maximum;
    }
}

/// ftCommon_ClampGrVel (8007D174's sibling, 8007D050).
pub fn clamp_ground_velocity(f: &mut Fighter, maximum: f32) {
    let velocity = f.physics.ground_velocity;
    if velocity < -maximum {
        f.physics.ground_velocity = -maximum;
    } else if velocity > maximum {
        f.physics.ground_velocity = maximum;
    }
}

/// take_dmg_cb and death2_cb = ftDk_Init_8010D774, set by Giant Punch's
/// and Spinning Kong's `setCallbacks` until the next motion change, with
/// Fighter_SetEffectHitlagCallbacks.
pub fn install_damage_callbacks(f: &mut Fighter) {
    f.character.get_mut::<DonkeyKong>().damage_callbacks = true;
    f.effect_state.hitlag_callbacks = true;
}

/// ftDk_Init_8010D774 (8010D774): ftDk_SpecialN_DestroyAllEffectsPlus (the
/// stored punch goes with every owned effect), then
/// ftDk_SpecialHi_DestroyAllEffects.
pub fn damage_callback(f: &mut Fighter) {
    if !f.character.get::<DonkeyKong>().damage_callbacks {
        return;
    }
    crate::special_n::set_swings(f, 0);
    f.effects.push(EffectRequest::DestroyOwned);
    f.effects.push(EffectRequest::DestroyOwned);
}
