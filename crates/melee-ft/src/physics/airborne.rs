//! ftcommon.c airborne integration. Audited with asm.py --fused:
//! Fall, ApplyFrictionAir, 8007D174 and 8007D28C have no fused sites.
use super::FighterPhysics;
use crate::desc::attributes::AirAttributes;

/// ftCommon_Fall (0x8007D494): subtract gravity, then clamp downward speed.
pub fn gravity(velocity: f32, gravity: f32, terminal: f32) -> f32 {
    let result = velocity - gravity;
    if result < -terminal {
        -terminal
    } else {
        result
    }
}

/// ftCommon_8007D28C -> ftCommon_8007D174 (0x8007D174).
/// Returns acceleration; Fighter_procUpdate adds it to self_vel afterwards.
pub fn drift(velocity: f32, stick: f32, attrs: &AirAttributes) -> f32 {
    // retail 0x8007D2AC fmuls, 0x8007D2C8 fadds: deliberately unfused.
    let scaled = stick * attrs.air_drift_stick_mul;
    let base = if stick > 0.0 {
        attrs.aerial_drift_base
    } else {
        -attrs.aerial_drift_base
    };
    let mut acceleration = scaled + base;
    let target = stick * attrs.air_drift_max;
    let friction = attrs.aerial_friction;
    if target == 0.0 {
        // ftCommon_ApplyFrictionAir (0x8007CE94): >=, unlike ground friction.
        return if friction.abs() >= velocity.abs() {
            -velocity
        } else if velocity > 0.0 {
            -friction
        } else {
            friction
        };
    }
    if (velocity * acceleration).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        if acceleration > 0.0 {
            if velocity + acceleration > target {
                acceleration = -friction;
                if velocity + acceleration < target {
                    acceleration = target - velocity;
                }
                if velocity + acceleration > attrs.air_max_horizontal_velocity {
                    acceleration = attrs.air_max_horizontal_velocity - velocity;
                }
            }
        } else if velocity + acceleration < target {
            acceleration = friction;
            if velocity + acceleration > target {
                acceleration = target - velocity;
            }
            if velocity + acceleration < -attrs.air_max_horizontal_velocity {
                acceleration = -attrs.air_max_horizontal_velocity - velocity;
            }
        }
    }
    acceleration
}

/// ft_80084DB0 (0x80084DB0), after CheckFallFast: terminal or fast-fall speed.
pub fn fall_physics(state: &mut FighterPhysics, attrs: &AirAttributes, stick_x: f32) {
    state.self_velocity.y = if state.fast_fall {
        -attrs.fast_fall_velocity
    } else {
        gravity(
            state.self_velocity.y,
            attrs.gravity,
            attrs.terminal_velocity,
        )
    };
    state.animation_velocity.x = drift(state.self_velocity.x, stick_x, attrs);
}
