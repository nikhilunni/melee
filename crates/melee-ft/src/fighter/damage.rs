//! Fighter-pair hit/shield detection and launch reactions, ftcoll.c / ftCo_Damage.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, Interaction, MotionData,
};
use gekko_math::{
    fma::fmadds,
    msl::{cosf, fctiwz, sinf, sqrtf},
};
use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_coll::{geometry::Contact, hitbox::HitCapsule, hurtbox::HurtHeight};
use melee_types::combat::HitboxDescriptor;
use melee_types::{CommonMotionState as S, GroundOrAir};

#[derive(Default)]
pub struct CombatState {
    /// Fighter.dmg.armor1 (+18B4), reset on motion change.
    pub armor: f32,
    pub charge_overlay: super::smash::ChargeOverlay,
    pub damage_overlay: Option<(u8, super::smash::ChargeOverlay)>,
    pub capture_geometry: super::grab_throw::CaptureGeometry,
    pub thrown_pose: Option<super::grab_throw::ThrownPose>,
    pub grab: Option<super::grab::GrabLink>,
    pub hitlag_remaining: f32,
    pub pending: Option<ReceivedHit>,
    /// ftColl_8007A06C: the selected hit came from this captured fighter's captor.
    pub pending_from_captor: bool,
    /// Fighter.dmg.x1908 / x190C: the hit sound and voice set queued by the
    /// launch calculation, played by the next hit proc that starts no hitlag
    /// (Fighter_ProcessHit's else branch -> ftCo_80090718).
    pub queued_hit_sfx: Option<u32>,
    pub queued_voice: Option<DamageVoice>,
    pub dealt_damage: i32,
    /// Fighter +1964: special shield minimum hitlag, consumed by ProcessHit.
    pub minimum_hitlag: f32,
    pub shield_pushback: Option<(f32, f32)>,
    /// Legacy throw-entry guard; normal attacks use the fixed stale history.
    pub has_recorded_hit: bool,
    pub stale: super::attack::stale::StaleHistory,
    pub combo: super::attack::combo::ComboState,
}
/// Which fighter voice table a queued damage voice draws from (ft_data->x4C_sfx +1C / +20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageVoice {
    Medium,
    Heavy,
}
/// ftCo_Damage.c:495/502: the hit sounds paired with the heavy / medium voices.
const HEAVY_HIT_SFX: u32 = 0x4F;
const MEDIUM_HIT_SFX: u32 = 0x50;

