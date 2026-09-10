//! Fire Fox / Fire Bird, ftfoxspecialhi.c (800E71AC..800E83E0).
use crate::{
    special_s::{air_friction, finish_air, finish_ground, row},
    FamilyState as S, FoxFamily,
};
use gekko_math::{
    fma::fnmsubs,
    msl::{cosf, fctiwz, sinf},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
    physics::{airborne, friction},
};
use melee_types::{CommonMotionState, FtPart, GroundOrAir};

#[derive(Clone, Debug, Default)]
pub struct SpecialHi {
    /// Fighter +2340, gravityDelay.
    pub gravity_delay: i32,
    /// Fighter +2344, rotateModel: launch direction relative to facing.
    pub angle: f32,
    /// Fighter +2348, travelFrames.
    pub travel_frames: i32,
    /// Fighter +234C: elapsed launch physics ticks.
    pub travel_ticks: i32,
    /// Fighter +2350: grounded launch collision ticks.
    pub collision_ticks: i32,
    /// accessory4_cb, one-shot charge or launch effect.
    pub pending_effect: Option<u16>,
}

pub const fn rows<C: FoxFamily>() -> [MotionRow; 6] {
    [
        row(
            S::SpecialHiHold,
            307,
            hold::<C>,
            no_input,
            callbacks::physics::guard_on,
            hold_ground_collision,
        ),
        row(
            S::SpecialHiHoldAir,
            308,
            hold::<C>,
            no_input,
            hold_air_physics::<C>,
            hold_air_collision,
        ),
        row(
            S::SpecialHi,
            309,
            travel::<C>,
            no_input,
            travel_ground_physics::<C>,
            travel_ground_collision,
        ),
        row(
            S::SpecialAirHi,
            309,
            travel::<C>,
            no_input,
            travel_air_physics::<C>,
            travel_air_collision,
        ),
        row(
            S::SpecialHiLanding,
            310,
            end::<C>,
            no_input,
            end_ground_physics::<C>,
            end_ground_collision,
        ),
        row(
            S::SpecialHiFall,
            311,
            end::<C>,
            no_input,
            callbacks::physics::fall,
            end_air_collision,
        ),
    ]
}
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftFx_SpecialHi_Enter / SpecialAirHiStart_Enter (800E7238 / 800E72C4).
pub fn enter<C: FoxFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let a = &f.character.get::<C>().attributes().fire_fox;
    let delay = fctiwz(a.gravity_delay);
    let divisor = a.vel_x;
    // Retail entry uses fdivs, with no fused arithmetic.
    if air {
        f.physics.self_velocity.x /= divisor;
        f.physics.self_velocity.y = 0.0;
    } else {
        f.physics.ground_velocity /= divisor;
    }
    *f.character.get_mut::<C>().special_hi() = SpecialHi {
        gravity_delay: delay,
        pending_effect: Some(0x48B),
        ..Default::default()
    };
    f.change_motion_state(
        (if air {
            S::SpecialHiHoldAir
        } else {
            S::SpecialHiHold
        })
        .into(),
        assets,
    )
    .expect("Fire Fox charge assets");
    f.step_animation(assets);
}
fn hold<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.physics.ground_or_air == GroundOrAir::Ground {
            let stick = f.input.current.stick;
            let minimum = f
                .character
                .get::<C>()
                .attributes()
                .fire_fox
                .direction_stick_min;
            if stick.x.abs() + stick.y.abs() >= minimum && stick.y <= 0.0 {
                unimplemented!("ftFx_SpecialAirHi_AirToGround: floor-directed launch");
            }
            // ftCommon_8007D60C: unlike ordinary Fall, five locked ECB ticks.
            f.physics.ground_or_air = GroundOrAir::Air;
            f.physics.ground_velocity = 0.0;
            f.physics.animation_velocity.y = 0.0;
            f.collision.lock_frames = 5;
            f.collision.data.x130_flags |= melee_types::mp::coll_data_x130::LOCKED;
        }
        launch::<C>(f, p.assets)?;
    }
    Ok(None)
}
/// ftFx_SpecialAirHi_Enter (800E7C98): separate fmuls for initial velocity.
fn launch<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (minimum, facing_minimum, speed, duration) = (
        a.direction_stick_min,
        a.facing_stick_min,
        a.speed,
        fctiwz(a.duration),
    );
    let stick = f.input.current.stick;
    let angle = if stick.x.abs() + stick.y.abs() >= minimum {
        if stick.x.abs() > facing_minimum {
            f.physics.facing = if stick.x < 0.0 { -1.0 } else { 1.0 };
        }
        melee_lb::trigf::atan2f(stick.y, stick.x * f.physics.facing)
    } else {
        std::f32::consts::FRAC_PI_2
    };
    f.change_motion_state(S::SpecialAirHi.into(), assets)?;
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.angle = angle;
    scratch.travel_frames = duration;
    scratch.travel_ticks = 0;
    scratch.collision_ticks = 0;
    scratch.pending_effect = Some(0x48C);
    f.physics.self_velocity.x = f.physics.facing * (speed * cosf(angle));
    f.physics.self_velocity.y = speed * sinf(angle);
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    let part = assets.parts.part_to_joint[FtPart::XRotN as usize].expect("XRotN");
    let joint = f.animation.parts[usize::from(part)].joint;
    // ftFox_SpecialHi_RotateModel: fsubs from single-precision 2*pi.
    f.skeleton
        .set_rotation_x(joint, std::f32::consts::TAU - angle);
    Ok(())
}
fn travel<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.travel_frames -= 1;
    if scratch.travel_frames <= 0 {
        // ftCommon_8007DB24 precedes the end-state transition.
        f.effects.push(EffectRequest::DestroyOwned);
        f.effect_state.destroy_on_state_change = false;
        f.change_motion_state(
            (if f.physics.ground_or_air == GroundOrAir::Air {
                S::SpecialHiFall
            } else {
                S::SpecialHiLanding
            })
            .into(),
            p.assets,
        )?;
    }
    Ok(None)
}
fn end<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.physics.ground_or_air == GroundOrAir::Air {
            let a = &f.character.get::<C>().attributes().fire_fox;
            let (mobility, lag) = (a.freefall_mobility, a.landing_lag);
            f.enter_special_fall(p.assets, false, true, mobility, lag)?;
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}
fn hold_air_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let scratch = f.character.get_mut::<C>().special_hi();
    let falling = scratch.gravity_delay == 0;
    if !falling {
        scratch.gravity_delay -= 1;
    }
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (gravity, friction) = (a.fall_accel, a.air_momentum_preserve_x);
    if falling {
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            gravity,
            f.attributes.air.terminal_velocity,
        );
    }
    air_friction(f, friction);
    finish_air(f, p);
}
fn travel_air_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.travel_ticks += 1;
    let (ticks, angle) = (scratch.travel_ticks, scratch.angle);
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (end, acceleration) = (a.duration_end, a.reverse_accel);
    if ticks as f32 >= end {
        // Retail 800E77C8 / 800E77E0 fnmsubs; inner horizontal fmuls is separate.
        f.physics.self_velocity.x = fnmsubs(
            f.physics.facing,
            acceleration * cosf(angle),
            f.physics.self_velocity.x,
        );
        f.physics.self_velocity.y = fnmsubs(acceleration, sinf(angle), f.physics.self_velocity.y);
    }
    finish_air(f, p);
}
fn travel_ground_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.travel_ticks += 1;
    let ticks = scratch.travel_ticks;
    let a = &f.character.get::<C>().attributes().fire_fox;
    if ticks as f32 >= a.duration_end {
        f.physics.ground_acceleration =
            friction::friction_acceleration(f.physics.ground_velocity, a.reverse_accel);
    }
    finish_ground(f, p);
}
fn end_ground_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = f
        .character
        .get::<C>()
        .attributes()
        .fire_fox
        .ground_momentum_end;
    f.physics.ground_acceleration =
        friction::friction_acceleration(f.physics.ground_velocity, friction);
    finish_ground(f, p);
}
pub(crate) fn grounded_support(f: &mut Fighter, p: CollisionPhase<'_>) -> bool {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let c = &mut f.core;
    matches!(
        map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        WaitGroundResult::Supported
    )
}
fn hold_ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if !grounded_support(f, p) {
        unimplemented!("ftFx_SpecialHiHold_GroundToAir: preserved charge transition");
    }
    Ok(())
}
fn travel_ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if !grounded_support(f, p) {
        unimplemented!("ftFx_SpecialHi_GroundToAir: preserved launch transition");
    }
    Ok(())
}
fn end_ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if !grounded_support(f, p) {
        unimplemented!("ftFx_SpecialHiLanding_Coll: lost-support special-fall entry");
    }
    Ok(())
}
fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<bool> {
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = melee_ft::collision::air::collide_fall(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    );
    if !landed {
        f.try_grab_ledge(p.assets.expect("Fire Fox collision assets"), p.map)?;
    }
    Ok(landed)
}
fn hold_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if air_collision(f, p)? {
        unimplemented!("ftFx_SpecialHiHoldAir_AirToGround: preserved charge transition");
    }
    Ok(())
}
fn travel_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if air_collision(f, p)? {
        unimplemented!("ftFx_SpecialAirHi_Coll: rebound or floor-directed launch");
    }
    Ok(())
}
fn end_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if air_collision(f, p)? {
        unimplemented!("ftFx_SpecialHiFall_Enter: frame-13 landing recovery");
    }
    Ok(())
}
pub fn accessory<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let pending = f
        .character
        .get_mut::<C>()
        .special_hi()
        .pending_effect
        .take();
    if let Some(id) = pending {
        let part = if id == 0x48B {
            FtPart::TransN
        } else {
            FtPart::HipN
        };
        let bone =
            usize::from(assets.parts.part_to_joint[part as usize].expect("Fire Fox effect bone"));
        f.effects.push(EffectRequest::SyncAttached { id, bone });
        f.effect_state.destroy_on_state_change = true;
    }
    if f.motion_state.action.0 == S::SpecialAirHi as u16
        || f.motion_state.action.0 == S::SpecialHi as u16
    {
        let angle = f.character.get_mut::<C>().special_hi().angle;
        // efLib_Cb_SetRotYZ_FromFighter: single-precision fsubs followed by fneg.
        let half_pi = std::f32::consts::FRAC_PI_2 - angle;
        let (y, z) = if f.physics.facing < 0.0 {
            (-std::f32::consts::FRAC_PI_2, half_pi)
        } else {
            (std::f32::consts::FRAC_PI_2, -half_pi)
        };
        f.effects.push(EffectRequest::OwnedRotation {
            model: 0xBBC,
            rotation: hsd_types::Vec3::new(0.0, y, z),
        });
    }
}
