//! Item_80269F14 and lbColl_80008688 mode7.
use crate::{ItemCore, ItemDispatch, ItemEventContext};
#[derive(Clone, Copy, Debug)]
pub struct PendingReflection {
    pub owner: u8,
    pub facing: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub preserve_owner: bool,
    /// DCC.b2; this does not suppress ownership transfer for lasers.
    pub exclude_master_ball_ownership: bool,
    /// ftLib_800866DC of the reflector (its camera bone, offset), which a
    /// kind's reflected callback may aim from (it_802A20E8).
    pub reflector_position: hsd_types::Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct RehitVictim {
    pub victim: u32,
    pub remaining: u8,
}
impl ItemCore {
    /// it_8026FA2C with ftColl_80077C60's mode for a hurtbox contact of
    /// hitbox `id`: 5 (a rehit timer) when its x41_b5 is set, else 0.
    pub fn record_fighter_victim(&mut self, id: usize, group: u8, victim: u32) {
        if self.hit_flags[id].damage_without_hitlag {
            self.record_timed_victim(group, victim);
        } else {
            melee_coll::detection::record_victim(&mut self.hitboxes, group, victim);
        }
    }
    /// ftColl_80077464's mode 7 for a reflector.
    pub fn record_reflector(&mut self, group: u8, victim: u32) {
        self.record_timed_victim(group, victim);
    }
    /// lbColl_80008688 in a timed mode (2, 4, 5, 7, 8) on every live hitbox
    /// of `group`: the victim, with that capsule's rehit timer (x40_b4),
    /// which Item_80269B60 counts down before forgetting the victim.
    pub fn record_timed_victim(&mut self, group: u8, victim: u32) {
        for (id, slot) in self.hitboxes.iter_mut().enumerate() {
            let Some(hit) = slot else { continue };
            if hit.descriptor.group != group {
                continue;
            }
            if !hit.victims.contains(&victim) {
                hit.victims.push(victim);
            }
            let timer = self.hit_flags[id].rehit_rate;
            let history = &mut self.reflection_history[id];
            let index = history.iter().position(|entry| entry.victim == victim);
            if let Some(index) = index {
                history.iter_mut().nth(index).unwrap().remaining = timer;
            } else {
                self.reflection_history[id].push(RehitVictim {
                    victim,
                    remaining: timer,
                });
            }
        }
    }
    /// Item_80269B60: decay after positioning at item link11, even while frozen.
    pub fn decay_reflection_history(&mut self) {
        for (id, slot) in self.hitboxes.iter_mut().enumerate() {
            let Some(hit) = slot else { continue };
            let history = &mut self.reflection_history[id];
            let mut i = 0;
            while i < history.len() {
                if history[i].remaining != 0 {
                    history.iter_mut().nth(i).unwrap().remaining -= 1;
                    if history[i].remaining == 0 {
                        let victim = history.remove(i).victim;
                        let position = hit.victims.iter().position(|v| *v == victim);
                        if let Some(j) = position {
                            hit.victims.remove(j);
                        }
                        continue;
                    }
                }
                i += 1;
            }
        }
    }
    /// it_8026B1D4 (8026B1D4): a hitbox's contact damage; after a throw
    /// (xDC8 x14) it grows with the item's speed, at least 1.
    pub fn contact_damage(&self, damage: f32, common: &crate::desc::ItemCommonData) -> f32 {
        if !self.speed_damage {
            return damage;
        }
        let v = self.velocity;
        // retail 8026B1F4..8026B20C: three fmuls, then z² + (x² + y²).
        let squares = v.x * v.x + v.y * v.y;
        let speed = gekko_math::msl::sqrtf(v.z * v.z + squares);
        // retail 8026B274: fmadds, then fadds.
        let damage = gekko_math::fma::fmadds(speed, common.speed_damage_scale, damage)
            + common.speed_damage_base;
        if f64::from(damage) <= 1.0 {
            1.0
        } else {
            damage
        }
    }
    /// it_80273030 (80273030), the common reflected callback: velocity back
    /// along itself at the reflector's speed multiplier (xC70), facing
    /// flipped, and the lifetime restarted from the half-life (xD48).
    pub fn reverse_on_reflect(&mut self, speed: f32) {
        self.velocity.x = -self.velocity.x * speed;
        self.velocity.y = -self.velocity.y * speed;
        self.facing = -self.facing;
        self.life_timer = self.half_life;
    }
    /// itColl_BounceOffShield (80273078): the velocity mirrored off the
    /// shield's normal (xC58, lbVector_Mirror); unless it is nearly level
    /// (|x| < 1e-5, fcmpo) with a facing already set, the facing follows its
    /// sign (x >= 0 faces right), and the collision takes that facing.
    pub fn bounce_off_shield(&mut self, normal: hsd_types::Vec3) {
        /// Retail @294.
        const LEVEL: f32 = 0.00001;
        self.velocity = melee_lb::vector::mirror(self.velocity, normal);
        let x = self.velocity.x;
        let magnitude = if x < 0.0 { -x } else { x };
        // Retail 802730B8 fcmpo + bge: NaN also takes the new facing.
        if magnitude.partial_cmp(&LEVEL) != Some(std::cmp::Ordering::Less) || self.facing == 0.0 {
            self.facing = if x >= 0.0 { 1.0 } else { -1.0 };
        }
        let facing = if self.facing == -1.0 { -1 } else { 1 };
        let collision = self.collision.as_mut().expect("item map collision");
        melee_mp::set_facing_dir(collision, facing);
    }
    pub(crate) fn reflect<D: ItemDispatch>(
        &mut self,
        pending: PendingReflection,
        stale: f32,
        cap: u32,
        assets: &crate::desc::ItemAssets,
        common: &crate::desc::ItemCommonData,
    ) {
        if !pending.preserve_owner {
            // No reflector is a player's second fighter.
            self.owner = Some(pending.owner);
            self.owner_secondary = false;
        }
        self.stale_multiplier = stale;
        // The original attack/instance (stale_source) survives this transfer.
        let context = ItemEventContext {
            reflected_facing: pending.facing,
            reflected_speed: pending.speed_multiplier,
            reflector_position: pending.reflector_position,
            ..ItemEventContext::new(assets, common)
        };
        self.destroyed |= (D::logic(self.kind).reflected)(self, &context);
        if self.destroyed {
            return;
        }
        for hit in self.hitboxes.iter_mut().flatten() {
            // Item_80269F14,8026A034: fmadds before positive unsigned truncation.
            let rounded =
                gekko_math::fma::fmadds(hit.descriptor.damage, pending.damage_multiplier, 0.99);
            assert!(
                (0.0..4294967296.0).contains(&rounded),
                "reflected damage unsigned domain"
            );
            let base = (rounded as u32).min(cap);
            hit.knockback_damage = base;
            hit.descriptor.damage = base as f32 * stale;
        }
    }
}
