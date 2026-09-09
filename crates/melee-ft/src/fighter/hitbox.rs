//! ftAction_8007121C and ftColl_8007AD18: owned attack capsules.
use super::{assets::Result, caches::bone_position};
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_archive::Archive;
use hsd_types::Vec3;

#[derive(Clone, Debug)]
pub struct HitboxDescriptor {
    pub group: u8,
    pub bone: usize,
    pub damage: f32,
    pub radius: f32,
    pub offset: Vec3,
    pub angle: u16,
    pub growth: u16,
    pub weight_knockback: u16,
    pub base_knockback: u16,
    pub element: melee_types::HitElement,
    pub hit_ground: bool,
    pub hit_air: bool,
    pub ignore_scale: bool,
    pub clank: bool,
    pub rebound: bool,
}
impl HitboxDescriptor {
    /// ftAction_8007121C (8007121C), lb/types.h:769-812.
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = archive.reader();
        let first = r.u32(offset)?;
        let flags = r.u32(offset + 12)?;
        let last = r.u32(offset + 16)?;
        if first & (1 << 10) != 0 {
            unimplemented!("ftaction.c:319: common hitbox bone mapping");
        }
        if flags & 8 != 0 {
            unimplemented!("ftaction.c:300: throw-only hitbox command");
        }
        // ftAction_8007121C --fused: none. Literal is 0.003906f, not 1/256.
        const SCALE: f32 = 0.003906;
        Ok(Self {
            group: ((first >> 20) & 7) as u8,
            bone: ((first >> 11) & 255) as usize,
            damage: (first & 1023) as f32,
            radius: SCALE * f32::from(r.u16(offset + 4)?),
            offset: Vec3::new(
                SCALE * f32::from(r.u16(offset + 6)? as i16),
                SCALE * f32::from(r.u16(offset + 8)? as i16),
                SCALE * f32::from(r.u16(offset + 10)? as i16),
            ),
            angle: (flags >> 23) as u16,
            growth: ((flags >> 14) & 511) as u16,
            weight_knockback: ((flags >> 5) & 511) as u16,
            base_knockback: (last >> 23) as u16,
            element: melee_types::HitElement::try_from(((last >> 18) & 31) as i32)?,
            hit_ground: last & 2 != 0,
            hit_air: last & 1 != 0,
            ignore_scale: flags & 4 != 0,
            clank: flags & 2 != 0,
            rebound: flags & 1 != 0,
        })
    }
}
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
    pub victims: Vec<u32>,
}
/// ftColl_800768A0 (800768A0): new group members inherit victims.
pub fn spawn(boxes: &mut [Option<HitCapsule>; 4], id: usize, descriptor: &HitboxDescriptor) {
    assert!(id < boxes.len(), "fighter hitbox id");
    if let Some(hit) = &mut boxes[id] {
        if hit.descriptor.group == descriptor.group {
            hit.descriptor = descriptor.clone();
            return;
        }
    }
    let victims = boxes
        .iter()
        .flatten()
        .find(|hit| hit.descriptor.group == descriptor.group)
        .map_or_else(Vec::new, |hit| hit.victims.clone());
    boxes[id] = Some(HitCapsule {
        descriptor: descriptor.clone(),
        phase: CapsulePhase::Enabled,
        position: Vec3::ZERO,
        previous_position: Vec3::ZERO,
        victims,
    });
}
impl HitCapsule {
    /// ftColl_8007AD18 (8007AD18): first position starts a degenerate sweep.
    pub fn update(&mut self, tree: &mut JObjTree, root: JObjId, scale: f32) {
        let mut offset = self.descriptor.offset;
        if self.descriptor.ignore_scale {
            let inverse = 1.0 / scale;
            offset = Vec3::new(offset.x * inverse, offset.y * inverse, offset.z * inverse);
        }
        let position = bone_position(tree, root, self.descriptor.bone, offset);
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
