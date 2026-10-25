//! Whole-model animation switches (grAnime_801C8138) decoded once at load.
//! A switch in the tick replays the prepared clocks into the model's
//! reserved track storage instead of decoding the AnimJoint again.
//! Subtree switches (grAnime_801C7FF8) are prepared the same way, on request.
use super::animation::BackgroundAnimation;
use hsd_anim::{
    aobj::{AObj, AOBJ_LOOP},
    jobj::{JObjId, JObjTree, JOBJ_CLASSICAL_SCALE},
    load::{anim_joint_targets, attach_anim_joint},
};
use hsd_archive::Archive;

/// What `HSD_JObjAddAnim` (jobj.c:298-313) leaves on one joint.
#[derive(Clone)]
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

/// One `grAnime_801C7FF8(gobj, bone, 7, index, 0.0, 1.0)` subtree switch
/// decoded at load, so the tick replays it without reading the archive.
#[derive(Clone)]
pub(super) struct PreparedSubtree {
    bone: usize,
    index: usize,
    attachments: Vec<PreparedAttachment>,
}
pub(super) type PreparedSubtrees = std::sync::Arc<[PreparedSubtree]>;

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
            prepare_attachments(&mut scratch, joints, bone, animation, looping, archive)
        })
        .collect()
}

/// `attach_subtree(archive, bone, animation, looping)` on `scratch` with
/// every joint animation removed first: the clocks it leaves, in
/// `attach_anim_joint` order.
fn prepare_attachments(
    scratch: &mut JObjTree,
    joints: &[JObjId],
    bone: JObjId,
    animation: &hsd_archive::desc::AnimJoint,
    looping: bool,
    archive: &Archive,
) -> Option<Vec<PreparedAttachment>> {
    for &joint in joints {
        scratch.remove_anim(joint);
    }
    let targets = anim_joint_targets(scratch, bone, animation).ok()?;
    attach_anim_joint(scratch, bone, animation, archive).ok()?;
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
}

/// The most tracks any prepared clock carries, for the model's reservation.
pub(super) fn maximum_tracks(
    selections: &PreparedSelections,
    subtrees: &PreparedSubtrees,
) -> usize {
    selections
        .iter()
        .flatten()
        .chain(subtrees.iter().map(|subtree| &subtree.attachments))
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
        self.replay_attachments(attachments);
        let root = self.tree.bone(self.root, 0).expect("stage animation bone");
        self.tree.req_anim_all(root, 0.0);
        true
    }

    /// Decode `grAnime_801C7FF8(gobj, bone, 7, index, 0.0, 1.0)` at load
    /// for [`Self::play_prepared_subtree`]: animation `index`'s AnimJoint
    /// for `bone` (`desc::animation_subtree`), attached as
    /// [`Self::attach_subtree`] attaches it.
    pub fn prepare_subtree(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
        bone: usize,
        index: usize,
    ) -> crate::desc::ReadResult<()> {
        let animation = crate::desc::animation_subtree(archive, model, index, bone as u32)?;
        let mut scratch = (*self.rest).clone();
        let root = scratch.bone(self.root, bone).expect("stage animation bone");
        let attachments = prepare_attachments(
            &mut scratch,
            &self.joints,
            root,
            &animation,
            model.animation_loops[index],
            archive,
        )
        .expect("a stage subtree animation attaches to its own model");
        let mut subtrees: Vec<PreparedSubtree> = self
            .subtrees
            .iter()
            .filter(|s| (s.bone, s.index) != (bone, index))
            .cloned()
            .collect();
        subtrees.push(PreparedSubtree {
            bone,
            index,
            attachments,
        });
        self.subtrees = subtrees.into();
        self.reserve_events(maximum_tracks(&self.selections, &self.subtrees));
        Ok(())
    }

    /// `grAnime_801C7FF8(gobj, bone, 7, index, 0.0, 1.0)` (0x801C7FF8) from
    /// the clocks [`Self::prepare_subtree`] decoded: the joint animations of
    /// `bone`'s subtree, requested at frame zero, then its material
    /// animations. Panics when that switch was not prepared at load.
    pub fn play_prepared_subtree(&mut self, bone: usize, index: usize) {
        let subtrees = std::sync::Arc::clone(&self.subtrees);
        let subtree = subtrees
            .iter()
            .find(|s| (s.bone, s.index) == (bone, index))
            .expect("a subtree switch prepared at load");
        self.replay_attachments(&subtree.attachments);
        let root = self
            .tree
            .bone(self.root, bone)
            .expect("stage animation bone");
        self.tree.req_anim_all(root, 0.0);
        self.play_material_subtree(bone, index);
    }

    /// `grAnime_801C7FF8(gobj, bone, 7, index, frame, rate)` (0x801C7FF8)
    /// from the clocks [`Self::prepare_subtree`] decoded: as
    /// [`Self::play_subtree`], without reading the archive.
    pub fn play_prepared_subtree_at(&mut self, bone: usize, index: usize, frame: f32, rate: f32) {
        self.play_prepared_subtree(bone, index);
        let root = self.joints[bone];
        self.tree.req_anim_all(root, frame);
        self.set_subtree_rate(bone, rate);
    }

    /// What `attach_anim_joint` leaves on each joint, replayed into the
    /// joints' reserved track storage.
    fn replay_attachments(&mut self, attachments: &[PreparedAttachment]) {
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
    }
}
