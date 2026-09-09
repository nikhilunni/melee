//! Fighter part selection, `ftanim.c` 0x8006F4C8 / 0x8006FCE4 / 0x8006FE08.

use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_archive::desc::FigaTree;
use melee_lb::anim::{attach_joint_tracks, attach_joint_tracks_without_translation, AttachError};

use crate::desc::PartTable;

/// Animation bits from `FighterBone.flags8` (+8, high byte), not host bitfields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartFlags(pub u8);
impl PartFlags {
    /// flags_b0: animation does not own rotation.
    pub const LOCKED: u8 = 0x80;
    /// flags_b1: joint participates in the runtime skeleton.
    pub const PRESENT: u8 = 0x40;
    /// flags_b2: joint is conditional on the motion's bone mask.
    pub const CONDITIONAL: u8 = 0x20;
    /// flags_b3: translations survive cross-character remapping.
    pub const TRANSLATION: u8 = 0x10;
    /// flags_b4: copy directly instead of interpolating.
    pub const COPY: u8 = 0x08;
    /// flags_b5: controlled by a separate part animation.
    pub const PART_ANIMATION: u8 = 0x04;

    pub fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
    pub fn eligible(self) -> bool {
        self.contains(Self::PRESENT) && self.0 & (Self::LOCKED | Self::PART_ANIMATION) == 0
    }
}

/// `Fighter.parts` (+0x5E8), indexed by runtime bone, not semantic FtPart.
#[derive(Clone, Debug)]
pub struct AnimationPart {
    /// FighterBone.joint (+0); the blend tree uses the same owned id mapping.
    pub joint: JObjId,
    pub flags: PartFlags,
    /// FighterBone.xC (+0xC): preorder depth, used to bound part subtrees.
    pub depth: usize,
    /// `ftParts_8007506C` mask for this bone (zero = unconditional).
    pub motion_mask: u32,
}

/// Source skeleton metadata for `ftPartsRemap` (0x80075028) and
/// `ftAnim_8006FCE4`. Absence means the same-character 0x8006F4C8 path.
#[derive(Clone, Debug)]
pub struct MotionRemap {
    pub source: PartTable,
    pub destination: PartTable,
    pub source_masks: Vec<u32>,
}

/// Validate selection before changing AObjs. Disabled joints still consume a
/// node and all of its tracks; mask-excluded joints consume neither.
pub fn attach_motion(
    tree: &mut JObjTree,
    parts: &[AnimationPart],
    animation: &FigaTree,
    motion_mask: u32,
    remap: Option<&MotionRemap>,
) -> Result<(), AttachError> {
    let mut selected = Vec::new();
    let mut index = 0;
    for _ in &animation.nodes {
        let destination = if let Some(remap) = remap {
            loop {
                let mask = *remap
                    .source_masks
                    .get(index)
                    .ok_or(AttachError::InvalidTracks)?;
                if mask == 0 || mask & motion_mask != 0 {
                    break;
                }
                index += 1;
            }
            remap
                .source
                .joint_to_part
                .get(index)
                .copied()
                .flatten()
                .and_then(|part| remap.destination.part_to_joint.get(usize::from(part)))
                .copied()
                .flatten()
                .map(usize::from)
        } else {
            while !parts
                .get(index)
                .ok_or(AttachError::InvalidTracks)?
                .flags
                .contains(PartFlags::PRESENT)
            {
                index += 1;
            }
            loop {
                let part = parts.get(index).ok_or(AttachError::InvalidTracks)?;
                if !part.flags.contains(PartFlags::CONDITIONAL)
                    || part.motion_mask == 0
                    || part.motion_mask & motion_mask != 0
                {
                    break;
                }
                index += 1;
            }
            Some(index)
        };
        let part = destination
            .map(|i| parts.get(i).ok_or(AttachError::InvalidTracks))
            .transpose()?;
        if let Some(part) = part {
            if part.joint.0 >= tree.len() {
                return Err(AttachError::InvalidJoint(part.joint));
            }
        }
        selected.push(part.filter(|p| {
            if remap.is_some() {
                p.flags.eligible()
            } else {
                // F4C8 checks PRESENT before walking conditional masks, not
                // again afterwards. FCE4 does check the remapped destination.
                p.flags.0 & (PartFlags::LOCKED | PartFlags::PART_ANIMATION) == 0
            }
        }));
        index += 1;
    }
    let total = animation.nodes.iter().try_fold(0usize, |n, &count| {
        n.checked_add(usize::try_from(count).ok()?)
    });
    if total != Some(animation.tracks.len())
        || animation
            .tracks
            .iter()
            .any(|t| usize::from(t.length) != t.ad.len())
    {
        return Err(AttachError::InvalidTracks);
    }
    // Check retail's undefined empty-prefix case before any joint is changed.
    for (part, tracks) in selected.iter().zip(animation.tracks_by_node()) {
        if part.is_some_and(|p| remap.is_some() && !p.flags.contains(PartFlags::TRANSLATION))
            && tracks.first().is_some_and(|t| matches!(t.obj_type, 5..=7))
        {
            return Err(AttachError::NoAcceptedTracks);
        }
    }
    for (part, tracks) in selected.into_iter().zip(animation.tracks_by_node()) {
        if let Some(part) = part {
            if remap.is_some() && !part.flags.contains(PartFlags::TRANSLATION) {
                attach_joint_tracks_without_translation(tree, part.joint, animation, tracks)?;
            } else {
                attach_joint_tracks(tree, part.joint, animation, tracks);
            }
        }
    }
    Ok(())
}
