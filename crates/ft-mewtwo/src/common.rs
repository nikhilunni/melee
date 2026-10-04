//! Motion helpers Mewtwo's specials share (ft_0819.c, ft_081B.c, ftcommon.c).
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::friction,
};
use melee_types::CommonMotionState;

/// A ported row of ftMt_Init_MotionStateTable playing submotion
/// `animation` (ftMt_SM_*, from ftCo_SM_Count = 295).
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

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge; false off the floor.
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

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
pub fn fall(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y =
        melee_ft::physics::airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_8007CEF4 (8007CEF4): the fighter's aerial friction.
pub fn aerial_friction(f: &mut Fighter) {
    let friction = f.attributes.air.aerial_friction;
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, friction);
}

/// Fighter_procUpdate's airborne tail.
pub fn finish_air(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    f.core.finish_air_update(p.assets, p.wind);
}

/// The Fighter part joint an efSync call names directly (`fp->parts[n]`
/// with an FtPart constant used as the joint index).
pub fn joint_position(f: &mut Fighter, joint: usize) -> hsd_types::Vec3 {
    let c = &mut f.core;
    melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        joint,
        hsd_types::Vec3::ZERO,
    )
}

/// ftCommon_GroundAirColl_MF: SkipMatAnim, SkipColAnim, UpdateCmd,
/// SkipItemVis, Unk19, SkipModelPartVis, SkipModelFlags, Unk27.
pub const GROUND_AIR_COLL_BASE_FLAGS: MotionEntryFlags =
    MotionEntryFlags(1 << 7 | 1 << 12 | 1 << 14 | 1 << 18 | 1 << 19 | 1 << 22 | 1 << 26 | 1 << 27);

/// ftCommon_GroundAirColl_MF (SkipMatAnim, SkipColAnim, UpdateCmd,
/// SkipItemVis, Unk19, SkipModelPartVis, SkipModelFlags, Unk27) with
/// KeepGfx, KeepColAnimHitStatus and SkipHit: the specials' ground/air
/// counterparts (Teleport's transition_flags1).
pub const GROUND_AIR_COLLISION_FLAGS: MotionEntryFlags = MotionEntryFlags(
    1 << 1
        | 1 << 2
        | 1 << 3
        | 1 << 7
        | 1 << 12
        | 1 << 14
        | 1 << 18
        | 1 << 19
        | 1 << 22
        | 1 << 26
        | 1 << 27,
);

/// ftCommon_ApplyGroundMovement (8007CB74), then Fighter_procUpdate's
/// grounded tail.
pub fn move_on_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    use melee_ft::physics::grounded;
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

/// ftCommon_ClampSelfVelX (8007D440).
pub fn clamp_self_velocity_x(f: &mut Fighter, maximum: f32) {
    let velocity = f.physics.self_velocity.x;
    if velocity < -maximum {
        f.physics.self_velocity.x = -maximum;
    } else if velocity > maximum {
        f.physics.self_velocity.x = maximum;
    }
}

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
