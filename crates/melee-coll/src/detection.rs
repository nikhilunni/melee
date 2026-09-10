//! Retail hit-ID and hurt-table order with lazy collider pose sampling.
use crate::{
    geometry::{capsule_contact, Capsule, Contact},
    hitbox::HitCapsule,
    hurtbox::{HurtCapsule, HurtHeight},
};
use hsd_types::Mtx;
use melee_types::{GroundOrAir, HitElement};

/// An owner supplies world-space hurt geometry only when visited. This retains
/// the retail cache/evaluation boundary; character callbacks remain with ft.
pub trait Collider {
    fn hurt_count(&self) -> usize;
    fn grabbable(&self, index: usize) -> bool;
    fn sample_hurt(&mut self, index: usize) -> (HurtCapsule, Mtx);
    fn scale(&self) -> f32;
}
/// Already positioned geometry, suitable for item-owned matrices and fixtures.
/// FighterCore provides the second implementation, with lazy JObj sampling.
pub struct PositionedCollider<'a> {
    pub capsules: &'a [HurtCapsule],
    pub matrices: &'a [Mtx],
    pub scale: f32,
}
impl Collider for PositionedCollider<'_> {
    fn hurt_count(&self) -> usize {
        self.capsules.len()
    }
    fn grabbable(&self, index: usize) -> bool {
        self.capsules[index].grabbable
    }
    fn sample_hurt(&mut self, index: usize) -> (HurtCapsule, Mtx) {
        (self.capsules[index].clone(), self.matrices[index])
    }
    fn scale(&self) -> f32 {
        self.scale
    }
}
/// ftColl_80078C70: visit a receiver's attack pair in increasing hitbox ID.
/// Inputs are reborrowed per candidate, after the consumer applies the previous
/// hit and records its group victims. No stale copy of the hit history is used.
#[derive(Default)]
pub struct PairCursor {
    next_id: usize,
}
impl PairCursor {
    pub fn next(
        &mut self,
        hits: &[Option<HitCapsule>],
        victim: u32,
        ground: GroundOrAir,
    ) -> Option<usize> {
        while self.next_id < hits.len() {
            let id = self.next_id;
            self.next_id += 1;
            let Some(hit) = &hits[id] else {
                continue;
            };
            if hit.victims.contains(&victim) || hit.descriptor.element == HitElement::Catch {
                continue;
            }
            let desc = &hit.descriptor;
            let grounded = ground == GroundOrAir::Ground;
            if (grounded && !desc.hit_ground) || (!grounded && !desc.hit_air) {
                continue;
            }
            return Some(id);
        }
        None
    }
}
/// ftColl_80078C70: the first colliding hurt capsule wins in data-table order.
pub fn first_contact<C: Collider>(
    victim: &mut C,
    hit: &HitCapsule,
    attacker_scale: f32,
) -> Option<(Contact, HurtHeight)> {
    let desc = &hit.descriptor;
    for index in 0..victim.hurt_count() {
        if desc.element == HitElement::Catch && !victim.grabbable(index) {
            continue;
        }
        let (hurt, matrix) = victim.sample_hurt(index);
        if let Some(contact) = capsule_contact(
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
            3.0 * victim.scale(),
        ) {
            return Some((contact, hurt.height));
        }
    }
    None
}
/// ftColl_80076808: every active member of a group inherits a contact.
pub fn record_victim(hits: &mut [Option<HitCapsule>], group: u8, victim: u32) {
    for hit in hits
        .iter_mut()
        .flatten()
        .filter(|hit| hit.descriptor.group == group)
    {
        hit.victims.push(victim);
    }
}
/// Preserve the current port boundary; resolving a clank also needs fighter
/// rebound states and spark effects (ftcoll.c:1758-1801), not yet ported.
pub fn require_uncontested_hit(hits: &[Option<HitCapsule>]) {
    if hits.iter().any(Option::is_some) {
        unimplemented!("ftcoll.c:1758-1801: attack clanking");
    }
}
