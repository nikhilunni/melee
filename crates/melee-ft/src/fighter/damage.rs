//! Fighter-pair hit/shield detection and launch reactions, ftcoll.c / ftCo_Damage.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    caches::{bone_position, HurtHeight},
    hitbox::{HitCapsule, HitboxDescriptor},
    CharacterCallbacks, Fighter, Interaction, MotionData,
};
use gekko_math::{
    fma::fmadds,
    msl::{cosf, fctiwz, sinf, sqrtf},
};
use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_lb::collision::{capsule_contact, Capsule, Contact};
use melee_types::{CommonMotionState as S, GroundOrAir};

#[derive(Default)]
pub struct CombatState {
    /// Fighter.dmg.armor1 (+18B4), reset on motion change.
    pub armor: f32,
    pub capture_geometry: super::grab_throw::CaptureGeometry,
    pub thrown_pose: Option<super::grab_throw::ThrownPose>,
    pub grab: Option<super::grab::GrabLink>,
    pub hitlag_remaining: f32,
    pub pending: Option<ReceivedHit>,
    pub dealt_damage: i32,
    pub shield_pushback: Option<(f32, f32)>,
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
    pub facing_override: Option<f32>,
}
pub struct DamageParameters {
    pub weight_scale: f32,
    pub throw_weight: f32,
    pub down_wait_frames: f32,
    pub tech_window: f32,
    pub tech_lockout: i32,
    pub tech_roll_threshold: f32,
    pub ground_knockback_limit: f32,
    pub trail_threshold: f32,
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
    pub sakurai_air_angle: f32,
    pub sakurai_ground_angle: f32,
    pub sakurai_maximum_threshold: f32,
    pub air_decay: f32,
    pub landing_threshold: f32,
    pub tumble_landing_threshold: f32,
    pub ledge_height_scale: f32,
    pub hitlag_scale: f32,
    pub hitlag_base: f32,
    pub maximum_hitlag: f32,
    pub phantom_threshold: f32,
    pub large_spark_threshold: f32,
}
impl DamageParameters {
    pub fn read(a: &Archive, p: u32) -> Result<Self> {
        let r = a.reader();
        Ok(Self {
            tech_window: r.f32(p + 0x250)?,
            tech_lockout: r.u32(p + 0x1C)? as i32,
            tech_roll_threshold: r.f32(p + 0x254)?,
            down_wait_frames: r.f32(p + 0x424)?,
            ground_knockback_limit: r.f32(p + 0x164)?,
            throw_weight: r.f32(p + 0x10C)?,
            trail_threshold: r.f32(p + 0x568)?,
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
            sakurai_air_angle: r.f32(p + 0x144)?,
            sakurai_ground_angle: r.f32(p + 0x148)?,
            sakurai_maximum_threshold: r.f32(p + 0x150)?,
            air_decay: r.f32(p + 0x204)?,
            landing_threshold: r.f32(p + 0x1e4)?,
            tumble_landing_threshold: r.f32(p + 0x1e0)?,
            ledge_height_scale: r.f32(p + 0x1cc)?,
            hitlag_scale: r.f32(p + 0x198)?,
            hitlag_base: r.f32(p + 0x19c)?,
            maximum_hitlag: r.f32(p + 0x194)?,
            phantom_threshold: r.f32(p + 0x7a8)?,
            large_spark_threshold: r.f32(p + 0x3f0)?,
        })
    }
    /// ftColl_80079AB0 (80079AB0), ordinary Vs 1.0 attack/defense/stage ratios.
    pub fn knockback(&self, hit: &HitboxDescriptor, percent: f32, weight: f32) -> f32 {
        let w = weight * self.weight_scale;
        let factor = self.weight_decay - (w * self.weight_decay) / (1.0 + w);
        let (p, d) = if hit.weight_knockback != 0 {
            (self.fixed_percent, f32::from(hit.weight_knockback))
        } else {
            (
                fctiwz(percent) as f32 + hit.damage,
                fctiwz(hit.damage) as f32,
            )
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
    /// ftCo_Damage_CalcAngle (8008D7F0): the 361-degree sentinel interpolates on ground.
    fn launch_angle(&self, angle: u16, knockback: f32, ground: GroundOrAir) -> f32 {
        const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;
        if angle != 361 {
            // retail 8008D854: fmuls after integer-to-single conversion.
            return DEG_TO_RAD * f32::from(angle);
        }
        if ground == GroundOrAir::Air {
            return self.sakurai_air_angle;
        }
        if knockback < self.grounded_angle_threshold {
            return 0.0;
        }
        let fraction = (knockback - self.grounded_angle_threshold)
            / (self.sakurai_maximum_threshold - self.grounded_angle_threshold);
        // retail 8008D8AC: fmadds, then separately rounded degree conversion.
        (DEG_TO_RAD * fmadds(self.sakurai_ground_angle, fraction, 1.0))
            .min(DEG_TO_RAD * self.sakurai_ground_angle)
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
    if victim.core.status.disabled
        || attacker.core.status.disabled
        || attacker.core.commands.thrown_by == Some(victim.core.spawn_number)
    {
        return;
    }
    for id in 0..attacker.core.commands.hitboxes.len() {
        let Some(hit) = &attacker.core.commands.hitboxes[id] else {
            continue;
        };
        if hit.victims.contains(&victim.core.spawn_number) {
            continue;
        }
        let desc = &hit.descriptor;
        if desc.element == melee_types::HitElement::Catch {
            continue;
        }
        let grounded = victim.core.physics.ground_or_air == GroundOrAir::Ground;
        if (grounded && !desc.hit_ground) || (!grounded && !desc.hit_air) {
            continue;
        }
        victim.character.check_hurtbox_interaction();
        detect_eligible_hit(&mut victim.core, &mut attacker.core, assets, id);
    }
}
/// ftColl_80076CBC (80076CBC): health damage, strongest impact and group victims.
fn record_shield_hit(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    desc: &HitboxDescriptor,
    contact: Contact,
) {
    // ftColl_80076CBC (80076CBC): shield contact wins over hurtboxes.
    if victim.shield.powershield_window {
        unimplemented!("ftColl_80076CBC: powershield contact");
    }
    if victim.shield.impact.is_some() {
        unimplemented!("ftColl_80076CBC: simultaneous shield impact selection");
    }
    let damage = fctiwz(desc.damage).max(1);
    let facing = if victim.physics.position.x > attacker.physics.position.x {
        -1.0
    } else {
        1.0
    };
    victim.shield.damage_taken += (damage + i32::from(desc.shield_damage)).max(0);
    victim.shield.impact = Some(super::shield::ShieldImpact {
        damage,
        facing,
        element: desc.element,
    });
    attacker.combat.dealt_damage = damage;
    if attacker.physics.ground_or_air == GroundOrAir::Ground {
        attacker.combat.shield_pushback =
            Some((victim.shield.lightshield * damage as f32, -facing));
    }
    let group = desc.group;
    for hit in attacker
        .commands
        .hitboxes
        .iter_mut()
        .flatten()
        .filter(|h| h.descriptor.group == group)
    {
        hit.victims.push(victim.spawn_number);
    }
    victim
        .effects
        .push(melee_ef::request::EffectRequest::ShieldSpark {
            position: contact.position,
        });
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_Damage_Phys (8008FB04): gravity/friction during hitstun, drift after it.
    pub(super) fn damage_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Vec3,
    ) {
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            use crate::physics::grounded::{self, GroundedParameters};
            let params = GroundedParameters::from_attributes(&self.core.attributes, &assets.common);
            grounded::friction_physics(
                &mut self.core.physics,
                &params,
                self.core.collision.data.floor.normal,
                map.floor_speed_scale(&self.core.collision.data),
            );
            grounded::finish_ground_update(
                &mut self.core.physics,
                &self.core.collision.data,
                &params,
                map,
                wind,
            );
            return;
        }
        let MotionData::Damage(damage) = &self.core.state_data else {
            panic!("damage scratch missing")
        };
        if damage.hitstun > 0.0 {
            crate::physics::airborne::fall_physics(
                &mut self.core.physics,
                &self.core.attributes.air,
                0.0,
            );
        } else {
            self.airborne_physics(assets);
        }
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.core.physics);
        crate::physics::integrate::integrate_environment(&mut self.core.physics, None, wind);
    }
    /// ftCo_Damage_Coll (8008FB64), ft_80081DD4 (80081DD4).
    pub(super) fn damage_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            let result = crate::collision::ground::map_ground_action(
                &mut self.core.physics,
                &mut self.core.collision,
                map,
                &mut self.core.skeleton,
                self.core.animation.root,
                self.core.input.current.stick.x,
            );
            if result == crate::collision::ground::WaitGroundResult::EnterFall {
                self.leave_ground();
            }
            return Ok(());
        }
        crate::collision::air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        let ledge_height = self.core.collision.data.ledge_snap_height;
        self.core.collision.data.ledge_snap_height *= assets.damage.ledge_height_scale;
        let landed = crate::collision::air::collide_pass(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.status.ledge_cooldown == 0,
        );
        self.core.collision.data.ledge_snap_height = ledge_height;
        if landed {
            if matches!(self.core.motion_state.id, S::DamageFlyN | S::DamageFall) {
                if self.try_tech(assets)? {
                    return Ok(());
                }
                return self.enter_down_bound(assets);
            }
            let v = self.core.physics.knockback_velocity;
            // retail 8008FBD4..E0: two fmuls then fadds, no contraction.
            let magnitude = sqrtf(v.x * v.x + v.y * v.y);
            if magnitude >= assets.damage.tumble_landing_threshold {
                unimplemented!("ftCo_Damage_Coll: high-speed landing -> DownBound");
            } else if magnitude >= assets.damage.landing_threshold {
                self.enter_landing(assets)?;
            } else {
                self.land();
            }
        }
        Ok(())
    }

    pub(super) fn process_damage(&mut self, assets: &FighterAssets) -> Result<()> {
        let mut hit_damage = std::mem::take(&mut self.core.combat.dealt_damage);
        if let Some((damage, direction)) = self.core.combat.shield_pushback.take() {
            if damage != 0.0 {
                // Fighter_ProcessHit, retail 8006D8D8: fmadds.
                let push = fmadds(
                    damage,
                    assets.shield.attacker_pushback_multiplier,
                    assets.shield.attacker_pushback_base,
                );
                self.core.physics.ground_shield_knockback_velocity = if direction < 0.0 {
                    push
                } else {
                    gekko_math::fma::negate_rounded(push)
                };
                let normal = self.core.collision.data.floor.normal;
                let speed = self.core.physics.ground_shield_knockback_velocity;
                // ftCommon_8007E2A4 (8007E2A4): separate tangent products.
                self.core.physics.shield_knockback_velocity =
                    Vec3::new(normal.y * speed, -normal.x * speed, 0.0);
            }
        }
        if let Some(hit) = self.core.combat.pending.take() {
            hit_damage = self.begin_damage_reaction(hit, assets)?;
        }
        if hit_damage != 0 {
            self.core.combat.hitlag_remaining = assets.damage.hitlag(hit_damage);
            if self.core.combat.hitlag_remaining > 0.0 {
                self.core.status.interaction = Interaction::Hitlag;
            }
        }
        Ok(())
    }
    /// ftCo_8008DCE0 (8008DCE0): launch and enter the strength/height reaction.
    pub(super) fn begin_damage_reaction(
        &mut self,
        hit: ReceivedHit,
        assets: &FighterAssets,
    ) -> Result<i32> {
        let (state, stun) = self.core.prepare_damage_reaction(&hit, assets);
        self.change_motion_state(state, assets)?;
        self.step_animation(assets);
        self.core.finish_damage_reaction(hit, stun)
    }
    /// ftCo_Damage_Anim (8008F7F0) -> ftCo_8008F744 (8008F744).
    pub(super) fn damage_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Damage(damage) = &mut self.core.state_data else {
            panic!("damage scratch missing")
        };
        if damage.hitstun > 0.0 {
            damage.hitstun -= 1.0;
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) && damage.hitstun <= 0.0 {
            self.change_motion_state(
                if self.core.physics.ground_or_air == GroundOrAir::Air {
                    if self.core.motion_state.id == S::DamageFlyN {
                        S::DamageFall
                    } else {
                        S::Fall
                    }
                } else {
                    S::Wait
                },
                assets,
            )?;
        }
        Ok(())
    }
    pub(super) fn damage_input(
        &mut self,
        assets: &FighterAssets,
        context: &crate::input::WaitContext,
    ) -> Result<()> {
        let MotionData::Damage(damage) = &self.core.state_data else {
            panic!("damage scratch missing")
        };
        if damage.hitstun <= 0.0 {
            if self.core.physics.ground_or_air == GroundOrAir::Air {
                use crate::input::WaitTransition as T;
                // A damage state is neither Jump nor JumpAerial, so the
                // float check (Peach) is always enabled here, as in procs.rs.
                let vertical_velocity = self.core.physics.self_velocity.y;
                let transition = super::fall::iasa(
                    &self.core.input,
                    &assets.input,
                    self.core.physics.jumps_used,
                    self.core.attributes.jumping.max_jumps,
                    |phase| {
                        self.character.check_float_input(
                            &self.core.input,
                            assets,
                            vertical_velocity,
                            phase,
                        );
                    },
                );
                return match transition {
                    T::None => Ok(()),
                    T::Jump => self.enter_aerial_jump(assets),
                    T::Escape => self.enter_air_dodge(assets),
                    transition => unimplemented!("ftCo_Damage_IASA: airborne {transition:?}"),
                };
            }
            let transition = crate::input::wait_iasa(&self.core.input, &assets.input, context);
            self.apply_ground_transition(assets, transition)?;
        } else if self
            .core
            .input
            .pressed
            .intersects(crate::input::Buttons::XY)
            || (self.core.input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
                && i32::from(self.core.input.vertical.tilt)
                    < assets.input.thresholds.tap_jump_window)
        {
            unimplemented!("ftCo_Damage.c:1015-1046: hitstun jump buffer");
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftColl_80078C70: first colliding hurt capsule wins, in ftData order.
    pub(super) fn contact_with_hurtboxes(
        &mut self,
        hit: &HitCapsule,
        attacker_scale: f32,
    ) -> Option<(Contact, HurtHeight)> {
        let desc = &hit.descriptor;
        for hurt in &mut self.hurtboxes {
            if desc.element == melee_types::HitElement::Catch && !hurt.grabbable {
                continue;
            }
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

    /// Fighter_procUpdate (8006B82C): decay follows the state's air physics.
    pub(super) fn decay_air_knockback(&mut self, assets: &FighterAssets) {
        let v = &mut self.physics.knockback_velocity;
        if v.x == 0.0 && v.y == 0.0 {
            return;
        }
        let angle = melee_lb::trigf::atan2f(v.y, v.x);
        // retail 8006B938: fmadds; sqrt is the audited MSL refinement.
        if sqrtf(fmadds(v.x, v.x, v.y * v.y)) < assets.damage.air_decay {
            v.x = 0.0;
            v.y = 0.0;
        } else {
            // retail 8006B9C8 / 8006B9E4: fnmsubs.
            v.x = gekko_math::fma::fnmsubs(assets.damage.air_decay, cosf(angle), v.x);
            v.y = gekko_math::fma::fnmsubs(assets.damage.air_decay, sinf(angle), v.y);
        }
        self.physics.ground_knockback_velocity = 0.0;
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
                } else if matches!(self.state_data, MotionData::Guard(_)) {
                    self.shield.allow_sdi = false;
                    self.status.interaction = Interaction::Shield;
                } else {
                    self.status.interaction = Interaction::Attack;
                }
            }
        }
    }
}

