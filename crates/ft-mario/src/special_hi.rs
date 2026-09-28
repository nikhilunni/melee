//! Super Jump Punch, ftmariospecialhi.c (800E1A54..800E2050).
//!
//! The script lifts Mario off the ground (SetAirborne) and rises him on his
//! animation's TransN, turned by the stick angle the IASA collects until
//! cmd_vars[0] closes it. The animation's end falls special.
use crate::init::Mario;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ecb::EcbPose},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    physics::{airborne, friction, grounded},
};
use melee_types::GroundOrAir;

/// ftMr_MS_SpecialHi (347) and ftMr_MS_SpecialAirHi (348).
pub const GROUND: ActionId = ActionId(347);
pub const AIR: ActionId = ActionId(348);

/// `MTXDegToRad(1)` as MWCC rounds it (retail @291).
const DEGREES_TO_RADIANS: f32 = 0.017453292;

/// Fighter +6BC lstick_angle, which Fighter_ChangeMotionState zeroes
/// (fighter.c:1171) and only this move reads here.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SuperJumpPunch {
    pub angle: f32,
}

fn attributes(f: &Fighter) -> &crate::attributes::SuperJumpPunchAttributes {
    &f.character.get::<Mario>().attributes.super_jump_punch
}

/// ftMr_SpecialHi_Enter (800E1A54) / ftMr_SpecialAirHi_Enter (800E1AB0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    if air {
        f.physics.self_velocity.y = 0.0;
        // 800E1AE0: fmuls.
        f.physics.self_velocity.x *= attributes(f).vel_x;
    }
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Super Jump Punch assets");
    f.character.get_mut::<Mario>().super_jump_punch = SuperJumpPunch::default();
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// ftMr_SpecialHi_Anim (800E1B24) / ftMr_SpecialAirHi_Anim: ftCo_80096900
/// (gobj, 0, 1, 0, freefall mobility, landing lag) at the end.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let (mobility, lag) = {
            let a = attributes(f);
            (a.freefall_mobility, a.landing_lag)
        };
        f.enter_special_fall(p.assets, false, true, false, mobility, lag)?;
    }
    Ok(None)
}

/// The steering angle a stick past `range` asks for (800E1C34..6C: fsubs,
/// then the double fsub, fdiv and fmul, rounded once; the degrees to
/// radians is an fmuls, negated for a stick to the right).
fn stick_angle(x: f32, range: f32, angle_diff: f32) -> f32 {
    let magnitude = if x < 0.0 { -x } else { x };
    let fraction = f64::from(magnitude - range) / (1.0 - f64::from(range));
    let degrees = (f64::from(angle_diff) * fraction) as f32;
    let radians = DEGREES_TO_RADIANS * degrees;
    if x > 0.0 {
        -radians
    } else {
        radians
    }
}

/// ftMr_SpecialHi_IASA (800E1BE4) / ftMr_SpecialAirHi_IASA: collect the
/// widest steering angle while cmd_vars[0] is clear; on throw_flags_b3 a
/// firm stick turns Mario around.
pub fn input(f: &mut Fighter, _: InputPhase<'_>) {
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    let (range, angle_diff, reverse) = {
        let a = attributes(f);
        (a.momentum_stick_range, a.angle_diff, a.reverse_stick_range)
    };
    if f.commands.variables[0] == 0 && magnitude > range {
        let angle = stick_angle(x, range, angle_diff);
        let scratch = &mut f.character.get_mut::<Mario>().super_jump_punch;
        let current = if scratch.angle < 0.0 {
            -scratch.angle
        } else {
            scratch.angle
        };
        let wanted = if angle < 0.0 { -angle } else { angle };
        if wanted > current {
            scratch.angle = angle;
        }
    }
    if f.commands.take_throw_flag_b3() && magnitude > reverse {
        // ftCommon_UpdateFacing, then ftPartSetRotY(fp, 0, M_PI_2 * facing)
        // (800E1CFC..D10: fmul in double, frsp).
        f.physics.facing = if x >= 0.0 { 1.0 } else { -1.0 };
        let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
        let root = f.animation.root;
        f.skeleton.set_rotation_y(root, rotation);
    }
}

