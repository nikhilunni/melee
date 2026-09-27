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
