//! Whole-model animation switches (grAnime_801C8138) decoded once at load.
//! A switch in the tick replays the prepared clocks into the model's
//! reserved track storage instead of decoding the AnimJoint again.
use super::animation::BackgroundAnimation;
use hsd_anim::{
    aobj::{AObj, AOBJ_LOOP},
    jobj::{JObjId, JObjTree, JOBJ_CLASSICAL_SCALE},
    load::{anim_joint_targets, attach_anim_joint},
};
use hsd_archive::Archive;

/// What `HSD_JObjAddAnim` (jobj.c:298-313) leaves on one joint.
pub(super) struct PreparedAttachment {
    joint: JObjId,
    /// The loaded, sorted clock with the model's loop flag, before the
    /// `HSD_JObjReqAnimAll` that follows every switch.
    aobj: Option<AObj>,
    /// AnimJoint flags bit 0: `JOBJ_CLASSICAL_SCALE` set, else cleared.
    classical_scale: bool,
    path_reference: Option<JObjId>,
}

/// One model animation's attachments in `attach_anim_joint` order; `None`
/// when the animation cannot be attached (the tick then reports the error).
pub(super) type PreparedSelections = std::sync::Arc<[Option<Vec<PreparedAttachment>>]>;

/// Attach each of the model's animations to a copy of the unanimated model
/// at bone 0, as `attach_subtree` does, and keep the resulting clocks.
pub(super) fn prepare(
    rest: &JObjTree,
    root: JObjId,
    joints: &[JObjId],
    archive: &Archive,
    model: &crate::desc::ModelDesc,
) -> PreparedSelections {
    let mut scratch = rest.clone();
    let bone = scratch.bone(root, 0).expect("stage animation bone");
    model
        .animations
        .iter()
        .zip(&model.animation_loops)
        .map(|(animation, &looping)| {
            for &joint in joints {
                scratch.remove_anim(joint);
            }
            let targets = anim_joint_targets(&scratch, bone, animation).ok()?;
            attach_anim_joint(&mut scratch, bone, animation, archive).ok()?;
            Some(
                targets
                    .into_iter()
                    .map(|(joint, flags)| {
                        let node = scratch.get(joint);
                        let mut aobj = node.aobj.clone();
                        if looping {
                            if let Some(aobj) = &mut aobj {
                                aobj.flags |= AOBJ_LOOP;
                            }
                        }
                        PreparedAttachment {
                            joint,
                            aobj,
                            classical_scale: flags & 1 != 0,
                            path_reference: node.path_reference,
                        }
                    })
                    .collect(),
            )
        })
        .collect()
}

/// The most tracks any prepared clock carries, for the model's reservation.
pub(super) fn maximum_tracks(selections: &PreparedSelections) -> usize {
    selections
        .iter()
        .flatten()
        .flatten()
        .filter_map(|attachment| attachment.aobj.as_ref())
        .map(|aobj| aobj.fobj.len())
        .max()
        .unwrap_or(0)
}

impl BackgroundAnimation {
    /// `attach_subtree(archive, 0, animations[index], loops[index])` from
    /// the prepared clocks; `false` when that animation could not be prepared.
    pub(super) fn attach_prepared(&mut self, index: usize) -> bool {
        let selections = std::sync::Arc::clone(&self.selections);
        let Some(Some(attachments)) = selections.get(index) else {
            return false;
        };
        for attachment in attachments {
            let joint = attachment.joint;
            self.tree
                .replace_aobj_prepared(joint, attachment.aobj.as_ref());
            if attachment.classical_scale {
                self.tree.set_flags(joint, JOBJ_CLASSICAL_SCALE);
            } else {
                self.tree.clear_flags(joint, JOBJ_CLASSICAL_SCALE);
            }
            self.tree.get_mut(joint).path_reference = attachment.path_reference;
        }
        let root = self.tree.bone(self.root, 0).expect("stage animation bone");
        self.tree.req_anim_all(root, 0.0);
        true
    }
}
