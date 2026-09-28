//! Item hit groups: Item.xAC4_ignoreItemID (it/itcoll.c).
//!
//! Items spawned under one id from Item_8026AE60 (Pikachu's Thunder bolt
//! chain, it_802B1DF8) share their hitboxes' victim histories. A victim one
//! member records goes to every member's live hitbox of the same hit group
//! (it_8026FAC4 / it_8026FC00), and a hitbox that starts copies the first
//! member's live history of that group instead of starting empty
//! (it_8026FCF8). Items without a group keep their own histories.
use crate::{ItemCore, ItemPool};
use melee_coll::hitbox::PhantomVictims;

/// A group member's victim histories before a detection call, so the
/// entries it records can be shared with the other members afterwards.
#[derive(Clone, Debug, Default)]
pub struct GroupVictimMark {
    group: u32,
    victims: [usize; 4],
    phantom: [PhantomVictims; 4],
}

impl ItemPool {
    /// Item_8026AE60 (8026AE60): the next hit group id; it_804D6D14 starts
    /// at 1 (item.c:154) and skips 0, which means "no group".
    pub fn allocate_hit_group(&mut self) -> u32 {
        let group = self.next_hit_group;
        self.next_hit_group = self.next_hit_group.wrapping_add(1);
        if self.next_hit_group == 0 {
            self.next_hit_group = 1;
        }
        group
    }

    /// Item_80268B18 copies SpawnItem.x40 into xAC4 before the item's first
    /// script runs; any hitbox that script started takes the group's history.
    pub fn join_hit_group(&mut self, id: u32, group: u32) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        item.hit_group = group;
        for (index, hit) in item.hitboxes.iter().enumerate() {
            if hit.is_some() {
                item.group_history_pending |= 1 << index;
            }
        }
        self.resolve_group_histories();
    }

    /// it_8026FCF8 (8026FCF8) for hitboxes a group member's script started:
    /// copy the first live hitbox of the same hit group among the members
    /// in item order (this one excluded), else start empty (lbColl_80008440).
    pub fn resolve_group_histories(&mut self) {
        for index in 0..self.items.len() {
            let pending = std::mem::take(&mut self.items[index].group_history_pending);
            for hitbox in 0..4 {
                if pending & (1 << hitbox) == 0 {
                    continue;
                }
                self.copy_group_history(index, hitbox);
            }
        }
    }

    fn copy_group_history(&mut self, index: usize, hitbox: usize) {
        let item = &self.items[index];
        let Some(hit) = &item.hitboxes[hitbox] else {
            return;
        };
        let (group, box_group) = (item.hit_group, hit.descriptor.group);
        let source = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, member)| member.hit_group == group)
            .flat_map(|(member, core)| {
                core.hitboxes
                    .iter()
                    .enumerate()
                    .filter_map(move |(slot, hit)| Some((member, slot, hit.as_ref()?)))
            })
            .find(|(member, slot, hit)| {
                (*member, *slot) != (index, hitbox) && hit.descriptor.group == box_group
            })
            .map(|(member, slot, hit)| {
                (
                    hit.victims.clone(),
                    hit.phantom_victims.clone(),
                    self.items[member].reflection_history[slot].clone(),
                )
            });
        let (victims, phantom, timers) = source.unwrap_or_default();
        let item = &mut self.items[index];
        let hit = item.hitboxes[hitbox].as_mut().expect("live hitbox");
        hit.victims = victims;
        hit.phantom_victims = phantom;
        // lbColl_CopyHitCapsule copies each victim's rehit timer too.
        item.reflection_history[hitbox] = timers;
    }

    /// Mark a group member's histories before a detection call; `None` for
    /// an item outside any group.
    pub fn group_victim_mark(item: &ItemCore) -> Option<GroupVictimMark> {
        if item.hit_group == 0 {
            return None;
        }
        let mut mark = GroupVictimMark {
            group: item.hit_group,
            ..Default::default()
        };
        for (index, hit) in item.hitboxes.iter().enumerate() {
            if let Some(hit) = hit {
                mark.victims[index] = hit.victims.len();
                mark.phantom[index] = hit.phantom_victims.clone();
            }
        }
        Some(mark)
    }

    /// it_8026FAC4 / it_8026FC00: the victims item `id` recorded since `mark`
    /// go to the other members' live hitboxes of the same hit group
    /// (lbColl_80008688 adds a victim only once).
    pub fn share_group_victims(&mut self, id: u32, mark: &GroupVictimMark) {
        let Some(source) = self.items.iter().position(|item| item.id == id) else {
            return;
        };
        for hitbox in 0..4 {
            let Some(hit) = &self.items[source].hitboxes[hitbox] else {
                continue;
            };
            let box_group = hit.descriptor.group;
            let victims = hit.victims.clone();
            let first_new = mark.victims[hitbox];
            let phantom = hit.phantom_victims.clone();
            // A victim the source recorded in a timed mode (it carries a
            // rehit timer) is timed on every member, each capsule with its
            // own x40_b4 (lbColl_80008688).
            let timed = self.items[source].reflection_history[hitbox].clone();
            for (index, member) in self.items.iter_mut().enumerate() {
                if index == source || member.hit_group != mark.group {
                    continue;
                }
                for (slot, other) in member.hitboxes.iter_mut().enumerate() {
                    let Some(other) = other else { continue };
                    if other.descriptor.group != box_group {
                        continue;
                    }
                    for &victim in victims.iter().skip(first_new) {
                        if !other.victims.contains(&victim) {
                            other.victims.push(victim);
                            if timed.iter().any(|entry| entry.victim == victim) {
                                member.reflection_history[slot].push(crate::RehitVictim {
                                    victim,
                                    remaining: member.hit_flags[slot].rehit_rate,
                                });
                            }
                        }
                    }
                    for victim in phantom.iter() {
                        if !mark.phantom[hitbox].contains(victim) {
                            other.phantom_victims.record(victim);
                        }
                    }
                }
            }
        }
    }
}
