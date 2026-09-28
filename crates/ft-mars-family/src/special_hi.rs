//! Dolphin Slash, ftmarsspecialhi.c (80138208..801389CC).
use crate::MarsFamily;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter,
    },
    physics::airborne,
};
use melee_types::GroundOrAir;

#[derive(Clone, Debug, Default)]
pub struct SpecialHi {
    /// Fighter +6BC lstick_angle, shared retail field used only by this move here.
    pub angle: f32,
}
pub fn enter<C: MarsFamily>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[..3].fill(0);
    f.commands.grab_release = false;
    f.commands.throw_reverse = false;
    if air {
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x *= f
            .character
            .get::<C>().attributes()
            .dolphin_slash
            .startup_momentum_multiplier;
    }
    f.character.get_mut::<C>().specials().special_hi = SpecialHi::default();
    crate::retain_scratch_word::<C>(f);
    f.change_motion_state(melee_ft::fighter::ActionId(if air { 368 } else { 367 }), a)
        .expect("Dolphin Slash assets");
    f.step_animation(a);
}
pub fn anim<C: MarsFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let a = &f.character.get::<C>().attributes().dolphin_slash;
        let (mobility, lag) = (a.freefall_mobility, a.landing_lag);
        f.enter_special_fall(p.assets, false, true, false, mobility, lag)?;
    }
    Ok(None)
}
pub fn input<C: MarsFamily>(f: &mut Fighter, _: InputPhase<'_>) {
    let x = f.input.current.stick.x;
    let a = &f.character.get::<C>().attributes().dolphin_slash;
    let (threshold, maximum, reverse) = (
        a.angle_stick_threshold,
        a.maximum_angle,
        a.reverse_stick_threshold,
    );
    if f.commands.variables[0] == 0 && x.abs() > threshold {
        // 801383A8..801384E8: double denominator, separate multiplies, no FMA.
        let degrees =
            (f64::from(x.abs() - threshold) / (1.0 - f64::from(threshold))) as f32 * maximum;
        let mut angle = (std::f32::consts::PI / 180.0) * degrees;
        if x > 0.0 {
            angle = -angle;
        }
        let scratch = &mut f.character.get_mut::<C>().specials().special_hi;
        if angle.abs() > scratch.angle.abs() {
            scratch.angle = angle;
        }
    }
    if std::mem::take(&mut f.commands.grab_release) && x.abs() > reverse {
        f.physics.facing = if x < 0.0 { -1.0 } else { 1.0 };
        let root = f.animation.root;
        let rotation = std::f32::consts::FRAC_PI_2 * f.physics.facing;
        f.skeleton.set_rotation_y(root, rotation);
    }
}
pub fn physics<C: MarsFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        callbacks::physics::jab(f, p);
        return;
    }
    let a = &f.character.get::<C>().attributes().dolphin_slash;
    let (gravity, terminal, mobility, multiplier) = (
        a.fall_acceleration,
        a.terminal_velocity,
        a.freefall_mobility,
        a.ending_momentum_multiplier,
    );
    if f.motion_state.action.0 == 368 && f.commands.variables[0] == 0 {
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            f.attributes.air.gravity,
            f.attributes.air.terminal_velocity,
        );
        f.physics.animation_velocity.x =
            airborne::drift_acceleration(f.physics.self_velocity.x, 0.0, 0.0, &f.attributes.air);
    } else if f.commands.variables[2] == 0 {
        let offset = f
            .animation
            .root_motion
            .as_ref()
            .expect("Dolphin Slash TransN")
            .primary_history
            .offset;
        let angle = f.character.get::<C>().specials_ref().special_hi.angle;
        let cosine = gekko_math::msl::cosf(angle);
        let sine = gekko_math::msl::sinf(angle);
        let horizontal = offset.z * f.physics.facing;
        // ft_80085154: 80085198 fmsubs, 8008519C fmadds.
        f.physics.self_velocity.x = gekko_math::fma::fmsubs(horizontal, cosine, offset.y * sine);
        f.physics.self_velocity.y = gekko_math::fma::fmadds(horizontal, sine, offset.y * cosine);
        if f.motion_state.action.0 == 368 {
            f.physics.self_velocity.x *= multiplier;
            f.physics.self_velocity.y *= multiplier;
            f.physics.self_velocity.z *= multiplier;
        }
        let direction = if f.physics.self_velocity.x < 0.0 {
            -1.0
        } else {
            1.0
        };
        if f.physics.facing != direction {
            f.physics.self_velocity.x *= -1.0;
        }
        if offset.y < 0.0 {
            f.commands.variables[2] = 1;
        }
    } else {
        f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
        let x = f.input.current.stick.x;
        f.physics.animation_velocity.x = airborne::drift_acceleration(
            f.physics.self_velocity.x,
            x * (f.attributes.air.air_drift_stick_mul * mobility),
            x * (f.attributes.air.air_drift_max * mobility),
            &f.attributes.air,
        );
    }
    f.core.finish_air_update(p.assets, p.wind);
}
pub fn collision<C: MarsFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        return callbacks::collision::escape(f, p);
    }
    let stay = if f.commands.variables[0] == 0 || f.physics.self_velocity.y >= 0.0 {
        true
    } else if f.commands.variables[1] == 0 {
        f.commands.variables[1] = 1;
        true
    } else {
        false
    };
    // ftCo_80096CC8 compares the stick against PlCo +25C.
    let assets = p.assets.expect("Dolphin Slash map assets");
    let drop_threshold = assets.input.platform_drop_threshold;
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if stay {
        // ft_80083B68 -> ft_80082578 -> mpColl_800477E0.
        let cd = &mut c.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = c.physics.position;
        let pose = melee_ft::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
        p.map.air_collide_stay(cd, Some(&|i| pose.position(i)));
        c.physics.position = cd.cur_pos;
        c.skeleton
            .set_translate(c.animation.root, &c.physics.position);
    } else if melee_ft::collision::air::collide_fall_filtered(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
        c.input.current.stick.y,
        drop_threshold,
    ) {
        let lag = f
            .character
            .get::<C>().attributes()
            .dolphin_slash
            .landing_lag;
        f.enter_special_landing(p.assets.expect("Dolphin landing assets"), false, lag)?;
    } else {
        f.try_grab_ledge(p.assets.expect("Dolphin ledge assets"), p.map)?;
    }
    Ok(())
}
