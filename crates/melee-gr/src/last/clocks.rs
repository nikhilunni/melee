//! Whole-model clock reads and writes Mute City's route uses.
use super::animation::BackgroundAnimation;
use hsd_anim::{
    aobj::{AObj, AOBJ_LOOP, AOBJ_REWINDED},
    jobj::{JObjId, JOBJ_INSTANCE},
};

impl BackgroundAnimation {
    /// Every joint, material and texture AObj of the model.
    fn for_each_aobj(&mut self, mut visit: impl FnMut(&mut AObj)) {
        for &joint in &self.joints {
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                visit(aobj);
            }
            if let Some(objects) = self.tree.dobj_mut(joint) {
                for object in objects {
                    if let Some(material) = &mut object.mobj {
                        if let Some(aobj) = &mut material.aobj {
                            visit(aobj);
                        }
                        for texture in &mut material.textures {
                            if let Some(aobj) = &mut texture.animation {
                                visit(aobj);
                            }
                        }
                    }
                }
            }
        }
    }
    /// `grAnime_801C775C(gobj, 0, 7, rewind, end)` (0x801C775C): every
    /// AObj loops (fn_801C6EE4: AOBJ_LOOP, then the rewind and end frames).
    pub fn set_loop_range(&mut self, rewind: f32, end: f32) {
        self.for_each_aobj(|aobj| {
            aobj.set_flags(AOBJ_LOOP);
            aobj.set_rewind_frame(rewind);
            aobj.set_end_frame(end);
        });
    }
    /// `grAnime_801C7A04(gobj, 0, 1, rate)` (0x801C7A04): the joint AObjs
    /// play at `rate` (HSD_AObjSetRate).
    pub fn set_joint_rates(&mut self, rate: f32) {
        for &joint in &self.joints {
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.set_rate(rate);
            }
        }
    }
    /// `grAnime_801C7A04(gobj, bone, 7, rate)`: every AObj of `bone`'s
    /// subtree.
    pub fn set_bone_rates(&mut self, bone: usize, rate: f32) {
        self.set_subtree_rate(bone, rate);
    }
    /// Ground_801C3F20 (0x801C3F20) on the GObj's JObj: the current frame
    /// of the first joint AObj in Ground_801C3E18's order (a joint, its
    /// children, then its next siblings), or zero.
    pub fn route_frame(&self) -> f32 {
        fn find(animation: &BackgroundAnimation, joint: Option<JObjId>) -> Option<f32> {
            let joint = joint?;
            if let Some(aobj) = &animation.tree.get(joint).aobj {
                return Some(aobj.curr_frame);
            }
            find(animation, animation.tree.child(joint))
                .or_else(|| find(animation, animation.tree.next(joint)))
        }
        find(self, Some(self.root)).unwrap_or(0.0)
    }
    /// lb_8000B134 (0x8000B134) from descendant `bone`: whether a joint
    /// AObj wrapped its loop this frame, walking from `bone` in preorder to
    /// the end of the whole tree (an instance or a leaf without siblings
    /// climbs to the next parent sibling).
    pub fn rewound_from(&self, bone: usize) -> bool {
        let tree = &self.tree;
        let mut cursor = Some(self.joints[bone]);
        while let Some(joint) = cursor {
            if tree
                .get(joint)
                .aobj
                .as_ref()
                .is_some_and(|a| a.flags & AOBJ_REWINDED != 0)
            {
                return true;
            }
            let child = tree.child(joint);
            let next = tree.next(joint);
            if (child.is_none() && next.is_none()) || tree.flags(joint) & JOBJ_INSTANCE != 0 {
                let mut up = joint;
                cursor = None;
                while let Some(parent) = tree.parent(up) {
                    if let Some(sibling) = tree.next(parent) {
                        cursor = Some(sibling);
                        break;
                    }
                    up = parent;
                }
            } else if child.is_some() {
                cursor = child;
            } else {
                cursor = next;
            }
        }
        false
    }
}
