//! Shield Breaker, ftmarsspecialn.c (80136800..8013741C).
use crate::init::Marth;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
    physics::{friction, grounded},
};

#[derive(Clone, Debug, Default)]
pub struct SpecialN {
    /// Fighter +2340, mv.ms.specialn.cur_frame: accumulated charge ticks.
    pub charge_ticks: i32,
    /// accessory4_cb = ftMs_SpecialN_801365A8.
    pub pending_effect: bool,
}
/// lb_800119DC's arguments for the charge loop's gust (80136B50..60).
const CHARGE_GUST: Gust = Gust {
    frames: 10,
    strength: 0.5,
    decay: 0.05,
};
/// The release's gust (80136FC8..D8), on this animation frame.
const RELEASE_GUST: Gust = Gust {
    frames: 120,
    strength: 0.9,
    decay: 0.02,
};
const RELEASE_GUST_FRAME: f32 = 9.0;

struct Gust {
    frames: i32,
    strength: f32,
    decay: f32,
}

/// lb_800119DC at the HipN joint (lb_8000B1CC with no offset): a radial
/// field that pushes nearby dynamics chains, phase step MTXDegToRad(60).
fn gust(f: &mut Fighter, a: &FighterAssets, gust: Gust) {
    let hip = a
        .parts
        .joint(melee_types::FtPart::HipN)
        .expect("Shield Breaker hip") as usize;
    let c = &mut f.core;
    let center = melee_ft::fighter::caches::bone_position(
        &mut c.skeleton,
        c.animation.root,
        hip,
        hsd_types::Vec3::ZERO,
    );
    c.commands
        .radial_impulses
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: gust.frames,
            strength: gust.strength,
            decay: gust.decay,
            phase_step: (std::f64::consts::PI / 3.0) as f32,
        });
}

pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let divisor = f
        .character
        .get::<Marth>()
        .attributes
        .shield_breaker
        .momentum_divisor;
    if air {
        f.physics.self_velocity.x /= divisor;
        if f.physics.self_velocity.y <= 0.0 {
            f.physics.self_velocity.y = 0.0;
        }
    } else {
        f.physics.ground_velocity /= divisor;
    }
    f.commands.variables[0] = 0;
    f.character.get_mut::<Marth>().special_n = Default::default();
    f.change_motion_state(ActionId(if air { 345 } else { 341 }), a)
        .expect("Shield Breaker assets");
    f.step_animation(a);
}
pub fn start(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(
            ActionId(
                if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
                    346
                } else {
                    342
                },
            ),
            p.assets,
        )?;
        f.commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 99,
                duration: 0,
            });
    }
    Ok(None)
}
pub fn hold(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let maximum = f
        .character
        .get::<Marth>()
        .attributes
        .shield_breaker
        .maximum_charge_levels
        * 30;
    // doLoopAnim: every 30 charge ticks a small gust leaves the hips.
    if f.character.get::<Marth>().special_n.charge_ticks % 30 == 0 {
        gust(f, p.assets, CHARGE_GUST);
    }
    let s = &mut f.character.get_mut::<Marth>().special_n;
    s.charge_ticks += 1;
    if s.charge_ticks > maximum {
        release(f, true, p.assets)?;
    }
    Ok(None)
}
fn release(f: &mut Fighter, full: bool, a: &FighterAssets) -> Result<()> {
    let base = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        347
    } else {
        343
    };
    f.change_motion_state_at(ActionId(base + u16::from(full)), a, 1.0)?;
    f.commands.variables[0] = u32::from(full);
    f.character.get_mut::<Marth>().special_n.pending_effect = true;
    Ok(())
}
pub fn input(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        release(f, false, p.assets).expect("Shield Breaker release");
    }
}
pub fn end(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == 0 {
        let m = f.character.get::<Marth>();
        let a = &m.attributes.shield_breaker;
        let damage = (a.base_damage + m.special_n.charge_ticks / 30 * a.damage_per_level) as f32;
        // ftColl_8007ABD0: integer knockback damage precedes staling
        // (ft_80089228 on the charged damage).
        let staled = f.commands.stale_damage(damage);
        for hit in f.commands.hitboxes.iter_mut().flatten() {
            if hit.phase == melee_coll::hitbox::CapsulePhase::Enabled {
                hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
                hit.descriptor.damage = staled;
            }
        }
    }
    // inlineA0 (80136F94): the release's gust on animation frame 9.
    if f.animation.frame == RELEASE_GUST_FRAME {
        gust(f, p.assets, RELEASE_GUST);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
            melee_types::CommonMotionState::Fall
        } else {
            melee_types::CommonMotionState::Wait
        };
        f.change_motion_state(state.into(), p.assets)?;
    }
    Ok(None)
}
pub fn startup_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.physics.ground_acceleration = friction::friction_acceleration(
        f.physics.ground_velocity,
        f.character
            .get::<Marth>()
            .attributes
            .shield_breaker
            .friction,
    );
    let c = &mut f.core;
    grounded::apply_ground_movement(
        &mut c.physics,
        c.collision.data.floor.normal,
        p.map.floor_speed_scale(&c.collision.data),
    );
    grounded::finish_ground_update(
        &mut c.physics,
        &c.collision.data,
        &grounded::GroundedParameters::from_attributes(&c.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    if !matches!(
        melee_ft::collision::ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        melee_ft::collision::ground::WaitGroundResult::Supported
    ) {
        let state = f.motion_state.action.0;
        assert!(
            (341..=344).contains(&state),
            "non-Shield Breaker transition"
        );
        // ftCommon_GroundToAirStateChange -> ftCommon_8007D5D4: one jump spent.
        f.leave_ground();
        f.change_ground_air_motion(
            ActionId(state + 4),
            p.assets.expect("Shield Breaker collision assets"),
            preservation(state),
        )?;
    }
    Ok(())
}
pub fn accessory(f: &mut Fighter, _: &FighterAssets) {
    if std::mem::take(&mut f.character.get_mut::<Marth>().special_n.pending_effect) {
        let id = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
            0x4F3
        } else {
            0x4F2
        };
        f.effects
            .push(melee_ef::request::EffectRequest::SyncAttached { id, bone: 0 });
        f.effect_state.destroy_on_state_change = true;
    }
}

/// ftMs_SpecialAirNStart_Phys / Loop_Phys / End_Phys: gravity and friction.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    use melee_ft::physics::{airborne, integrate};
    let friction = if f.motion_state.action.0 == 345 {
        f.character
            .get::<Marth>()
            .attributes
            .shield_breaker
            .friction
    } else {
        f.attributes.air.aerial_friction
    };
    f.physics.self_velocity.y = airborne::gravity(
        f.physics.self_velocity.y,
        f.attributes.air.gravity,
        f.attributes.air.terminal_velocity,
    );
    // ftCommon_ApplyFrictionAir (8007CE94): inclusive magnitude comparison.
    let x = f.physics.self_velocity.x;
    f.physics.animation_velocity.x = if friction.abs() >= x.abs() {
        -x
    } else if x > 0.0 {
        -friction
    } else {
        friction
    };
    f.decay_air_knockback(p.assets);
    integrate::integrate_velocity(&mut f.physics);
    integrate::integrate_environment(&mut f.physics, None, p.wind);
}
/// ftMs_SpecialAirN*_Coll -> ft_80081D0C; preserved ground counterpart.
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        let state = f.motion_state.action.0;
        f.land();
        f.change_ground_air_motion(
            ActionId(state - 4),
            p.assets.expect("Shield Breaker landing assets"),
            preservation(state),
        )?;
    }
    Ok(())
}

fn preservation(state: u16) -> melee_ft::fighter::MotionPreservation {
    let phase = (state - 341) % 4;
    melee_ft::fighter::MotionPreservation {
        hit_status: true,
        hitboxes: phase >= 2,
        effects: phase != 0,
    }
}
