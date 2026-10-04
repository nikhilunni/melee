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
use melee_coll::damage_log::{DamageLog, HitCredit, HitSource, LoggedHit};
use melee_coll::{geometry::Contact, hitbox::HitCapsule, hurtbox::HurtHeight};
use melee_gr::wind::Wind;
use melee_types::combat::HitboxDescriptor;
use melee_types::{CommonMotionState as S, FighterKind, GroundOrAir};

#[derive(Default, Clone)]
pub struct CombatState {
    /// dmg.x18F4 / x1954 / x2220_b4: a cape's turnaround.
    pub cape_turn: super::cape_turn::CapeTurn,
    /// Fighter.dmg.armor1 (+18B4), reset on motion change.
    pub armor: f32,
    /// SmashAttr.x2130_sfxBool: the charge sound already played.
    pub charge_sound_played: bool,
    /// Fighter.x408: the primary color animation (damage tints, burning,
    /// powershield flash); see `color_overlay`.
    pub color_overlay: super::color_overlay::ColorOverlaySlot,
    /// Fighter.x488: the secondary color animation, cleared by motion
    /// changes without Ft_MF_SkipColAnim (ftCo_800C0134). A smash charge
    /// installs its color here and clears it on release (ftCo_800C0200).
    pub secondary_color_overlay: super::color_overlay::ColorOverlaySlot,
    /// ftData_UnkMotionStates4[kind]: the color animation the kind installs
    /// whenever the secondary slot empties (Samus's full charge glow, 53,
    /// while u.ss.x2230 == x18). The character keeps it current.
    pub secondary_color_fallback: Option<u8>,
    /// What this cargo carrier's state changes ask of the fighter it
    /// holds; the scene applies them after the proc (`cargo::apply`).
    pub cargo_requests: super::cargo::CargoRequests,
    /// ftCo_Shouldered_Anim found the mash timer spent: the scene ends the
    /// carry for both fighters (`cargo::escape`).
    pub shoulder_escape: bool,
    /// The carrier's ground_or_air as this shouldered fighter's animation
    /// callback reads it (the scene offers it before the proc).
    pub carrier_airborne: bool,
    pub capture_geometry: super::grab_throw::CaptureGeometry,
    pub thrown_pose: Option<super::grab_throw::ThrownPose>,
    pub grab: Option<super::grab::GrabLink>,
    pub hitlag_remaining: f32,
    /// x2219_b7 and a freeze held past the countdown (hitlag_link).
    pub hitlag_link: super::hitlag_link::HitlagLink,
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
    /// This frame's strongest phantom contact is electric: ftColl_8007A06C
    /// (retail 8007AAF4..8007AB08) writes x1960 = PlCo +1A4 for the phantom
    /// log as well, whatever branch ProcessHit then takes.
    pub phantom_electric: bool,
    /// dmg.x1870..x1898: the last resolved phantom contact, applied when its
    /// hitlag lockout expires without an ordinary hit (ftColl_8007BE3C).
    pub phantom: Option<PhantomHit>,
    /// dmg.x189C: phantom hitlag frames still to run; blocks new phantoms.
    pub phantom_lockout: f32,
    /// The hit ftColl_8007AB48 selected from `hit_log`, with its knockback.
    pub pending: Option<ReceivedHit>,
    /// dmg.x18C4 / x18C8: who is credited with this fighter's next fall.
    pub ko_source: super::ko_source::KoSource,
    /// A phantom's source (a fighter, or an item's owner) to credit (stale
    /// moves, combo) after this fighter's ProcessHit applied the phantom's
    /// damage (ftColl_8007BE3C).
    pub pending_credit: Option<HitSource>,
    /// ftColl_8007A06C: the selected hit came from this captured fighter's captor.
    pub pending_from_captor: bool,
    /// ftCo_8008EC90 inlineB1 for a captured fighter whose hit came from
    /// someone other than its captor but dealt under PlCo +3C0 this frame:
    /// the hit is taken as capture damage (set by `resolve_linked_hit`).
    pub light_capture_hit: bool,
    /// x1828: the grab partner's ftCo_8008EC90 order for this ProcessHit.
    pub pair_order: Option<super::grab_damage::PairHitOrder>,
    /// The captor (spawn number) that ftCo_800DE2F0 launches once this
    /// fighter's ProcessHit has launched it out of that captor's grab.
    pub release_captor: Option<u32>,
    /// Fighter.dmg.x1908 / x190C: the hit sound and voice set queued by the
    /// launch calculation, played by the next hit proc that starts no hitlag
    /// (Fighter_ProcessHit's else branch -> ftCo_80090718).
    pub queued_hit_sfx: Option<u32>,
    pub queued_voice: Option<DamageVoice>,
    /// dmg.x1914: the largest damage this fighter's hits dealt to fighters
    /// this frame (deal_dmg_cb's trigger).
    pub dealt_damage: i32,
    /// A Falcon Dive captor changed to its throw this tick; the scene then
    /// runs ftCo_800DDDE4 / ftCo_800DE7C0 on the pair.
    pub special_throw_release: bool,
    /// What a Koopa Klaw captor's callback asked of the fighter it holds
    /// this proc; the scene runs it on the pair.
    pub koopa_request: Option<super::capture_koopa::CaptorRequest>,
    /// unk_gobj / x221C_b5: the fighter one of this fighter's inert
    /// hitboxes touched this frame (ftColl_80078C70), cleared by ProcessHit.
    pub detected: Option<InertTouch>,
    /// dmg.x1924: the largest damage its hits dealt to shields this frame;
    /// hitlag falls back to it when x1914 is zero.
    pub shield_recoil_damage: i32,
    pub clank: super::clank::Pending,
    pub reflection: Option<super::reflection::Pending>,
    pub reflector_enabled: bool,
    /// x2218_b6, absorb_hit and AbsorbAttr: an absorbing bubble.
    pub absorb: super::absorb::AbsorbState,
    /// Fighter x44_mtx while x34_scale.z is not 1 (a flat fighter):
    /// Fighter_UnkApplyTransformation_8006C0F0's matrix, which takes the
    /// flattened bones back to full depth for the hurt, shield, reflect
    /// and absorb tests (ftCommon_8007F804).
    pub flat_matrix: Option<hsd_types::Mtx>,
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
    /// Egg Lay's swallow, for the scene to apply to the captured fighter.
    pub capture_requests: super::capture_yoshi::CaptureRequests,
}
/// unk_gobj and x221C_b5 as hurtbox_detect_cb reads them (doAirEnd0,
/// retail 8011C440..84: the x221C bit, then unk_gobj's kind at +4 and
/// motion at +10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InertTouch {
    /// unk_gobj: the last object an inert hitbox touched this frame.
    pub target: InertTarget,
    /// x221C_b5: some touch this frame was on a shield volume; sticky.
    pub shield: bool,
}
/// What unk_gobj points at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InertTarget {
    /// A fighter's hurtbox or shield (ftColl_80078C70).
    Fighter {
        /// The touched fighter's spawn number.
        spawn_number: u32,
        kind: FighterKind,
        action: super::ActionId,
    },
    /// An item's hurt capsule (it_802703E8, itcoll.c:482).
    Item { kind: melee_types::ItemKind },
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
    /// mv.damage.x19: the surface of the last bounce (reset per launch).
    pub last_bounce: Option<super::fly_reflect::BounceSurface>,
    /// mv.damage.x18: frames before another bounce or wall tech (a byte).
    pub bounce_lock: u8,
    /// mv.damage.x1A/x1B: a downward launch (a meteor) and the frames left
    /// before a jump or up special may cancel it.
    pub meteor_cancel: Option<u8>,
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
    /// PlCo +648..+654: a cape's turnaround.
    pub cape_turn: super::cape_turn::CapeTurnParameters,
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
    /// PlCo +428 (int): a prone fighter hit for less damage stays down
    /// (ftCo_8009F0F0).
    pub down_damage_limit: i32,
    pub crouch_hitlag_scale: f32,
    pub electric_hitlag_scale: f32,
    /// PlCo +128: the share of a third party's damage a captured fighter
    /// takes (ftColl_80076ED8 inlineB3; items, ftColl_80077C60).
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
    /// PlCo +1B0: knockback speed toward a wall or ceiling that bounces a
    /// launched fighter off it (ftCo_800C15F4, ftCo_800C17CC).
    pub fly_reflect_speed: f32,
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
    /// PlCo +170 / +174: scaled knockback at which an airborne tumble launch
    /// shakes the camera with a medium / large quake (ftCo_Damage.c block_75).
    pub medium_quake_threshold: f32,
    pub large_quake_threshold: f32,
    /// PlCo +23C (int) / +240: percent floor and Randf chance for DamageFlyRoll.
    pub fly_roll_percent: i32,
    pub fly_roll_chance: f32,
    /// PlCo +7E8/+7EC/+7F0: launch angles (degrees) that are meteors, and
    /// the frames before one may be cancelled (ftColl_8007AC68).
    pub meteor_angles: [u32; 2],
    pub meteor_cancel_frames: i32,
    /// PlCo +418: a hit of this much damage always knocks a held light item
    /// loose (Fighter_8006CDA4 draws Randi(+418) < damage).
    pub item_drop_range: i32,
    /// PlCo +414: the frames ftCo_800D705C's catch window stays open.
    pub catch_window_frames: i32,
    pub grounded_angle_threshold: f32,
    pub sakurai_air_angle: f32,
    pub sakurai_ground_angle: f32,
    pub sakurai_maximum_threshold: f32,
    pub air_decay: f32,
    /// PlCo +0x3E8: per-frame air decay of an attacker's shield recoil.
    pub shield_recoil_air_decay: f32,
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
            cape_turn: super::cape_turn::CapeTurnParameters {
                frames: r.s32(p + 0x648)?,
                air_speed: r.f32(p + 0x64C)?,
                ground_speed: r.f32(p + 0x650)?,
                shield_speed: r.f32(p + 0x654)?,
            },
            jump_buffer_window: r.f32(p + 0x1D0)?,
            knockback_replace_window: r.s32(p + 0xFC)?,
            air_cancel_window: r.s32(p + 0x18C)?,
            air_cancel_scale: r.f32(p + 0x190)?,
            crouch_knockback_scale: r.f32(p + 0x124)?,
            frozen_knockback_scale: r.f32(p + 0x718)?,
            smash_charge_knockback_scale: r.f32(p + 0x7C4)?,
            minimum_knockback: r.f32(p + 0x104)?,
            down_damage_limit: r.s32(p + 0x428)?,
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
            fly_reflect_speed: r.f32(p + 0x1B0)?,
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
            medium_quake_threshold: r.f32(p + 0x170)?,
            large_quake_threshold: r.f32(p + 0x174)?,
            fly_roll_percent: r.s32(p + 0x23C)?,
            fly_roll_chance: r.f32(p + 0x240)?,
            meteor_angles: [r.u32(p + 0x7E8)?, r.u32(p + 0x7EC)?],
            meteor_cancel_frames: r.s32(p + 0x7F0)?,
            item_drop_range: r.s32(p + 0x418)?,
            catch_window_frames: r.s32(p + 0x414)?,
            reaction_thresholds: [r.f32(p + 0x158)?, r.f32(p + 0x15c)?, r.f32(p + 0x160)?],
            grounded_angle_threshold: r.f32(p + 0x14c)?,
            sakurai_air_angle: r.f32(p + 0x144)?,
            sakurai_ground_angle: r.f32(p + 0x148)?,
            sakurai_maximum_threshold: r.f32(p + 0x150)?,
            air_decay: r.f32(p + 0x204)?,
            shield_recoil_air_decay: r.f32(p + 0x3E8)?,
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
    /// ftColl_8007AC68 (8007AC68): a fixed launch angle inside PlCo's meteor
    /// range.
    fn is_meteor(&self, angle: u16) -> bool {
        let angle = u32::from(angle);
        angle != 361 && self.meteor_angles[0] <= angle && angle <= self.meteor_angles[1]
    }
    /// ftCo_Damage_CalcAngle (8008D7F0): the 361-degree sentinel interpolates on ground.
    pub(super) fn launch_angle(&self, angle: u16, knockback: f32, ground: GroundOrAir) -> f32 {
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
    /// ftCommon_CalcHitlag (8007DA74) in full, before any caller's clamps:
    /// 8007DAA8 fmadds and an integer conversion, 8007DACC fmuls by the
    /// vibration multiplier (x1960) and another integer conversion, then for
    /// a crouching fighter (Squat, SquatWait) 8007DAF8 fmuls and fctiwz.
    pub fn frozen_frames(&self, damage: i32, multiplier: f32, crouching: bool) -> f32 {
        let base = fctiwz(fmadds(damage as f32, self.hitlag_scale, self.hitlag_base)) as f32;
        let hitlag = fctiwz(base * multiplier) as f32;
        if crouching {
            fctiwz(hitlag * self.crouch_hitlag_scale) as f32
        } else {
            hitlag
        }
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
    attacker_thrower_player: Option<u8>,
    rng: &mut gekko_math::HsdRng,
) {
    if victim.core.status.disabled
        || attacker.core.status.disabled
        || attacker.core.commands.thrown_by == Some(victim.core.spawn_number)
    {
        return;
    }
    // ftcoll.c:1664-1676: a thrown attacker hits anyone but its grabber's
    // player (grabber_unk1, ftColl_8007B8CC); any other attacker hits only
    // other players, so Popo and Nana never hit each other.
    let excluded_player = attacker_thrower_player.unwrap_or(attacker.core.player.id);
    if victim.core.player.id == excluded_player {
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
                rng,
            )
        {
            continue;
        }
        if let Some(contact) = victim.character.table().defense_contact {
            if contact(victim, attacker, assets, id) {
                continue;
            }
        }
        // ftColl_8007A06C: the hit's source is the fighter its log entry
        // names (`hit_owner`), a thrown fighter's thrower.
        let credit = HitCredit::Player(excluded_player);
        detect_eligible_hit(&mut victim.core, &mut attacker.core, assets, id, credit);
    }
}
/// A character's shield volume as its bone places it this frame.
#[derive(Clone, Copy, Debug)]
pub struct DefenseVolume {
    pub position: Vec3,
    /// The volume bone's world matrix.
    pub matrix: hsd_types::Mtx,
    pub radius: f32,
    /// x221B_b1: a counter's volume, off which a bouncing item leaves at
    /// PlCo +2D0 degrees whatever the geometry (ftColl_80077688).
    pub fixed_bounce: bool,
    /// x221B_b2: nothing bounces off the volume (ftColl_80077688 clears the
    /// item's xDCE b5; Link's Hylian shield).
    pub no_bounce: bool,
}

