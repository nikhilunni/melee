//! The aerial grapple (ftCo_AirCatch.c, Samus arms): the Z-air and air
//! dodge tether (ftCo_800C3B10 / ftCo_800C3BE8) and ftSs_MS_AirCatch (357).
//!
//! The beam spawns at ThrowN on the timeline's first frame and is thrown
//! with Samus's drift added; the rope then runs from accessory2 as for a
//! grab. A tip that meets a wall hangs Samus from it (it_802BAB40, states
//! 6..8 in the grapple module, ftSs_MS_AirCatchHit).
use super::{grapple, remove, set_state, state, AIR_CATCH, AIR_CATCH_HIT};
use crate::init::Samus;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::{FtPart, GroundOrAir};

/// ftSs_MS_AirCatch (357) and AirCatchHit (358).
pub const ROW: ActionId = ActionId(AIR_CATCH);
/// ftCo_AirCatch_Phys: the first 20 frames fall at a fifth of gravity.
const SLOW_FALL_FRAMES: f32 = 20.0;
/// ftCo_AirCatch_Phys: `co->gravity * 0.2` (a double product, rounded).
const SLOW_FALL_SCALE: f64 = 0.2;

/// ftSs_MS_AirCatchHit (358): the common hanging row (ftCo_AirCatchHit_*),
/// its article's steps doing the rest.
pub const HIT_ROW: ActionId = ActionId(AIR_CATCH_HIT);

pub const fn rows() -> [MotionRow; 2] {
    use melee_ft::fighter::state::callbacks;
    [
        crate::common::row(ROW, animation, crate::common::no_input, physics, collision),
        crate::common::row(
            HIT_ROW,
            hit_animation,
            crate::common::no_input,
            callbacks::physics::air_catch_hit,
            callbacks::collision::air_catch_hit,
        ),
    ]
}

/// ftCo_AirCatchHit_Anim (800C4380): empty; the animation steps on.
fn hit_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftCo_800C3B10's Samus arm: no beam callbacks installed, then
/// ftCo_800C3BE8: in the air the counter and animation velocity clear and
/// the AirCatch row starts, excluding every grab (ftCommon_8007E2F4 0x1FF).
pub fn try_tether(f: &mut Fighter, assets: &FighterAssets) -> bool {
    if grapple(f).callbacks {
        return false;
    }
    if f.physics.ground_or_air == GroundOrAir::Air {
        f.character.get_mut::<Samus>().grab_frames = 0.0;
        f.physics.animation_velocity = Vec3::ZERO;
        crate::common::change(f, ROW, MotionEntryFlags(0), 0.0, 1.0, assets)
            .expect("AirCatch assets");
        f.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions::ALL;
    }
    true
}

/// ftCo_800968C8: FallSpecial, ordinary gravity and landing, full mobility,
/// and the kind's landing lag (x2EC).
fn special_fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let lag = assets.motions[&35].animation.frames;
    f.enter_special_fall(assets, true, false, true, 1.0, lag)
}

/// ftCo_AirCatch_Anim (800C3E8C), FTKIND_SAMUS: the counter rises (a double
/// add); on xBC the beam appears at ThrowN (none: special fall); on xC0 a
/// wall in front of the hand (800C429C: double fmadd, rounded) removes it
/// and falls, otherwise it is thrown at x40 * facing plus Samus's motion
/// (fmuls, fadds); on xC4 it is reeled in, on xC8 removed. Special fall at
/// the animation's end.
fn animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let times = f.character.get::<Samus>().attributes.air_beam;
    let counter = {
        let samus = f.character.get_mut::<Samus>();
        samus.grab_frames = (f64::from(samus.grab_frames) + 1.0) as f32;
        samus.grab_frames
    };
    if counter == times.spawn as f32 {
        let position = {
            let c = &mut f.core;
            melee_ft::fighter::caches::part_position(
                &mut c.skeleton,
                &c.animation,
                crate::common::part(FtPart::ThrowN),
                Vec3::ZERO,
            )
        };
        if !super::spawn(f, position, p.map) {
            special_fall(f, p.assets)?;
            return Ok(None);
        }
    } else if counter > times.spawn as f32 && counter <= times.remove as f32 {
        if counter == times.extend as f32 {
            if super::wall_in_front(f, p.map) {
                remove(f);
                special_fall(f, p.assets)?;
                return Ok(None);
            }
            let speed = grapple(f).chain.attrs.throw_speed * f.physics.facing;
            let velocity = Vec3::new(speed + f.physics.position_delta.x, 0.0, 0.0);
            let g = grapple(f);
            let tip = g.chain.tip();
            g.chain.links[tip].vel = velocity;
            set_state(f, state::THROWN);
        } else if counter == times.retract as f32 {
            set_state(f, state::RETRACTING);
        } else if counter == times.remove as f32 {
            remove(f);
        }
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        special_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftCo_AirCatch_Phys (800C438C): the fast fall, else a fifth of gravity
/// for the first 20 frames while sinking, else gravity; then the ordinary
/// drift (ftCommon_8007D268).
fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.core.check_fast_fall(p.assets);
    let air = f.core.attributes.air.clone();
    let counter = f.character.get::<Samus>().grab_frames;
    let velocity = &mut f.core.physics.self_velocity.y;
    if f.core.physics.fast_fall {
        *velocity = -air.fast_fall_velocity;
    } else if counter < SLOW_FALL_FRAMES && f64::from(f.core.physics.position_delta.y) < 0.0 {
        let gravity = (SLOW_FALL_SCALE * f64::from(air.gravity)) as f32;
        *velocity = melee_ft::physics::airborne::gravity(*velocity, gravity, air.terminal_velocity);
    } else {
        *velocity =
            melee_ft::physics::airborne::gravity(*velocity, air.gravity, air.terminal_velocity);
    }
    crate::common::ordinary_drift(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCo_AirCatch_Coll (800C4438): ft_80081D0C; a landing is Landing
/// without its interrupt (ftCo_Landing_Enter(.., false, ..)).
fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if crate::common::lands(f, &mut p) {
        f.enter_landing_with(p.assets.expect("AirCatch landing assets"), false)?;
    }
    Ok(())
}
