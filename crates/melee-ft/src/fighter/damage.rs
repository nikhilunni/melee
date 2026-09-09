//! Fighter-pair hit detection and grounded damage, ftcoll.c / ftCo_Damage.c.
use super::{
    assets::{FighterAssets, Result},
    caches::{bone_position, HurtHeight},
    hitbox::{HitCapsule, HitboxDescriptor},
    CharacterCallbacks, Fighter, Interaction, MotionData,
};
use gekko_math::{
    fma::fmadds,
    msl::{cosf, fctiwz},
};
use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_lb::collision::{capsule_contact, Capsule, Contact};
use melee_types::{CommonMotionState as S, GroundOrAir};

#[derive(Default)]
pub struct CombatState {
    pub hitlag_remaining: f32,
    pub pending: Option<ReceivedHit>,
    pub dealt_damage: i32,
    /// The stale-move queue is outside this first-hit slice. Do not silently
    /// treat a subsequent contact as another unstaled attack.
    pub has_recorded_hit: bool,
}
#[derive(Clone, Debug)]
pub struct DamageState {
    pub hitstun: f32,
}
pub struct ReceivedHit {
    pub descriptor: HitboxDescriptor,
    pub height: HurtHeight,
    pub facing: f32,
    pub knockback: f32,
}
pub struct DamageParameters {
    pub weight_scale: f32,
    pub weight_decay: f32,
    pub velocity_scale: f32,
    pub maximum: f32,
    pub percent_scale: f32,
    pub damage_scale: f32,
    pub fixed_percent: f32,
    pub growth_scale: f32,
    pub base: f32,
    pub hitstun_scale: f32,
    pub reaction_thresholds: [f32; 3],
    pub grounded_angle_threshold: f32,
    pub hitlag_scale: f32,
    pub hitlag_base: f32,
    pub maximum_hitlag: f32,
    pub phantom_threshold: f32,
}
impl DamageParameters {
    pub fn read(a: &Archive, p: u32) -> Result<Self> {
        let r = a.reader();
        Ok(Self {
            weight_scale: r.f32(p + 0xf4)?,
            weight_decay: r.f32(p + 0xf8)?,
            velocity_scale: r.f32(p + 0x100)?,
            maximum: r.f32(p + 0x108)?,
            percent_scale: r.f32(p + 0x110)?,
            damage_scale: r.f32(p + 0x114)?,
            fixed_percent: r.f32(p + 0x118)?,
            growth_scale: r.f32(p + 0x11c)?,
            base: r.f32(p + 0x120)?,
            hitstun_scale: r.f32(p + 0x154)?,
            reaction_thresholds: [r.f32(p + 0x158)?, r.f32(p + 0x15c)?, r.f32(p + 0x160)?],
            grounded_angle_threshold: r.f32(p + 0x14c)?,
            hitlag_scale: r.f32(p + 0x198)?,
            hitlag_base: r.f32(p + 0x19c)?,
            maximum_hitlag: r.f32(p + 0x194)?,
            phantom_threshold: r.f32(p + 0x7a8)?,
        })
    }
    /// ftColl_80079AB0 (80079AB0), ordinary Vs 1.0 attack/defense/stage ratios.
    pub fn knockback(&self, hit: &HitboxDescriptor, percent: f32, weight: f32) -> f32 {
        let w = weight * self.weight_scale;
        let factor = self.weight_decay - (w * self.weight_decay) / (1.0 + w);
        let (p, d) = if hit.weight_knockback != 0 {
            (self.fixed_percent, f32::from(hit.weight_knockback))
        } else {
            (fctiwz(percent) as f32 + hit.damage, hit.damage)
        };
        // retail 80079C34 (normal) / 80079B48 (fixed weight): fmadds.
        let inner = fmadds(self.percent_scale, p, self.damage_scale * (d * p));
        // retail 80079C40/C44 (fixed: 9B50/9B54).
        let scaled = fmadds(self.growth_scale, factor * inner, self.base);
        let result = fmadds(
            0.01 * f32::from(hit.growth),
            scaled,
            f32::from(hit.base_knockback),
        );
        if result >= self.maximum {
            self.maximum
        } else {
            result
        }
    }
    /// ftCommon_CalcHitlag (8007DA74), 8007DAA8 fmadds then fctiwz.
    pub fn hitlag(&self, damage: i32) -> f32 {
        (fctiwz(fmadds(damage as f32, self.hitlag_scale, self.hitlag_base)) as f32)
            .min(self.maximum_hitlag)
    }
}
/// ftColl_80078C70 (80078C70): receiver first, other fighters then hitbox IDs,
/// then hurt-table order. The scene supplies that entity-list ordering.
pub fn detect_hit<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &mut Fighter<A>,
    assets: &FighterAssets,
) {
    if victim.status.disabled || attacker.status.disabled {
        return;
    }
    for id in 0..attacker.commands.hitboxes.len() {
        let Some(hit) = &attacker.commands.hitboxes[id] else {
            continue;
        };
        if hit.victims.contains(&victim.spawn_number) {
            continue;
        }
        let desc = &hit.descriptor;
        let grounded = victim.physics.ground_or_air == GroundOrAir::Ground;
        if (grounded && !desc.hit_ground) || (!grounded && !desc.hit_air) {
            continue;
        }
        if victim.commands.hurt_status == super::escape::HurtStatus::Intangible
            || victim.status.ledge_intangibility != 0
        {
            continue;
        }
        if victim.shield.active {
            unimplemented!("ftcoll.c:1804-1846: shield hit");
        }
        let contact = victim.contact_with_hurtboxes(hit, attacker.player.scale);
        if let Some((contact, height)) = contact {
            if victim.motion_state.id != S::Wait {
                unimplemented!(
                    "ftColl_80079AB0: crouch/other damage modifiers outside idle victim"
                );
            }
            if attacker.combat.has_recorded_hit {
                unimplemented!("ftColl_8007ABD0: stale-move damage after the first recorded hit");
            }
            if victim.commands.hitboxes.iter().any(Option::is_some) {
                unimplemented!("ftcoll.c:1758-1801: attack clanking");
            }
            if contact.overlap < assets.damage.phantom_threshold {
                unimplemented!("ftcoll.c:589-623: phantom hit");
            }
            if victim.commands.hurt_status != super::escape::HurtStatus::Normal {
                unimplemented!("ftcoll.c:658-662: invincible contact");
            }
            if victim.combat.pending.is_some() {
                unimplemented!("ftcoll.c:2534: simultaneous damage log selection");
            }
            let descriptor = desc.clone();
            let knockback = assets.damage.knockback(
                &descriptor,
                victim.physics.percent,
                victim.attributes.size.weight,
            );
            victim.combat.pending = Some(ReceivedHit {
                descriptor: descriptor.clone(),
                height,
                facing: if victim.physics.position.x > attacker.physics.position.x {
                    -1.0
                } else {
                    1.0
                },
                knockback,
            });
            attacker.combat.has_recorded_hit = true;
            attacker.combat.dealt_damage = fctiwz(descriptor.damage).max(1);
            for hit in attacker
                .commands
                .hitboxes
                .iter_mut()
                .flatten()
                .filter(|h| h.descriptor.group == descriptor.group)
            {
                hit.victims.push(victim.spawn_number);
            }
            // ftColl_8007A06C -> efSync_Spawn; slash uses effect 1004.
            victim
                .effects
                .push(super::effects::EffectRequest::HitSpark {
                    position: contact.position,
                    element: descriptor.element,
                });
        }
    }
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftColl_80078C70: first colliding hurt capsule wins, in ftData order.
    fn contact_with_hurtboxes(
        &mut self,
        hit: &HitCapsule,
        attacker_scale: f32,
    ) -> Option<(Contact, HurtHeight)> {
        let desc = &hit.descriptor;
        for hurt in &mut self.hurtboxes {
            if !hurt.cached {
                hurt.positions = hurt.offsets.map(|offset| {
                    bone_position(&mut self.skeleton, self.animation.root, hurt.bone, offset)
                });
                hurt.cached = true;
            }
            let bone = self
                .skeleton
                .bone(self.animation.root, hurt.bone)
                .expect("hurt bone");
            let matrix = *self.skeleton.get_mtx(bone);
            if let Some(c) = capsule_contact(
                Capsule {
                    start: hit.previous_position,
                    end: hit.position,
                    radius: desc.radius
                        * if desc.ignore_scale {
                            1.0
                        } else {
                            attacker_scale
                        },
                },
                Capsule {
                    start: hurt.positions[0],
                    end: hurt.positions[1],
                    radius: hurt.radius,
                },
                &matrix,
                3.0 * self.player.scale,
            ) {
                return Some((c, hurt.height));
            }
        }
        None
    }

    pub(super) fn process_damage(&mut self, assets: &FighterAssets) -> Result<()> {
        let mut hit_damage = std::mem::take(&mut self.combat.dealt_damage);
        if let Some(hit) = self.combat.pending.take() {
            if self.physics.ground_or_air != GroundOrAir::Ground {
                unimplemented!("ftCo_Damage.c:346: airborne hit");
            }
            if hit.descriptor.angle != 361
                || hit.knockback >= assets.damage.grounded_angle_threshold
            {
                unimplemented!("ftCo_Damage.c:79-102: nonzero knockback angle");
            }
            let stun = hit.knockback * assets.damage.hitstun_scale;
            let level = assets
                .damage
                .reaction_thresholds
                .iter()
                .position(|&t| stun < t)
                .unwrap_or(3);
            if level != 1 || !matches!(hit.height, HurtHeight::Middle) {
                unimplemented!(
                    "ftCo_Damage.c:63: damage state table level {level}, height {:?}",
                    hit.height
                );
            }
            self.physics.percent += hit.descriptor.damage;
            // ftCo_8008DCE0 --fused: these products are separate (8008DECC,
            // 8008DFCC..E0F4); flat floor and grounded angle 361 take angle 0.
            let speed = hit.knockback * assets.damage.velocity_scale;
            self.physics.facing = hit.facing;
            let horizontal = -(speed * cosf(0.0)) * hit.facing;
            self.physics.ground_knockback_velocity = horizontal;
            let normal = self.collision.data.floor.normal;
            self.physics.knockback_velocity =
                Vec3::new(normal.y * horizontal, -normal.x * horizontal, 0.0);
            self.physics.self_velocity = Vec3::ZERO;
            self.physics.ground_velocity = 0.0;
            self.change_motion_state(S::DamageN2, assets)?;
            self.step_animation(assets);
            self.state_data = MotionData::Damage(DamageState {
                hitstun: (fctiwz(stun).max(1)) as f32,
            });
            self.status.time_since_hit = 0;
            self.status.interaction = Interaction::Damage;
            self.input.horizontal.tilt = 254;
            self.input.vertical.tilt = 254;
            hit_damage = fctiwz(hit.descriptor.damage).max(1);
        }
        if hit_damage != 0 {
            self.combat.hitlag_remaining = assets.damage.hitlag(hit_damage);
            if self.combat.hitlag_remaining > 0.0 {
                self.status.interaction = Interaction::Hitlag;
            }
        }
        Ok(())
    }
    /// Fighter_8006A1BC (8006A1BC): expire before animation and input.
    pub(super) fn tick_hitlag(&mut self) {
        if self.combat.hitlag_remaining > 0.0 {
            self.combat.hitlag_remaining -= 1.0;
            if self.combat.hitlag_remaining <= 0.0 {
                self.combat.hitlag_remaining = 0.0;
                if matches!(self.state_data, MotionData::Damage(_)) {
                    if self.input.current.stick.x != 0.0
                        || self.input.current.stick.y != 0.0
                        || self.input.current.cstick.x != 0.0
                        || self.input.current.cstick.y != 0.0
                    {
                        unimplemented!("ftCo_Damage.c:624-664: ASDI/DI");
                    }
                    self.status.interaction = Interaction::Damage;
                } else {
                    self.status.interaction = Interaction::Attack;
                }
            }
        }
    }
    /// ftCo_Damage_Anim (8008F7F0) -> ftCo_8008F744 (8008F744).
    pub(super) fn damage_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Damage(damage) = &mut self.state_data else {
            panic!("damage scratch missing")
        };
        if damage.hitstun > 0.0 {
            damage.hitstun -= 1.0;
        }
        if !self.animation.frames_remaining(&self.skeleton) && damage.hitstun <= 0.0 {
            self.change_motion_state(S::Wait, assets)?;
        }
        Ok(())
    }
    pub(super) fn damage_input(
        &mut self,
        assets: &FighterAssets,
        context: &crate::input::WaitContext,
    ) -> Result<()> {
        let MotionData::Damage(damage) = &self.state_data else {
            panic!("damage scratch missing")
        };
        if damage.hitstun <= 0.0 {
            let transition = crate::input::wait_iasa(&self.input, &assets.input, context);
            self.apply_ground_transition(assets, transition)?;
        } else if self.input.pressed.0 != 0 {
            unimplemented!("ftCo_Damage.c:1015-1046: hitstun jump buffer");
        }
        Ok(())
    }
}
