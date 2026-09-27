//! Hits landing on items (itcoll.c): hurt capsules placed on the model
//! (it_8027163C, lbColl_8000805C), the per-frame damage log (it_8026F9AC)
//! and its resolution into knockback (it_80270E30).
use crate::{desc::ItemAssets, ItemCore, ItemEvent};
use gekko_math::fma::fmadds;
use hsd_types::{Mtx, Vec3};

/// One hurt capsule in world space, with the bone matrix that scales it.
#[derive(Clone, Copy, Debug)]
pub struct HurtCapsule {
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
    pub matrix: Mtx,
}

/// A damage log entry (it_804A0E70) for a fighter's hitbox: the hit's
/// numbers and the contact point lbColl_80006E58 found.
#[derive(Clone, Copy, Debug)]
pub struct ItemHit {
    pub attacker: u8,
    /// The attacker's cur_pos.x, for the incoming direction.
    pub attacker_x: f32,
    pub damage: f32,
    pub angle: u16,
    pub growth: u16,
    pub weight_knockback: u16,
    pub base_knockback: u16,
    pub contact: Vec3,
}

/// Room for every fighter hitbox that can land on one item in a frame.
pub const ITEM_HIT_LOG_CAPACITY: usize = 16;
pub type HurtCapsules = melee_types::fixed::FixedVec<HurtCapsule, 2>;
pub type ItemHitLog = melee_types::fixed::FixedVec<ItemHit, ITEM_HIT_LOG_CAPACITY>;

/// it_804D6D28->x80[7..=12]: item knockback constants.
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemKnockback {
    /// x9C: the knockback cap.
    pub cap: f32,
    /// xA0: weight of the percent term.
    pub percent: f32,
    /// xA4: weight of the damage-times-percent term.
    pub damage_percent: f32,
    /// xA8: the percent a weight-set hit assumes.
    pub weight_set_percent: f32,
    /// xAC / xB0: the scale and base applied after the item's multiplier.
    pub scale: f32,
    pub base: f32,
}

impl ItemCore {
    /// The item's hurt capsules this frame. Each endpoint is lb_8000B1CC of
    /// its offset on the capsule's bone; the model root has no parent, so an
    /// unrotated, unscaled root only adds the offset to its translation.
    pub fn hurt_capsules(&self, assets: &ItemAssets) -> HurtCapsules {
        let mut matrix = Mtx::default();
        hsd_anim::mtx::hsd_mtx_srt(
            &mut matrix,
            &self.model_scale,
            &self.rotation,
            &self.position,
            None,
        );
        let plain = self.rotation == Vec3::ZERO && self.model_scale == Vec3::new(1.0, 1.0, 1.0);
        let place = |offset: Vec3| -> Vec3 {
            if offset == Vec3::ZERO {
                self.position
            } else if plain {
                Vec3::new(
                    self.position.x + offset.x,
                    self.position.y + offset.y,
                    self.position.z + offset.z,
                )
            } else {
                let mut out = Vec3::ZERO;
                hsd_anim::mtx::mtx_mult_vec(&matrix, &offset, &mut out);
                out
            }
        };
        assets
            .hurtboxes
            .iter()
            .map(|hurtbox| {
                assert_eq!(
                    hurtbox.bone, 0,
                    "it_8027163C: an item hurt capsule on a dynamic bone"
                );
                HurtCapsule {
                    start: place(hurtbox.offsets[0]),
                    end: place(hurtbox.offsets[1]),
                    radius: hurtbox.radius,
                    matrix,
                }
            })
            .collect()
    }

    /// it_80270E30 (80270E30): every logged hit sparks and gets a knockback;
    /// the strongest names the attacker, direction and angle.
    pub fn resolve_hits(
        &mut self,
        hits: &ItemHitLog,
        constants: &ItemKnockback,
        assets: &ItemAssets,
    ) {
        let mut strongest_knockback = -1.0;
        let mut strongest = None;
        for hit in hits.iter() {
            let knockback = self.knockback(hit, constants, assets.collision_damage_multiplier);
            // xDCF b1 is never set here; hold kinds 4 and 6 (Pokemon) use
            // the element's effect instead.
            assert!(
                !matches!(self.hold_kind, 4 | 6),
                "it_80270E30: element hit effects"
            );
            self.events.push(ItemEvent::HitSpark {
                position: hit.contact,
                damage: hit.damage,
            });
            if knockback > strongest_knockback {
                strongest_knockback = knockback;
                strongest = Some(*hit);
            }
        }
        let Some(hit) = strongest else {
            return;
        };
        // Damage log kind 1, a fighter.
        self.hit_by = Some(hit.attacker);
        self.hit_direction = if self.position.x > hit.attacker_x {
            -1.0
        } else {
            1.0
        };
        self.knockback_angle = hit.angle;
        self.pending_knockback = strongest_knockback;
    }

    /// it_80270E30's knockback, capped at x9C. The integer fields convert
    /// exactly; retail 80270EF0..F18 and 80270FA0..FB0 fuse as written.
    fn knockback(&self, hit: &ItemHit, k: &ItemKnockback, multiplier: f32) -> f32 {
        let growth = 0.01 * f32::from(hit.growth);
        let base = f32::from(hit.base_knockback);
        let scaled = if hit.weight_knockback != 0 {
            let weight = f32::from(hit.weight_knockback);
            let inner = fmadds(
                k.weight_set_percent,
                k.percent,
                k.damage_percent * (k.weight_set_percent * weight),
            );
            fmadds(growth, fmadds(k.scale, multiplier * inner, k.base), base)
        } else {
            let percent = self.damage_percent as f32 + self.pending_damage_taken as f32;
            let inner = fmadds(
                k.percent,
                percent,
                k.damage_percent * (hit.damage * percent),
            );
            fmadds(growth, fmadds(k.scale, multiplier * inner, k.base), base)
        };
        if scaled >= k.cap {
            k.cap
        } else {
            scaled
        }
    }
}
