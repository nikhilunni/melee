//! `ftcommon.c` ground friction; `ABS` keeps negative zero.

fn c_abs(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

/// `ftCommon_ApplyFrictionGround` (0x8007C930, ftcommon.c:51-59).
/// Returns acceleration, not the resulting velocity. The strict comparison
/// and sign of zero matter when projecting it onto the floor tangent.
pub fn friction_acceleration(velocity: f32, mut friction: f32) -> f32 {
    if c_abs(friction) > c_abs(velocity) {
        friction = -velocity;
    } else if velocity > 0.0 {
        friction = -friction;
    }
    friction
}

/// `ftCommon_ApplyFrictionAir` (0x8007CE94, ftcommon.c:253-261): unlike the
/// ground version, friction at least as large as the speed stops it.
pub fn air_friction_acceleration(velocity: f32, friction: f32) -> f32 {
    if c_abs(friction) >= c_abs(velocity) {
        -velocity
    } else if velocity > 0.0 {
        -friction
    } else {
        friction
    }
}

/// `ftCommon_8007CF58` (0x8007CF58, ftcommon.c:283-308): air friction that
/// switches to PlCo +1FC while horizontal speed exceeds the air drift maximum.
pub fn air_drift_friction_acceleration(
    velocity: f32,
    aerial_friction: f32,
    drift_max: f32,
    over_drift_friction: f32,
) -> f32 {
    let friction = if c_abs(velocity) > drift_max {
        over_drift_friction
    } else {
        aerial_friction
    };
    air_friction_acceleration(velocity, friction)
}

/// `ft_80084F3C` (0x80084F3C, ft_084E.c:42-53), friction selection.
pub fn wait_friction(velocity: f32, friction: f32, walk_max: f32, above_walk: f32) -> f32 {
    if c_abs(velocity) > walk_max {
        // retail 0x80084F84: fmuls; no fused sites in this function.
        friction * above_walk
    } else {
        friction
    }
}

/// `ftCommon_8007CCA0` (0x8007CCA0), also the arithmetic of
/// `ftCommon_8007CE4C` (0x8007CE4C): signed ground knockback decay.
pub fn decay_knockback(mut velocity: f32, friction: f32) -> f32 {
    if velocity < 0.0 {
        velocity += friction;
        if velocity > 0.0 {
            velocity = 0.0;
        }
    } else {
        velocity -= friction;
        if velocity < 0.0 {
            velocity = 0.0;
        }
    }
    velocity
}