#[derive(Clone, Debug)]
pub struct DamageState {
    pub hitstun: f32,
    /// Remaining hitstun when the last jump input arrived (mv.damage.x14).
    pub jump_buffer: f32,
    pub trail_timer: u32,
    pub influence: InfluenceParameters,
}
/// PlCo values retained by the damage state's status callback, which runs
/// before input sampling and has no archive resource argument.
#[derive(Clone, Copy, Debug)]
pub struct InfluenceParameters {
    pub minimum_stick: f32,
    pub tap_window: i32,
    pub sdi_distance: f32,
    pub asdi_distance: f32,
    pub maximum_angle_degrees: f32,
    pub shield_velocity_scale: f32,
}
use melee_coll::damage::ReceivedHit;
pub struct DamageParameters {
    pub influence: InfluenceParameters,
    pub jump_buffer_window: f32,
    pub knockback_replace_window: i32,
    pub air_cancel_window: i32,
    pub air_cancel_scale: f32,
    pub crouch_knockback_scale: f32,
    pub crouch_hitlag_scale: f32,
    pub electric_hitlag_scale: f32,
    pub floor_bounce_angle: f32,
    pub floor_bounce_scale: f32,
    pub down_stand_threshold: f32,
    pub down_roll_threshold: f32,
    pub down_attack_buffer: f32,
    pub down_cstick_attack_threshold: f32,
    pub tumble_exit_threshold: f32,
    pub tumble_exit_window: i32,
    pub weight_scale: f32,
    pub throw_weight: f32,
    pub down_wait_frames: f32,
    pub tech_window: f32,
    pub tech_lockout: i32,
    pub tech_roll_threshold: f32,
    pub ground_knockback_limit: f32,
    pub trail_threshold: f32,
    pub trail_thresholds: [f32; 4],
    pub trail_intervals: [u32; 4],
    pub top_angle_range: [f32; 2],
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
    /// PlCo +208 / +20C: scaled knockback at which the medium / heavy hit
    /// sound and voice are queued (ftCo_Damage.c block_70).
    pub medium_voice_threshold: f32,
    pub heavy_voice_threshold: f32,
    /// PlCo +23C (int) / +240: percent floor and Randf chance for DamageFlyRoll.
    pub fly_roll_percent: i32,
    pub fly_roll_chance: f32,
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
    pub extra_spark_bounds: [i32; 2],
}
impl DamageParameters {
    pub fn read(a: &Archive, p: u32) -> Result<Self> {
        let r = a.reader();
        Ok(Self {
            influence: InfluenceParameters {
                minimum_stick: r.f32(p + 0x4B0)?,
                tap_window: r.s32(p + 0x4B4)?,
                sdi_distance: r.f32(p + 0x4B8)?,
                asdi_distance: r.f32(p + 0x4BC)?,
                maximum_angle_degrees: r.f32(p + 0x1A8)?,
                shield_velocity_scale: r.f32(p + 0x1AC)?,
            },
            jump_buffer_window: r.f32(p + 0x1D0)?,
            knockback_replace_window: r.s32(p + 0xFC)?,
            air_cancel_window: r.s32(p + 0x18C)?,
            air_cancel_scale: r.f32(p + 0x190)?,
            crouch_knockback_scale: r.f32(p + 0x124)?,
            crouch_hitlag_scale: r.f32(p + 0x1A0)?,
            electric_hitlag_scale: r.f32(p + 0x1A4)?,
            floor_bounce_angle: r.f32(p + 0x1E8)?,
            floor_bounce_scale: r.f32(p + 0x1EC)?,
            down_stand_threshold: r.f32(p + 0x244)?,
            down_roll_threshold: r.f32(p + 0x248)?,
            down_attack_buffer: r.f32(p + 0x24C)?,
            down_cstick_attack_threshold: r.f32(p + 0x7F4)?,
            tumble_exit_threshold: r.f32(p + 0x210)?,
            tumble_exit_window: r.s32(p + 0x214)?,
            tech_window: r.f32(p + 0x250)?,
            tech_lockout: r.u32(p + 0x1C)? as i32,
            tech_roll_threshold: r.f32(p + 0x254)?,
            down_wait_frames: r.f32(p + 0x424)?,
            ground_knockback_limit: r.f32(p + 0x164)?,
            throw_weight: r.f32(p + 0x10C)?,
            trail_threshold: r.f32(p + 0x568)?,
            trail_thresholds: [
                r.f32(p + 0x570)?,
                r.f32(p + 0x574)?,
                r.f32(p + 0x578)?,
                f32::INFINITY,
            ],
            trail_intervals: [
                r.u32(p + 0x57C)?,
                r.u32(p + 0x580)?,
                r.u32(p + 0x584)?,
                r.u32(p + 0x588)?,
            ],
            top_angle_range: [r.f32(p + 0x234)?, r.f32(p + 0x238)?],
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
            medium_voice_threshold: r.f32(p + 0x208)?,
            heavy_voice_threshold: r.f32(p + 0x20C)?,
            fly_roll_percent: r.s32(p + 0x23C)?,
            fly_roll_chance: r.f32(p + 0x240)?,
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
            extra_spark_bounds: [r.s32(p + 0x3f4)?, r.s32(p + 0x3f8)?],
        })
    }
    pub fn knockback(&self, hit: &HitboxDescriptor, percent: f32, weight: f32) -> f32 {
        self.knockback_with_damage(hit, percent, weight, fctiwz(hit.damage) as u32)
    }
    pub(super) fn knockback_with_damage(
        &self,
        hit: &HitboxDescriptor,
        percent: f32,
        weight: f32,
        damage: u32,
    ) -> f32 {
        melee_coll::damage::KnockbackParameters {
            weight_scale: self.weight_scale,
            weight_decay: self.weight_decay,
            fixed_percent: self.fixed_percent,
            percent_scale: self.percent_scale,
            damage_scale: self.damage_scale,
            growth_scale: self.growth_scale,
            base: self.base,
            maximum: self.maximum,
        }
        .knockback_with_damage(hit, percent, weight, damage)
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
        melee_coll::damage::hitlag(
            damage,
            self.hitlag_scale,
            self.hitlag_base,
            self.maximum_hitlag,
        )
    }
}
/// ftColl_80078C70 (80078C70): receiver first, other fighters then hitbox IDs,
/// then hurt-table order. The scene supplies that entity-list ordering.
pub fn detect_hit(victim: &mut Fighter, attacker: &mut Fighter, assets: &FighterAssets) {
    if victim.core.status.disabled
        || attacker.core.status.disabled
        || attacker.core.commands.thrown_by == Some(victim.core.spawn_number)
    {
        return;
    }
    let mut cursor = melee_coll::detection::PairCursor::default();
    while let Some(id) = cursor.next(
        &attacker.core.commands.hitboxes,
        victim.core.spawn_number,
        victim.core.physics.ground_or_air,
    ) {
        if let Some(contact) = victim.character.table().defense_contact {
            if contact(victim, attacker, assets, id) {
                continue;
            }
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
    assets: &FighterAssets,
) {
    // ftColl_80076CBC (80076CBC): shield contact wins over hurtboxes.
    if victim.shield.impact.is_some() {
        unimplemented!("ftColl_80076CBC: simultaneous shield impact selection");
    }
    let damage = fctiwz(desc.damage).max(1);
    let facing = if victim.physics.position.x > attacker.physics.position.x {
        -1.0
    } else {
        1.0
    };
    if !victim.shield.powershield_window {
        victim.shield.damage_taken += (damage + i32::from(desc.shield_damage)).max(0);
    }
    victim.shield.impact = Some(super::shield::ShieldImpact {
        damage,
        facing,
        element: desc.element,
    });
    attacker.record_shield_recoil(damage, victim.shield.lightshield, -facing);
    let group = desc.group;
    melee_coll::detection::record_victim(
        &mut attacker.commands.hitboxes,
        group,
        victim.spawn_number,
    );
    if victim.shield.powershield_window {
        // ftCo_80094138: permit attacks during GuardOff and clear minimum hold.
        victim.guard().interrupt_frames = assets.shield.powershield_interrupt_frames;
        victim.guard().minimum_hold = 0.0;
        victim.shield.flash = Some(super::smash::ChargeOverlay::default());
        victim
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 118,
                duration: 0,
            });
        victim
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: 104,
                volume: 127,
                pan: 64,
            });
        victim
            .effects
            .push(melee_ef::request::EffectRequest::PowershieldSpark {
                position: contact.position,
            });
    } else {
        victim
            .effects
            .push(melee_ef::request::EffectRequest::ShieldSpark {
                position: contact.position,
            });
    }
}

