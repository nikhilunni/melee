//! Hit capsule phases and shared attack-group history, ftcoll.c.
use hsd_types::Vec3;
use melee_types::combat::HitboxDescriptor;

/// Port bound matching the combined two twelve-entry retail victim arrays.
pub const VICTIM_CAPACITY: usize = 24;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapsulePhase {
    Enabled,
    FirstPosition,
    Sweeping,
}
#[derive(Clone, Debug)]
pub struct HitCapsule {
    pub descriptor: HitboxDescriptor,
    /// ftColl_8007ABD0 (8007ABD0): charged damage before staling.
    pub knockback_damage: u32,
    pub phase: CapsulePhase,
    pub position: Vec3,
    pub previous_position: Vec3,
    /// Port history bound, matching the combined two twelve-entry retail
    /// arrays (lb/types.h:79-80). Overflow is explicit; no victim is dropped.
    pub victims: melee_types::fixed::FixedVec<u32, VICTIM_CAPACITY>,
    /// HitCapsule.victims_2 (+D4): fighters this hitbox already touched in
    /// the phantom range, so it does not phantom them again.
    pub phantom_victims: PhantomVictims,
    /// x42_b5: may hit (and clank with) fighters. Set by every spawn
    /// command; ftAction_80071708 (opcode 14) can clear it.
    pub hits_fighters: bool,
    /// x42_b7: may hit items; as `hits_fighters`.
    pub hits_items: bool,
}
/// HitCapsule.victims_2 / x45 (lb/types.h:72,80): twelve slots filled in
/// order, then overwritten round-robin from `next_overwrite`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhantomVictims {
    slots: [Option<u32>; PHANTOM_VICTIM_SLOTS],
    next_overwrite: usize,
}
pub const PHANTOM_VICTIM_SLOTS: usize = 12;
impl PhantomVictims {
    pub fn contains(&self, victim: u32) -> bool {
        self.slots.contains(&Some(victim))
    }
    /// lbColl_80008820 with type 0 (no rehit timer): add a new victim.
    pub fn record(&mut self, victim: u32) {
        if self.contains(victim) {
            return;
        }
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(victim);
            return;
        }
        self.slots[self.next_overwrite] = Some(victim);
        self.next_overwrite = (self.next_overwrite + 1) % PHANTOM_VICTIM_SLOTS;
    }
}
/// ftColl_800768A0 (800768A0): new group members inherit victims.
pub fn spawn(boxes: &mut [Option<HitCapsule>], id: usize, descriptor: &HitboxDescriptor) {
    assert!(id < boxes.len(), "hitbox id");
    if let Some(hit) = &mut boxes[id] {
        if hit.descriptor.group == descriptor.group {
            hit.knockback_damage = gekko_math::msl::fctiwz(descriptor.damage) as u32;
            hit.descriptor = descriptor.clone();
            // ftAction_8007121C (ftaction.c:345-346), on every spawn.
            hit.hits_fighters = true;
            hit.hits_items = true;
            return;
        }
    }
    let (victims, phantom_victims) = group_history(boxes, descriptor.group);
    boxes[id] = Some(HitCapsule {
        descriptor: descriptor.clone(),
        knockback_damage: gekko_math::msl::fctiwz(descriptor.damage) as u32,
        phase: CapsulePhase::Enabled,
        position: Vec3::ZERO,
        previous_position: Vec3::ZERO,
        victims,
        phantom_victims,
        hits_fighters: true,
        hits_items: true,
    });
}
/// Copy the first active group member's histories in capsule-table order
/// (lbColl_CopyHitCapsule copies both victim arrays).
fn group_history(
    boxes: &[Option<HitCapsule>],
    group: u8,
) -> (
    melee_types::fixed::FixedVec<u32, VICTIM_CAPACITY>,
    PhantomVictims,
) {
    for hit in boxes.iter().flatten() {
        if hit.descriptor.group == group {
            return (hit.victims.clone(), hit.phantom_victims.clone());
        }
    }
    Default::default()
}

impl HitCapsule {
    /// ftColl_8007AD18 (8007AD18): first position starts a degenerate sweep.
    pub fn update_position(&mut self, position: Vec3) {
        self.previous_position = if self.phase == CapsulePhase::Enabled {
            position
        } else {
            self.position
        };
        self.phase = if self.phase == CapsulePhase::Enabled {
            CapsulePhase::FirstPosition
        } else {
            CapsulePhase::Sweeping
        };
        self.position = position;
    }
}
