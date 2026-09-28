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
            self.play_materials(index, slot, slot + 1);
            let joint = self.joints[slot];
            let Some(source) = &self.prepared[index][slot] else {
                self.tree.remove_anim_by_flags(joint, 1);
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
    pub fn joint_scale(&self, bone: usize) -> hsd_types::Vec3 {
        self.tree.scale(self.joints[bone])
    }
    /// `HSD_JObjGetTranslation` of a model bone.
    pub fn joint_translation(&self, bone: usize) -> hsd_types::Vec3 {
        self.tree.translation(self.joints[bone])
    }
    /// `HSD_JObjSetTranslate` of a model bone.
    pub fn set_joint_translate(&mut self, bone: usize, translate: hsd_types::Vec3) {
        self.tree.set_translate(self.joints[bone], &translate);
    }
    /// `HSD_JObjSetTranslateY` of a model bone.
    pub fn set_joint_translate_y(&mut self, bone: usize, y: f32) {
        self.tree.set_translate_y(self.joints[bone], y);
    }
    /// `HSD_JObjGetFlags(bone) & JOBJ_HIDDEN`.
    pub fn joint_hidden(&self, bone: usize) -> bool {
        self.tree.flags(self.joints[bone]) & hsd_anim::jobj::JOBJ_HIDDEN != 0
    }
    /// `HSD_JObjSetFlagsAll` / `HSD_JObjClearFlagsAll(bone, JOBJ_HIDDEN)`.
    pub fn set_joint_hidden(&mut self, bone: usize, hidden: bool) {
        let joint = self.joints[bone];
        if hidden {
            self.tree.set_flags_all(joint, hsd_anim::jobj::JOBJ_HIDDEN);
        } else {
            self.tree
                .clear_flags_all(joint, hsd_anim::jobj::JOBJ_HIDDEN);
        }
    }
    /// `HSD_JObjSetTranslate` on the Ground GObj's JObj (the map-scale wrapper).
    pub fn set_gobj_translate(&mut self, translate: hsd_types::Vec3) {
        let joint = self.tree.parent(self.root).unwrap_or(self.root);
        self.tree.set_translate(joint, &translate);
    }
    /// `HSD_JObjGetTranslation` of the Ground GObj's JObj.
    pub fn gobj_translation(&self) -> hsd_types::Vec3 {
        self.tree
            .translation(self.tree.parent(self.root).unwrap_or(self.root))
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

impl BackgroundAnimation {
    /// Prepared programs for one display object, across all stage phases.
    pub fn material_programs(
        &self,
        joint: JObjId,
        display: usize,
    ) -> impl Iterator<Item = &hsd_anim::material_playback::PreparedMaterial> {
        let slot = self.joints.iter().position(|&id| id == joint);
        self.prepared_materials.iter().filter_map(move |programs| {
            programs
                .get(slot?)
                .and_then(|materials| materials.get(display))
        })
    }
    pub(super) fn prepare_material_switches(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
    ) -> crate::desc::ReadResult<()> {
        for (index, &offset) in model.material_animation_offsets.iter().enumerate() {
            let desc = hsd_archive::desc::MatAnimJoint::read(archive, offset)?;
            let animation = hsd_anim::load::material_animation(archive, &desc)?;
            let mut programs = Vec::with_capacity(self.joints.len());
            collect_materials(&self.tree, self.root, Some(&animation), &mut programs);
            assert_eq!(programs.len(), self.joints.len());
            for (slot, materials) in programs.iter_mut().enumerate() {
                if let Some(objects) = self.tree.dobj_mut(self.joints[slot]) {
                    for (object, program) in objects.iter_mut().zip(materials) {
                        if let Some(material) = &mut object.mobj {
                            program.set_loop(model.animation_loops.get(index) == Some(&true));
                            program.reserve(material);
                        }
                    }
                }
            }
            self.prepared_materials.push(programs);
        }
        Ok(())
    }
    pub(super) fn play_materials(&mut self, index: usize, start: usize, end: usize) {
        let Some(programs) = self.prepared_materials.get(index) else {
            return;
        };
        for (slot, materials) in programs.iter().enumerate().take(end).skip(start) {
            if let Some(objects) = self.tree.dobj_mut(self.joints[slot]) {
                for (object, program) in objects.iter_mut().zip(materials) {
                    if let Some(material) = &mut object.mobj {
                        program.apply(material, 0.0);
                    }
                }
            }
        }
    }
}
fn collect_materials(
    tree: &hsd_anim::jobj::JObjTree,
    joint: JObjId,
    animation: Option<&hsd_anim::jobj::MatAnimJoint>,
    output: &mut Vec<Vec<hsd_anim::material_playback::PreparedMaterial>>,
) {
    use hsd_anim::material_playback::PreparedMaterial;
    output.push(animation.map_or_else(Vec::new, |a| {
        a.matanim.iter().map(PreparedMaterial::new).collect()
    }));
    let mut child = tree.get(joint).child;
    let mut animations = animation.map(|a| a.children.iter());
    while let Some(id) = child {
        collect_materials(
            tree,
            id,
            animations.as_mut().and_then(Iterator::next),
            output,
        );
        child = tree.get(id).next;
    }
}