impl Fighter {
    /// ftCo_Damage_Phys (8008FB18): gravity/friction during hitstun, drift after it.
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
                // ft_800848DC: slipping off the back edge enters MissFoot.
                let slip = if self.physics.facing < 0.0 {
                    melee_types::mp::collide::RIGHT_LEDGE_SLIP
                } else {
                    melee_types::mp::collide::LEFT_LEDGE_SLIP
                };
                if self.collision.data.env_flags as u32 & slip != 0 {
                    self.enter_missed_footing(assets)?;
                } else {
                    self.leave_ground();
                }
            }
            return Ok(());
        }
        crate::collision::air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        if self.core.combat.hitlag_remaining > 0.0 {
            // ft_80081DD4: mpColl_800477E0 clamps SDI against the floor
            // without a landing transition while hitlag is active.
            let cd = &mut self.core.collision.data;
            cd.last_pos = cd.cur_pos;
            cd.cur_pos = self.core.physics.position;
            let pose = crate::collision::ecb::EcbPose::read(
                &mut self.core.skeleton,
                self.core.animation.root,
                cd,
            );
            map.air_collide_stay(cd, Some(&|bone| pose.position(bone)));
            self.core.physics.position = cd.cur_pos;
            self.core
                .skeleton
                .set_translate(self.core.animation.root, &self.core.physics.position);
            return Ok(());
        }
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
            if is_tumble(self.core.motion_state.id) || self.core.motion_state.id == S::DamageFall {
                if self.try_tech(assets)? {
                    return Ok(());
                }
                return self.enter_down_bound(assets);
            }
            let v = self.core.physics.knockback_velocity;
            // retail 8008FBD4..E0: two fmuls then fadds, no contraction.
            let magnitude = sqrtf(v.x * v.x + v.y * v.y);
            if magnitude >= assets.damage.tumble_landing_threshold {
                return self.enter_down_bound(assets);
            } else if magnitude >= assets.damage.landing_threshold {
                self.enter_landing(assets)?;
            } else {
                self.land();
            }
        }
        if !landed && self.core.motion_state.id == S::DamageFall {
            self.try_grab_ledge(assets, map)?;
        }
        Ok(())
    }

    pub(super) fn process_damage(
        &mut self,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        let crouching = matches!(self.core.motion_state.id, S::Squat | S::SquatWait);
        let mut hit_damage = std::mem::take(&mut self.core.combat.dealt_damage);
        let mut hitlag_multiplier = 1.0;
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
            // ftColl_8007A06C: only the electric-hit victim gets x1960 = PlCo +1A4.
            if hit.descriptor.element == melee_types::HitElement::Electric {
                hitlag_multiplier = assets.damage.electric_hitlag_scale;
            }
            if std::mem::take(&mut self.core.combat.pending_from_captor) {
                // ftCo_8008EC90: the captor's hit preserves the grab and shares hitlag.
                hit_damage = super::grab_escape::capture_damage(self, &hit, assets)?;
            } else if hit.knockback == 0.0 {
                // Fighter_ProcessHit (fighter.c:2958) / Fighter_UnkTakeDamage_8006CC30: zero-knockback damage
                // updates percent without a damage-state transition or hitlag.
                self.core.physics.percent += hit.descriptor.damage;
            } else {
                if let Some(take_damage) = self.character.table().take_damage {
                    take_damage(self);
                }
                hit_damage = self.begin_damage_reaction(hit, None, assets, rng)?;
            }
        }
        if hit_damage == 0 {
            // Fighter_ProcessHit, fighter.c:2990: no hitlag started this frame, so
            // ftCo_80090718 plays whatever the previous hit queued.
            self.core.play_queued_damage_sounds(assets, rng);
        }
        if hit_damage != 0 {
            // ftCommon_CalcHitlag: 8007DAA8 fmadds, integer conversion, then
            // 8007DACC fmuls and another integer conversion before crouch scaling.
            let base = fctiwz(fmadds(
                hit_damage as f32,
                assets.damage.hitlag_scale,
                assets.damage.hitlag_base,
            )) as f32;
            let mut hitlag = fctiwz(base * hitlag_multiplier) as f32;
            if crouching {
                // ftCommon_CalcHitlag, 8007DAF8 fmuls then fctiwz.
                hitlag = fctiwz(hitlag * assets.damage.crouch_hitlag_scale) as f32;
            }
            self.core.combat.hitlag_remaining = hitlag
                .max(self.core.combat.minimum_hitlag)
                .min(assets.damage.maximum_hitlag);
            if self.core.combat.hitlag_remaining > 0.0 {
                self.core.status.interaction = Interaction::Hitlag;
            }
        }
        self.core.combat.minimum_hitlag = 0.0;
        Ok(())
    }
    /// ftCo_8008DCE0 (8008DCE0): launch and enter the strength/height reaction.
    pub(super) fn begin_damage_reaction(
        &mut self,
        mut hit: ReceivedHit,
        forced_motion: Option<S>,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<i32> {
        if matches!(self.core.motion_state.id, S::Squat | S::SquatWait) {
            // ftCo_Damage_CalcKnockback, 8008D974: separate fmuls.
            hit.knockback *= assets.damage.crouch_knockback_scale;
        }
        let (state, stun) = self
            .core
            .prepare_damage_reaction(&hit, forced_motion, assets, rng);
        if hit.descriptor.element == melee_types::HitElement::Electric {
            // ftCo_8008DA4C -> ftCo_800BFFD0: the damage level selects color 15..18.
            let level = if forced_motion.is_some() {
                3
            } else {
                assets
                    .damage
                    .reaction_thresholds
                    .iter()
                    .position(|&t| stun < t)
                    .unwrap_or(3)
            };
            self.combat.damage_overlay = Some((15 + level as u8, Default::default()));
        }
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        let result = self.core.finish_damage_reaction(hit, stun, assets)?;
        if let MotionData::Damage(damage) = &mut self.core.state_data {
            let velocity = self.core.physics.knockback_velocity;
            // ftCo_Damage_SetMv8FromKbThreshold: separate products and sums.
            let speed = if self.core.physics.ground_or_air == GroundOrAir::Air {
                sqrtf(velocity.x * velocity.x + velocity.y * velocity.y + velocity.z * velocity.z)
            } else {
                gekko_math::msl::fabsf(self.core.physics.ground_knockback_velocity)
            };
            damage.trail_timer = u32::from(speed >= assets.damage.trail_threshold);
        }
        Ok(result)
    }
    /// ftCo_Damage_Anim (8008F7F0) -> ftCo_8008F744 (8008F744).
    pub(super) fn damage_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Damage(damage) = &mut self.core.state_data else {
            panic!("damage scratch missing")
        };
        if is_tumble(self.core.motion_state.id) && damage.trail_timer != 0 {
            damage.trail_timer -= 1;
            if damage.trail_timer == 0 {
                let v = self.core.physics.knockback_velocity;
                let trajectory = melee_lb::trigf::atan2f(-v.x, v.y);
                self.core
                    .effects
                    .push(melee_ef::request::EffectRequest::DamageTrail { trajectory });
                // ftCo_Damage_SetMv8FromKbThreshold: separate products/adds, no fused sites.
                let speed = sqrtf(v.x * v.x + v.y * v.y + v.z * v.z);
                if speed >= assets.damage.trail_threshold {
                    let index = assets
                        .damage
                        .trail_thresholds
                        .iter()
                        .position(|&threshold| speed < threshold)
                        .unwrap_or(3);
                    damage.trail_timer = assets.damage.trail_intervals[index];
                }
            }
        }
        if damage.hitstun > 0.0 {
            damage.hitstun -= 1.0;
            if damage.hitstun <= 0.0 {
                self.core.combat.combo.grace = assets.combo.grace_frames;
            }
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) && damage.hitstun <= 0.0 {
            // ftCo_Damage_Anim / DamageFly_Anim inlineC0: consume a stored
            // jump only at animation end. DamageFlyRoll has no buffer branch.
            if self.core.physics.ground_or_air == GroundOrAir::Air
                && self.core.motion_state.id != S::DamageFlyRoll
                && damage.jump_buffer != 0.0
                && damage.jump_buffer <= assets.damage.jump_buffer_window
            {
                self.core.input.pressed |= crate::input::Buttons::XY;
                if i32::from(self.core.physics.jumps_used) < self.core.attributes.jumping.max_jumps
                {
                    return self.enter_aerial_jump(assets);
                }
            }
            self.change_motion_state(
                (if self.core.physics.ground_or_air == GroundOrAir::Air {
                    if is_tumble(self.core.motion_state.id) {
                        S::DamageFall
                    } else {
                        S::Fall
                    }
                } else {
                    S::Wait
                })
                .into(),
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
        let MotionData::Damage(damage) = &mut self.core.state_data else {
            panic!("damage scratch missing")
        };
        if damage.hitstun <= 0.0 {
            // ftCo_Damage_IASA (8008FA44): synthesize XY for a jump pressed
            // within PlCo's final hitstun window. The stored value does not age.
            if !is_tumble(self.core.motion_state.id)
                && damage.jump_buffer != 0.0
                && damage.jump_buffer <= assets.damage.jump_buffer_window
            {
                self.core.input.pressed |= crate::input::Buttons::XY;
            }
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
                    T::None => {
                        if (self.core.motion_state.id == S::DamageFall
                            || is_tumble(self.core.motion_state.id))
                            && gekko_math::msl::fabsf(self.core.input.current.stick.x)
                                >= assets.damage.tumble_exit_threshold
                            && i32::from(self.core.input.horizontal.tilt)
                                < assets.damage.tumble_exit_window
                        {
                            self.change_motion_state(S::Fall.into(), assets)?;
                        }
                        Ok(())
                    }
                    T::Jump => self.enter_aerial_jump(assets),
                    T::Escape => self.enter_air_dodge(assets),
                    T::Special => {
                        self.enter_buffered_special(assets, true);
                        Ok(())
                    }
                    transition => unimplemented!("ftCo_Damage_IASA: airborne {transition:?}"),
                };
            }
            let transition = crate::input::wait_iasa(&self.core.input, &assets.input, context);
            self.apply_ground_transition(assets, transition)?;
        } else if crate::input::human::jump_input(&self.core.input, &assets.input) {
            // doIasa (8008F938): record the remaining hitstun, not the input age.
            damage.jump_buffer = damage.hitstun;
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftColl_80076CBC (80076CBC): ordinary shields and Counter both record
    /// attacker recoil from the defender's retained lightshield amount.
    pub fn record_shield_recoil(&mut self, damage: i32, lightshield: f32, direction: f32) {
        if damage > self.combat.dealt_damage {
            self.combat.dealt_damage = damage;
            if self.physics.ground_or_air == GroundOrAir::Ground {
                // Retail 80076CBC: separately rounded lightshield * integer damage.
                self.combat.shield_pushback = Some((lightshield * damage as f32, direction));
            }
        }
    }
    /// ftCo_Damage_OnEveryHitlag (8008E4F0), ftCo_Damage.c:569-589.
    pub(super) fn damage_hitlag_input(&mut self) {
        let MotionData::Damage(damage) = &self.state_data else {
            return;
        };
        let parameters = damage.influence;
        let stick = self.input.current.stick;
        if stick_magnitude_passes(stick, parameters.minimum_stick)
            && (i32::from(self.input.horizontal.tilt) < parameters.tap_window
                || i32::from(self.input.vertical.tilt) < parameters.tap_window)
        {
            // 8008E560..574: products and position sums round separately.
            self.physics.position.x += stick.x * parameters.sdi_distance;
            self.physics.position.y += stick.y * parameters.sdi_distance;
            self.input.horizontal.tilt = 254;
            self.input.vertical.tilt = 254;
        }
    }

    /// ftCo_Damage_OnExitHitlag (8008E714), ftCo_Damage.c:625-665.
    fn exit_damage_hitlag(&mut self) {
        let MotionData::Damage(damage) = &self.state_data else {
            return;
        };
        let parameters = damage.influence;
        let input = &self.input.current;
        // ftCo_800DF608: C-stick takes priority, with the same PlCo radius.
        let stick = if stick_magnitude_passes(input.cstick, parameters.minimum_stick) {
            input.cstick
        } else {
            input.stick
        };
        if stick_magnitude_passes(stick, parameters.minimum_stick) {
            // 8008E7A8..7DC: separately rounded products and sums.
            self.physics.position.x += stick.x * parameters.asdi_distance;
            self.physics.position.y += stick.y * parameters.asdi_distance;
        }
        apply_directional_influence(
            &mut self.physics.knockback_velocity,
            input.stick,
            parameters.maximum_angle_degrees,
        );
        if input.held.intersects(crate::input::Buttons::SHIELD) {
            let velocity = &mut self.physics.knockback_velocity;
            if velocity.x != 0.0 || velocity.y != 0.0 {
                let angle = melee_lb::trigf::atan2f(velocity.y, velocity.x);
                // 8008E860 fmadds; 8008E8BC fmuls after sqrt refinement.
                let speed = sqrtf(fmadds(velocity.x, velocity.x, velocity.y * velocity.y))
                    * parameters.shield_velocity_scale;
                velocity.x = speed * cosf(angle);
                velocity.y = speed * sinf(angle);
            }
        }
    }

    /// ftColl_80078C70 item pass -> ftColl_8007A06C: receiver and capsule order.
    pub fn detect_item_hit(
        &mut self,
        item: &mut melee_it::ItemCore,
        assets: &FighterAssets,
        stale_multiplier: f32,
    ) -> Option<f32> {
        if item.owner == Some(self.player.id)
            || item.destroyed
            || self.status.disabled
            || self.commands.hurt_status == melee_types::combat::HurtStatus::Intangible
            || self.status.ledge_intangibility != 0
        {
            return None;
        }
        let mut cursor = melee_coll::detection::PairCursor::default();
        while let Some(id) = cursor.next(
            &item.hitboxes,
            self.spawn_number,
            self.physics.ground_or_air,
        ) {
            if !item.hit_flags[id].hits_hurtboxes {
                continue;
            }
            let hit = item.hitboxes[id].as_ref().unwrap();
            if self.shield.active {
                unimplemented!("item shield response");
            }
            let Some((contact, height)) =
                melee_coll::detection::first_contact(self, hit, item.scale)
            else {
                continue;
            };
            if contact.overlap < assets.damage.phantom_threshold {
                unimplemented!("item phantom hit");
            }
            let mut descriptor = hit.descriptor.clone();
            let knockback_damage = fctiwz(descriptor.damage) as u32;
            // ft_80089258 / ft_80089118: item damage uses the owner's current
            // stale table and the move captured at spawn; knockback keeps base damage.
            descriptor.damage *= stale_multiplier;
            let knockback = assets.damage.knockback_with_damage(
                &descriptor,
                self.physics.percent,
                self.attributes.size.weight,
                knockback_damage,
            );
            self.combat.pending = Some(ReceivedHit {
                descriptor: descriptor.clone(),
                height,
                knockback,
                facing: if self.physics.position.x > item.position.x {
                    -1.0
                } else {
                    1.0
                },
                facing_override: None,
            });
            melee_coll::detection::record_victim(
                &mut item.hitboxes,
                descriptor.group,
                self.spawn_number,
            );
            // ftColl_8007A06C -> efSync_Spawn, shared with fighter hits.
            self.effects
                .push(melee_ef::request::EffectRequest::HitSpark {
                    position: contact.position,
                    element: descriptor.element,
                    damage: fctiwz(descriptor.damage) as f32,
                    large: knockback >= assets.damage.large_spark_threshold,
                });
            if descriptor.element == melee_types::HitElement::Normal
                && descriptor.sound_severity >= 1
            {
                let variant = self.attributes.combat.hit_spark_variant;
                if let Some(&random_bound) = assets.damage.extra_spark_bounds.get(variant as usize)
                {
                    self.effects
                        .push(melee_ef::request::EffectRequest::NormalSparkExtra {
                            position: contact.position,
                            facing: self.physics.facing,
                            variant,
                            random_bound,
                        });
                }
            }
            return Some(descriptor.damage);
        }
        None
    }

    /// ftColl_80078C70: first colliding hurt capsule wins, in ftData order.
    pub(super) fn contact_with_hurtboxes(
        &mut self,
        hit: &HitCapsule,
        attacker_scale: f32,
    ) -> Option<(Contact, HurtHeight)> {
        melee_coll::detection::first_contact(self, hit, attacker_scale)
    }

    /// Fighter_procUpdate (8006B82C): decay follows the state's air physics.
    pub fn decay_air_knockback(&mut self, assets: &FighterAssets) {
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
    /// ftCo_80090718: play the queued hit sound (ft_PlaySFX 127/64) and one voice
    /// drawn with HSD_Randi over the fighter's voice table (ft_800889F4).
    pub(super) fn play_queued_damage_sounds(
        &mut self,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        use super::commands::{FootstepSound, SoundChannel};
        if let Some(id) = self.combat.queued_hit_sfx.take() {
            self.commands.footstep_sounds.push(FootstepSound {
                channel: SoundChannel::Ordinary,
                id,
                volume: 127,
                pan: 64,
            });
        }
        if let Some(set) = self.combat.queued_voice.take() {
            let voices = match set {
                DamageVoice::Heavy => &assets.heavy_voices,
                DamageVoice::Medium => &assets.medium_voices,
            };
            if !voices.is_empty() {
                let id = voices[rng.randi(voices.len() as i32) as usize];
                self.commands.footstep_sounds.push(FootstepSound {
                    channel: SoundChannel::FighterVoice,
                    id,
                    volume: 127,
                    pan: 64,
                });
            }
        }
    }
    /// Fighter_8006A1BC (8006A1BC): expire before animation and input.
    pub(super) fn tick_hitlag(&mut self) {
        if melee_coll::damage::tick_hitlag(&mut self.combat.hitlag_remaining) {
            if matches!(self.state_data, MotionData::Damage(_)) {
                self.exit_damage_hitlag();
                self.status.interaction = Interaction::Damage;
            } else if matches!(self.state_data, MotionData::Guard(_)) {
                self.shield.allow_sdi = false;
                self.status.interaction = Interaction::Shield;
            } else if self.combat.grab.is_some() {
                // Pummel freezes both members of the pair without damage-state scratch.
                self.status.interaction = Interaction::Idle;
            } else {
                self.status.interaction = Interaction::Attack;
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
    if victim.commands.hurt_status == melee_types::combat::HurtStatus::Intangible
        || victim.status.ledge_intangibility != 0
    {
        return;
    }
    if victim.shield.active {
        if let Some(contact) = victim.shield_contact(hit, attacker.player.scale) {
            let descriptor = desc.clone();
            record_shield_hit(victim, attacker, &descriptor, contact, assets);
            return;
        }
    }
    let contact = victim.contact_with_hurtboxes(hit, attacker.player.scale);
    if let Some((contact, height)) = contact {
        // Retail's hit path reads the victim's state only for DamageIce
        // (ftcoll.c:199/576/1155); crouch cancel (ftCo_Damage.c:124-127) and the
        // airborne launch states (ftCo_Damage.c:543-558) are applied by the
        // reaction in prepare_damage_reaction. Every other grounded victim state
        // takes the ordinary path.
        if victim.motion_state.id == S::DamageIce {
            unimplemented!("ftcoll.c:199: DamageIce victim");
        }
        // ftColl_80078C70, ftcoll.c:1758-1780: both attacks must allow
        // clanking and both fighters must be grounded. Already recorded victims
        // are excluded by lbColl_8000ACFC before the clank traversal.
        if desc.clank
            && victim.physics.ground_or_air == GroundOrAir::Ground
            && attacker.physics.ground_or_air == GroundOrAir::Ground
        {
            melee_coll::detection::require_uncontested_hit(
                &victim.commands.hitboxes,
                attacker.spawn_number,
                attacker.physics.ground_or_air,
            );
        }
        if contact.overlap < assets.damage.phantom_threshold {
            unimplemented!("ftcoll.c:589-623: phantom hit");
        }
        if victim.status.revival_invincibility != 0 {
            // ftColl_80078C70 inlineB3: record the contact and attacker hitlag,
            // but omit damage logging/staling while x198C selects invincibility.
            let group = desc.group;
            attacker.combat.dealt_damage =
                attacker.combat.dealt_damage.max(fctiwz(desc.damage).max(1));
            melee_coll::detection::record_victim(
                &mut attacker.commands.hitboxes,
                group,
                victim.spawn_number,
            );
            victim
                .effects
                .push(melee_ef::request::EffectRequest::ShieldSpark {
                    position: contact.position,
                });
            return;
        }
        if victim.commands.hurt_status != melee_types::combat::HurtStatus::Normal {
            unimplemented!("ftcoll.c:658-662: invincible contact");
        }
        if victim.combat.pending.is_some() {
            unimplemented!("ftcoll.c:2534: simultaneous damage log selection");
        }
        let descriptor = desc.clone();
        let knockback = assets.damage.knockback_with_damage(
            &descriptor,
            victim.physics.percent,
            victim.attributes.size.weight,
            hit.knockback_damage,
        );
        if let Some(super::grab::GrabLink::Captured { captor }) = victim.combat.grab {
            if captor != attacker.spawn_number {
                unimplemented!("ftCo_8008EC90: third-party hit on a captured fighter");
            }
            victim.combat.pending_from_captor = true;
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
        attacker.combat.combo.record(
            victim.spawn_number,
            attacker.combat.stale.current_move(),
            &assets.combo,
        );
        attacker.combat.stale.record();
        attacker.commands.stale_multiplier =
            Some(attacker.combat.stale.multiplier(&assets.stale_weights));
        // ftColl_8007891C -> plStale_UpdateStaleMovesFromFighter: the first
        // queue entry is visible to later hitbox commands of this same move.
        attacker.commands.first_hit_stale_penalty = Some(assets.first_stale_penalty);
        attacker.combat.dealt_damage = fctiwz(descriptor.damage).max(1);
        melee_coll::detection::record_victim(
            &mut attacker.commands.hitboxes,
            descriptor.group,
            victim.spawn_number,
        );
        // ftColl_8007A06C -> efSync_Spawn; slash uses effect 1004.
        victim
            .effects
            .push(melee_ef::request::EffectRequest::HitSpark {
                position: contact.position,
                element: descriptor.element,
                // ftColl_8007A06C: the effect receives entry->x20 converted to u32.
                damage: fctiwz(descriptor.damage) as f32,
                large: knockback >= assets.damage.large_spark_threshold,
            });
        if descriptor.element == melee_types::HitElement::Normal && descriptor.sound_severity >= 1 {
            let variant = victim.attributes.combat.hit_spark_variant;
            if let Some(&random_bound) = assets.damage.extra_spark_bounds.get(variant as usize) {
                victim
                    .effects
                    .push(melee_ef::request::EffectRequest::NormalSparkExtra {
                        position: contact.position,
                        facing: victim.physics.facing,
                        variant,
                        random_bound,
                    });
            }
        }
    }
}

impl FighterCore {
    /// ftCo_800C0408: color programs advance during hitlag as well as ordinary animation.
    pub(super) fn advance_damage_overlay(&mut self, assets: &FighterAssets) {
        if let Some((id, overlay)) = &mut self.combat.damage_overlay {
            overlay.step(
                &assets.charge_overlays[id],
                &mut self.commands.graphics,
                &mut self.commands.footstep_sounds,
            );
        }
    }
    /// ftCo_8008DCE0 (8008DCE0): common launch calculation before motion entry.
    fn prepare_damage_reaction(
        &mut self,
        hit: &ReceivedHit,
        forced_motion: Option<S>,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> (S, f32) {
        let airborne = self.physics.ground_or_air == GroundOrAir::Air;
        let stun = hit.knockback * assets.damage.hitstun_scale;
        let base_level = assets
            .damage
            .reaction_thresholds
            .iter()
            .position(|&t| stun < t)
            .unwrap_or(3);
        // ftCo_8008DCE0 block_9: an explicit motion forces level 3, not the angle.
        let level = if forced_motion.is_some() {
            3
        } else {
            base_level
        };
        // ftCo_803C5520: reaction depends on hit strength and hurtbox height.
        const REACTIONS: [[S; 3]; 4] = [
            [S::DamageLw1, S::DamageN1, S::DamageHi1],
            [S::DamageLw2, S::DamageN2, S::DamageHi2],
            [S::DamageLw3, S::DamageN3, S::DamageHi3],
            [S::DamageFlyLw, S::DamageFlyN, S::DamageFlyHi],
        ];
        let height = match hit.height {
            HurtHeight::Low => 0,
            HurtHeight::Middle => 1,
            HurtHeight::High => 2,
        };
        const AIR_REACTIONS: [S; 3] = [S::DamageAir1, S::DamageAir2, S::DamageAir3];
        let mut state = if airborne && level < 3 {
            AIR_REACTIONS[level]
        } else {
            REACTIONS[level][height]
        };
        self.physics.percent += hit.descriptor.damage;
        let angle = assets.damage.launch_angle(
            hit.descriptor.angle,
            hit.knockback,
            self.physics.ground_or_air,
        );
        if level == 3
            && angle > assets.damage.top_angle_range[0]
            && angle < assets.damage.top_angle_range[1]
        {
            state = S::DamageFlyTop;
        }
        let mut speed = hit.knockback * assets.damage.velocity_scale;
        // ftCo_Damage_CheckAirMotion (8008E498), then 8008DEFC fmuls.
        if airborne
            && matches!(
                self.motion_state.id,
                S::JumpF
                    | S::JumpB
                    | S::JumpAerialF
                    | S::JumpAerialB
                    | S::Fall
                    | S::FallF
                    | S::FallB
                    | S::FallAerial
                    | S::FallAerialF
                    | S::FallAerialB
                    | S::FallSpecial
                    | S::FallSpecialF
                    | S::FallSpecialB
                    | S::DamageFall
                    | S::EscapeAir
            )
            && i32::from(self.input.buttons.digital_shield) <= assets.damage.air_cancel_window
            && i32::from(self.input.buttons.previous_digital_shield) >= assets.damage.tech_lockout
        {
            speed *= assets.damage.air_cancel_scale;
        }
        self.physics.facing = hit.facing;
        // ftCo_8008DCE0: separate products before testing the floor normal.
        let horizontal = -(speed * cosf(angle)) * hit.facing;
        let vertical = speed * sinf(angle);
        let bounced = self.apply_damage_velocity(horizontal, vertical, level, airborne, assets);
        // ftCo_Damage.c block_33: a level-3 launch that leaves the ground (or
        // was airborne) outside the top range rolls for DamageFlyRoll once the
        // percent (with this hit applied) reaches PlCo +23C.
        if level == 3
            && state != S::DamageFlyTop
            && self.physics.ground_or_air == GroundOrAir::Air
            && self.physics.percent >= assets.damage.fly_roll_percent as f32
            && rng.randf() < assets.damage.fly_roll_chance
        {
            state = S::DamageFlyRoll;
        }
        // Retail block_36 overrides the motion only after the fly-roll draw.
        if let Some(forced_motion) = forced_motion {
            state = forced_motion;
        }
        // ftCo_Damage.c block_70: queue the hit sound and voice set by scaled
        // knockback (var_r27 is only cleared on the unported steep-floor path).
        if !bounced && stun >= assets.damage.heavy_voice_threshold {
            self.combat.queued_hit_sfx = Some(HEAVY_HIT_SFX);
            self.combat.queued_voice = Some(DamageVoice::Heavy);
        } else if !bounced && stun >= assets.damage.medium_voice_threshold {
            self.combat.queued_hit_sfx = Some(MEDIUM_HIT_SFX);
            self.combat.queued_voice = Some(DamageVoice::Medium);
        }
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        if let Some(facing) = hit.facing_override {
            self.physics.facing = facing;
        }
        (state, stun)
    }

    /// ftCo_8008DCE0 blocks 20..28: grounded launches and downward tumble bounce.
    fn apply_damage_velocity(
        &mut self,
        x: f32,
        mut y: f32,
        level: usize,
        airborne: bool,
        assets: &FighterAssets,
    ) -> bool {
        let normal = self.collision.data.floor.normal;
        let floor_angle = if airborne {
            0.0
        } else {
            melee_lb::dynamics::arithmetic::angle(normal, Vec3::new(x, y, 0.0))
        };
        let bounced = !airborne
            && level == 3
            && f64::from(floor_angle)
                > std::f64::consts::FRAC_PI_2 + f64::from(assets.damage.floor_bounce_angle);
        if airborne || floor_angle < std::f32::consts::FRAC_PI_2 || level == 3 {
            if !airborne {
                self.leave_ground();
            }
            if bounced {
                // retail 8008DFF4 fneg, 8008DFFC fmuls.
                y = gekko_math::fma::negate_rounded(y) * assets.damage.floor_bounce_scale;
                let floor_angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
                self.effects
                    .push(melee_ef::request::EffectRequest::Graphics {
                        id: 0x406,
                        bone: usize::from(assets.parts.joint(melee_types::FtPart::TopN).unwrap()),
                        offset: Vec3::ZERO,
                        facing: self.physics.facing,
                        floor_angle,
                    });
            }
            self.combine_knockback(x, y, assets);
            self.physics.ground_knockback_velocity = 0.0;
        } else {
            self.physics.ground_knockback_velocity = x;
            self.combine_knockback(normal.y * x, -normal.x * x, assets);
        }
        bounced
    }

    /// ftCo_Damage_CalcVel (8008DC0C): old and new opposite components add;
    /// same-direction components keep the larger magnitude after PlCo +FC.
    fn combine_knockback(&mut self, x: f32, y: f32, assets: &FighterAssets) {
        let velocity = &mut self.physics.knockback_velocity;
        if self.status.time_since_hit < assets.damage.knockback_replace_window {
            velocity.x = x;
            velocity.y = y;
            return;
        }
        // Retail uses separate fmuls/fadds; preserve component signed zero.
        for (current, incoming) in [(&mut velocity.x, x), (&mut velocity.y, y)] {
            if *current * incoming < 0.0 {
                *current += incoming;
            } else if gekko_math::msl::fabsf(incoming) > gekko_math::msl::fabsf(*current) {
                *current = incoming;
            }
        }
    }
    /// ftCo_8008DCE0: hitstun, timers and input ages after frame-zero playback.
    fn finish_damage_reaction(
        &mut self,
        hit: ReceivedHit,
        stun: f32,
        assets: &FighterAssets,
    ) -> Result<i32> {
        self.state_data = MotionData::Damage(DamageState {
            hitstun: (fctiwz(stun).max(1)) as f32,
            jump_buffer: 0.0,
            trail_timer: 0,
            influence: assets.damage.influence,
        });
        self.status.time_since_hit = 0;
        self.status.interaction = Interaction::Damage;
        self.input.horizontal.tilt = 254;
        self.input.vertical.tilt = 254;
        Ok(fctiwz(hit.descriptor.damage).max(1))
    }
}

fn is_tumble(state: S) -> bool {
    matches!(
        state,
        S::DamageFlyHi | S::DamageFlyN | S::DamageFlyLw | S::DamageFlyTop | S::DamageFlyRoll
    )
}

/// SDI/ASDI radius checks deliberately do not fuse (8008E744..754).
fn stick_magnitude_passes(stick: crate::input::Stick, minimum: f32) -> bool {
    stick.x * stick.x + stick.y * stick.y >= minimum * minimum
}

/// ftCo_8008E5A4 (ftCo_Damage.c:592-622): signed squared perpendicular
/// stick projection rotates the launch while preserving its magnitude.
pub(super) fn apply_directional_influence(
    velocity: &mut Vec3,
    stick: crate::input::Stick,
    degrees: f32,
) {
    if stick.x == 0.0 && stick.y == 0.0 {
        return;
    }
    let (x, y) = (velocity.x, velocity.y);
    let negative_x = gekko_math::fma::negate_rounded(x);
    // 8008E5FC / 8008E624: fmadds, with the second product rounded first.
    let squared_speed = fmadds(negative_x, negative_x, y * y);
    const MINIMUM_SQUARED_SPEED: f32 = 0.00001; // ftCo_8008E5A4's zero guard.
    if squared_speed < MINIMUM_SQUARED_SPEED {
        return;
    }
    let perpendicular = fmadds(y, stick.x, negative_x * stick.y);
    let mut influence = (perpendicular * perpendicular) / squared_speed;
    // PSVECCrossProduct's Z sign; its first product rounds before ps_msub.
    let cross = gekko_math::fma::fnmsubs(y, stick.x, stick.y * x);
    if cross < 0.0 {
        influence = gekko_math::fma::negate_rounded(influence);
    }
    let angle = melee_lb::trigf::atan2f(y, x);
    // 8008E660 fmadds, then the audited three-step double sqrt refinement.
    let speed = sqrtf(fmadds(x, x, y * y));
    let radians = (std::f32::consts::PI / 180.0) * degrees;
    // 8008E6CC: fmadds, not a separately rounded angle increment.
    let angle = fmadds(radians, influence, angle);
    velocity.x = speed * cosf(angle);
    velocity.y = speed * sinf(angle);
}
