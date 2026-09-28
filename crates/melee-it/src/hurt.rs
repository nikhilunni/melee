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

/// Who landed a logged hit (it_804A0E70 x0/x4).
#[derive(Clone, Copy, Debug)]
pub enum ItemHitSource {
    /// Kind 1, a fighter: its player slot and cur_pos.x.
    Fighter { player: u8, x: f32 },
    /// Kind 2, another item (it_802706D0): its fighter owner, if any (xCB0
    /// is 6 otherwise), and its position and horizontal speed.
    Item {
        owner: Option<u8>,
        position: Vec3,
        velocity_x: f32,
    },
}

/// A damage log entry (it_804A0E70): the hit's numbers and the contact
/// point the capsule test found.
#[derive(Clone, Copy, Debug)]
pub struct ItemHit {
    pub source: ItemHitSource,
    pub damage: f32,
    pub angle: u16,
    pub growth: u16,
    pub weight_knockback: u16,
    pub base_knockback: u16,
    pub element: melee_types::HitElement,
    pub contact: Vec3,
}

/// The fighter common data (p_ftCommonData) it_8027B798 reads to turn an
/// item's knockback into a launch.
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemLaunch {
    /// +100: speed per unit of knockback.
    pub velocity_scale: f32,
    /// +144: the 361-degree angle in the air, in radians.
    pub air_angle: f32,
    /// +148 / +14C / +150: on the ground, the angle in degrees reached at
    /// the maximum knockback, and the knockback band it grows over.
    pub ground_angle: f32,
    pub ground_threshold: f32,
    pub ground_maximum: f32,
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
    /// x78: an item hitter slower than this counts as still, so the
    /// incoming direction follows the two positions.
    pub still_speed: f32,
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
            // xDCF b1 is never set here; stage enemies and Pokemon (hold
            // kinds 4 and 6) show the element's effect instead.
            let spark = if matches!(self.hold_kind, 4 | 6) {
                element_spark(hit)
            } else {
                Some(ItemEvent::HitSpark {
                    position: hit.contact,
                    damage: hit.damage,
                })
            };
            if let Some(spark) = spark {
                self.events.push(spark);
            }
            if knockback > strongest_knockback {
                strongest_knockback = knockback;
                strongest = Some(*hit);
            }
        }
        let Some(hit) = strongest else {
            return;
        };
        let (hit_by, direction) = match hit.source {
            ItemHitSource::Fighter { player, x } => {
                (Some(player), if self.position.x > x { -1.0 } else { 1.0 })
            }
            ItemHitSource::Item {
                owner,
                position,
                velocity_x,
            } => (
                owner,
                hit_direction(self.position.x, position.x, velocity_x, constants.still_speed),
            ),
        };
        self.hit_by = hit_by;
        self.hit_direction = direction;
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

/// it_80270E30's hold kind 4 / 6 branch: hit_effect_ids[element] through
/// efSync_Spawn. The plain spark keeps its damage-scaled size; the others
/// receive the item (or its facing), which their generators ignore.
fn element_spark(hit: &ItemHit) -> Option<ItemEvent> {
    use melee_types::HitElement::*;
    let position = hit.contact;
    match hit.element {
        // Ef_Id_Unk1000.
        Normal | Ground | Cape => Some(ItemEvent::HitSpark {
            position,
            damage: hit.damage,
        }),
        // Ef_Id_Unk1001 / Unk1002: positional generators.
        Electric => Some(ItemEvent::Effect {
            id: 0x3E9,
            position,
        }),
        Fire => Some(ItemEvent::Effect {
            id: 0x3EA,
            position,
        }),
        // Ef_Id_Unk1004.
        Slash => Some(ItemEvent::SlashSpark { position }),
        // -1: no effect.
        Nap | Sleep | Catch | Inert | Disable | ScrewAttack | Lipstick => None,
        Coin | Ice | Dark | ReDead => {
            unimplemented!("it_80270E30: {:?} hit effect on a stage enemy", hit.element)
        }
    }
}

impl ItemCore {
    /// it_8027B798 (8027B798): the launch velocity for this frame's
    /// knockback (xCC8, its angle xCAC); the item turns to face the hit
    /// (xCCC). Returns whether a grounded item's launch leaves its floor.
    /// Only the airborne branch is ported.
    pub fn knockback_launch(&mut self, launch: &ItemLaunch) -> (Vec3, bool) {
        const DEG_TO_RAD: f32 = 0.017_453_292;
        let knockback = self.pending_knockback;
        // retail 8027B7E0: fmuls.
        let speed = knockback * launch.velocity_scale;
        let angle = if self.knockback_angle != 361 {
            DEG_TO_RAD * f32::from(self.knockback_angle)
        } else if self.ground_or_air == melee_types::GroundOrAir::Air {
            launch.air_angle
        } else if knockback < launch.ground_threshold {
            0.0
        } else {
            let fraction = (knockback - launch.ground_threshold)
                / (launch.ground_maximum - launch.ground_threshold);
            // retail 8027B854: fmadds, then the separately rounded conversion.
            let angle = DEG_TO_RAD * fmadds(launch.ground_angle, fraction, 1.0);
            let limit = DEG_TO_RAD * launch.ground_angle;
            if angle > limit {
                limit
            } else {
                angle
            }
        };
        let dx = speed * gekko_math::msl::cosf(angle);
        let dy = speed * gekko_math::msl::sinf(angle);
        self.facing = self.hit_direction;
        if self.ground_or_air != melee_types::GroundOrAir::Air {
            unimplemented!("it_8027B798: a grounded item's launch along its floor");
        }
        (Vec3::new(-dx * self.facing, dy, 0.0), false)
    }
}

/// ftColl_8007A06C's item entry (and it_80270E30's): an item moving faster
/// than it_804D6D28 +78 pushes against its velocity, a slower one away
/// from its position.
pub fn hit_direction(victim_x: f32, item_x: f32, item_velocity_x: f32, still_speed: f32) -> f32 {
    if gekko_math::msl::fabsf(item_velocity_x) < still_speed {
        if victim_x > item_x {
            -1.0
        } else {
            1.0
        }
    } else if item_velocity_x < 0.0 {
        1.0
    } else {
        -1.0
    }
}