/// ft_80085154 (80085154): TransN turned by the steering angle
/// (80085198: fmsubs, 8008519C: fmadds).
fn steered_root_motion(f: &mut Fighter) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Super Jump Punch TransN")
        .primary_history
        .offset;
    let angle = f.character.get::<Mario>().super_jump_punch.angle;
    let cosine = gekko_math::msl::cosf(angle);
    let sine = gekko_math::msl::sinf(angle);
    let horizontal = offset.z * f.physics.facing;
    f.physics.self_velocity.x = gekko_math::fma::fmsubs(horizontal, cosine, offset.y * sine);
    f.physics.self_velocity.y = gekko_math::fma::fmadds(horizontal, sine, offset.y * cosine);
}

/// Fighter_procUpdate's tail after the state's physics callback.
fn finish_update(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        grounded::finish_ground_update(
            &mut f.core.physics,
            &f.core.collision.data,
            &grounded::GroundedParameters::from_attributes(&f.core.attributes, &p.assets.common),
            p.map,
            p.wind,
        );
    } else {
        f.core.finish_air_update(p.assets, p.wind);
    }
}

/// ftMr_SpecialHi_Phys (800E1E74): ft_80085154 once the script lifted him,
/// ft_80084FA8's root motion before.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        return callbacks::physics::jab(f, p);
    }
    steered_root_motion(f);
    finish_update(f, &p);
}

/// ftMr_SpecialAirHi_Phys (800E1EAC): the steered rise scaled by vel_mul
/// once cmd_vars[0] is set (three fmuls, x, y, z); before it, gravity
/// (ftCommon_Fall with PlCo terminal velocity) and ftCommon_8007CF58's
/// friction.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, multiplier) = {
        let a = attributes(f);
        (a.gravity, a.vel_mul)
    };
    if f.commands.variables[0] != 0 {
        steered_root_motion(f);
        let v = &mut f.physics.self_velocity;
        v.x *= multiplier;
        v.y *= multiplier;
        v.z *= multiplier;
    } else {
        let terminal = f.attributes.air.terminal_velocity;
        f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
        let air = &f.core.attributes.air;
        f.core.physics.animation_velocity.x = friction::air_drift_friction_acceleration(
            f.core.physics.self_velocity.x,
            air.aerial_friction,
            air.air_drift_max,
            p.assets.common.over_drift_air_friction,
        );
    }
    finish_update(f, &p);
}

/// ft_80083B68 -> ft_80082578 -> mpColl_800477E0: airborne collision that
/// never lands.
fn stay_airborne(f: &mut Fighter, p: &mut CollisionPhase<'_>) {
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
    p.map.air_collide_stay(cd, Some(&|i| pose.position(i)));
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
}

/// ftMr_SpecialHi_Coll (800E1F70) / ftMr_SpecialAirHi_Coll: on the ground
/// ft_80084104; in the air, never landing while rising, then ft_800831CC
/// with ftCo_80096CC8's floor filter, the special landing
/// (ftMr_SpecialHi_CheckLanding), wall jump and ledge.
pub fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        return callbacks::collision::escape(f, p);
    }
    if f.commands.variables[0] == 0 || f.physics.self_velocity.y >= 0.0 {
        stay_airborne(f, &mut p);
        return Ok(());
    }
    let assets = p.assets.expect("Super Jump Punch map assets");
    let drop_threshold = assets.input.platform_drop_threshold;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_fall_filtered(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
        c.input.current.stick.y,
        drop_threshold,
    ) {
        let lag = attributes(f).landing_lag;
        f.enter_special_landing(assets, false, lag)?;
    } else if !f.try_wall_jump(assets, p.map)? {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}
