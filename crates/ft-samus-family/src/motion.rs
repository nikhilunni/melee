//! Motion helpers Samus's specials and Kirby's copy of the Charge Shot
//! share (ft_081B.c, ft_084E.c, ftcommon.c).
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::{CommonMotionState, GroundOrAir};

/// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12): SkipMatAnim |
/// SkipColAnim | UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27.
pub const GROUND_AIR: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);

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

/// ft_80084F3C (80084F3C): ground friction (scaled above walk speed) and
/// ftCommon_ApplyGroundMovement, with Fighter_procUpdate's tail.
pub fn ground_friction(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::guard_on(f, p);
}

/// ft_80084EEC: gravity and air friction.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::air_friction(f, p);
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
