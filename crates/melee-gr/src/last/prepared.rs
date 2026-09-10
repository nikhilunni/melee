//! Preloaded stage joint clocks. Animation switches recycle track storage.
use super::animation::BackgroundAnimation;
use hsd_anim::{
    aobj::AObj,
    jobj::JObjId,
    load::{attach_anim_joint, load_joint_tree},
};
use hsd_archive::Archive;

impl BackgroundAnimation {
    /// Prepare every grAnime_801C7C1C joint animation before the tick loop.
    pub fn prepare_switches(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
    ) -> crate::desc::ReadResult<()> {
        let (mut tree, root) = load_joint_tree(archive, &model.joint)?;
        let mut maximum_tracks = 0;
        for (index, animation) in model.animations.iter().enumerate() {
            for &joint in &self.joints {
                tree.remove_anim(joint);
            }
            attach_anim_joint(&mut tree, root, animation, archive)?;
            let clocks = self
                .joints
                .iter()
                .map(|&joint| {
                    let mut clock = tree.get(joint).aobj.clone();
                    if let Some(a) = &mut clock {
                        maximum_tracks = maximum_tracks.max(a.fobj.len());
                        if model.animation_loops[index] {
                            a.flags |= hsd_anim::aobj::AOBJ_LOOP;
                        }
                    }
                    clock
                })
                .collect();
            self.prepared.push(clocks);
        }
        self.subtree_ends = self
            .joints
            .iter()
            .enumerate()
            .map(|(index, &joint)| {
                let mut count = 0;
                self.tree.walk_tree(joint, &mut |_, _| count += 1);
                index + count
            })
            .collect();
        self.reserve_events(maximum_tracks);
        Ok(())
    }

    /// grAnime_801C7FF8 / 801C8098: all descendants or one joint respectively.
    pub fn play_prepared(&mut self, bone: usize, index: usize, subtree: bool) {
        let end = if subtree {
            self.subtree_ends[bone]
        } else {
            bone + 1
        };
        for slot in bone..end {
            let joint = self.joints[slot];
            let Some(source) = &self.prepared[index][slot] else {
                self.tree.remove_anim(joint);
                continue;
            };
            let mut tracks = self.tree.take_animation_tracks(joint);
            tracks.clear();
            tracks.extend(source.fobj.iter().cloned());
            let mut clock = AObj {
                flags: source.flags,
                curr_frame: source.curr_frame,
                rewind_frame: source.rewind_frame,
                end_frame: source.end_frame,
                framerate: 1.0,
                fobj: tracks,
            };
            clock.req_anim(0.0);
            self.tree.get_mut(joint).aobj = Some(clock);
        }
    }
    /// grAnime_801C7980 clears looping; it does not stop playback.
    pub fn clear_joint_loop(&mut self, bone: usize) {
        if let Some(a) = &mut self.tree.get_mut(self.joints[bone]).aobj {
            a.clear_flags(hsd_anim::aobj::AOBJ_LOOP);
        }
    }
    /// grAnime_801C7A94 affects one joint's rate, without rewinding its clock.
    pub fn set_joint_rate(&mut self, bone: usize, rate: f32) {
        if let Some(a) = &mut self.tree.get_mut(self.joints[bone]).aobj {
            a.framerate = rate;
        }
    }
    pub fn joint_ended(&self, bone: usize) -> bool {
        self.tree
            .get(self.joints[bone])
            .aobj
            .as_ref()
            .is_none_or(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0)
    }
    pub fn joint_frame(&self, bone: usize) -> Option<f32> {
        self.tree
            .get(self.joints[bone])
            .aobj
            .as_ref()
            .map(|a| a.curr_frame)
    }
    pub fn joint_rewound(&self, bone: usize) -> bool {
        self.tree
            .get(self.joints[bone])
            .aobj
            .as_ref()
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_REWINDED != 0)
    }
    pub fn joint_matrix(&mut self, bone: usize) -> hsd_types::Mtx {
        *self.tree.get_mtx(self.joints[bone])
    }
    pub fn set_joint_scale(&mut self, bone: usize, scale: hsd_types::Vec3) {
        self.tree.set_scale(self.joints[bone], &scale);
    }
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }
    pub fn set_background_rotation(&mut self, pitch: f32, yaw: f32) {
        self.tree.set_rotation_x(self.root, pitch);
        self.tree.set_rotation_y(self.root, yaw);
    }
    /// Saved stage clocks have already evaluated their current frame.
    pub fn restore_joint_clock<T: hsd_anim::mtx::InverseTrig>(&mut self, bone: usize, frame: f32) {
        let joint: JObjId = self.joints[bone];
        if self.tree.get(joint).aobj.is_none() {
            return;
        }
        for _ in 0..=frame as usize {
            self.tree
                .anim::<T>(joint, &mut hsd_anim::aobj::AObjEndCallback::default());
        }
        self.tree.events.clear();
    }
}
