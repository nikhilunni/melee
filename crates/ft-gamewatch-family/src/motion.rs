//! Motion helpers Mr. Game & Watch's specials and Kirby's copy of Chef share
//! (ft_0819.c, ft_084E.c, ftcommon.c).
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{airborne, grounded},
};
use melee_types::{CommonMotionState, GroundOrAir};

/// Fighter_ChangeMotionState(gobj, state, flags, start, 1, 0, NULL).
pub fn change(
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
pub fn ground_to_air(
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
pub fn air_to_ground(
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
pub fn finish(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let state = if air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
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

/// ft_80084EEC (80084EEC): gravity and air friction, no stick input.
pub fn fall_without_drift(f: &mut Fighter, p: PhysicsPhase<'_>) {
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
pub fn finish_update(f: &mut Fighter, p: &PhysicsPhase<'_>) {
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
