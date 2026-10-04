//! Motion helpers Mr. Game & Watch's specials share (ft_0819.c, ft_084E.c,
//! ftcommon.c).
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{airborne, friction, grounded},
};
use melee_types::{CommonMotionState, GroundOrAir};

/// ftGw_MS_SpecialN, the first row that plays one of the character's own
/// submotions (ftGw_SM_SpecialN = ftCo_SM_Count).
const FIRST_SPECIAL_ACTION: u16 = 353;
const FIRST_SUBMOTION: i32 = 295;

/// Motion-change flag words (Ft_MF_*).
pub(crate) mod flags {
    /// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12): SkipMatAnim |
    /// SkipColAnim | UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis |
    /// SkipModelFlags | Unk27.
    pub const GROUND_AIR: u32 = 0x0C4C_5080;
    pub const KEEP_GFX: u32 = 1 << 1;
    pub const KEEP_COL_ANIM_HIT_STATUS: u32 = 1 << 2;
    pub const SKIP_HIT: u32 = 1 << 3;
    pub const SKIP_MODEL: u32 = 1 << 4;
}

/// A special row of ftGw_Init_MotionStateTable (353..380), whose anim_id
/// counts from ftGw_SM_SpecialN in step with the motion ids.
pub(crate) const fn row(
    action: ActionId,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation: FIRST_SUBMOTION + (action.0 - FIRST_SPECIAL_ACTION) as i32,
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

/// Fighter_ChangeMotionState(gobj, state, flags, start, 1, 0, NULL).
pub(crate) fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, MotionEntryFlags(flags), start, 1.0)
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
    change(f, state, flags, frame, assets)
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
    change(f, state, flags, frame, assets)
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

/// ft_80082708 (80082708): ordinary ground collision that lets the fighter
/// walk off the floor's edge; false off the floor.
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

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge; false off the floor.
pub(crate) fn stays_on_edge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ft_CheckGroundAndLedge (800822A4) toward the fighter's facing; true on
/// landing.
pub(crate) fn lands_facing(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCommon_ApplyFrictionAir (8007CE94) with `friction`.
pub(crate) fn air_friction(f: &mut Fighter, friction: f32) {
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, friction);
}

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
pub(crate) fn fall(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_FallBasic (8007D4B8): the attribute gravity and terminal speed.
pub(crate) fn fall_basic(f: &mut Fighter) {
    let (gravity, terminal) = {
        let air = &f.attributes.air;
        (air.gravity, air.terminal_velocity)
    };
    fall(f, gravity, terminal);
}

/// ft_80084EEC (80084EEC): gravity and air friction, no stick input.
pub(crate) fn fall_without_drift(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let air = &f.core.attributes.air;
    let physics = &mut f.core.physics;
    physics.self_velocity.y =
        airborne::gravity(physics.self_velocity.y, air.gravity, air.terminal_velocity);
    physics.animation_velocity.x =
        airborne::drift_acceleration(physics.self_velocity.x, 0.0, 0.0, air);
    finish_update(f, &p);
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
