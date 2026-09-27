//! Counter, ftmarsspeciallw.c (801389CC..80139344).
use crate::init::Marth;
use melee_coll::defense::AbsorbDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation,
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
        // retail 80138A60: fdivs before zeroing vertical speed.
        f.physics.self_velocity.x /= f
            .character
            .get::<Marth>()
            .attributes
            .counter
            .momentum_divisor;
    }
    f.physics.self_velocity.y = 0.0;
    f.change_motion_state(ActionId(if air { 371 } else { 369 }), a)
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
        finish(f, p.assets)?;
    }
    Ok(None)
}
pub fn hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    // ftMs_SpecialLwHit_Anim: only Roy scales its hitboxes; Marth uses script damage.
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets)?;
    }
    Ok(None)
}
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    // ftColl_8007AEE0's sampled shield position belongs to the incoming-hit hook.
}

/// ftMs_SpecialAirLw*_Anim: return to ordinary Fall, retaining remaining jumps.
fn finish(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        f.change_motion_state(melee_types::CommonMotionState::Fall.into(), assets)
    } else {
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), assets)
    }
}

/// ftMs_SpecialAirLw_Phys (80138C5C) and Hit_Phys (80138FE8):
/// custom gravity/friction during the stance; ft_80084EEC during retaliation.
/// Neither accepts aerial steering or fast fall.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    use melee_ft::physics::{airborne, integrate};
    let (gravity, terminal, friction) = if f.motion_state.action.0 == 371 {
        let a = &f.character.get::<Marth>().attributes.counter;
        (a.fall_acceleration, a.terminal_velocity, a.air_friction)
    } else {
        let a = &f.attributes.air;
        (a.gravity, a.terminal_velocity, a.aerial_friction)
    };
    f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
    // ftCommon_ApplyFrictionAir (8007CE94): inclusive clamp, separate subtract.
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

/// ftMs_SpecialLw*_Coll (80138CC0/80139008): stance uses the escape
/// floor probe; retaliation uses the ordinary ground-action probe.
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::ground::{map_escape, map_ground_action, WaitGroundResult};
    let stance = f.motion_state.action.0 == 369;
    let probe = if stance {
        map_escape
    } else {
        map_ground_action
    };
    let c = &mut f.core;
    if !matches!(
        probe(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        WaitGroundResult::Supported
    ) {
        // ftCommon_8007D5D4: preserve the double jump when walking off a ledge.
        f.leave_ground();
        transition(
            f,
            p.assets.expect("Counter collision assets"),
            if stance { 371 } else { 372 },
            stance,
        )?;
    }
    Ok(())
}

/// ftMs_SpecialAirLw*_Coll (80138CFC/80139044): land without restarting.
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::air;
    let stance = f.motion_state.action.0 == 371;
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
        f.land();
        transition(
            f,
            p.assets.expect("Counter landing assets"),
            if stance { 369 } else { 370 },
            stance,
        )?;
    }
    Ok(())
}

/// ftMs_SpecialLw_80138D38/80138DD0/80139080/801390E0:
/// flags 0x0C4C508C preserve hit status/hitboxes; hit variant also keeps graphics.
fn transition(f: &mut Fighter, a: &FighterAssets, state: u16, stance: bool) -> Result<()> {
    f.change_ground_air_motion(
        ActionId(state),
        a,
        MotionPreservation {
            hit_status: true,
            hitboxes: true,
            effects: !stance,
        },
    )?;
    if stance && f.commands.variables[1] == 2 {
        let m = f.character.get_mut::<Marth>();
        let v = &m.attributes.counter_volume;
        m.special_lw.volume = Some(AbsorbDescriptor {
            bone: v.bone as usize,
            offset: v.offset,
            radius: v.radius,
        });
    }
    Ok(())
}

/// The Counter's shield volume where its bone holds it this frame
/// (ftColl_8007B1B8's shield_hit, placed by lbColl_80007BCC).
fn counter_volume(f: &mut Fighter) -> Option<melee_ft::fighter::damage::DefenseVolume> {
    let volume = f.character.get::<Marth>().special_lw.volume.as_ref()?;
    let (bone, offset, radius) = (volume.bone, volume.offset, volume.radius);
    let c = &mut f.core;
    let position =
        melee_ft::fighter::caches::bone_position(&mut c.skeleton, c.animation.root, bone, offset);
    let matrix = *c.skeleton.get_mtx(c.animation.parts[bone].joint);
    Some(melee_ft::fighter::damage::DefenseVolume {
        position,
        matrix,
        radius,
    })
}

/// lbColl_80007BCC: a swept hit capsule against the Counter volume.
fn counter_contact(
    f: &Fighter,
    volume: &melee_ft::fighter::damage::DefenseVolume,
    hit: &melee_coll::hitbox::HitCapsule,
    attacker_scale: f32,
) -> Option<melee_coll::geometry::Contact> {
    use melee_coll::geometry::{capsule_contact, Capsule};
    capsule_contact(
        Capsule {
            start: hit.previous_position,
            end: hit.position,
            radius: hit.descriptor.radius
                * if hit.descriptor.ignore_scale {
                    1.0
                } else {
                    attacker_scale
                },
        },
        Capsule {
            start: volume.position,
            end: volume.position,
            radius: volume.radius,
        },
        &volume.matrix,
        20.0 * f.core.player.scale,
    )
}

/// ftColl_8007925C's shield step against the Counter, then ftColl_80077688:
/// an item's hit is caught like a fighter's, keeping the strongest contact.
pub fn item_contact(f: &mut Fighter, item: &mut melee_it::ItemCore, id: usize) -> bool {
    let Some(volume) = counter_volume(f) else {
        return false;
    };
    let hit = item.hitboxes[id].clone().expect("eligible item hit");
    let Some(contact) = counter_contact(f, &volume, &hit, item.scale) else {
        return false;
    };
    let own_hitlag = f.character.get::<Marth>().special_lw.collision_multiplier;
    let damage = f.record_item_volume_hit(item, id, contact, &volume, own_hitlag);
    let facing = if f.physics.position.x > item.position.x {
        -1.0
    } else {
        1.0
    };
    let scratch = &mut f.character.get_mut::<Marth>().special_lw;
    if scratch.pending.is_none_or(|(old, _)| damage > old) {
        scratch.pending = Some((damage, facing));
    }
    // ftColl_80077688: x1964 takes shield_unk1 for a defense-interacting hit.
    if item.hit_flags[id].defense_interaction {
        f.combat.minimum_hitlag = own_hitlag;
    }
    true
}

/// ftColl_8007B1B8 / lbColl_80007BCC, before ordinary hurt-capsule tests.
pub fn contact(f: &mut Fighter, attacker: &mut Fighter, _: &FighterAssets, id: usize) -> bool {
    let Some(volume) = counter_volume(f) else {
        return false;
    };
    let hit = attacker.commands.hitboxes[id]
        .as_ref()
        .expect("eligible hit");
    let Some(contact) = counter_contact(f, &volume, hit, attacker.player.scale) else {
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
    attacker.record_shield_recoil(damage, f.shield.lightshield, -facing);
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
    f.change_motion_state(
        ActionId(
            if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
                372
            } else {
                370
            },
        ),
        a,
    )
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
