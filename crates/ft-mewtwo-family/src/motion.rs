//! Motion helpers Mewtwo's specials and Kirby's copy of the Shadow Ball
//! share (ft_0819.c, ft_081B.c, ftcommon.c).
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::CommonMotionState;

/// A ported motion row playing submotion `animation`.
pub const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
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

/// ftCommon_GroundToAirStateChange (ftCommon/inlines.h:71): leave the
/// ground (ftCommon_8007D5D4), then the aerial counterpart at the current
/// frame.
pub fn ground_to_air(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, flags, frame, 1.0)
}

/// ftCommon_AirToGroundStateChange (ftCommon/inlines.h:81): land
/// (ftCommon_8007D7FC), then the grounded counterpart at the current frame.
pub fn air_to_ground(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, flags, frame, 1.0)
}

/// ft_8008A2BC on the ground (Wait), ftCo_Fall_Enter in the air.
pub fn finish(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let state = if air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// ft_80082708 (80082708): ordinary ground collision; false off the floor.
pub fn grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCommon_GroundAirColl_MF: SkipMatAnim, SkipColAnim, UpdateCmd,
/// SkipItemVis, Unk19, SkipModelPartVis, SkipModelFlags, Unk27.
pub const GROUND_AIR_COLL_BASE_FLAGS: MotionEntryFlags =
    MotionEntryFlags(1 << 7 | 1 << 12 | 1 << 14 | 1 << 18 | 1 << 19 | 1 << 22 | 1 << 26 | 1 << 27);

/// ft_80084F3C (80084F3C): ground friction (scaled above walk speed) and
/// ftCommon_ApplyGroundMovement, with Fighter_procUpdate's tail.
pub fn ground_friction(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::guard_on(f, p);
}

/// ft_80084EEC (80084EEC): gravity and air friction, no stick input, with
/// Fighter_procUpdate's tail.
pub fn air_friction_fall(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::air_friction(f, p);
}

/// ftCo_Fall_Enter: Fall, leaving the ground first when grounded.
pub fn enter_fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        f.leave_ground();
    }
    f.change_motion_state(CommonMotionState::Fall.into(), assets)
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
