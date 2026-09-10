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
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        unimplemented!("ftMs_SpecialAirN_Enter");
    }
    f.physics.ground_velocity /= f
        .character
        .get::<Marth>()
        .attributes
        .shield_breaker
        .momentum_divisor;
    f.commands.variables[0] = 0;
    f.character.get_mut::<Marth>().special_n = Default::default();
    f.change_motion_state(ActionId(341), a)
        .expect("Shield Breaker assets");
    f.step_animation(a);
}
pub fn start(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(ActionId(342), p.assets)?;
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
    let s = &mut f.character.get_mut::<Marth>().special_n;
    s.charge_ticks += 1;
    if s.charge_ticks > maximum {
        release(f, true, p.assets)?;
    }
    Ok(None)
}
fn release(f: &mut Fighter, full: bool, a: &FighterAssets) -> Result<()> {
    f.change_motion_state_at(ActionId(if full { 344 } else { 343 }), a, 1.0)?;
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
        for hit in f.commands.hitboxes.iter_mut().flatten() {
            if hit.phase == melee_coll::hitbox::CapsulePhase::Enabled {
                // ftColl_8007ABD0: integer knockback damage precedes staling.
                hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
                hit.descriptor.damage = damage;
            }
        }
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
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
        unimplemented!("Shield Breaker preserved ground-to-air transition");
    }
    Ok(())
}
pub fn accessory(f: &mut Fighter, _: &FighterAssets) {
    if std::mem::take(&mut f.character.get_mut::<Marth>().special_n.pending_effect) {
        f.effects
            .push(melee_ef::request::EffectRequest::SyncAttached { id: 0x4F2, bone: 0 });
        f.effect_state.destroy_on_state_change = true;
    }
}