/// getEnvDmg (ftcoll.c:205, inlined): zero stays zero; nonzero values which
/// truncate to zero become one. Negative integers retain their sign.
fn environment_damage(damage: f32) -> i32 {
    let integer = fctiwz(damage);
    if damage == 0.0 {
        0
    } else if integer == 0 {
        1
    } else {
        integer
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
    // ftColl_80076CBC (80076CBC): shield contact wins over hurtboxes; the
    // frame's strongest impact (x19A4, from zero) keeps its source.
    let damage = environment_damage(desc.damage);
    let facing = if victim.physics.position.x > attacker.physics.position.x {
        -1.0
    } else {
        1.0
    };
    if damage
        > victim
            .shield
            .impact
            .as_ref()
            .map_or(0, |impact| impact.damage)
    {
        victim.shield.impact = Some(super::shield::ShieldImpact {
            damage,
            facing,
            element: desc.element,
        });
    }
    attacker.record_shield_recoil(damage, victim.shield.lightshield, -facing);
    let group = desc.group;
    melee_coll::detection::record_victim(
        &mut attacker.commands.hitboxes,
        group,
        victim.spawn_number,
    );
    if victim.shield_contact_feedback(damage, desc.shield_damage, contact.position) {
        // ftCo_80094138: permit attacks during GuardOff and clear minimum hold.
        victim.guard().interrupt_frames = assets.shield.powershield_interrupt_frames;
        victim.guard().minimum_hold = 0.0;
    }
}

impl FighterCore {
    /// ftColl_80076CBC (80076CBC), ftcoll.c:487-501: the defender's side of
    /// a hit on a shield-like volume (Guard or a counter's ftColl_8007B1B8
    /// volume). Returns whether the powershield window (x221C_b2) was open;
    /// the caller then applies ftCo_80094138 to its own scratch. The flag
    /// only counts down in Guard states, so it outlives the shield.
    pub fn shield_contact_feedback(
        &mut self,
        damage: i32,
        shield_damage: i8,
        position: Vec3,
    ) -> bool {
        if !self.shield.powershield_window {
            self.shield.damage_taken += (damage + i32::from(shield_damage)).max(0);
            self.effects
                .push(melee_ef::request::EffectRequest::ShieldSpark { position });
            return false;
        }
        self.commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 118,
                duration: 0,
            });
        self.commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: 104,
                volume: 127,
                pan: 64,
            });
        self.effects
            .push(melee_ef::request::EffectRequest::PowershieldSpark { position });
        true
    }
}

