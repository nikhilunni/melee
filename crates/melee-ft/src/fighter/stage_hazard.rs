//! A stage's collision device hurting a fighter (ftColl_8007BAC0's
//! ft_80459A8C entries, ftco_bury.c's ftCo_800C08A0): Brinstar's acid
//! registers one of type 3 (ftCo_800C07F8(gobj, 3, grZebes_801DCBFC)).
//! The device's callback decides contact and hands back a hit record
//! (lbColl_80008D30_arg1); the hit joins this frame's damage log as an
//! entry of kind 3, resolved with the others (ftColl_8007A06C).
use super::{
    assets::{FighterAssets, Result},
    FighterCore,
};
use melee_coll::{
    damage::ReceivedHit,
    damage_log::{HitCredit, HitSource, LoggedHit},
};
use melee_types::combat::{HurtStatus, ThrowHitbox};

/// A stage device's hit: the record (its damage as lbColl_80008D30 makes
/// it a float) and its integer damage (`DynamicsDesc.count`, the log
/// entry's knockback count).
#[derive(Clone, Debug)]
pub struct StageHit {
    pub record: ThrowHitbox,
    pub count: i32,
}

impl StageHit {
    /// lbColl_80008D30_arg1 at `at`: state, damage, angle, growth,
    /// weight-set and base knockback, element, sound severity and kind.
    pub fn read(archive: &hsd_archive::Archive, at: u32) -> Result<Self> {
        let r = archive.reader();
        let word = |offset| r.u32(at + offset);
        let count = word(0x4)? as i32;
        Ok(Self {
            record: ThrowHitbox {
                damage: count as f32,
                angle: word(0x8)? as u16,
                growth: word(0xC)? as u16,
                weight_knockback: word(0x10)? as u16,
                base_knockback: word(0x14)? as u16,
                element: melee_types::HitElement::try_from(word(0x18)? as i32)
                    .expect("a stage hit's element"),
                sound_severity: word(0x1C)? as u8,
                sound_kind: word(0x20)? as u8,
            },
            count,
        })
    }
}

impl FighterCore {
    /// ftCo_800C0A28 (BuryType_Unk3): the device may hurt this fighter.
    pub fn stage_hazard_ready(&self) -> bool {
        self.status.barrel_cooldown == 0
    }

    /// ftColl_GetHitStatus (ftcoll.c): none of x221D_b6, x1988 (the
    /// subaction's hurt status) and x198C (the timed invincibility) set.
    pub fn hurtable_by_stage(&self) -> bool {
        self.commands.hurt_status == HurtStatus::Normal
            && self.status.revival_invincibility == 0
            && self.status.ledge_intangibility == 0
    }

    /// ftCo_800C08A0 (0x800C08A0) for type 3: the damage
    /// (ftColl_800765F0), the device's cooldown, then ftColl_80076640 and
    /// the log entry (ftColl_80076764) at hurt capsule 0. ftColl_80078384's
    /// sound and pl_8003EC30's statistics are not game state.
    pub fn take_stage_hit(&mut self, hit: &StageHit, kind: u32, assets: &FighterAssets) {
        self.log_stage_hit(hit, kind, assets, |core| {
            // BuryType_Unk3: bury_timer_2 = PlCo +5E4.
            core.status.barrel_cooldown = assets.barrel.hazard_cooldown;
        });
    }

    /// ftCo_800C08A0 and ftCo_800C0B20's shared body: the damage
    /// (ftColl_800765F0), the caller's cooldown, then ftColl_80076640 and
    /// the log entry of `kind` (ftColl_80076764) at hurt capsule 0.
    pub(super) fn log_stage_hit(
        &mut self,
        hit: &StageHit,
        kind: u32,
        assets: &FighterAssets,
        start_cooldown: impl FnOnce(&mut Self),
    ) {
        // ftColl_800765F0(fp, NULL, count): a captured fighter takes PlCo
        // +128 of it (fmuls), then its damage scale (fmuls).
        let mut damage = hit.count as f32;
        if matches!(
            self.combat.grab,
            Some(super::grab::GrabLink::Captured { .. })
        ) {
            damage *= assets.damage.captured_item_damage_scale;
        }
        let damage = damage * self.received_damage_scale();
        start_cooldown(self);
        let count = super::hit_log::damage_count(damage);
        let Some(damage) = self.combat.through_shell(damage) else {
            return;
        };
        self.combat.frame_damage += damage;
        self.combat.frame_max_damage = self.combat.frame_max_damage.max(count);
        let descriptor = super::grab_throw::throw_descriptor(&hit.record);
        let height = self.hurtboxes.first().expect("hurt capsule 0").height;
        let position = self.physics.position;
        self.combat.hit_log.push(LoggedHit {
            source: HitSource::Stage(kind),
            // ftColl_8007861C(NULL, gobj, 3, kind, .., 1): no fighter, and
            // the earlier source stays.
            credit: HitCredit::Unchanged,
            hit: ReceivedHit {
                descriptor,
                height,
                // ftColl_8007A06C: the victim's own facing.
                facing: self.physics.facing,
                facing_override: None,
                knockback: 0.0,
                percent_damage: damage,
            },
            position,
            knockback_damage: hit.count as u32,
            damage,
            effect_damage: damage,
            attack_ratio: 1.0,
        });
    }
}
