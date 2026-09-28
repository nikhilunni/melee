//! Motion helpers the family's states share (ft_0819.c, ft_081B.c,
//! ft_084E.c, ftcommon.c).
use melee_ft::{
    collision::{air, ecb::EcbPose, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, InputPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::CommonMotionState;

/// Ft_MF_* (ft/forward.h).
pub(crate) mod flags {
    pub const KEEP_GFX: u32 = 1 << 1;
    pub const SKIP_HIT: u32 = 1 << 3;
    /// The ground/air counterparts' transition flags (SkipMatAnim,
    /// SkipColAnim, UpdateCmd, SkipItemVis, Unk19, SkipModelPartVis,
    /// SkipModelFlags, Unk27).
    pub const GROUND_AIR: u32 = 0x0C4C_5080;
}

/// The empty IASA callbacks.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// Fighter_ChangeMotionState(gobj, state, flags, start, rate, 0, NULL).
pub(crate) fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, MotionEntryFlags(flags), start, 1.0)
}

/// ftCommon_GroundToAirStateChange: leave the ground (ftCommon_8007D5D4),
/// then the aerial counterpart at the current frame.
pub(crate) fn ground_to_air(
    f: &mut Fighter,
    state: ActionId,
    assets: &FighterAssets,
) -> Result<()> {
    f.leave_ground();
    let frame = f.animation.frame;
    change(f, state, flags::GROUND_AIR, frame, assets)
}

/// ftCommon_AirToGroundStateChange: land (ftCommon_8007D7FC), then the
/// grounded counterpart at the current frame.
pub(crate) fn air_to_ground(
    f: &mut Fighter,
    state: ActionId,
    assets: &FighterAssets,
) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    change(f, state, flags::GROUND_AIR, frame, assets)
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

/// ft_8008A2BC (Wait) on the ground, ftCo_Fall_Enter in the air.
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

/// ft_CheckGroundAndLedge (800822A4) with CLIFFCATCH_BOTH (0): true on
/// landing; with no ledge cooldown the ledges on either side are tested.
pub(crate) fn lands_or_catches_ledge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
pub(crate) fn fall(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y =
        melee_ft::physics::airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_8007D344 (8007D344): drift toward the stick past `threshold`,
/// through ftCommon_8007D140 with the fighter's aerial friction.
pub(crate) fn drift(f: &mut Fighter, threshold: f32, acceleration: f32, maximum: f32) {
    let x = f.input.current.stick.x;
    let (acceleration, target) = if x.abs() >= threshold {
        (x * acceleration, x * maximum)
    } else {
        (0.0, 0.0)
    };
    let air = &f.core.attributes.air;
    f.core.physics.animation_velocity.x = melee_ft::physics::airborne::drift_acceleration(
        f.core.physics.self_velocity.x,
        acceleration,
        target,
        air,
    );
}