/// ftColl_80078C70 / ftColl_80079AB0: capsule work after the character's
/// hurtbox-interaction hook and before advancing to the next hitbox ID.
fn detect_eligible_hit(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
    id: usize,
) {
    let hit = attacker.commands.hitboxes[id]
        .as_ref()
        .expect("eligible hitbox");
    let desc = &hit.descriptor;
    if victim.combat.armor != 0.0 {
        unimplemented!("ftColl_80079AB0: double-jump armor damage response");
    }
    if victim.commands.hurt_status == super::escape::HurtStatus::Intangible
        || victim.status.ledge_intangibility != 0
    {
        return;
    }
    if victim.shield.active {
        if let Some(contact) = victim.shield_contact(hit, attacker.player.scale) {
            let descriptor = desc.clone();
            record_shield_hit(victim, attacker, &descriptor, contact);
            return;
        }
    }
    let contact = victim.contact_with_hurtboxes(hit, attacker.player.scale);
    if let Some((contact, height)) = contact {
        if victim.motion_state.id != S::Wait {
            unimplemented!("ftColl_80079AB0: crouch/other damage modifiers outside idle victim");
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
        if descriptor.element == melee_types::HitElement::Normal
            && (knockback >= assets.damage.large_spark_threshold || descriptor.sound_severity >= 1)
        {
            unimplemented!("ftColl_80078538: large normal spark / severity-specific extra spark");
        }
        victim.combat.pending = Some(ReceivedHit {
            descriptor: descriptor.clone(),
            height,
            facing: if victim.physics.position.x > attacker.physics.position.x {
                -1.0
            } else {
                1.0
            },
            knockback,
            facing_override: None,
        });
        attacker.combat.has_recorded_hit = true;
        // ftColl_8007891C -> plStale_UpdateStaleMovesFromFighter: the first
        // queue entry is visible to later hitbox commands of this same move.
        attacker.commands.first_hit_stale_penalty = Some(assets.first_stale_penalty);
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
            .push(melee_ef::request::EffectRequest::HitSpark {
                position: contact.position,
                element: descriptor.element,
                damage: descriptor.damage,
            });
    }
}

impl FighterCore {
    /// ftCo_8008DCE0 (8008DCE0): common launch calculation before motion entry.
    fn prepare_damage_reaction(&mut self, hit: &ReceivedHit, assets: &FighterAssets) -> (S, f32) {
        if self.physics.ground_or_air != GroundOrAir::Ground && self.motion_state.id != S::ThrownB {
            unimplemented!("ftCo_Damage.c:346: airborne hit");
        }
        let stun = hit.knockback * assets.damage.hitstun_scale;
        let level = assets
            .damage
            .reaction_thresholds
            .iter()
            .position(|&t| stun < t)
            .unwrap_or(3);
        // ftCo_803C5520: reaction depends on hit strength and hurtbox height.
        let state = match (level, hit.height) {
            (0, HurtHeight::Middle) => S::DamageN1,
            (1, HurtHeight::Middle) => S::DamageN2,
            (2, HurtHeight::High) => S::DamageHi3,
            (3, HurtHeight::Middle) => S::DamageFlyN,
            _ => unimplemented!(
                "ftCo_Damage.c:63: damage level {level}, height {:?}",
                hit.height
            ),
        };
        self.physics.percent += hit.descriptor.damage;
        let angle = assets.damage.launch_angle(
            hit.descriptor.angle,
            hit.knockback,
            self.physics.ground_or_air,
        );
        let speed = hit.knockback * assets.damage.velocity_scale;
        self.physics.facing = hit.facing;
        // ftCo_8008DCE0, 8008DFCC..E0F4: separate products.
        let horizontal = -(speed * cosf(angle)) * hit.facing;
        let vertical = speed * sinf(angle);
        let normal = self.collision.data.floor.normal;
        if normal.x != 0.0 || normal.y <= 0.0 {
            unimplemented!("ftCo_8008DCE0: sloped-floor launch angle");
        }
        if vertical > 0.0 || self.physics.ground_or_air == GroundOrAir::Air {
            if self.physics.ground_or_air == GroundOrAir::Ground {
                self.leave_ground();
            }
            self.physics.knockback_velocity = Vec3::new(horizontal, vertical, 0.0);
            self.physics.ground_knockback_velocity = 0.0;
        } else {
            self.physics.ground_knockback_velocity = horizontal;
            self.physics.knockback_velocity =
                Vec3::new(normal.y * horizontal, -normal.x * horizontal, 0.0);
        }
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        if let Some(facing) = hit.facing_override {
            self.physics.facing = facing;
        }
        if state == S::DamageFlyN && speed >= assets.damage.trail_threshold {
            unimplemented!("ftCo_Damage_SetMv8FromKbThreshold: damage fly smoke");
        }
        (state, stun)
    }
    /// ftCo_8008DCE0: hitstun, timers and input ages after frame-zero playback.
    fn finish_damage_reaction(&mut self, hit: ReceivedHit, stun: f32) -> Result<i32> {
        self.state_data = MotionData::Damage(DamageState {
            hitstun: (fctiwz(stun).max(1)) as f32,
        });
        self.status.time_since_hit = 0;
        self.status.interaction = Interaction::Damage;
        self.input.horizontal.tilt = 254;
        self.input.vertical.tilt = 254;
        Ok(fctiwz(hit.descriptor.damage).max(1))
    }
}
