//! The roll's ends: End (352/353, 360/361) and the bounce off a hit
//! opponent (Hit, 362).
use super::{
    attributes,
    charge::{assets, fall, lands, stays_grounded},
    end, flags, model,
    roll::collide_air_box,
    scratch,
};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::Result,
        state::{AnimationPhase, CollisionPhase, PhysicsPhase},
        Fighter,
    },
    physics::{airborne, friction, grounded},
};
use melee_types::CommonMotionState;

type AnimResult = Result<Option<WaitChoice>>;

/// ftPr_SpecialNEnd_Anim (8013EAD8): at the end, the pose returns and Wait.
pub(super) fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    scratch(f).pending_facing = 0.0;
    model::squash(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        model::restore(f);
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// ftPr_SpecialAirNEnd_Anim (8013F9C0): at the end, Fall, or a special
/// fall with the landing lag when the attribute sets one.
pub(super) fn air_end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    scratch(f).pending_facing = 0.0;
    model::squash(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        model::restore(f);
        let lag = attributes(f).landing_lag;
        if lag == 0.0 {
            f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
        } else {
            // ftCo_80096900(gobj, 1, 0, 1, 1.0, xD8).
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftPr_SpecialNEnd_Phys (80140BAC): ground friction.
pub(super) fn end_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let ground_friction = f.attributes.ground.ground_friction;
    f.physics.ground_acceleration =
        friction::friction_acceleration(f.physics.ground_velocity, ground_friction);
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        p.map.floor_speed_scale(&core.collision.data),
    );
    super::charge::finish_ground(f, &p);
}

/// ftPr_SpecialNEnd_Coll (801416D0): off an edge, the aerial end at the
/// same frame.
pub(super) fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if stays_grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    let frame = f.animation.frame;
    end(f, true, flags::GROUND_AIR, frame, assets(&p))
}

/// ftPr_SpecialAirNEnd_Coll (80142070): landing, the grounded end at the
/// same frame.
pub(super) fn air_end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    let frame = f.animation.frame;
    end(f, false, flags::GROUND_AIR, frame, assets(&p))
}

/// ftPr_SpecialNHit_Anim (8013FCAC): the ball keeps spinning backward
/// (8013FE64 fmul, 8013FE74 fmul, frsp, 8013FE7C fmadds).
pub(super) fn hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    model::squash(f);
    let a = attributes(f);
    let (rotation, aerial) = (a.turn_rotation_speed, a.air_rotation_multiplier);
    let rollout = scratch(f);
    let step = (0.2 * f64::from(rotation) * f64::from(-rollout.direction)) as f32;
    rollout.roll_angle = gekko_math::fma::fmadds(aerial, step, rollout.roll_angle);
    model::set_roll(f);
    model::face_forward(f);
    Ok(None)
}

/// ftPr_SpecialNHit_Phys (80140F40): drift once falling at terminal
/// speed (ftCommon_8007D268), under the roll's gravity.
pub(super) fn hit_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.self_velocity.y <= -attributes(f).terminal_velocity {
        f.physics.animation_velocity.x = airborne::drift(
            f.physics.self_velocity.x,
            f.input.current.stick.x,
            &f.attributes.air,
        );
    }
    fall(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPr_SpecialNHit_Coll (801420D0): landing restores the pose, then Wait
/// or a special landing with the attribute's lag.
pub(super) fn hit_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !collide_air_box(f, &mut p) {
        return Ok(());
    }
    model::restore(f);
    f.land();
    let assets = assets(&p);
    let lag = attributes(f).landing_lag;
    if lag == 0.0 {
        f.change_motion_state(CommonMotionState::Wait.into(), assets)
    } else {
        // ftCo_LandingFallSpecial_Enter(gobj, 0, xD8).
        f.enter_special_landing(assets, false, lag)
    }
}
