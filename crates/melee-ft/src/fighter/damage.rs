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
use melee_coll::damage_log::{DamageLog, HitSource, LoggedHit};
use melee_coll::{geometry::Contact, hitbox::HitCapsule, hurtbox::HurtHeight};
use melee_types::combat::HitboxDescriptor;
use melee_types::{CommonMotionState as S, GroundOrAir};

#[derive(Default, Clone)]
pub struct CombatState {
    /// Fighter.dmg.armor1 (+18B4), reset on motion change.
    pub armor: f32,
    pub charge_overlay: super::smash::ChargeOverlay,
    /// Fighter.x408: the primary color animation (damage tints, burning,
    /// powershield flash); see `color_overlay`.
    pub color_overlay: super::color_overlay::ColorOverlaySlot,
    pub capture_geometry: super::grab_throw::CaptureGeometry,
    pub thrown_pose: Option<super::grab_throw::ThrownPose>,
    pub grab: Option<super::grab::GrabLink>,
    pub hitlag_remaining: f32,
    /// ftcoll.c dmg_log0: ordinary hits logged against this fighter during
    /// its hit detection (ftColl_80078C70), resolved by ftColl_8007AB48.
    pub hit_log: DamageLog,
    /// ftcoll.c dmg_log1: phantom contacts, resolved by ftColl_8007AB80.
    pub phantom_log: DamageLog,
    /// dmg.x1838_percentTemp: damage of every hit logged this frame. Each
    /// logged hit's knockback and the percent applied use the total.
    pub frame_damage: f32,
    /// dmg.x183C_applied: largest integer damage logged this frame.
    pub frame_max_damage: i32,
    /// dmg.x1840: largest halved damage among this frame's phantom contacts.
    pub phantom_max_damage: i32,
    /// dmg.x187c / x18a0: this frame's strongest phantom knockback; nonzero
    /// selects ProcessHit's phantom branch. Cleared by every resolution.
    pub phantom_knockback: f32,
    /// dmg.x1870..x1898: the last resolved phantom contact, applied when its
    /// hitlag lockout expires without an ordinary hit (ftColl_8007BE3C).
    pub phantom: Option<PhantomHit>,
    /// dmg.x189C: phantom hitlag frames still to run; blocks new phantoms.
    pub phantom_lockout: f32,
    /// The hit ftColl_8007AB48 selected from `hit_log`, with its knockback.
    pub pending: Option<ReceivedHit>,
    /// A phantom's source fighter to credit (stale moves, combo) after this
    /// fighter's ProcessHit applied the phantom's damage (ftColl_8007BE3C).
    pub pending_credit: Option<u32>,
    /// ftColl_8007A06C: the selected hit came from this captured fighter's captor.
    pub pending_from_captor: bool,
    /// Fighter.dmg.x1908 / x190C: the hit sound and voice set queued by the
    /// launch calculation, played by the next hit proc that starts no hitlag
    /// (Fighter_ProcessHit's else branch -> ftCo_80090718).
    pub queued_hit_sfx: Option<u32>,
    pub queued_voice: Option<DamageVoice>,
    pub dealt_damage: i32,
    pub clank: super::clank::Pending,
    pub reflection: Option<super::reflection::Pending>,
    pub reflector_enabled: bool,
    /// hitlag_cb / post_hitlag_cb, cleared by every motion change
    /// (fighter.c:1381-1383). They carry SDI during and ASDI after hitlag.
    pub hitlag_callbacks: HitlagCallbacks,
    /// Fighter +1964: special shield minimum hitlag, consumed by ProcessHit.
    pub minimum_hitlag: f32,
    pub shield_pushback: Option<(f32, f32)>,
    /// Legacy throw-entry guard; normal attacks use the fixed stale history.
    pub has_recorded_hit: bool,
    pub stale: super::attack::stale::StaleHistory,
    pub combo: super::attack::combo::ComboState,
}
/// ftColl_8007A06C's DmgResult for the phantom log (Fighter.dmg.x1870..x1898).
#[derive(Clone, Debug)]
pub struct PhantomHit {
    /// x1880: the contact position.
    pub position: Vec3,
    /// x188c / x1890: element and sound severity of the phantom hitbox.
    pub element: melee_types::HitElement,
    pub sound_severity: u8,
    /// x1894: the fighter whose move receives the stale-move update.
    pub source: HitSource,
    /// x1898: the halved damage applied at expiry.
    pub damage: f32,
}
/// The installed hitlag_cb / post_hitlag_cb pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitlagCallbacks {
    #[default]
    None,
    /// ftCo_Damage_OnEveryHitlag / OnExitHitlag (ftCo_Damage.c:464-467).
    Damage,
    /// ftCo_80093240 / ftCo_800932DC, installed by ftCo_80092F2C (GuardSetOff).
    Guard,
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
#[derive(Clone, Copy, Debug, Default)]
pub struct InfluenceParameters {
    pub minimum_stick: f32,
    pub tap_window: i32,
    pub sdi_distance: f32,
    pub asdi_distance: f32,
    pub maximum_angle_degrees: f32,
    pub shield_velocity_scale: f32,
    /// PlCo +4C0: scales shield SDI and ASDI (ftCo_80093240 / ftCo_800932DC).
    pub shield_influence_scale: f32,
}
use melee_coll::damage::ReceivedHit;
pub struct DamageParameters {
    pub influence: InfluenceParameters,
    pub jump_buffer_window: f32,
    pub knockback_replace_window: i32,
    pub air_cancel_window: i32,
    pub air_cancel_scale: f32,
    /// PlCo +124: ftCo_Damage_CalcKnockback's Squat / SquatWait multiplier.
    pub crouch_knockback_scale: f32,
    /// PlCo +718: the frozen (DamageIce) multiplier.
    pub frozen_knockback_scale: f32,
    /// PlCo +7C4: the multiplier while charging a smash attack.
    pub smash_charge_knockback_scale: f32,
    /// PlCo +104: knockback never drops below this after armor.
    pub minimum_knockback: f32,
    pub crouch_hitlag_scale: f32,
    pub electric_hitlag_scale: f32,
    pub captured_item_damage_scale: f32,
    pub thrown_hitbox_minimum_speed: f32,
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
                shield_influence_scale: r.f32(p + 0x4C0)?,
            },
            jump_buffer_window: r.f32(p + 0x1D0)?,
            knockback_replace_window: r.s32(p + 0xFC)?,
            air_cancel_window: r.s32(p + 0x18C)?,
            air_cancel_scale: r.f32(p + 0x190)?,
            crouch_knockback_scale: r.f32(p + 0x124)?,
            frozen_knockback_scale: r.f32(p + 0x718)?,
            smash_charge_knockback_scale: r.f32(p + 0x7C4)?,
            minimum_knockback: r.f32(p + 0x104)?,
            crouch_hitlag_scale: r.f32(p + 0x1A0)?,
            electric_hitlag_scale: r.f32(p + 0x1A4)?,
            captured_item_damage_scale: r.f32(p + 0x128)?,
            thrown_hitbox_minimum_speed: r.f32(p + 0x1c8)?,
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
    pub(super) fn knockback_for_frame(
        &self,
        hit: &HitboxDescriptor,
        percent: f32,
        frame_damage: f32,
        weight: f32,
        damage: u32,
    ) -> f32 {
        self.knockback_parameters()
            .knockback_for_frame(hit, percent, frame_damage, weight, damage)
    }
    pub(super) fn knockback_with_damage(
        &self,
        hit: &HitboxDescriptor,
        percent: f32,
        weight: f32,
        damage: u32,
    ) -> f32 {
        self.knockback_parameters()
            .knockback_with_damage(hit, percent, weight, damage)
    }
    fn knockback_parameters(&self) -> melee_coll::damage::KnockbackParameters {
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
pub fn detect_hit(
    victim: &mut Fighter,
    attacker: &mut Fighter,
    assets: &FighterAssets,
    incoming_after_receiver: bool,
) {
    if victim.core.status.disabled
        || attacker.core.status.disabled
        || attacker.core.commands.thrown_by == Some(victim.core.spawn_number)
    {
        return;
    }
    let mut clank_mask = super::clank::candidates(&victim.core, &attacker.core);
    let mut cursor = melee_coll::detection::PairCursor::default();
    while let Some(id) = cursor.next(
        &attacker.core.commands.hitboxes,
        victim.core.spawn_number,
        victim.core.physics.ground_or_air,
    ) {
        if incoming_after_receiver
            && super::clank::contact(
                &mut victim.core,
                &mut attacker.core,
                id,
                &mut clank_mask,
                &assets.clank,
            )
        {
            continue;
        }
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
            self.finish_damage_physics_callback(assets);
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
        self.finish_damage_physics_callback(assets);
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.core.physics);
        crate::physics::integrate::integrate_environment(&mut self.core.physics, None, wind);
    }
    /// ftCo_DamageFlyRoll_Phys (8009035C), ftCo_Damage.c: the two
    /// doFlyRoll calls straddle ftColl_8007AFF8, before procPhysics KB decay.
    fn finish_damage_physics_callback(&mut self, assets: &FighterAssets) {
        self.update_damage_roll_rotation(assets);
        self.clear_slow_thrown_hitboxes(assets);
        self.update_damage_roll_rotation(assets);
    }

    /// Inlined doFlyRoll at 0x800903C4 and 0x8009045C, ftCo_Damage.c.
    fn update_damage_roll_rotation(&mut self, assets: &FighterAssets) {
        if self.motion_state.id != S::DamageFlyRoll {
            return;
        }
        let velocity = self.physics.self_velocity;
        let knockback = self.physics.knockback_velocity;
        // 800903D8/DC and 80090470/74: separate fadds, atan2f(X, Y).
        // 800903F0 / 80090488: fmuls by Fighter+2C (facing), no fusion.
        // Entry repeats this at 8008E308/30C (fadds), 8008E320 (fmuls).
        let angle = self.physics.facing
            * melee_lb::trigf::atan2f(velocity.x + knockback.x, velocity.y + knockback.y);
        let bone = usize::from(
            assets
                .parts
                .joint(melee_types::FtPart::XRotN)
                .expect("XRotN"),
        );
        let joint = self.animation.parts[bone].joint;
        // ftPartSetRotX (8007592C): quaternion main joints redirect to
        // FighterBone.x4_jobj2, whose rotation must be Euler.
        let core = &mut self.core;
        let tree = if core.skeleton.get(joint).flags & hsd_anim::jobj::JOBJ_USE_QUATERNION != 0 {
            &mut core.animation.blend_tree
        } else {
            &mut core.skeleton
        };
        tree.set_rotation_x(joint, angle);
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
        let clank = self.core.combat.clank;
        self.core.combat.clank.damage = 0;
        self.core.combat.clank.duration = 0.0;
        // +1920 facing is retained; only +1918/+191C reset at the retail tail.
        let reflection = self.core.combat.reflection.take();
        let received_knockback = self.core.combat.pending_from_captor
            || self
                .core
                .combat
                .pending
                .as_ref()
                .is_some_and(|hit| hit.knockback != 0.0);
        // Fighter_ProcessHit, fighter.c:2844-2858: the phantom lockout counts
        // down first; an ordinary hit with knockback cancels it.
        self.core.expire_phantom_lockout(received_knockback, assets);
        if received_knockback {
            self.core.combat.phantom_lockout = 0.0;
        }
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
            } else if hit.knockback != 0.0 {
                if let Some(take_damage) = self.character.table().take_damage {
                    take_damage(self);
                }
                self.begin_damage_reaction(hit, None, None, assets, rng)?;
                // fighter.c:2888: hitlag uses dmg.x183C_applied, the largest
                // damage logged this frame.
                hit_damage = self.core.combat.frame_max_damage;
            }
        }
        // fighter.c:2907: without knockback, a resolved phantom contact
        // (dmg.x18a0) takes the hitlag branch with its halved damage (x1840).
        let phantom_hitlag = !received_knockback && self.core.combat.phantom_knockback != 0.0;
        if phantom_hitlag {
            if self.core.shield.impact.is_some() {
                unimplemented!("fighter.c:2907-2918: phantom contact and shield impact together");
            }
            hit_damage = self.core.combat.phantom_max_damage;
        }
        // fighter.c:2956-2963: damage without knockback (zero-knockback hits,
        // an expired phantom) still reaches percent.
        if !received_knockback && self.core.combat.frame_damage != 0.0 {
            self.core.physics.percent += self.core.combat.frame_damage;
        }
        // Fighter_ProcessHit: received damage and shield impact precede clank;
        // clank precedes ordinary damage dealt. Every path consumes the scratch.
        if !received_knockback
            && !phantom_hitlag
            && self.core.shield.impact.is_none()
            && clank.damage != 0
        {
            hit_damage = clank.damage;
            if clank.duration != 0.0 && self.core.combat.grab.is_none() {
                self.enter_rebound(assets, clank)?;
            }
        }
        if !received_knockback
            && !phantom_hitlag
            && self.core.shield.impact.is_none()
            && hit_damage == 0
        {
            if let Some(reflection) = reflection {
                self.process_reflection(reflection, assets)?;
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
                if phantom_hitlag {
                    // fighter.c:2982: bool4 -> x189C = the phantom's hitlag.
                    self.core.combat.phantom_lockout = self.core.combat.hitlag_remaining;
                }
            }
        }
        self.core.combat.minimum_hitlag = 0.0;
        // fighter.c:3016-3022: this frame's damage bookkeeping resets.
        self.core.combat.frame_damage = 0.0;
        self.core.combat.frame_max_damage = 0;
        self.core.combat.phantom_max_damage = 0;
        self.core.combat.phantom_knockback = 0.0;
        Ok(())
    }
    /// ftCo_8008DCE0 (8008DCE0): launch and enter the strength/height reaction.
    pub(super) fn begin_damage_reaction(
        &mut self,
        mut hit: ReceivedHit,
        forced_motion: Option<S>,
        throw_owner: Option<u32>,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<i32> {
        hit.knockback = self.core.modified_knockback(hit.knockback, assets);
        let (state, stun) = self
            .core
            .prepare_damage_reaction(&hit, forced_motion, assets, rng);
        // ftCo_8008DA4C: with damage this frame (x1838_percentTemp), the
        // element's color animation for the reaction level, else the plain
        // damage flash (4), before Fighter_ChangeMotionState evaluates frame zero.
        if hit.percent_damage != 0.0 {
            let level = if forced_motion.is_some() {
                3
            } else {
                assets
                    .damage
                    .reaction_thresholds
                    .iter()
                    .position(|&t| stun < t)
                    .unwrap_or(3) as u8
            };
            let id = match hit.descriptor.element {
                melee_types::HitElement::Fire => 11 + level,
                melee_types::HitElement::Electric => 15 + level,
                melee_types::HitElement::Ice => 31 + level,
                melee_types::HitElement::Dark => 35 + level,
                _ => DAMAGE_FLASH,
            };
            self.core
                .commands
                .color_animations
                .push(melee_cmd::ColorAnimationRequest { id, duration: 0 });
        }
        self.change_damage_motion(state.into(), assets, throw_owner)?;
        self.step_animation(assets);
        let result = self.core.finish_damage_reaction(hit, stun, assets)?;
        // ftCo_8008DCE0 inlineA1 (8008E2F4..8008E334): initialize XRotN
        // after frame-zero playback, before hitlag can suppress physics.
        self.update_damage_roll_rotation(assets);
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
        // ftCo_8008DCE0 block_83: AFTER initial animation and reaction setup.
        (self.character.table().knockback_enter)(self, assets);
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
        }
        // ftCo_8008F744: the ownership bit, not a positive timer crossing,
        // controls this one-shot callback (including imported nonpositive timers).
        if self.core.status.in_hitstun && damage.hitstun <= 0.0 {
            self.core.status.in_hitstun = false;
            self.core.combat.combo.grace = assets.combo.grace_frames;
            (self.character.table().knockback_exit)(self, assets);
        }
        // ftCo_DamageFlyRoll_Anim (800901D0), 800902F4..80090304:
        // enter DamageFall immediately after hitstun, independent of animation.
        if self.core.motion_state.id == S::DamageFlyRoll {
            return if self.core.status.in_hitstun {
                Ok(())
            } else {
                self.enter_damage_fall(assets)
            };
        }
        let MotionData::Damage(damage) = &self.core.state_data else {
            panic!("damage scratch missing")
        };
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
    /// ftCo_DamageFall_IASA (80090828): no mv.damage access. Also used by
    /// DamageFly after hitstun and ordinary airborne damage with dodge enabled.
    fn post_hitstun_air_input(&mut self, assets: &FighterAssets, tumbling: bool) -> Result<()> {
        use crate::input::WaitTransition as T;
        // A damage state is neither Jump nor JumpAerial, so the
        // float check (Peach) is always enabled here, as in procs.rs.
        let vertical_velocity = self.core.physics.self_velocity.y;
        let transition = super::fall::iasa(
            &self.core.input,
            &assets.input,
            self.core.physics.jumps_used,
            self.core.attributes.jumping.max_jumps,
            // DamageFly delegates to DamageFall (80090828), whose
            // input chain omits ordinary Fall's air-dodge check.
            !tumbling,
            |phase| {
                self.character.check_float_input(
                    &self.core.input,
                    assets,
                    vertical_velocity,
                    phase,
                );
            },
        );
        match transition {
            T::None => {
                if tumbling
                    && gekko_math::msl::fabsf(self.core.input.current.stick.x)
                        >= assets.damage.tumble_exit_threshold
                    && i32::from(self.core.input.horizontal.tilt) < assets.damage.tumble_exit_window
                {
                    self.change_motion_state(S::Fall.into(), assets)?;
                }
                Ok(())
            }
            T::Attack => (self.character.table().enter_aerial)(self, assets),
            T::Jump => self.enter_aerial_jump(assets),
            T::Escape => self.enter_air_dodge(assets),
            T::Special => {
                self.enter_buffered_special(assets, true);
                Ok(())
            }
            transition => unimplemented!("ftCo_Damage_IASA: airborne {transition:?}"),
        }
    }
    /// ftCo_DamageFall_IASA: Cliff timeout retains Cliff scratch; ordinary
    /// tumble entry retains Damage scratch. Neither is read by this callback.
    pub(super) fn damage_fall_input(&mut self, assets: &FighterAssets) -> Result<()> {
        self.post_hitstun_air_input(assets, true)
    }
    /// ftCo_DamageFall_Coll (80090960) -> ft_8008370C: ordinary ledge
    /// snap height, then ftCo_80090984 tech/down-bound landing.
    /// TODO: shared ftWallJump_8008169C before the ledge predicate.
    pub(super) fn damage_fall_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        crate::collision::air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        let landed = crate::collision::air::collide_pass(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.status.ledge_cooldown == 0,
        );
        if landed {
            if !self.try_tech(assets)? {
                self.enter_down_bound(assets)?;
            }
        } else if !self.try_wall_jump(assets, map)? {
            self.try_grab_ledge(assets, map)?;
        }
        Ok(())
    }
    /// ftCo_DamageFall_Phys -> ft_80084DB0: fastfall, gravity, drift;
    /// Fighter_procUpdate subsequently decays knockback and integrates.
    pub(super) fn damage_fall_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        self.airborne_physics(assets);
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.core.physics);
        crate::physics::integrate::integrate_environment(&mut self.core.physics, None, wind);
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
            let tumbling =
                self.core.motion_state.id == S::DamageFall || is_tumble(self.core.motion_state.id);
            // ftCo_Damage_IASA (8008FA44): synthesize XY for a jump pressed
            // within PlCo's final hitstun window. The stored value does not age.
            if !tumbling
                && damage.jump_buffer != 0.0
                && damage.jump_buffer <= assets.damage.jump_buffer_window
            {
                self.core.input.pressed |= crate::input::Buttons::XY;
            }
            if self.core.physics.ground_or_air == GroundOrAir::Air {
                return self.post_hitstun_air_input(assets, tumbling);
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
    /// ftCo_DamageFly_Phys (80090030..C8): disable collateral throw hits
    /// once knockback speed drops below PlCo+1C8, before shared decay.
    fn clear_slow_thrown_hitboxes(&mut self, assets: &FighterAssets) {
        if is_tumble(self.motion_state.id) && self.commands.thrown_by.is_some() {
            let v = self.physics.knockback_velocity;
            // 80090044..5C: separate fmuls/fadds; sqrt has three Newton steps.
            if sqrtf(v.z * v.z + (v.x * v.x + v.y * v.y))
                < assets.damage.thrown_hitbox_minimum_speed
            {
                self.commands.hitboxes.fill(None);
            }
        }
    }
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
            panic!("damage hitlag callback without damage scratch");
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
            panic!("damage hitlag callback without damage scratch");
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

    /// ftColl_80077688 (80077688): projectile shield damage and strongest impact.
    /// The mature-shield path has no fused arithmetic (retail asm audited).
    /// Unlike fighter shield contacts, this does not recoil or hitlag the owner.
    fn record_item_shield_hit(
        &mut self,
        item: &mut melee_it::ItemCore,
        id: usize,
        contact: Contact,
    ) {
        let desc = &item.hitboxes[id]
            .as_ref()
            .expect("eligible item hit")
            .descriptor;
        // retail 800776DC..80077714: zero stays zero; nonzero values which
        // truncate to zero become one. Negative integers retain their sign.
        let integer = fctiwz(desc.damage);
        let damage = if desc.damage == 0.0 {
            0
        } else if integer == 0 {
            1
        } else {
            integer
        };
        if damage > item.pending_damage_dealt {
            item.pending_shield_damage = damage;
            item.pending_shield_deflection = if item.hit_flags[id].shield_bounce {
                let volume = &self.shield.hit;
                let joint = self.animation.parts[volume.bone].joint;
                let matrix = *self.skeleton.get_mtx(joint);
                Some(melee_lb::shield::deflection(
                    volume.position,
                    &matrix,
                    item.hitboxes[id].as_ref().unwrap().previous_position,
                    item.hitboxes[id].as_ref().unwrap().position,
                    volume.radius,
                    desc.radius * if desc.ignore_scale { 1.0 } else { item.scale },
                ))
            } else {
                None
            };
        }
        self.shield.damage_taken += (damage + i32::from(desc.shield_damage)).max(0);
        if damage
            > self
                .shield
                .impact
                .as_ref()
                .map_or(0, |impact| impact.damage)
        {
            self.shield.impact = Some(super::shield::ShieldImpact {
                damage,
                facing: if self.physics.position.x > item.position.x {
                    -1.0
                } else {
                    1.0
                },
                element: desc.element,
            });
        }
        let group = desc.group;
        melee_coll::detection::record_victim(&mut item.hitboxes, group, self.spawn_number);
        self.effects
            .push(melee_ef::request::EffectRequest::ShieldSpark {
                position: contact.position,
            });
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
    /// Fighter_procUpdate's airborne tail (8006B82C): residual knockback
    /// decays after every state's physics callback, then velocity and the
    /// environment integrate. Airborne callbacks finish through this.
    pub fn finish_air_update(&mut self, assets: &FighterAssets, wind: Vec3) {
        if self.physics.ground_or_air == GroundOrAir::Air {
            self.decay_air_knockback(assets);
        }
        crate::physics::integrate::integrate_velocity(&mut self.physics);
        crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
    }
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
            if self.combat.hitlag_callbacks == HitlagCallbacks::Damage {
                self.exit_damage_hitlag();
                self.status.interaction = Interaction::Damage;
            } else if matches!(self.state_data, MotionData::Guard(_)) {
                self.shield.allow_sdi = false;
                if self.combat.hitlag_callbacks == HitlagCallbacks::Guard {
                    self.exit_guard_hitlag();
                }
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
        if contact.overlap < assets.damage.phantom_threshold {
            log_phantom_contact(victim, attacker, id, contact, height);
            return;
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
        let descriptor = desc.clone();
        if let Some(super::grab::GrabLink::Captured { captor }) = victim.combat.grab {
            if captor != attacker.spawn_number {
                unimplemented!("ftCo_8008EC90: third-party hit on a captured fighter");
            }
        }
        // ftColl_80076ED8's ordinary branch: log the hit for ftColl_8007AB48,
        // whose knockback and effects wait until every contact is logged.
        victim.combat.log_hit(LoggedHit {
            source: HitSource::Fighter(hit_owner(attacker)),
            hit: ReceivedHit {
                descriptor: descriptor.clone(),
                height,
                facing: if victim.physics.position.x > attacker.physics.position.x {
                    -1.0
                } else {
                    1.0
                },
                knockback: 0.0,
                facing_override: None,
                percent_damage: descriptor.damage,
            },
            position: contact.position,
            knockback_damage: hit.knockback_damage,
            damage: descriptor.damage,
            effect_damage: descriptor.damage,
        });
        attacker.combat.has_recorded_hit = true;
        attacker.credit_hit(victim.spawn_number, assets);
        // ftColl_80076ED8: x1914 keeps the largest damage dealt this frame.
        attacker.combat.dealt_damage = attacker
            .combat
            .dealt_damage
            .max(super::hit_log::damage_count(descriptor.damage));
        melee_coll::detection::record_victim(
            &mut attacker.commands.hitboxes,
            descriptor.group,
            victim.spawn_number,
        );
    }
}

/// ftColl_80076ED8 logs a thrown fighter's hitbox under its thrower.
fn hit_owner(attacker: &FighterCore) -> u32 {
    attacker.commands.thrown_by.unwrap_or(attacker.spawn_number)
}

/// Halve a hit's damage for a phantom contact: ftColl_80076ED8 keeps a
/// nonzero damage at least 1 (`!(int)(0.5f * dmg) && dmg`).
fn phantom_damage(damage: f32) -> f32 {
    let half = 0.5 * damage;
    if fctiwz(half) == 0 && damage != 0.0 {
        1.0
    } else {
        half
    }
}

/// ftColl_80076ED8's phantom branch (hit0->coll_distance < PlCo +7A8): no
/// ordinary hit this frame, no phantom lockout and a hitbox that has not
/// phantomed this victim yet. The contact is marked on the attacking group,
/// logged for ftColl_8007AB80 unless the victim is invincible, and plays the
/// victim's phantom sound (ftColl_80078488, ft_PlaySFX 85).
fn log_phantom_contact(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    id: usize,
    contact: Contact,
    height: HurtHeight,
) {
    let hit = attacker.commands.hitboxes[id]
        .as_ref()
        .expect("eligible hitbox");
    if !victim.combat.hit_log.is_empty()
        || victim.combat.phantom_lockout != 0.0
        || hit.phantom_victims.contains(victim.spawn_number)
    {
        return;
    }
    let descriptor = hit.descriptor.clone();
    let damage = phantom_damage(descriptor.damage);
    // len = unk_count >> 1, at least 1 when nonzero.
    let count = match hit.knockback_damage >> 1 {
        0 if hit.knockback_damage != 0 => 1,
        half => half,
    };
    melee_coll::detection::record_phantom_victim(
        &mut attacker.commands.hitboxes,
        descriptor.group,
        victim.spawn_number,
    );
    if victim.commands.hurt_status == melee_types::combat::HurtStatus::Normal
        && victim.status.revival_invincibility == 0
    {
        victim.combat.phantom_max_damage = victim.combat.phantom_max_damage.max(fctiwz(damage));
        victim.combat.phantom_log.push(LoggedHit {
            source: HitSource::Fighter(hit_owner(attacker)),
            hit: ReceivedHit {
                descriptor,
                height,
                facing: if victim.physics.position.x > attacker.physics.position.x {
                    -1.0
                } else {
                    1.0
                },
                knockback: 0.0,
                facing_override: None,
                percent_damage: damage,
            },
            position: contact.position,
            knockback_damage: count,
            damage,
            effect_damage: damage,
        });
    }
    victim
        .commands
        .footstep_sounds
        .push(super::commands::FootstepSound {
            channel: super::commands::SoundChannel::Ordinary,
            id: PHANTOM_HIT_SFX,
            volume: 127,
            pan: 64,
        });
}
/// ftCo_8008DA4C: the color animation of an ordinary damaging hit.
const DAMAGE_FLASH: u8 = 4;
/// ftColl_80078488: ft_PlaySFX(fp, 85, 0x7F, 0x40) on a phantom contact.
const PHANTOM_HIT_SFX: u32 = 85;

impl FighterCore {
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
        // Fighter_ProcessHit: Fighter_UnkTakeDamage_8006CC30(fp, x1838_percentTemp).
        self.physics.percent += hit.percent_damage;
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
        self.status.in_hitstun = true;
        self.status.time_since_hit = 0;
        self.combat.hitlag_callbacks = HitlagCallbacks::Damage;
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

/// Item contact response is independent of whether the victim logged damage.
#[derive(Clone, Copy, Debug)]
pub struct ItemHurtContact {
    pub damage: f32,
    pub logged_damage: bool,
}

impl Fighter {
    /// ftColl_80078C70 item pass -> ftColl_8007A06C: receiver and capsule order.
    pub fn detect_item_hit(
        &mut self,
        item: &mut melee_it::ItemCore,
        assets: &FighterAssets,
    ) -> Option<ItemHurtContact> {
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
            let hit = item.hitboxes[id].as_ref().unwrap().clone();
            if item.hit_flags[id].reflectable
                && item.hit_flags[id].defense_interaction
                && hit.descriptor.element != melee_types::HitElement::Inert
            {
                if self.shield.reflecting
                    && self.core.shield_reflect_contact(&hit, item.scale).is_some()
                {
                    let volume = self.shield.reflect.clone();
                    let response = match self.shield.on_reflect {
                        Some(super::shield::ReflectHitCallback::Powershield) => {
                            super::reflection::Response::Powershield
                        }
                        _ => super::reflection::Response::None,
                    };
                    self.core.record_reflection(
                        item,
                        id,
                        super::reflection::Settings {
                            maximum: fctiwz(volume.maximum_damage),
                            damage_multiplier: volume.damage_multiplier,
                            speed_multiplier: volume.speed_multiplier,
                            exclude_master_ball_ownership: volume.reflect_behavior,
                        },
                        response,
                    );
                    continue;
                }
                if let Some(contact) = self.character.table().reflector_contact {
                    if let Some(descriptor) = contact(self, &hit, item.scale) {
                        self.core.record_reflection(
                            item,
                            id,
                            super::reflection::Settings {
                                maximum: descriptor.maximum_damage,
                                damage_multiplier: descriptor.damage_multiplier,
                                speed_multiplier: descriptor.speed_multiplier,
                                exclude_master_ball_ownership: descriptor
                                    .exclude_master_ball_ownership,
                            },
                            super::reflection::Response::Character,
                        );
                        continue;
                    }
                }
            }
            if self.shield.active && item.hit_flags[id].shieldable {
                if let Some(contact) = self.core.shield_contact(&hit, item.scale) {
                    self.record_item_shield_hit(item, id, contact);
                    continue;
                }
            }
            if !item.hit_flags[id].hits_hurtboxes {
                continue;
            }
            let Some((contact, height)) =
                melee_coll::detection::first_contact(&mut self.core, &hit, item.scale)
            else {
                continue;
            };
            if contact.overlap < assets.damage.phantom_threshold {
                self.log_item_phantom_contact(item, &hit, contact, height, assets);
                return None;
            }
            let mut descriptor = hit.descriptor.clone();
            // ftColl_80077C60: an item entry's damage count is its hitbox damage
            // (it_8026B1D4, already staled) truncated, not the command count.
            let raw_damage = hit.descriptor.damage;
            let knockback_damage = fctiwz(raw_damage) as u32;
            // ftColl_80077C60, 80077DE4: item hits scale damage while captured.
            if matches!(
                self.combat.grab,
                Some(super::grab::GrabLink::Captured { .. })
            ) {
                descriptor.damage *= assets.damage.captured_item_damage_scale;
            }
            if self.status.revival_invincibility != 0 {
                melee_coll::detection::record_victim(
                    &mut item.hitboxes,
                    descriptor.group,
                    self.spawn_number,
                );
                self.effects
                    .push(melee_ef::request::EffectRequest::ShieldSpark {
                        position: contact.position,
                    });
                return Some(ItemHurtContact {
                    damage: descriptor.damage,
                    logged_damage: false,
                });
            }
            // ftColl_80077C60's ordinary branch: log for ftColl_8007AB48.
            let facing = if self.physics.position.x > item.position.x {
                -1.0
            } else {
                1.0
            };
            self.combat.log_hit(LoggedHit {
                source: HitSource::Item,
                hit: ReceivedHit {
                    descriptor: descriptor.clone(),
                    height,
                    knockback: 0.0,
                    facing,
                    facing_override: None,
                    percent_damage: descriptor.damage,
                },
                position: contact.position,
                knockback_damage,
                damage: descriptor.damage,
                effect_damage: raw_damage,
            });
            melee_coll::detection::record_victim(
                &mut item.hitboxes,
                descriptor.group,
                self.spawn_number,
            );
            return Some(ItemHurtContact {
                damage: descriptor.damage,
                logged_damage: true,
            });
        }
        None
    }

    /// ftColl_80077C60's phantom branch for an item hitbox: the damage is
    /// truncated before halving (at least 1), the knockback count is the
    /// item's integer damage halved (at least 1), and it_8026FC00 marks the
    /// victim on the item's group. Invincible victims log nothing.
    fn log_item_phantom_contact(
        &mut self,
        item: &mut melee_it::ItemCore,
        hit: &HitCapsule,
        contact: Contact,
        height: HurtHeight,
        assets: &FighterAssets,
    ) {
        if !self.combat.hit_log.is_empty()
            || self.combat.phantom_lockout != 0.0
            || hit.phantom_victims.contains(self.spawn_number)
        {
            return;
        }
        let mut descriptor = hit.descriptor.clone();
        let raw = fctiwz(descriptor.damage);
        if matches!(
            self.combat.grab,
            Some(super::grab::GrabLink::Captured { .. })
        ) {
            descriptor.damage *= assets.damage.captured_item_damage_scale;
        }
        let scaled = fctiwz(descriptor.damage);
        let damage = match 0.5 * scaled as f32 {
            half if fctiwz(half) == 0 && scaled != 0 => 1.0,
            half => half,
        };
        let count = match raw / 2 {
            0 if raw != 0 => 1,
            half => half as u32,
        };
        melee_coll::detection::record_phantom_victim(
            &mut item.hitboxes,
            descriptor.group,
            self.spawn_number,
        );
        if self.commands.hurt_status == melee_types::combat::HurtStatus::Normal
            && self.status.revival_invincibility == 0
        {
            let facing = if self.physics.position.x > item.position.x {
                -1.0
            } else {
                1.0
            };
            self.combat.phantom_max_damage = self.combat.phantom_max_damage.max(fctiwz(damage));
            self.combat.phantom_log.push(LoggedHit {
                source: HitSource::Item,
                hit: ReceivedHit {
                    descriptor,
                    height,
                    knockback: 0.0,
                    facing,
                    facing_override: None,
                    percent_damage: damage,
                },
                position: contact.position,
                knockback_damage: count,
                damage,
                effect_damage: damage,
            });
        }
        self.commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: PHANTOM_HIT_SFX,
                volume: 127,
                pan: 64,
            });
    }
}

