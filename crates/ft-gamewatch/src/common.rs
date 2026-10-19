//! Motion helpers Mr. Game & Watch's specials share (ft_0819.c, ft_084E.c,
//! ftcommon.c). Those Kirby's copied Chef needs too live in
//! `ft_gamewatch_family::motion`.
use melee_ft::{
    collision::{air, ground},
    fighter::{
        state::{callbacks, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter,
    },
    physics::{airborne, friction},
};
use melee_types::CommonMotionState;

pub(crate) use ft_gamewatch_family::motion::{
    air_to_ground, change, fall_without_drift, finish, finish_update, ground_to_air, lands,
    stays_on_edge,
};

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
