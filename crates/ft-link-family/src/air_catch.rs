//! ftLk_MS_AirCatch (360), the aerial hookshot: ftCo_AirCatch.c's common
//! motion with Link's arm (the throw frames are in [`crate::hookshot`]).
use crate::{common, hookshot, FamilyState, LinkFamily};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
};

/// The frames the aerial hookshot falls slowly while rising stops
/// (ftCo_AirCatch_Phys: mv+0 < 20).
const SLOW_FALL_FRAMES: f32 = 20.0;
/// Gravity's share while falling slowly (a double literal).
const SLOW_FALL_GRAVITY: f64 = 0.2;

/// ftCo_800C3BE8 (800C3BE8) in the air: mv+0 and the animation velocity
/// clear, AirCatch from frame 0, grab-proof (ftCommon_8007E2F4 0x1FF).
pub fn enter<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    hookshot::restart_count::<C>(f.character.get_mut::<C>().specials());
    f.physics.animation_velocity = hsd_types::Vec3::ZERO;
    common::change(f, FamilyState::AirCatch.action(), 0, 0.0, assets)?;
    f.core.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions::ALL;
    Ok(())
}

pub(crate) const fn motion_row<C: LinkFamily>() -> MotionRow {
    crate::row(
        FamilyState::AirCatch,
        animation::<C>,
        common::no_input,
        physics::<C>,
        collision,
    )
}

/// ftCo_AirCatch_Anim (800C3E24).
fn animation<C: LinkFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    hookshot::aerial_animation::<C>(f, p.assets, p.map)?;
    Ok(None)
}

/// ftCo_AirCatch_Phys (800C4388): fast fall, else a fifth of gravity
/// (in double) while falling in the first twenty frames, else gravity;
/// then the drift (ftCommon_8007D268).
fn physics<C: LinkFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.check_fast_fall(p.assets);
    let timer = f.character.get::<C>().specials_ref().hookshot_timer;
    let air = &f.core.attributes.air;
    let (gravity, terminal, fast) = (air.gravity, air.terminal_velocity, air.fast_fall_velocity);
    if f.physics.fast_fall {
        f.physics.self_velocity.y = -fast;
    } else if timer < SLOW_FALL_FRAMES && f64::from(f.physics.position_delta.y) < 0.0 {
        common::fall(f, (SLOW_FALL_GRAVITY * f64::from(gravity)) as f32, terminal);
    } else {
        common::fall(f, gravity, terminal);
    }
    let c = &mut f.core;
    c.physics.animation_velocity.x = melee_ft::physics::airborne::drift(
        c.physics.self_velocity.x,
        c.input.current.stick.x,
        &c.attributes.air,
    );
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCo_AirCatch_Coll (800C4430): ft_80081D0C, landing into ftCo_MS_Landing
/// without the landing interrupt.
fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("AirCatch landing assets");
        f.enter_landing_with(assets, false)?;
    }
    Ok(())
}