impl FighterCore {
    /// ftCo_Damage_CalcKnockback (8008D930): the victim's state scales the
    /// computed knockback (separate fmuls), then armor subtracts from it and
    /// PlCo +104 floors it.
    fn modified_knockback(&self, mut knockback: f32, assets: &FighterAssets) -> f32 {
        if knockback == 0.0 {
            return knockback;
        }
        let parameters = &assets.damage;
        if matches!(self.motion_state.id, S::Squat | S::SquatWait) {
            knockback *= parameters.crouch_knockback_scale;
        }
        if self.motion_state.id == S::DamageIce {
            knockback *= parameters.frozen_knockback_scale;
        }
        // smash_attrs.state == SmashState_Charging.
        if self
            .commands
            .smash_charge
            .is_some_and(|c| matches!(c.phase, melee_cmd::ChargePhase::Charging))
        {
            knockback *= parameters.smash_charge_knockback_scale;
        }
        // x34_scale.y copies Player_GetModelScale (fighter.c:255).
        if self.player.scale != 1.0 {
            unimplemented!("ftCo_CalcYScaledKnockback: model-scaled victim");
        }
        // Retail subtracts max(armor0, armor1), plus PlCo +6F0 when metal:
        // zero here, as detection rejects armored victims and no supported
        // mode makes a fighter metal.
        if knockback < parameters.minimum_knockback {
            knockback = parameters.minimum_knockback;
        }
        knockback
    }
}
