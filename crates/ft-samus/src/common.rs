//! Motion helpers Samus's specials share (ft_081B.c, ft_084E.c, ftcommon.c).
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{airborne, friction, grounded},
};
use melee_types::{CommonMotionState, GroundOrAir};

/// ftSs_SM_SpecialLw = ftCo_SM_Count: row 341 plays submotion 295.
const FIRST_SPECIAL_ACTION: u16 = 341;
const FIRST_SPECIAL_ANIMATION: i32 = 295;

/// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12): SkipMatAnim |
/// SkipColAnim | UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27.
pub const GROUND_AIR: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);

/// A ported row of ftSs_Init_MotionStateTable (341..358).
pub const fn row(
    action: ActionId,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation: FIRST_SPECIAL_ANIMATION + (action.0 - FIRST_SPECIAL_ACTION) as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: melee_ft::fighter::state::callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The empty IASA callbacks.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// Fighter_ChangeMotionState(gobj, state, flags, start, rate, 0, NULL).
pub fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    start: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, flags, start, rate)
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
    change(f, state, flags, frame, 1.0, assets)
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
    change(f, state, flags, frame, 1.0, assets)
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

/// ft_80082888 (80082888) with a fixed box: false off the floor.
pub fn stays_grounded_in_box(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    ecb: melee_types::mp::FtCollisionBox,
) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    ground::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        ecb,
        false,
    )
}

/// ft_800824A0 (800824A0) with a fixed box: true on landing.
pub fn lands_in_box(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    ecb: melee_types::mp::FtCollisionBox,
) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        ecb,
    )
}

/// ft_80084F3C (80084F3C): ground friction (scaled above walk speed) and
/// ftCommon_ApplyGroundMovement, with Fighter_procUpdate's tail.
pub fn ground_friction(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::guard_on(f, p);
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
    finish_ground(f, p);
}

/// Fighter_procUpdate's grounded tail after the callback set the velocities.
pub fn finish_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}

/// Fighter_procUpdate's tail for whichever side of the floor the fighter
/// is on.
pub fn finish_update(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        finish_ground(f, p);
    } else {
        f.core.finish_air_update(p.assets, p.wind);
    }
}

/// ftCommon_FallBasic (8007D4B8): PlCo gravity to terminal velocity.
pub fn fall_basic(f: &mut Fighter) {
    let air = &f.core.attributes.air;
    f.core.physics.self_velocity.y = airborne::gravity(
        f.core.physics.self_velocity.y,
        air.gravity,
        air.terminal_velocity,
    );
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

/// ftCommon_8007D3A8 (8007D3A8) -> ftCommon_8007D2E8: drift toward the
/// stick's target with no friction; a zero target stops outright.
pub fn drift_without_friction(f: &mut Fighter, threshold: f32, acceleration: f32, target: f32) {
    let stick = f.input.current.stick.x;
    let (mut acceleration, target) = if stick.abs() >= threshold {
        (stick * acceleration, stick * target)
    } else {
        (0.0, 0.0)
    };
    let velocity = f.physics.self_velocity.x;
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
    f.physics.animation_velocity.x = acceleration;
}

/// ftCommon_8007D268 (8007D268): the ordinary aerial drift.
pub fn ordinary_drift(f: &mut Fighter) {
    let stick = f.input.current.stick.x;
    let air = &f.core.attributes.air;
    f.core.physics.animation_velocity.x =
        airborne::drift(f.core.physics.self_velocity.x, stick, air);
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

/// `fp->parts[FtPart_X]`: Samus's code indexes the parts array with the
/// part enum directly (no ftParts_GetBoneIndex), so these are parts indices.
pub fn part(part: melee_types::FtPart) -> usize {
    i32::from(part) as usize
}

/// Fighter_ChangeMotionState's efAsync_QueueFlush (fighter.c:951) from an
/// animation callback: what the outgoing script issued this frame spawns
/// before the change, ahead of the new script's frame-0 effects and colour
/// step. Resolving those graphics commands now (their offset draws are the
/// proc's next RNG work) lets the change seal them with the outgoing pose.
pub fn seal_graphics(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    if !f.core.commands.graphics.is_empty() {
        f.core.resolve_graphics_commands(assets, rng);
    }
}
