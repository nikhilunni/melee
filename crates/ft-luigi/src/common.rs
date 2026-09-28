//! Motion helpers Luigi's specials share (ft_0819.c, ft_084E.c,
//! ftcommon.c).
use melee_ef::request::EffectRequest;
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{friction, grounded},
};
use melee_types::{CommonMotionState, GroundOrAir};

/// ftLg_Init_MotionStateTable's first row (ftLg_MS_SpecialN = ftCo_MS_Count).
pub(crate) const FIRST_ACTION: u16 = 341;
/// ftLg_SM_SpecialN = ftCo_SM_Count.
const FIRST_SUBMOTION: i32 = 295;

/// Motion-change flag words (Ft_MF_*).
pub(crate) mod flags {
    /// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12).
    pub const GROUND_AIR: u32 = 0x0C4C_5080;
    pub const KEEP_GFX: u32 = 1 << 1;
    pub const KEEP_COL_ANIM_HIT_STATUS: u32 = 1 << 2;
    pub const SKIP_HIT: u32 = 1 << 3;
    pub const KEEP_SFX: u32 = 1 << 9;
}

/// A ported row of ftLg_Init_MotionStateTable; `submotion` counts from
/// ftLg_SM_SpecialN (the table's anim_id column).
pub(crate) const fn row(
    action: ActionId,
    submotion: i32,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation: FIRST_SUBMOTION + submotion,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The empty IASA callbacks.
pub(crate) fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// Fighter_ChangeMotionState(gobj, state, flags, start, rate, 0, NULL).
pub(crate) fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    start: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, MotionEntryFlags(flags), start, rate)
}

/// ftCommon_GroundToAirStateChange (ftCommon/inlines.h:71): leave the
/// ground (ftCommon_8007D5D4), then the aerial counterpart at the current
/// frame.
pub(crate) fn ground_to_air(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    assets: &FighterAssets,
) -> Result<()> {
    f.leave_ground();
    let frame = f.animation.frame;
    change(f, state, flags, frame, 1.0, assets)
}

/// ftCommon_AirToGroundStateChange (ftCommon/inlines.h:81): land
/// (ftCommon_8007D7FC), then the grounded counterpart at the current frame.
pub(crate) fn air_to_ground(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    assets: &FighterAssets,
) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    change(f, state, flags, frame, 1.0, assets)
}

/// ft_8008A2BC on the ground (Wait), ftCo_Fall_Enter in the air.
pub(crate) fn finish(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let state = if air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// ft_80082708 (80082708): ordinary ground collision; false off the floor.
pub(crate) fn grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
pub(crate) fn lands(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCommon_ApplyFrictionGround (8007C930) with `friction`.
pub(crate) fn ground_friction(f: &mut Fighter, friction: f32) {
    f.physics.ground_acceleration =
        friction::friction_acceleration(f.physics.ground_velocity, friction);
}

/// ftCommon_ApplyFrictionAir (8007CE94) with `friction`.
pub(crate) fn air_friction(f: &mut Fighter, friction: f32) {
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, friction);
}

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
pub(crate) fn fall(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y =
        melee_ft::physics::airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_ApplyGroundMovement (8007CB74): the ground speeds along the
/// collision's floor normal, whichever way the fighter stands.
pub(crate) fn apply_ground_movement(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        p.map.floor_speed_scale(&core.collision.data),
    );
}

/// ft_80084F3C (80084F3C): PlCo ground friction (scaled above walking
/// speed), then ftCommon_ApplyGroundMovement. Luigi's aerial misfire runs
/// it in the air too.
pub(crate) fn ground_friction_movement(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let params =
        grounded::GroundedParameters::from_attributes(&f.core.attributes, &p.assets.common);
    let friction = friction::wait_friction(
        f.physics.ground_velocity,
        params.friction,
        params.walk_max_velocity,
        params.above_walk_multiplier,
    );
    ground_friction(f, friction);
    apply_ground_movement(f, p);
}

/// Fighter_procUpdate's tail after the state's physics callback, by the
/// fighter's current ground state.
pub(crate) fn finish_update(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        let core = &mut f.core;
        grounded::finish_ground_update(
            &mut core.physics,
            &core.collision.data,
            &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
            p.map,
            p.wind,
        );
    } else {
        f.core.finish_air_update(p.assets, p.wind);
    }
}

/// ftCommon_ClampSelfVelX (8007D440).
pub(crate) fn clamp_self_velocity_x(f: &mut Fighter, maximum: f32) {
    let velocity = f.physics.self_velocity.x;
    if velocity < -maximum {
        f.physics.self_velocity.x = -maximum;
    } else if velocity > maximum {
        f.physics.self_velocity.x = maximum;
    }
}

/// ftCommon_8007DB24: x2219_b0 clears and every owned effect goes.
pub(crate) fn remove_effects(f: &mut Fighter) {
    f.effect_state.destroy_on_state_change = false;
    f.effects.push(EffectRequest::DestroyOwned);
}
