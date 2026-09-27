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
}
#[derive(Clone, Copy, Debug)]
pub struct RehitVictim {
    pub victim: u32,
    pub remaining: u8,
}
impl ItemCore {
    pub fn record_reflector(&mut self, group: u8, victim: u32) {
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
        let damage =
            gekko_math::fma::fmadds(speed, common.speed_damage_scale, damage) + common.speed_damage_base;
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
    pub(crate) fn reflect<D: ItemDispatch>(
        &mut self,
        pending: PendingReflection,
        stale: f32,
        cap: u32,
        assets: &crate::desc::ItemAssets,
    ) {
        if !pending.preserve_owner {
            self.owner = Some(pending.owner);
        }
        self.stale_multiplier = stale;
        // The original attack/instance (stale_source) survives this transfer.
        let context = ItemEventContext {
            reflected_facing: pending.facing,
            reflected_speed: pending.speed_multiplier,
            ..ItemEventContext::new(assets)
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
