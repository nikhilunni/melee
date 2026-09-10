//! Counter, ftmarsspeciallw.c (801389CC..80139344).
use crate::init::Marth;
use melee_coll::defense::AbsorbDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, PhysicsPhase},
        ActionId, Fighter,
    },
};
#[derive(Clone, Debug, Default)]
pub struct SpecialLw {
    /// Fighter +2340, mv.ms.speciallw.x0: scaled damage (used by future Roy).
    pub damage: i32,
    /// ftColl_8007B1B8: owned Counter shield volume, using the shared defense layout.
    pub volume: Option<AbsorbDescriptor>,
    /// Fighter shield_unk0 / shield_unk1, both assigned attribute +60.
    pub collision_multiplier: f32,
    /// Fighter +19A4 and specialn_facing_dir: strongest pending shield contact.
    pub pending: Option<(i32, f32)>,
}
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        unimplemented!("ftMs_SpecialAirLw_Enter");
    }
    f.physics.self_velocity.y = 0.0;
    f.change_motion_state(ActionId(369), a)
        .expect("Counter assets");
    f.step_animation(a);
    f.commands.variables[1] = 0;
    f.character.get_mut::<Marth>().special_lw = Default::default();
}
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    match f.commands.variables[1] {
        1 => {
            f.commands.variables[1] = 2;
            let m = f.character.get_mut::<Marth>();
            let v = &m.attributes.counter_volume;
            m.special_lw.volume = Some(AbsorbDescriptor {
                bone: v.bone as usize,
                offset: v.offset,
                radius: v.radius,
            });
            m.special_lw.collision_multiplier = m.attributes.counter.collision_multiplier;
        }
        0 => f.character.get_mut::<Marth>().special_lw.volume = None,
        _ => {}
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.character.get_mut::<Marth>().special_lw.volume = None;
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
pub fn hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    // ftMs_SpecialLwHit_Anim: only Roy scales its hitboxes; Marth uses script damage.
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    // ftColl_8007AEE0's sampled shield position belongs to the incoming-hit hook.
}

/// ftColl_8007B1B8 / lbColl_80007BCC, before ordinary hurt-capsule tests.
pub fn contact(f: &mut Fighter, attacker: &mut Fighter, _: &FighterAssets, id: usize) -> bool {
    use melee_coll::geometry::{capsule_contact, Capsule};
    let Some(volume) = f.character.get::<Marth>().special_lw.volume.as_ref() else {
        return false;
    };
    let (bone, offset, radius) = (volume.bone, volume.offset, volume.radius);
    let c = &mut f.core;
    let position =
        melee_ft::fighter::caches::bone_position(&mut c.skeleton, c.animation.root, bone, offset);
    let matrix = *c.skeleton.get_mtx(c.animation.parts[bone].joint);
    let hit = attacker.commands.hitboxes[id]
        .as_ref()
        .expect("eligible hit");
    let Some(contact) = capsule_contact(
        Capsule {
            start: hit.previous_position,
            end: hit.position,
            radius: hit.descriptor.radius
                * if hit.descriptor.ignore_scale {
                    1.0
                } else {
                    attacker.player.scale
                },
        },
        Capsule {
            start: position,
            end: position,
            radius,
        },
        &matrix,
        20.0 * c.player.scale,
    ) else {
        return false;
    };
    let damage = gekko_math::msl::fctiwz(hit.descriptor.damage).max(1);
    let group = hit.descriptor.group;
    let facing = if f.physics.position.x > attacker.physics.position.x {
        -1.0
    } else {
        1.0
    };
    let scratch = &mut f.character.get_mut::<Marth>().special_lw;
    if scratch.pending.is_none_or(|(old, _)| damage > old) {
        scratch.pending = Some((damage, facing));
    }
    let minimum = scratch.collision_multiplier;
    f.combat.minimum_hitlag = minimum;
    attacker.combat.minimum_hitlag = minimum;
    attacker.combat.dealt_damage = attacker.combat.dealt_damage.max(damage);
    melee_coll::detection::record_victim(&mut attacker.commands.hitboxes, group, f.spawn_number);
    f.effects
        .push(melee_ef::request::EffectRequest::ShieldSpark {
            position: contact.position,
        });
    true
}
/// ftMs_SpecialLw_80139140, called by Fighter_ProcessHit after pair detection.
pub fn process_hit(f: &mut Fighter, a: &FighterAssets) {
    let Some((damage, facing)) = f.character.get_mut::<Marth>().special_lw.pending.take() else {
        return;
    };
    f.physics.facing = facing;
    let multiplier = f
        .character
        .get::<Marth>()
        .attributes
        .counter
        .damage_multiplier;
    f.character.get_mut::<Marth>().special_lw.damage =
        gekko_math::msl::fctiwz(damage as f32 * multiplier);
    f.character.get_mut::<Marth>().special_lw.volume = None;
    let hip = a
        .parts
        .joint(melee_types::FtPart::HipN)
        .expect("Counter hip") as usize;
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
            frames: 120,
            strength: 0.9,
            decay: 0.02,
            phase_step: (std::f64::consts::PI / 3.0) as f32,
        });
    f.change_motion_state(ActionId(370), a)
        .expect("Counter hit assets");
    let bone = a
        .parts
        .joint(melee_types::FtPart::RShoulderN)
        .expect("Counter shoulder") as usize;
    f.effects
        .push(melee_ef::request::EffectRequest::SyncAttached { id: 0x4F1, bone });
    f.effect_state.destroy_on_state_change = true;
    f.combat.dealt_damage = damage;
}