impl Fighter {
    /// ftCo_Damage_Phys (8008FB18): gravity/friction during hitstun, drift after it.
    pub(super) fn damage_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
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
        // ftCo_Damage_Phys (8008FB18) tests x221C_b6, which DownDamage's
        // animation never clears, not the remaining hitstun.
        if self.core.status.in_hitstun {
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
        self.core
            .set_part_rotation(bone, super::part_rotation::Axis::X, angle);
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
                if self.core.slipped_off_back_edge() {
                    self.enter_missed_footing(assets)?;
                } else {
                    self.leave_ground();
                }
            }
            return Ok(());
        }
        let landed = self.land_from_damage_air(assets, map)?;
        if landed != Some(true) && self.fly_surface_contact(assets, map)? {
            return Ok(());
        }
        let Some(landed) = landed else {
            return Ok(());
        };
        if landed {
            if is_tumble(self.core.motion_state.id) || self.core.motion_state.id == S::DamageFall {
                return self.tumble_landing(assets);
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

    /// ftCo_80090184 (80090184): a tumbling fighter lands in a tech or a
    /// DownBound.
    pub(super) fn tumble_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.try_tech(assets)? {
            return Ok(());
        }
        self.enter_down_bound(assets)
    }

    /// ftCo_DamageFly_Coll (8008FFC0) / ftCo_DamageFlyRoll_Coll: without a
    /// landing, touching a wall or ceiling offers a wall tech (ftCo_800C1D38)
    /// or ceiling tech (ftCo_800C23A0) inside the tech window, else a bounce
    /// (ftCo_800C15F4 / ftCo_800C17CC) when the knockback drives into the
    /// surface faster than PlCo x1B0. The ceiling tech and the bounces are
    /// not ported: a contact that would take one fails closed. mv.damage.x19
    /// (the last bounced surface) stays unset because no bounce has run.
    fn fly_surface_contact(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<bool> {
        use melee_types::mp::collide::{CEILING_HUG, LEFT_WALL_HUG, RIGHT_WALL_HUG};
        let flying = matches!(
            self.core.motion_state.id,
            S::DamageFlyHi | S::DamageFlyN | S::DamageFlyLw | S::DamageFlyTop | S::DamageFlyRoll
        );
        let env = self.core.collision.data.env_flags as u32;
        if !flying || env & (RIGHT_WALL_HUG | LEFT_WALL_HUG | CEILING_HUG) == 0 {
            return Ok(false);
        }
        if self.try_wall_tech(assets, map)? {
            return Ok(true);
        }
        if self.try_ceiling_tech(assets, map)? {
            return Ok(true);
        }
        // ftCo_800C17CC (800C17CC): the wall bounce, then the ceiling's.
        if self.try_wall_bounce(assets, map)? {
            return Ok(true);
        }
        self.try_ceiling_bounce(assets, map)
    }

    /// ft_80081DD4 (80081DD4): the airborne damage collision. During hitlag
    /// mpColl_800477E0 only clamps SDI against the floor; otherwise the ledge
    /// snap height is scaled for the pass and the result says whether the
    /// fighter touched down (`None`: the hitlag clamp ran, nothing else may).
    pub(super) fn land_from_damage_air(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<Option<bool>> {
        crate::collision::air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        Ok(self.damage_air_pass(assets, map))
    }

    /// ft_80081DD4's pass itself, for callers already inside a map proc
    /// (ftCo_80090574 from the ceiling tech).
    pub(super) fn damage_air_pass(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Option<bool> {
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
            return None;
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
        Some(landed)
    }

    pub(super) fn process_damage(
        &mut self,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        let crouching = matches!(self.core.motion_state.id, S::Squat | S::SquatWait);
        let dealt_damage = std::mem::take(&mut self.core.combat.dealt_damage);
        let shield_recoil_damage = std::mem::take(&mut self.core.combat.shield_recoil_damage);
        // Fighter_ProcessHit, fighter.c:2928-2941: x1914 before x1924.
        let mut hit_damage = if dealt_damage != 0 {
            dealt_damage
        } else {
            shield_recoil_damage
        };
        let detected = self.core.combat.detected.take();
        let clank = self.core.combat.clank;
        self.core.combat.clank.damage = 0;
        self.core.combat.clank.duration = 0.0;
        // +1920 facing is retained; only +1918/+191C reset at the retail tail.
        let reflection = self.core.combat.reflection.take();
        let absorbed = self.core.take_absorbed();
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
        // x1960_vibrateMult: 1.0 unless a log resolved this frame chose an
        // electric hit (fighter.c:3039 resets it after every ProcessHit).
        let mut hitlag_multiplier = if std::mem::take(&mut self.core.combat.phantom_electric) {
            assets.damage.electric_hitlag_scale
        } else {
            1.0
        };
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
        // fighter.c:3019: x1828 is consumed by this ProcessHit.
        let pair_order = self.core.combat.pair_order.take();
        let light_capture_hit = std::mem::take(&mut self.core.combat.light_capture_hit);
        if let Some(hit) = self.core.combat.pending.take() {
            if self.core.motion_state.id == S::CaptureYoshi {
                unimplemented!("ftCo_8008EC90: a hit on Egg Lay's catch");
            }
            // ftColl_8007A06C: only the electric-hit victim gets x1960 = PlCo +1A4.
            if hit.descriptor.element == melee_types::HitElement::Electric {
                hitlag_multiplier = assets.damage.electric_hitlag_scale;
            }
            if hit.knockback != 0.0 {
                // fighter.c:2862: take_dmg_2_cb, before the reaction.
                if let Some(hit_taken) = self.character.table().hit_taken {
                    hit_taken(self);
                }
            }
            if self.core.motion_state.id == S::YoshiEgg && hit.knockback != 0.0 {
                // take_dmg_2_cb = ftCo_800BC3D0 sets x1828 = 4, which no
                // reaction case takes: the egg keeps rolling into hitlag.
                super::capture_yoshi::egg_hit(&mut self.core, &hit);
                hit_damage = self.core.combat.frame_max_damage;
            } else if pair_order == Some(super::grab_damage::PairHitOrder::Launch)
                && hit.knockback != 0.0
            {
                self.core.combat.pending_from_captor = false;
                self.launch_by_pair_order(hit, assets, rng)?;
                hit_damage = self.core.combat.frame_max_damage;
            } else if (self.core.combat.cape_turn.turning || self.core.status.no_hit_reaction)
                && hit.knockback != 0.0
            {
                // ftCo_8008EC90 (ftCo_Damage.c:844): x2220_b4, a cape turn in
                // progress, takes inlineB2 before any other branch: the
                // damage without a reaction, even for a second cape.
                let knockback = self.core.modified_knockback(hit.knockback, assets);
                self.core.take_hit_while_turning(&hit, knockback, assets);
                hit_damage = self.core.combat.frame_max_damage;
            } else if std::mem::take(&mut self.core.combat.pending_from_captor) || light_capture_hit
            {
                // ftCo_8008EC90: the captor's hit, or any light hit (inlineB1),
                // preserves the grab and shares hitlag.
                hit_damage = super::grab_escape::capture_damage(self, &hit, assets)?;
            } else if self.core.modified_knockback(hit.knockback, assets) == 0.0 {
                // ftCo_8008EC90 (8008ECB0): armor left no knockback.
                if hit.knockback != 0.0 {
                    self.core.absorb_hit(&hit, assets);
                    hit_damage = self.core.combat.frame_max_damage;
                }
            } else {
                // ftCo_8009F0F0 -> ftCo_8009F184: a light hit on a prone fighter
                // keeps its facing (ftCo_8008DCE0's argument is fp->facing_dir,
                // applied after the knockback used the hit's direction)
                // and skips ftCommon_8007DB58, which only the ordinary branch calls.
                let element = hit.descriptor.element;
                if element == melee_types::HitElement::Cape {
                    // Fighter_UnkTakeDamage_8006CC30, then ftCo_8008EC90's
                    // cape branch: ftCo_800C3538's ftCo_800C3598, or, when it
                    // is blocked, the ordinary reaction facing as before and
                    // ftCo_800C3598 at ret_A8C.
                    let knockback = self.core.modified_knockback(hit.knockback, assets);
                    if self.cape_turn_blocked() {
                        assert!(
                            self.core.combat.grab.is_none(),
                            "ftCo_8008EC90: a blocked cape hit on a grab pair member"
                        );
                        let facing = self.core.physics.facing;
                        self.interrupt_actions(assets);
                        self.core.reserve_item_launch_draws(rng);
                        self.core.start_ledge_cooldown_if_hanging(assets);
                        self.begin_damage_reaction(
                            hit.clone(),
                            None,
                            Some(facing),
                            None,
                            false,
                            assets,
                            rng,
                        )?;
                    } else {
                        self.core.physics.percent += hit.percent_damage;
                    }
                    self.core.cape_turn(&hit, knockback, assets);
                } else {
                    let down = self.core.down_damage_state(hit.percent_damage, assets);
                    if down.is_none()
                        && matches!(
                            element,
                            melee_types::HitElement::Nap | melee_types::HitElement::Sleep
                        )
                        && !self.core.status.ledge_grab_disabled
                    {
                        // ftCommon_8007DB58, then ftCo_8008E908's sleep branch
                        // (ftCo_Damage.c:676-677): asleep, not launched.
                        self.interrupt_actions(assets);
                        self.core.start_ledge_cooldown_if_hanging(assets);
                        self.enter_damage_song(element == melee_types::HitElement::Sleep, assets)?;
                    } else if down.is_none() && self.bury_hit(element) {
                        // ftCo_800C0CB8 (ftCo_Damage.c:952), after the prone
                        // check: buried rather than launched.
                        self.enter_bury(hit, assets, rng)?;
                    } else {
                        let facing = if down.is_some() {
                            Some(self.core.physics.facing)
                        } else {
                        self.interrupt_actions(assets);
                        self.core.reserve_item_launch_draws(rng);
                            self.core.start_ledge_cooldown_if_hanging(assets);
                            None
                        };
                        self.begin_damage_reaction(hit, down, facing, None, false, assets, rng)?;
                        if down.is_some() {
                            self.core.status.grab_exclusions = super::ledge::GrabExclusions(1);
                        }
                    }
                }
                // fighter.c:2888: hitlag uses dmg.x183C_applied, the largest
                // damage logged this frame.
                hit_damage = self.core.combat.frame_max_damage;
            }
        }
        // fighter.c:2907: without knockback, a resolved phantom contact
        // (dmg.x18a0) takes the hitlag branch with its halved damage (x1840).
        let phantom_hitlag = !received_knockback && self.core.combat.phantom_knockback != 0.0;
        if phantom_hitlag {
            // fighter.c:2907-2918: the phantom branch precedes x19A4's, so a
            // shield impact the same frame gets no response (no stun, push or
            // ShieldBreak); its health loss already applied (fighter.c:2816).
            self.core.shield.impact = None;
            hit_damage = self.core.combat.phantom_max_damage;
        }
        // fighter.c:2956-2963: damage without knockback (zero-knockback hits,
        // an expired phantom) still reaches percent.
        if !received_knockback && self.core.combat.frame_damage != 0.0 {
            self.core.physics.percent += self.core.combat.frame_damage;
            // ftCommon_800804FC (fighter.c:2961).
            let ground_or_air = self.core.physics.ground_or_air;
            self.core.combat.ko_source.clear_if_grounded(ground_or_air);
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
        // fighter.c:2927-2931: dmg.x1914 (damage this fighter's hit dealt)
        // runs deal_dmg_cb when nothing received, no shield impact and no
        // clank came first.
        if !received_knockback
            && !phantom_hitlag
            && self.core.shield.impact.is_none()
            && clank.damage == 0
            && dealt_damage != 0
        {
            if let Some(deal_damage) = self.character.table().deal_damage {
                deal_damage(self, assets);
            }
        }
        if !received_knockback
            && !phantom_hitlag
            && self.core.shield.impact.is_none()
            && hit_damage == 0
        {
            if let Some(reflection) = reflection {
                self.process_reflection(reflection, assets)?;
            } else if let Some(absorbed) = absorbed {
                // fighter.c:2947-2950: ftData_OnAbsorb.
                if let Some(on_absorb) = self.character.table().on_absorb {
                    on_absorb(self, assets, absorbed);
                }
            } else if let Some(touch) = detected {
                // fighter.c:2950-2954: the last branch, an inert touch.
                if let Some(detect) = self.character.table().hurtbox_detect {
                    detect(self, assets, touch);
                }
            }
        }
        if hit_damage == 0 {
            // Fighter_ProcessHit, fighter.c:2990: no hitlag started this frame, so
            // ftCo_80090718 plays whatever the previous hit queued.
            self.core.play_queued_damage_sounds(assets, rng);
        }
        if hit_damage != 0 {
            let hitlag = assets
                .damage
                .frozen_frames(hit_damage, hitlag_multiplier, crouching);
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
    /// ftCommon_8007DB58 (8007DB58), before a damage, rebound or capture
    /// entry: stop the action and override-voice sound handles, then run the
    /// character's take-damage hook (Fox and Falco put the Blaster away).
    /// No supported character has a death1 hook.
    pub fn interrupt_actions(&mut self, assets: &FighterAssets) {
        for channel in [
            super::commands::SoundChannel::StopAction,
            super::commands::SoundChannel::StopOverrideVoice,
        ] {
            self.core
                .commands
                .footstep_sounds
                .push(super::commands::FootstepSound {
                    channel,
                    id: 0x83D61,
                    volume: 0,
                    pan: 64,
                });
        }
        if self.core.swing_hand_armed {
            // take_dmg_cb = ft_800CD31C (retail 0x8007DB98 blrl ->
            // 0x800CD33C bl ftCommon_8007E7E4): the hand takes the held item
            // again before the damage entry.
            self.swing_restore_hand(assets);
        }
        if let Some(take_damage) = self.character.table().take_damage {
            take_damage(self);
        }
    }
    /// ftCo_8008DCE0 (8008DCE0): launch and enter the strength/height reaction.
    /// `percent_pending`: the damage went to x1838_percentTemp
    /// (ftColl_80076640, every ftCo_800DDDE4 throw release, with or without
    /// a throw owner) and reaches x1830_percent only in Fighter_ProcessHit.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin_damage_reaction(
        &mut self,
        mut hit: ReceivedHit,
        forced_motion: Option<S>,
        facing: Option<f32>,
        throw_owner: Option<u32>,
        percent_pending: bool,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<i32> {
        hit.knockback = self.core.modified_knockback(hit.knockback, assets);
        let (state, stun) =
            self.core
                .prepare_damage_reaction(&hit, forced_motion, percent_pending, assets, rng);
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
        // ftCo_8008DCE0 block_42: a nonzero facing argument replaces the
        // hit's facing once the knockback has been computed.
        if let Some(facing) = facing {
            self.core.physics.facing = facing;
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
                // efAsync_Spawn(1032) queues behind the graphics commands
                // already issued and still unresolved: a thrower's anim proc
                // (Falcon Dive / Dark Dive's doCatchAnim) can enter this
                // motion earlier in the same tick. (What
                // push_effect_after_issued_graphics does, on disjoint fields.)
                let issued =
                    self.core.commands.graphics.len() + self.core.commands.landing_effects.len();
                self.core.effects.push_after_graphics(
                    melee_ef::request::EffectRequest::DamageTrail { trajectory },
                    issued,
                );
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
                // ftCo_800CB870: a multijumper's later jumps need held X/Y.
                if self.aerial_jump_requested(assets) {
                    return self.enter_aerial_jump(assets);
                }
            }
            if is_tumble(self.core.motion_state.id) {
                // ftCo_DamageFly_Anim (8008FD98) -> ftCo_80090780: DamageFall
                // keeps fast fall, clamps drift and rumbles, even from ground.
                return self.enter_damage_fall(assets);
            }
            self.change_motion_state(
                (if self.core.physics.ground_or_air == GroundOrAir::Air {
                    S::Fall
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
        // A damage state is neither Jump nor JumpAerial, so ftCo_Damage_IASA's
        // ftCo_Fall_IASA_Inner always checks float (Peach). DamageFly and
        // DamageFall use ftCo_DamageFall_IASA instead, which never calls
        // ftPe_8011BA54 / ftPe_8011BAD8 (retail 80090828..8009091C).
        let vertical_velocity = self.core.physics.self_velocity.y;
        // ftCo_80095328, ftCo_800D7100, then ftCo_800C3B10, after the
        // special check.
        let special = self.air_special_pressed(assets);
        if !special
            && (self.try_air_item_throw(assets)?
                || self.try_aerial_item_catch(assets)
                || self.try_air_tether(assets))
        {
            return Ok(());
        }
        // ftCo_800CB870: a multijumper's later jumps take held X/Y
        // (ftCo_800D730C), not the press ftCo_Damage_IASA synthesizes.
        let jump = self.aerial_jump_requested(assets);
        let transition = super::fall::iasa_with_jump(
            &self.core.input,
            &assets.input,
            special,
            jump,
            // DamageFly delegates to DamageFall (80090828), whose
            // input chain omits ordinary Fall's air-dodge check.
            !tumbling,
            |phase| {
                !tumbling
                    && self.character.check_float_input(
                        &self.core.input,
                        assets,
                        vertical_velocity,
                        phase,
                    )
            },
        );
        match transition {
            T::None => {
                // ftCo_DamageFall_IASA (0x800908C4); UCF hooks the age
                // compare (0x800908F4).
                if tumbling
                    && gekko_math::msl::fabsf(self.core.input.current.stick.x)
                        >= assets.damage.tumble_exit_threshold
                    && crate::input::controller_fix::tumble_wiggle(
                        &self.core.input,
                        assets.damage.tumble_exit_threshold,
                        assets.damage.tumble_exit_window,
                    )
                {
                    self.change_motion_state(S::Fall.into(), assets)?;
                }
                Ok(())
            }
            T::Attack => (self.character.table().enter_aerial)(self, assets),
            T::Jump => self.enter_aerial_jump(assets),
            T::Escape => self.enter_air_dodge(assets),
            T::Float => (self.character.table().enter_float)(self, assets),
            T::AirSpecial => {
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
    pub(super) fn damage_fall_physics(&mut self, assets: &FighterAssets, wind: Wind) {
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
        } else {
            if self.try_meteor_cancel(assets)? {
                return Ok(());
            }
            let MotionData::Damage(damage) = &mut self.core.state_data else {
                unreachable!()
            };
            if crate::input::human::jump_input(&self.core.input, &assets.input) {
                // doIasa (8008F938): record the remaining hitstun, not the input age.
                damage.jump_buffer = damage.hitstun;
            }
        }
        Ok(())
    }

    /// doIasa (8008F938): once a meteor's countdown ends, an airborne fighter
    /// still driven down cancels it with an up special (ftCo_800D69C4) or an
    /// aerial jump (ftCo_800CB8E0); the knockback stops.
    fn try_meteor_cancel(&mut self, assets: &FighterAssets) -> Result<bool> {
        let MotionData::Damage(damage) = &mut self.core.state_data else {
            panic!("damage scratch missing")
        };
        let Some(frames) = &mut damage.meteor_cancel else {
            return Ok(false);
        };
        *frames = frames.saturating_sub(1);
        if *frames != 0
            || self.core.physics.ground_or_air != GroundOrAir::Air
            || self.core.physics.knockback_velocity.y >= 0.0
        {
            return Ok(false);
        }
        let input = &self.core.input;
        let lockout = assets.damage.tech_lockout;
        // ftCo_800D69C4: a fresh up special, not within PlCo +1C of the last.
        let up_special = self.core.capabilities.specials[1]
            && input.buttons.special_up == 0
            && i32::from(input.buttons.previous_special_up) >= lockout;
        // ft_did_jump(fp, true): a jump left, a tap or X/Y, and not a second
        // press within PlCo +1C of the last.
        // ftCo_800CB8E0 (800CB8E0): a multi-jump fighter (can_multijump)
        // takes ftCo_800D730C(gobj, true) instead, whose later jumps need
        // only a held stick or X/Y.
        let jump = !up_special
            && if self.character.multi_jump_attributes().is_some() {
                self.aerial_jump_requested_after(assets, Some(lockout))
            } else {
                i32::from(self.core.physics.jumps_used) < self.core.attributes.jumping.max_jumps
                    && crate::input::human::jump_input(input, &assets.input)
                    && i32::from(input.buttons.previous_jump) >= lockout
            };
        if up_special {
            (self.character.table().enter_special)(self, super::SpecialSlot::Up, true, assets);
        } else if jump {
            self.enter_aerial_jump(assets)?;
        } else {
            return Ok(false);
        }
        self.core.physics.knockback_velocity = Vec3::ZERO;
        self.core
            .commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: 12,
                duration: 0,
            });
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 121,
                duration: 0,
            });
        Ok(true)
    }
}
impl FighterCore {
    /// The take-damage hook's articles exist in retail before the reaction
    /// draws (ftCommon_8007DB58 precedes ftCo_8008DCE0's draw at
    /// 0x8008E124), while the port creates them after the proc: the draws
    /// their launch makes leave the stream here, ahead of the reaction's.
    fn reserve_item_launch_draws(&mut self, rng: &mut gekko_math::HsdRng) {
        for request in self.item_requests.iter_mut() {
            if let melee_it::ItemRequest::Spawn(spawn) = request {
                spawn.reserve_launch_draws(rng);
            }
        }
    }
    /// ftCo_8008E908 (0x8008E908), before the reaction's motion change clears
    /// the flag: a fighter hit out of a ledge state (x221D_b7: CliffCatch,
    /// CliffWait and every ledge option) cannot grab a ledge again for PlCo's
    /// ledge cooldown (+498).
    fn start_ledge_cooldown_if_hanging(&mut self, assets: &FighterAssets) {
        if self.status.on_ledge {
            self.status.ledge_cooldown = assets.ledge.cooldown;
        }
    }
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
        if damage > self.combat.shield_recoil_damage {
            self.combat.shield_recoil_damage = damage;
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
        // UCF 0.84 hooks the vertical tap compare (0x8008E54C).
        if stick_magnitude_passes(stick, parameters.minimum_stick)
            && (i32::from(self.input.horizontal.tilt) < parameters.tap_window
                || crate::input::controller_fix::sdi_vertical_tap(
                    &self.input,
                    parameters.tap_window,
                    parameters.minimum_stick,
                ))
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
        // retail 800776DC..80077714.
        let damage = environment_damage(desc.damage);
        if damage > item.pending_damage_dealt {
            item.pending_shield_damage = damage;
            item.pending_shield_owner = Some(self.player.id);
            item.pending_shield_deflection = if item.hit_flags[id].shield_bounce {
                let volume = &self.shield.hit;
                let joint = self.animation.parts[volume.bone].joint;
                // ftcoll.c:2271: lbColl_80007DD8 takes ftCommon_8007F804, so
                // a flat fighter's bubble keeps its size (PSMTXConcat of
                // x44_mtx and the bone).
                let matrix = super::caches::unflattened(
                    self.combat.flat_matrix.as_ref(),
                    self.skeleton.get_mtx(joint),
                );
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
        // it_8026FAC4 with mode 2 (x41_b6, timed) or 1.
        if item.hit_flags[id].shield_rehit {
            item.record_timed_victim(group, self.spawn_number);
        } else {
            melee_coll::detection::record_victim(&mut item.hitboxes, group, self.spawn_number);
        }
        self.effects
            .push(melee_ef::request::EffectRequest::ShieldSpark {
                position: contact.position,
            });
    }

    /// ftColl_80077688's item half for a character shield volume such as
    /// Marth's Counter: the item records the contact, its shield bounce
    /// against the volume and the volume's own hitlag (`shield_unk1`, xCC0).
    /// Returns the contact's integer damage for the character's reaction.
    pub fn record_item_volume_hit(
        &mut self,
        item: &mut melee_it::ItemCore,
        id: usize,
        contact: Contact,
        volume: &DefenseVolume,
        own_hitlag: f32,
        assets: &FighterAssets,
    ) -> i32 {
        let hit = item.hitboxes[id].as_ref().expect("eligible item hit");
        let desc = &hit.descriptor;
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
            // inlineItemA1: xCF4 = the volume's fighter.
            item.pending_shield_owner = Some(self.player.id);
            let bounces = item.hit_flags[id].shield_bounce && !volume.no_bounce;
            item.pending_shield_deflection = bounces.then(|| {
                let deflection = melee_lb::shield::deflection(
                    volume.position,
                    &volume.matrix,
                    hit.previous_position,
                    hit.position,
                    volume.radius,
                    desc.radius * if desc.ignore_scale { 1.0 } else { item.scale },
                );
                if !volume.fixed_bounce {
                    return deflection;
                }
                // ftcoll.c:909-921: angle 0 and a fixed normal, mirrored by
                // the side the geometric normal points to.
                let radians = std::f32::consts::PI / 180.0 * assets.shield.counter_bounce_degrees;
                let cosine = gekko_math::msl::cosf(radians);
                melee_lb::shield::ShieldDeflection {
                    normal: Vec3::new(
                        if deflection.normal.x >= 0.0 {
                            cosine
                        } else {
                            -cosine
                        },
                        gekko_math::msl::sinf(radians),
                        0.0,
                    ),
                    angle: 0.0,
                }
            });
            if own_hitlag != 0.0 {
                item.pending_shield_hitlag = own_hitlag;
            }
        }
        let group = desc.group;
        // inlineItemA1: it_8026FAC4 with mode 4 (x41_b5, timed) or 3.
        if item.hit_flags[id].damage_without_hitlag {
            item.record_timed_victim(group, self.spawn_number);
        } else {
            melee_coll::detection::record_victim(&mut item.hitboxes, group, self.spawn_number);
        }
        self.effects
            .push(melee_ef::request::EffectRequest::ShieldSpark {
                position: contact.position,
            });
        damage
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
    pub fn finish_air_update(&mut self, assets: &FighterAssets, wind: Wind) {
        if self.physics.ground_or_air == GroundOrAir::Air {
            self.decay_air_knockback(assets);
        }
        crate::physics::integrate::integrate_velocity(&mut self.physics);
        crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
    }
    /// Fighter_procUpdate's air branch (8006B8F8..8006BB60): decay the hit
    /// knockback, then the attacker's shield recoil.
    pub fn decay_air_knockback(&mut self, assets: &FighterAssets) {
        let v = &mut self.physics.knockback_velocity;
        if v.x != 0.0 || v.y != 0.0 {
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
        self.decay_air_shield_recoil(assets.damage.shield_recoil_air_decay);
    }
    /// Fighter_procUpdate 8006BA5C..8006BB60: the attacker's shield recoil
    /// (x98) decays in the air like knockback.
    fn decay_air_shield_recoil(&mut self, decay: f32) {
        let recoil = self.physics.shield_knockback_velocity;
        if recoil.x == 0.0 && recoil.y == 0.0 {
            return;
        }
        let angle = melee_lb::trigf::atan2f(recoil.y, recoil.x);
        // retail 8006BAA8: fmadds, y product first.
        if sqrtf(fmadds(recoil.x, recoil.x, recoil.y * recoil.y)) < decay {
            // retail 8006BB14/8006BB18: the store meant for the recoil's y
            // clears the knockback's y instead (the invisible ceiling bug).
            self.physics.knockback_velocity.y = 0.0;
            self.physics.shield_knockback_velocity.x = 0.0;
        } else {
            let recoil = &mut self.physics.shield_knockback_velocity;
            // retail 8006BB34 / 8006BB50: fnmsubs.
            recoil.x = gekko_math::fma::fnmsubs(decay, cosf(angle), recoil.x);
            recoil.y = gekko_math::fma::fnmsubs(decay, sinf(angle), recoil.y);
        }
        self.physics.ground_shield_knockback_velocity = 0.0;
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
        // fighter.c:1398-1405: a cape's freeze (dmg.x1954) counts first; its
        // end with no hitlag left ends the frozen state.
        if self.tick_cape_freeze() && !self.combat.hitlag_link.held {
            self.end_hitlag();
        }
        if melee_coll::damage::tick_hitlag(&mut self.combat.hitlag_remaining)
            && self.combat.cape_turn.freeze <= 0.0
        {
            if matches!(self.state_data, MotionData::Guard(_)) {
                self.shield.allow_sdi = false;
            }
            // fighter.c:1421: x2219_b7 skips Fighter_8006D10C; the grab
            // partner's hitlag end releases this fighter (hitlag_link).
            if self.combat.hitlag_link.held {
                self.combat.hitlag_link.frozen = true;
                return;
            }
            self.end_hitlag();
        }
    }
    /// Fighter_8006D10C (8006D10C): post_hitlag_cb and x2219_b5 = 0.
    pub(super) fn end_hitlag(&mut self) {
        self.combat.hitlag_link.frozen = false;
        if self.combat.hitlag_callbacks == HitlagCallbacks::Damage {
            self.exit_damage_hitlag();
            self.status.interaction = Interaction::Damage;
        } else if matches!(self.state_data, MotionData::Guard(_)) {
            if self.combat.hitlag_callbacks == HitlagCallbacks::Guard {
                self.exit_guard_hitlag();
            }
            self.status.interaction = Interaction::Shield;
        } else if self.combat.grab.is_some() {
            // Pummel freezes both members of the pair without damage-state scratch.
            self.status.interaction = Interaction::Idle;
        } else if self.in_attack_state() {
            self.status.interaction = Interaction::Attack;
        } else {
            // An attacker's hitlag can outlast its attack (Illusion's end).
            self.status.interaction = Interaction::Idle;
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
    credit: HitCredit,
) {
    let hit = attacker.commands.hitboxes[id]
        .as_ref()
        .expect("eligible hitbox");
    let desc = &hit.descriptor;
    // ftColl_80078C70: x1988/x198C == 2 (retail 0x800790B4, 0x800790C0)
    // skip only the hurt-capsule loop. The shield test (0x80079074) comes
    // first, so an intangible fighter's shield (a ledgedash into Guard)
    // still takes the hit.
    let intangible = victim.commands.hurt_status == melee_types::combat::HurtStatus::Intangible
        || victim.status.ledge_intangibility != 0;
    let inert = desc.element == melee_types::HitElement::Inert;
    let mut shield_touch = false;
    if victim.shield.active {
        if let Some(contact) = victim.shield_contact(hit, attacker.player.scale) {
            if !inert {
                let descriptor = desc.clone();
                record_shield_hit(victim, attacker, &descriptor, contact, assets);
                return;
            }
            // ftColl_80078C70: an inert hitbox on a shield marks the touch
            // (x221C_b5, unk_gobj) and still tests the hurtboxes.
            shield_touch = true;
        }
    }
    let contact = if intangible {
        None
    } else {
        melee_coll::detection::first_hurt_contact(victim, hit, attacker.player.scale)
    };
    if inert {
        // ftColl_80078C70: an inert hitbox only records the touched
        // fighter; it logs no hit and never marks the victim on its group.
        if shield_touch || contact.is_some() {
            record_inert_touch(victim, attacker, shield_touch);
        }
        return;
    }
    if let Some((contact, height, capsule_status)) = contact {
        // Retail's hit path reads the victim's state only for DamageIce
        // (ftcoll.c:199/576/1155); crouch cancel (ftCo_Damage.c:124-127) and the
        // airborne launch states (ftCo_Damage.c:543-558) are applied by the
        // reaction in prepare_damage_reaction. Every other grounded victim state
        // takes the ordinary path.
        if victim.motion_state.id == S::DamageIce {
            unimplemented!("ftcoll.c:199: DamageIce victim");
        }
        // ftColl_80076ED8: x1988, x198C, x221D_b6 or the capsule's own state
        // (hit1->state) keep the hit out of the damage and phantom logs.
        let invincible = capsule_status == melee_types::combat::HurtStatus::Invincible
            || victim.commands.hurt_status != melee_types::combat::HurtStatus::Normal;
        if contact.overlap < assets.damage.phantom_threshold {
            log_phantom_contact(victim, attacker, id, contact, height, invincible, credit);
            return;
        }
        if victim.status.revival_invincibility != 0 || invincible {
            // ftColl_80078C70 inlineB3: record the contact and attacker hitlag,
            // but omit damage logging/staling while x198C selects invincibility.
            // ftColl_80076ED8: x1914 takes the scaled damage's count, zero
            // for a zero-damage hit (Sing), so no attacker hitlag then.
            let group = desc.group;
            let damage = desc.damage * victim.received_damage_scale();
            attacker.combat.dealt_damage = attacker
                .combat
                .dealt_damage
                .max(super::hit_log::damage_count(damage));
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
        let descriptor = desc.clone();
        // ftColl_80076ED8 inlineB3: a captured fighter takes PlCo +128 of a
        // third party's damage (fmuls); ftCo_8008EC90
        // (grab_damage::resolve_linked_hit) decides the reaction later. Then
        // the victim's damage scale (fmuls).
        let mut damage = descriptor.damage;
        if matches!(victim.combat.grab, Some(super::grab::GrabLink::Captured { captor }) if captor != attacker.spawn_number)
        {
            damage *= assets.damage.captured_item_damage_scale;
        }
        let damage = damage * victim.received_damage_scale();
        // ftColl_80076ED8's ordinary branch: log the hit for ftColl_8007AB48,
        // whose knockback and effects wait until every contact is logged.
        victim.combat.log_hit(LoggedHit {
            source: HitSource::Fighter(hit_owner(attacker)),
            credit,
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
                percent_damage: damage,
            },
            position: contact.position,
            knockback_damage: hit.knockback_damage,
            damage,
            // DmgLogEntry.x20: the hitbox's own damage.
            effect_damage: descriptor.damage,
        });
        attacker.combat.has_recorded_hit = true;
        attacker.credit_hit(victim.spawn_number, assets);
        // ftColl_80076ED8: x1914 keeps the largest damage dealt this frame.
        attacker.combat.dealt_damage = attacker
            .combat
            .dealt_damage
            .max(super::hit_log::damage_count(damage));
        melee_coll::detection::record_victim(
            &mut attacker.commands.hitboxes,
            descriptor.group,
            victim.spawn_number,
        );
    }
}

/// ftColl_80078C70 (ftcoll.c:1843-1845, 1954): unk_gobj becomes the touched
/// fighter; a shield touch also sets x221C_b5, which only ProcessHit clears.
fn record_inert_touch(victim: &FighterCore, attacker: &mut FighterCore, shield: bool) {
    let shield = shield || attacker.combat.detected.is_some_and(|touch| touch.shield);
    attacker.combat.detected = Some(InertTouch {
        target: InertTarget::Fighter {
            spawn_number: victim.spawn_number,
            kind: victim.kind,
            action: victim.motion_state.action,
        },
        shield,
    });
}

/// ftColl_8007A06C's item arm (ftcoll.c:2930-2952): an item credits its
/// owner when that is a fighter (ftLib_80086960). An ownerless one clears
/// the credit, unless its kind is one of pl_8003D60C's (0x8003D60C: the
/// random Pokemon, Whispy's apples, the Arwing and Great Fox lasers, Birdo's
/// egg and kind 0xED), which leave it alone.
fn item_credit(item: &melee_it::ItemCore) -> HitCredit {
    match item.owner {
        Some(player) => HitCredit::Player(player),
        None if matches!(i32::from(item.kind), 0xA0 | 0xE1 | 0xEA..=0xED) => HitCredit::Unchanged,
        None => HitCredit::Nobody,
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
    invincible_capsule: bool,
    credit: HitCredit,
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
    // ftColl_80076ED8 inlineB3: the victim's damage scale before halving.
    let damage = phantom_damage(descriptor.damage * victim.received_damage_scale());
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
        && !invincible_capsule
    {
        victim.combat.phantom_max_damage = victim.combat.phantom_max_damage.max(fctiwz(damage));
        victim.combat.phantom_log.push(LoggedHit {
            source: HitSource::Fighter(hit_owner(attacker)),
            credit,
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
    /// A throw release (ftCo_800DDDE4) only adds its damage to
    /// x1838_percentTemp (ftColl_80076640); the victim's ProcessHit applies it
    /// later, so this launch's fly-roll check still reads the old percent.
    /// Other hits the victim takes the same frame add to that sum (and read it
    /// for knockback) before ProcessHit applies it in one addition.
    fn prepare_damage_reaction(
        &mut self,
        hit: &ReceivedHit,
        forced_motion: Option<S>,
        percent_pending: bool,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> (S, f32) {
        self.maybe_drop_held_item(hit, assets, rng);
        let airborne = self.physics.ground_or_air == GroundOrAir::Air;
        let stun = hit.knockback * assets.damage.hitstun_scale;
        let base_level = assets
            .damage
            .reaction_thresholds
            .iter()
            .position(|&t| stun < t)
            .unwrap_or(3);
        // ftCo_Damage.c:331, 430, 536: a strong Ice hit bends the angle,
        // plays DamageIce and freezes the victim (ftCo_DamageIce_Init).
        if base_level >= 2 && hit.descriptor.element == melee_types::HitElement::Ice {
            unimplemented!("ftCo_8008DCE0: an Ice hit at knockback level {base_level} (DamageIce)");
        }
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
        if !percent_pending {
            self.physics.percent += hit.percent_damage;
        }
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
        if percent_pending {
            // ftColl_80076640 (called by ftCo_800DDDE4 before this launch):
            // x1838_percentTemp += damage, x183C_applied = max(its count).
            self.combat.frame_damage += hit.percent_damage;
            self.combat.frame_max_damage = self
                .combat
                .frame_max_damage
                .max(super::hit_log::damage_count(hit.percent_damage));
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
        // ftCo_Damage.c block_75..82: an airborne tumble launch shakes the
        // camera (Camera_RequestQuake at the fighter), sized by scaled knockback.
        if level == 3 && self.physics.ground_or_air == GroundOrAir::Air {
            self.quake_request = Some(if stun >= assets.damage.large_quake_threshold {
                melee_cm::QuakeKind::Large
            } else if stun >= assets.damage.medium_quake_threshold {
                melee_cm::QuakeKind::Medium
            } else {
                melee_cm::QuakeKind::Small
            });
        }
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        if let Some(facing) = hit.facing_override {
            self.physics.facing = facing;
        }
        (state, stun)
    }

    /// Fighter_8006CDA4 (8006CDA4), first in ftCo_8008DCE0: a launch draws
    /// whether it knocks a held light item loose. The armour flags
    /// (x2220_b3/b4), x2226_b2 and the excluded Kirby/ice states are
    /// unreachable here; Fox and Marth hold no second item (x1978/x197C).
    fn maybe_drop_held_item(
        &mut self,
        hit: &ReceivedHit,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        let Some(held) = self.held_item else {
            self.maybe_drop_article(hit, assets, rng);
            return;
        };
        if held.heavy || hit.descriptor.element == melee_types::HitElement::Cape {
            return;
        }
        let damage = fctiwz(hit.percent_damage);
        // ftCo_8008E984: armour could still hold the item; not in scope.
        assert!(
            self.combat.armor == 0.0,
            "ftCo_8008E984: an armoured launch while holding"
        );
        if rng.randi(assets.damage.item_drop_range) < damage {
            self.drop_held_item(held, assets);
        }
    }

    /// Fighter_8006CDA4 with fp->item_gobj holding a light article of the
    /// fighter's own: the same draw knocks it out of the hand (Item_8026ABD8
    /// with no push).
    fn maybe_drop_article(
        &mut self,
        hit: &ReceivedHit,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        let Some(article) = self.article_in_hand else {
            return;
        };
        if hit.descriptor.element == melee_types::HitElement::Cape {
            return;
        }
        assert!(
            self.combat.armor == 0.0,
            "ftCo_8008E984: an armoured launch while holding"
        );
        assert_ne!(
            article.use_kind, 3,
            "Fighter_8006CDA4: a shooting article's second draw"
        );
        let damage = fctiwz(hit.percent_damage);
        if rng.randi(assets.damage.item_drop_range) < damage {
            let hold = *self
                .skeleton
                .get_mtx(self.animation.parts[article.part].joint);
            let holder = self.item_holder(self.bones.model.animation_translation, assets);
            let (center, attack) = (holder.center, holder.attack);
            self.item_requests.push(melee_it::ItemRequest::DropArticle {
                owner: self.player.id,
                kind: article.kind,
                hold,
                center,
                attack,
            });
            // Item_8026A848 -> ftCommon_8007E6DC: the hand lets go.
            self.article_in_hand = None;
        }
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
    pub(super) fn combine_knockback(&mut self, x: f32, y: f32, assets: &FighterAssets) {
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
            // ftCo_Damage.c:463.
            last_bounce: None,
            bounce_lock: 0,
            // ftCo_Damage_CalcAngle (8008D7F0) -> ftColl_8007AC68 (8007AC68).
            meteor_cancel: assets
                .damage
                .is_meteor(hit.descriptor.angle)
                .then_some(assets.damage.meteor_cancel_frames as u8),
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
        S::DamageFlyHi
            | S::DamageFlyN
            | S::DamageFlyLw
            | S::DamageFlyTop
            | S::DamageFlyRoll
            // ftCo_FlyReflect_Anim / _IASA delegate to DamageFly's.
            | S::FlyReflectWall
            | S::FlyReflectCeil
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
    /// `common` is it_804D6D28: ftColl_8007A06C's item direction threshold
    /// (+78) and it_8026B1D4's speed damage.
    pub fn detect_item_hit(
        &mut self,
        item: &mut melee_it::ItemCore,
        assets: &FighterAssets,
        common: &melee_it::desc::ItemCommonData,
    ) -> Option<ItemHurtContact> {
        // ftColl_8007925C (ftcoll.c:2026): an owned item misses its owner
        // unless xDCD b5 lets it through (the Bob-omb blast).
        if (item.owner == Some(self.player.id) && !item.hits_owner)
            || item.destroyed
            || self.status.disabled
        {
            return None;
        }
        // ftcoll.c:2285-2287: intangibility (x1988 or x198C == 2) skips only
        // the hurtbox step; reflection, clank and shield come first.
        let intangible = self.commands.hurt_status == melee_types::combat::HurtStatus::Intangible
            || self.status.ledge_intangibility != 0;
        let (mut clank_mask, clank_candidates) = super::clank::item_candidates(&self.core, item);
        let mut cursor = melee_coll::detection::PairCursor::default();
        while let Some(id) = cursor.next(
            &item.hitboxes,
            self.spawn_number,
            self.physics.ground_or_air,
        ) {
            let hit = item.hitboxes[id].as_ref().unwrap().clone();
            // ftColl_8007925C: x42_b5 gates every test of this item hitbox.
            if !item.hit_flags[id].hits_hurtboxes {
                continue;
            }
            // ftcoll.c:2099-2103: x42_b2.
            if item.hit_flags[id].misses_same_facing && self.physics.facing == item.facing {
                continue;
            }
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
            // ftcoll.c:2133-2183: the absorbing bubble, after the reflectors.
            if item.hit_flags[id].defense_interaction
                && hit.descriptor.element != melee_types::HitElement::Inert
                && self.core.absorb_item_hit(item, id, &hit)
            {
                continue;
            }
            // catch_path: clank with this fighter's own hitboxes first.
            if clank_candidates
                && super::clank::item_contact(
                    &mut self.core,
                    item,
                    id,
                    &mut clank_mask,
                    &assets.clank,
                )
            {
                continue;
            }
            if self.shield.active && item.hit_flags[id].shieldable {
                if let Some(contact) = self.core.shield_contact(&hit, item.scale) {
                    self.record_item_shield_hit(item, id, contact);
                    continue;
                }
            }
            // A character's own shield volume (Marth's Counter) in the same slot.
            if item.hit_flags[id].shieldable
                && hit.descriptor.element != melee_types::HitElement::Inert
            {
                if let Some(contact) = self.character.table().item_defense_contact {
                    if contact(self, item, id, assets) {
                        continue;
                    }
                }
            }
            if intangible {
                continue;
            }
            let Some((contact, height, capsule_status)) =
                melee_coll::detection::first_hurt_contact(&mut self.core, &hit, item.scale)
            else {
                continue;
            };
            // ftColl_80077C60 (retail 0x80077F08 phantom, 0x80078188
            // ordinary): x1988, x198C or the capsule's own state
            // (hit2->state) keep the hit out of the damage and phantom logs;
            // the item still records the contact and its victim, and the
            // spark is efSync_Spawn 0x41C (0x80078358).
            let invincible = self.status.revival_invincibility != 0
                || self.commands.hurt_status != melee_types::combat::HurtStatus::Normal
                || capsule_status != melee_types::combat::HurtStatus::Normal;
            if contact.overlap < assets.damage.phantom_threshold {
                self.log_item_phantom_contact(
                    item, &hit, contact, height, invincible, assets, common,
                );
                // ftColl_8007925C (ftcoll.c:2315): a phantom contact only
                // breaks the hurt-capsule loop; the item's next hitbox is
                // still tested and can land a full hit (Fox's throw laser
                // has two).
                continue;
            }
            let mut descriptor = hit.descriptor.clone();
            // ftColl_80077C60: an item entry's damage count is its contact
            // damage (it_8026B1D4 of the staled hitbox damage) truncated, not
            // the command count.
            let raw_damage = item.contact_damage(hit.descriptor.damage, common);
            descriptor.damage = raw_damage;
            let knockback_damage = fctiwz(raw_damage) as u32;
            // ftColl_80077C60, 80077DE4: item hits scale damage while captured.
            if matches!(
                self.combat.grab,
                Some(super::grab::GrabLink::Captured { .. })
            ) {
                descriptor.damage *= assets.damage.captured_item_damage_scale;
            }
            // ftColl_80077C60 (ftcoll.c:1158): the victim's damage scale.
            descriptor.damage *= self.received_damage_scale();
            if invincible {
                item.record_fighter_victim(id, descriptor.group, self.spawn_number);
                self.effects
                    .push(melee_ef::request::EffectRequest::ShieldSpark {
                        position: contact.position,
                    });
                return Some(ItemHurtContact {
                    damage: descriptor.damage,
                    logged_damage: false,
                });
            }
            // ftColl_80077C60's ordinary branch: log for ftColl_8007AB48,
            // whose ftColl_8007A06C takes the item's direction.
            let facing = melee_it::hurt::hit_direction(
                self.physics.position.x,
                item.position.x,
                item.velocity.x,
                common.knockback.still_speed,
            );
            self.combat.log_hit(LoggedHit {
                source: HitSource::Item(melee_coll::damage_log::ItemSource {
                    owner: item.owner,
                    secondary: item.owner_secondary,
                    attack: item.stale_source,
                }),
                credit: item_credit(item),
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
            item.record_fighter_victim(id, descriptor.group, self.spawn_number);
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
    #[allow(clippy::too_many_arguments)] // The contact, its capsule and both data tables stay separate.
    fn log_item_phantom_contact(
        &mut self,
        item: &mut melee_it::ItemCore,
        hit: &HitCapsule,
        contact: Contact,
        height: HurtHeight,
        invincible: bool,
        assets: &FighterAssets,
        common: &melee_it::desc::ItemCommonData,
    ) {
        if !self.combat.hit_log.is_empty()
            || self.combat.phantom_lockout != 0.0
            || hit.phantom_victims.contains(self.spawn_number)
        {
            return;
        }
        let mut descriptor = hit.descriptor.clone();
        // it_8026B1D4's contact damage, as in the ordinary branch.
        descriptor.damage = item.contact_damage(descriptor.damage, common);
        let raw = fctiwz(descriptor.damage);
        if matches!(
            self.combat.grab,
            Some(super::grab::GrabLink::Captured { .. })
        ) {
            descriptor.damage *= assets.damage.captured_item_damage_scale;
        }
        descriptor.damage *= self.received_damage_scale();
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
        if !invincible {
            let facing = melee_it::hurt::hit_direction(
                self.physics.position.x,
                item.position.x,
                item.velocity.x,
                common.knockback.still_speed,
            );
            self.combat.phantom_max_damage = self.combat.phantom_max_damage.max(fctiwz(damage));
            self.combat.phantom_log.push(LoggedHit {
                source: HitSource::Item(melee_coll::damage_log::ItemSource {
                    owner: item.owner,
                    secondary: item.owner_secondary,
                    attack: item.stale_source,
                }),
                credit: item_credit(item),
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
    pub(super) fn modified_knockback(&self, mut knockback: f32, assets: &FighterAssets) -> f32 {
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
        // 8008D9E0..8008DA18: max(armor0, armor1) by fcmpo, plus PlCo +6F0
        // when metal (no supported mode makes a fighter metal), then fsubs.
        // armor0 is the kind's OnDeath value (Nana, Bowser, Giga Bowser).
        let armor0 = self.capabilities.armor;
        let armor = if armor0 > self.combat.armor {
            armor0
        } else {
            self.combat.armor
        };
        knockback -= armor;
        if knockback < parameters.minimum_knockback {
            knockback = parameters.minimum_knockback;
        }
        knockback
    }

    /// ftCo_8008EC90 inlineB2's ftCo_8008DA4C and ftCo_800C0408: a hit taken
    /// without a launch flashes the color animation for the element at the
    /// reaction level of ftCo_8008D8E8(kb_applied * PlCo +154) (fmuls), when
    /// this frame dealt damage (x1838_percentTemp).
    pub(super) fn unlaunched_damage_flash(
        &mut self,
        knockback: f32,
        hit: &ReceivedHit,
        assets: &FighterAssets,
    ) {
        if hit.percent_damage == 0.0 {
            return;
        }
        let scaled = knockback * assets.damage.hitstun_scale;
        let level = assets
            .damage
            .reaction_thresholds
            .iter()
            .position(|&t| scaled < t)
            .unwrap_or(3) as u8;
        let id = match hit.descriptor.element {
            melee_types::HitElement::Fire => 11 + level,
            melee_types::HitElement::Electric => 15 + level,
            melee_types::HitElement::Ice => 31 + level,
            melee_types::HitElement::Dark => 35 + level,
            _ => DAMAGE_FLASH,
        };
        // ftCo_8008DA4C -> ftCo_800BFFD0; an installed program runs now
        // (ftCo_800C0408), not at the next color step.
        if self.install_color_overlay_now(id, assets) {
            self.advance_color_overlay(assets);
        }
    }

    /// ftCo_8008EC90 inlineB2 (ftCo_Damage.c:795-811) after
    /// Fighter_UnkTakeDamage_8006CC30: the hit's damage and flash without a
    /// reaction; the motion continues into hitlag.
    fn absorb_hit(&mut self, hit: &ReceivedHit, assets: &FighterAssets) {
        // ftCo_800C8D00 returns at once without x2224_b3 (no supported mode).
        // The capture states (0xE0/0xE1, 0xE3/0xE4) are unarmored.
        self.physics.percent += hit.percent_damage;
        // kb_applied is zero here: armor absorbed all of it.
        self.unlaunched_damage_flash(0.0, hit, assets);
        // ftCommon_800804FC: a grounded victim's KO credit goes.
        self.combat
            .ko_source
            .clear_if_grounded(self.physics.ground_or_air);
    }

    /// ftCo_8008EC90 inlineB2 for a victim mid cape turn (x2220_b4): the
    /// hit's damage and its flash at the knockback's reaction level
    /// (ftCo_8008D8E8 of kb_applied * PlCo +154); the motion, the turn and
    /// the velocity run on. ftCo_800C8D00 returns at once (no x2224_b3),
    /// a turn never survives into a capture motion (a motion change ends
    /// it); ftCommon_800804FC then drops a grounded victim's KO credit.
    fn take_hit_while_turning(
        &mut self,
        hit: &ReceivedHit,
        knockback: f32,
        assets: &FighterAssets,
    ) {
        self.physics.percent += hit.percent_damage;
        self.unlaunched_damage_flash(knockback, hit, assets);
        self.combat
            .ko_source
            .clear_if_grounded(self.physics.ground_or_air);
    }
}
