use super::{
    friction::{decay_knockback, friction_acceleration, wait_friction},
    integrate, FighterPhysics,
};
use crate::desc::{common::CommonFighterData, FighterAttributes};
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::{mp::CollData, GroundOrAir};

#[derive(Clone, Copy, Debug)]
pub struct GroundedParameters {
    pub friction: f32,
    pub walk_max_velocity: f32,
    pub above_walk_multiplier: f32,
    pub knockback_multiplier: f32,
    pub shield_knockback_multiplier: f32,
}
impl GroundedParameters {
    pub fn from_attributes(attrs: &FighterAttributes, common: &CommonFighterData) -> Self {
        Self {
            friction: attrs.ground.ground_friction,
            walk_max_velocity: attrs.walking.walk_max_vel,
            above_walk_multiplier: common.friction_when_above_walk_speed,
            knockback_multiplier: common.ground_knockback_friction_multiplier,
            shield_knockback_multiplier: common.shield_ground_friction_multiplier,
        }
    }
}

/// `ftCo_Wait_Phys` (0x8008A644) -> `ft_80084F3C` ->
/// `ftCommon_ApplyGroundMovement` (0x8007CB74), then `ftColl_8007AEE0`.
pub fn wait_physics(
    state: &mut FighterPhysics,
    params: &GroundedParameters,
    normal: Vec3,
    terrain: f32,
) {
    assert_eq!(state.ground_or_air, GroundOrAir::Ground);
    let friction = wait_friction(
        state.ground_velocity,
        params.friction,
        params.walk_max_velocity,
        params.above_walk_multiplier,
    );
    state.ground_acceleration = friction_acceleration(state.ground_velocity, friction);
    if terrain < 1.0 {
        // retail 0x8007CBA8: fmuls.
        state.ground_acceleration *= terrain;
    }
    // retail 0x8007CBB8/CBCC/CBE4/CBF8: separate fmuls. These products
    // are stored before PSVECAdd at 0x8006BBF8; never fuse across the call.
    state.animation_velocity = tangent(normal, state.ground_acceleration);
    state.self_velocity = tangent(normal, state.ground_velocity);
    state.shield_position_cached = false;
}

fn tangent(normal: Vec3, speed: f32) -> Vec3 {
    Vec3::new(normal.y * speed, -normal.x * speed, 0.0)
}

fn ground_knockback(velocity: &mut Vec3, ground_speed: &mut f32, friction: f32, normal: Vec3) {
    if velocity.x != 0.0 || velocity.y != 0.0 {
        if *ground_speed == 0.0 {
            *ground_speed = velocity.x;
        }
        *ground_speed = decay_knockback(*ground_speed, friction);
        // retail 0x8006BA40/BA54 and 0x8006BBAC/BBC0: fmuls.
        velocity.x = normal.y * *ground_speed;
        velocity.y = -normal.x * *ground_speed;
    }
}

/// Grounded portion of `Fighter_procUpdate` (0x8006B82C), s_link 4.
/// The caller supplies accumulated wind (zero on FD), after any stage procs.
/// Hitlag, capture, deferred displacement and action dispatch belong to the
/// outer fighter scheduler; this entry requires its ordinary Wait branch.
pub fn step_wait(
    state: &mut FighterPhysics,
    collision: &CollData,
    params: &GroundedParameters,
    map: &CollMap,
    wind: Vec3,
) {
    let terrain = map.floor_speed_scale(collision);
    let normal = collision.floor.normal;
    wait_physics(state, params, normal, terrain);
    // retail 0x8006BA28/BA30 and 0x8006BB94/BB9C: two fmuls, no fusion.
    let friction = params.friction * terrain;
    ground_knockback(
        &mut state.knockback_velocity,
        &mut state.ground_knockback_velocity,
        params.knockback_multiplier * friction,
        normal,
    );
    ground_knockback(
        &mut state.shield_knockback_velocity,
        &mut state.ground_shield_knockback_velocity,
        params.shield_knockback_multiplier * friction,
        normal,
    );
    integrate::integrate_velocity(state);
    let speed = map.line_speed(collision.floor.index, &state.position);
    integrate::integrate_environment(state, speed, wind);
}
