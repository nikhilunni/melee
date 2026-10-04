//! Motion helpers Bowser's specials share (ft_081B.c, ft_084E.c, ftcommon.c).
use melee_ft::{
    collision::{air, ecb::EcbPose, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, PhysicsPhase},
        state::{AnimFn, CollisionFn, CollisionPhase, InputFn, InputPhase, MotionRow, PhysicsFn},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{airborne, friction, grounded},
};
use melee_types::{CommonMotionState, GroundOrAir};

/// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12): SkipMatAnim |
/// SkipColAnim | UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27.
pub const GROUND_AIR: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);
/// GROUND_AIR with Ft_MF_SkipHit: a row change that keeps its hitboxes.
pub const GROUND_AIR_KEEP_HIT: MotionEntryFlags = MotionEntryFlags(0x0C4C_5088);

/// A ported row of ftKp_Init_MotionStateTable (341..363) playing
/// submotion `animation` (the table's first column, 295..315).
pub const fn row(
    action: ActionId,
    animation: i32,
    anim: AnimFn,
    iasa: InputFn,
    physics: PhysicsFn,
    collision: CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The table's NULL and empty IASA callbacks.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// Fighter_ChangeMotionState(gobj, state, flags, start, rate, 0, NULL).
pub fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, flags, start, 1.0)
}

/// ft_8008A2BC on the ground (Wait).
pub fn wait(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(CommonMotionState::Wait.into(), assets)
}

/// ftCo_Fall_Enter: Fall, leaving the ground first when grounded.
pub fn fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        f.leave_ground();
    }
    f.change_motion_state(CommonMotionState::Fall.into(), assets)
}

/// ftCommon_GroundToAirStateChange: ftCommon_8007D5D4, then the airborne
/// row at the current frame and rate with `flags`.
pub fn ground_to_air(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    f.leave_ground();
    let frame = f.animation.frame;
    change(f, state, flags, frame, assets)
}

/// ftCommon_AirToGroundStateChange: ftCommon_8007D7FC, then the grounded
/// row at the current frame with `flags`.
pub fn air_to_ground(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    change(f, state, flags, frame, assets)
}

/// ft_80082708 (80082708): ordinary ground collision that walks off the
/// floor's edge. True while supported.
pub fn stays_grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge. True while supported.
pub fn stays_on_edge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_escape(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
pub fn lands(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    )
}

/// ft_CheckGroundAndLedge (80081D0C's ledge-aware sibling): airborne
/// collision that also notes a ledge in reach; true on landing.
pub fn lands_or_finds_ledge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    )
}

/// ft_CheckGroundAndLedge (800822A4) with CLIFFCATCH_BOTH (0): true on
/// landing; with no ledge cooldown the ledges on either side are tested.
pub fn lands_or_finds_ledge_either_side(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let landed = if c.status.ledge_cooldown == 0 {
        melee_mp::set_facing_dir(cd, 0);
        p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    } else {
        p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    landed
}

/// ft_80084F3C (80084F3C): ground friction (scaled above walk speed) and
/// ftCommon_ApplyGroundMovement, with Fighter_procUpdate's tail.
pub fn ground_friction(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
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

/// ftCommon_Fall (8007D494): `gravity` down to `terminal`.
pub fn gravity(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.core.physics.self_velocity.y =
        airborne::gravity(f.core.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_FallBasic (8007D4B8): the fighter's own gravity.
pub fn fall_basic(f: &mut Fighter) {
    let air = &f.core.attributes.air;
    let (g, terminal) = (air.gravity, air.terminal_velocity);
    gravity(f, g, terminal);
}

/// ftCommon_ApplyFrictionAir (8007CE94) with `friction`.
pub fn air_friction(f: &mut Fighter, friction: f32) {
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, friction);
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

/// ftCommon_ClampGrVel (8007CC78).
pub fn clamp_ground_velocity(f: &mut Fighter, maximum: f32) {
    let velocity = f.physics.ground_velocity;
    if velocity < -maximum {
        f.physics.ground_velocity = -maximum;
    } else if velocity > maximum {
        f.physics.ground_velocity = maximum;
    }
}

/// ft_80085134 (80085134): TransN's delta drives both velocity axes.
pub fn root_motion_velocity(f: &mut Fighter) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("special TransN")
        .primary_history
        .offset;
    // Retail 80085140: fmuls.
    f.physics.self_velocity.x = offset.z * f.physics.facing;
    f.physics.self_velocity.y = offset.y;
}

/// ftCommon_UnlockECB (8007D5BC).
pub fn unlock_ecb(f: &mut Fighter) {
    f.collision.lock_frames = 0;
    f.collision.data.x130_flags &= !melee_types::mp::coll_data_x130::LOCKED;
}
