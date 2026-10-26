//! Joint-track-only switches (grAnime_801C7C1C with only bit 0 of its type
//! mask): `grAnime_801C7FF8(gobj, bone, 1, ..)` replaces the joint tracks
//! of a bone's subtree, `grAnime_801C8098(gobj, bone, 1, ..)` those of the
//! bone alone; material clocks keep running. Like
//! [`BackgroundAnimation::prepare_subtree`], the switches are decoded at
//! load and replayed into the joints' reserved storage.
//!
//! grAnime_801C6A54_inner replaces a joint's AObj only when the new
//! AnimJoint has one (`animjoint->aobjdesc != NULL`): a joint the new
//! animation does not animate keeps its old clock, which the following
//! HSD_JObjReqAnim(All)ByFlags then restarts like the others.
use super::animation::BackgroundAnimation;
use hsd_anim::jobj::JObjId;
use hsd_archive::Archive;

impl BackgroundAnimation {
    /// Decode `grAnime_801C8098(gobj, bone, 1, index, ..)`: the bone's own
    /// track of animation `index`.
    pub fn prepare_joint_switch(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
        bone: usize,
        index: usize,
    ) -> crate::desc::ReadResult<()> {
        self.prepare_switch(archive, model, bone, index, true)
    }

    /// `grAnime_801C7FF8(gobj, bone, 1, index, frame, rate)` from the
    /// switch [`Self::prepare_subtree`] decoded: the subtree's joint tracks
    /// that the animation has replace theirs; every joint clock of the
    /// subtree is requested at `frame` and plays at `rate`.
    pub fn play_subtree_tracks(&mut self, bone: usize, index: usize, frame: f32, rate: f32) {
        self.replay_tracks(bone, index, false);
        for part in self.subtree_range(bone) {
            let joint = self.joints[part];
            self.restart_track(joint, frame, rate);
        }
    }

    /// `grAnime_801C8098(gobj, bone, 1, index, frame, rate)` from the
    /// switch [`Self::prepare_joint_switch`] decoded: the bone's own track,
    /// requested at `frame` (HSD_JObjReqAnimByFlags) at `rate`.
    pub fn play_joint_track(&mut self, bone: usize, index: usize, frame: f32, rate: f32) {
        self.replay_tracks(bone, index, true);
        let joint = self.joints[bone];
        self.restart_track(joint, frame, rate);
    }

    /// `grAnime_801C78FC(gobj, bone, 1)` (0x801C78FC): the subtree's joint
    /// clocks stop looping.
    pub fn clear_subtree_loops(&mut self, bone: usize) {
        for part in self.subtree_range(bone) {
            let joint = self.joints[part];
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.clear_flags(hsd_anim::aobj::AOBJ_LOOP);
            }
        }
    }

    /// `grAnime_801C7B24(gobj, bone, 1, frame)` (0x801C7B24): the subtree's
    /// joint clocks are requested at `frame` (HSD_JObjReqAnimAllByFlags).
    pub fn request_subtree_frame(&mut self, bone: usize, frame: f32) {
        for part in self.subtree_range(bone) {
            let joint = self.joints[part];
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.req_anim(frame);
            }
        }
    }

    /// `grAnime_801C83D0(gobj, bone, 1)` (0x801C83D0): the first joint
    /// clock of the subtree (HSD_ForeachAnim's order: the joint, then its
    /// children) has ended. No clock reads as not ended.
    pub fn subtree_track_ended(&self, bone: usize) -> bool {
        let mut found = None;
        self.tree.walk_tree(self.joints[bone], &mut |joint, _| {
            if found.is_none() {
                found = self
                    .tree
                    .get(joint)
                    .aobj
                    .as_ref()
                    .map(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0);
            }
        });
        found.unwrap_or(false)
    }

    /// `grAnime_801C78FC(gobj, 0, 7)`'s material half: every material and
    /// texture clock stops looping.
    pub fn clear_material_loops(&mut self) {
        for part in 0..self.joints.len() {
            let joint = self.joints[part];
            if let Some(objects) = self.tree.dobj_mut(joint) {
                for object in objects {
                    if let Some(material) = &mut object.mobj {
                        if let Some(aobj) = &mut material.aobj {
                            aobj.clear_flags(hsd_anim::aobj::AOBJ_LOOP);
                        }
                        for texture in &mut material.textures {
                            if let Some(aobj) = &mut texture.animation {
                                aobj.clear_flags(hsd_anim::aobj::AOBJ_LOOP);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The subtree's descendant indices, which follow `bone` in walk order.
    fn subtree_range(&self, bone: usize) -> std::ops::Range<usize> {
        let mut count = 0;
        self.tree
            .walk_tree(self.joints[bone], &mut |_, _| count += 1);
        bone..bone + count
    }

    fn restart_track(&mut self, joint: JObjId, frame: f32, rate: f32) {
        if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
            aobj.req_anim(frame);
            aobj.set_rate(rate);
        }
    }
}
