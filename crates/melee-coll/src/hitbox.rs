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
    pub phase: CapsulePhase,
    pub position: Vec3,
    pub previous_position: Vec3,
    /// Port history bound, matching the combined two twelve-entry retail
    /// arrays (lb/types.h:79-80). Overflow is explicit; no victim is dropped.
    pub victims: melee_types::fixed::FixedVec<u32, VICTIM_CAPACITY>,
}
/// ftColl_800768A0 (800768A0): new group members inherit victims.
pub fn spawn(boxes: &mut [Option<HitCapsule>], id: usize, descriptor: &HitboxDescriptor) {
    assert!(id < boxes.len(), "hitbox id");
    if let Some(hit) = &mut boxes[id] {
        if hit.descriptor.group == descriptor.group {
            hit.descriptor = descriptor.clone();
            return;
        }
    }
    let victims = group_history(boxes, descriptor.group);
    boxes[id] = Some(HitCapsule {
        descriptor: descriptor.clone(),
        phase: CapsulePhase::Enabled,
        position: Vec3::ZERO,
        previous_position: Vec3::ZERO,
        victims,
    });
}
/// Copy the first active group member's history in capsule-table order.
fn group_history(
    boxes: &[Option<HitCapsule>],
    group: u8,
) -> melee_types::fixed::FixedVec<u32, VICTIM_CAPACITY> {
    for hit in boxes.iter().flatten() {
        if hit.descriptor.group == group {
            return hit.victims.clone();
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
