//! Ground animation and grLib_801C99C0 particle callback, shared with Battlefield.
//! The scene owns particle allocation; this crate owns stage keys and joints.
use hsd_anim::{
    jobj::{JObjEvent, JObjId, JObjTree},
    load::{attach_anim_joint, load_joint_tree},
    mtx::InverseTrig,
};
use hsd_archive::Archive;
use hsd_types::Mtx;

pub struct BackgroundAnimation {
    pub overlay: melee_lb::color_overlay::ColorOverlay,
    pub overlay_script: Option<usize>,
    pub(super) rest: std::sync::Arc<JObjTree>,
    pub(super) tree: JObjTree,
    pub(super) root: JObjId,
    // Animation replacement preserves the model hierarchy and these identities.
    pub(super) joints: Vec<JObjId>,
    requests: Vec<ParticleRequest>,
    pub(super) prepared: Vec<Vec<Option<hsd_anim::aobj::AObj>>>,
    pub(super) prepared_materials: Vec<Vec<Vec<hsd_anim::material_playback::PreparedMaterial>>>,
    pub(super) subtree_ends: Vec<usize>,
    /// Every model animation decoded at load for `select_animation`.
    pub(super) selections: super::selection::PreparedSelections,
    /// Subtree switches decoded at load for `play_prepared_subtree`.
    pub(super) subtrees: super::selection::PreparedSubtrees,
}
/// grLib_801C99C0 (grlib.c:151-158): DPtcl calls hsd_8039EFAC
/// with link 0 and the animation's bank, kind and attachment joint.
#[derive(Clone)]
pub struct ParticleRequest {
    pub bank: u8,
    pub kind: u32,
    pub joint: usize,
    pub matrix: Mtx,
}
impl BackgroundAnimation {
    /// Ground_GetStageGObj (ground.c:835ff), map_id beyond the authored model count:
    /// an identity joint under the ordinary map-scale wrapper, reserved before birth.
    pub fn identity_root(archive: &Archive, map_scale: f32) -> crate::desc::ReadResult<Self> {
        let mut joint = hsd_archive::desc::JObjDesc::default();
        joint.scale = hsd_archive::desc::Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };
        let model = crate::desc::ModelDesc {
            present: true,
            joint,
            animations: Vec::new(),
            animation_loops: Vec::new(),
            joint_mappings: Vec::new(),
            material_animation_offsets: Vec::new(),
            shape_animation_offsets: Vec::new(),
            light_list_offset: None,
            fog_offset: None,
        };
        let mut result = Self::load_model(archive, &model)?;
        result.set_map_scale(map_scale);
        Ok(result)
    }

    /// Read-only pose source for presentation scratch. Matrix evaluation must
    /// happen in the caller's scratch so rendering cannot affect continuation.
    pub fn pose_tree(&self) -> &JObjTree {
        &self.tree
    }
    /// grLast_8021B920 (grlast.c:789ff), LayeredStart: model 4, animation 0.
    /// Request frame zero without evaluating it until Ground's s_link 1 proc.
    pub fn load(archive: &Archive, desc: &crate::desc::StageDesc) -> crate::desc::ReadResult<Self> {
        Self::load_model(archive, &desc.models[4])
    }
    /// `grAnime_801C8138` (0x801C8138): attach a map model animation.
    pub fn load_model(
        archive: &Archive,
        model: &crate::desc::ModelDesc,
    ) -> crate::desc::ReadResult<Self> {
        let (mut tree, root) = load_joint_tree(archive, &model.joint)?;
        let rest = std::sync::Arc::new(tree.clone());
        if let Some(animation) = model.animations.first() {
            attach_anim_joint(&mut tree, root, animation, archive)?;
        }
        if model.animation_loops.first() == Some(&true) {
            let mut joints = Vec::new();
            tree.walk_tree(root, &mut |joint, _| joints.push(joint));
            for joint in joints {
                if let Some(aobj) = &mut tree.get_mut(joint).aobj {
                    aobj.flags |= hsd_anim::aobj::AOBJ_LOOP;
                }
            }
        }
        tree.req_anim_all(root, 0.0);
        let mut joints = Vec::new();
        tree.walk_tree(root, &mut |joint, _| joints.push(joint));
        // At most one callback per loaded FObj in one animation evaluation.
        let requests = Vec::with_capacity(tree.events.capacity());
        let selections = super::selection::prepare(&rest, root, &joints, archive, model);
        let mut result = Self {
            rest,
            overlay: Default::default(),
            overlay_script: None,
            tree,
            root,
            joints,
            requests,
            prepared: Vec::new(),
            prepared_materials: Vec::new(),
            subtree_ends: Vec::new(),
            selections,
            subtrees: std::sync::Arc::from([]),
        };
        result.reserve_events(super::selection::maximum_tracks(
            &result.selections,
            &result.subtrees,
        ));
        result.prepare_material_switches(archive, model)?;
        result.play_materials(0, 0, result.joints.len());
        Ok(result)
    }
    pub(super) fn reserve_events(&mut self, tracks: usize) {
        self.tree.reserve_animation_tracks(tracks);
        self.requests.reserve(self.tree.events.capacity());
    }
    /// grAnime_801C8138: replace all joint tracks without replacing the model.
    pub fn select_animation(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
        index: usize,
    ) -> crate::desc::ReadResult<()> {
        self.clear_animation();
        if !self.attach_prepared(index) {
            self.attach_subtree(
                archive,
                0,
                &model.animations[index],
                model.animation_loops[index],
            )?;
        }
        self.play_materials(index, 0, self.joints.len());
        // Replacement frame-zero particle requests read the outgoing pose's
        // matrix cache. Materialize those targets before evaluating new SRTs;
        // unattached joints may have remained dirty throughout the old animation.
        for &joint in &self.joints {
            if self.tree.get(joint).aobj.as_ref().is_some_and(|a| {
                a.fobj
                    .iter()
                    .any(|f| f.obj_type == hsd_anim::jobj::HSD_A_J_DPTCL)
            }) {
                self.tree.setup_matrix(joint);
            }
        }
        Ok(())
    }
    pub fn clear_animation(&mut self) {
        for &joint in &self.joints {
            self.tree.remove_anim_by_flags(joint, 1);
            if let Some(objects) = self.tree.dobj_mut(joint) {
                for object in objects {
                    if let Some(material) = &mut object.mobj {
                        material.clear_prepared_animation();
                    }
                }
            }
        }
    }
    /// grAnime_801C83D0: completion flag of the first joint AObj.
    pub fn ended(&self) -> bool {
        let mut result = None;
        self.tree.walk_tree(self.root, &mut |joint, _| {
            if result.is_none() {
                result = self
                    .tree
                    .get(joint)
                    .aobj
                    .as_ref()
                    .map(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0);
            }
        });
        result.unwrap_or(false)
    }
    /// `grAnime_801C84A4(gobj, 0, 1)` (0x801C84A4): the first joint AObj
    /// (as [`Self::ended`]) wrapped its loop this frame.
    pub fn rewound(&self) -> bool {
        let mut result = None;
        self.tree.walk_tree(self.root, &mut |joint, _| {
            if result.is_none() {
                result = self
                    .tree
                    .get(joint)
                    .aobj
                    .as_ref()
                    .map(|a| a.flags & hsd_anim::aobj::AOBJ_REWINDED != 0);
            }
        });
        result.unwrap_or(false)
    }
    /// grAnime_801C78FC(gobj, 0, mask) (0x801C78FC): every joint
    /// animation stops looping (HSD_AObjClearFlags(AOBJ_LOOP)).
    pub fn clear_loops(&mut self) {
        for bone in 0..self.joints.len() {
            self.clear_joint_loop(bone);
        }
    }
    /// `grAnime_801C83D0(gobj, 0, 7)` (0x801C83D0): `HSD_ForeachAnim` over
    /// every animation type from the archive root stops at the first AObj
    /// (JObjForeachAnim order: joint, then its DObjs' own, material and
    /// texture AObjs, then children) and tests `AOBJ_NO_ANIM`. No AObj reads
    /// as not ended. The loaded stage models carry no RObj or shape AObjs.
    pub fn first_animation_ended(&self) -> bool {
        self.first_aobj(self.root)
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0)
    }
    /// `grAnime_801C84A4(gobj, 0, 7)` (0x801C84A4): the same AObj as
    /// [`Self::first_animation_ended`], tested for `AOBJ_REWINDED` (a loop
    /// wrapped this frame).
    pub fn first_animation_rewound(&self) -> bool {
        self.first_aobj(self.root)
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_REWINDED != 0)
    }
    /// `grAnime_801C83D0(gobj, bone, 7)` (0x801C83D0): as
    /// [`Self::first_animation_ended`] from descendant `bone`.
    pub fn subtree_animation_ended(&self, bone: usize) -> bool {
        self.first_aobj(self.joints[bone])
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0)
    }
    /// The end frame of the AObj `first_animation_ended` tests.
    pub fn first_animation_end_frame(&self) -> Option<f32> {
        self.first_aobj(self.root).map(|a| a.end_frame)
    }
    /// `grAnime_801C7FF8(gobj, bone, 2, index, 0.0, 1.0)` (0x801C7FF8): the
    /// material animations of model animation `index` replace those of
    /// `bone`'s subtree from frame zero; joint animations keep running.
    pub fn play_material_subtree(&mut self, bone: usize, index: usize) {
        let mut count = 0;
        self.tree.walk_tree(self.joints[bone], &mut |_, _| count += 1);
        self.play_materials(index, bone, bone + count);
    }
    /// `grAnime_801C83D0(gobj, bone, 2)` (0x801C83D0): `HSD_ForeachAnim`
    /// over `bone`'s subtree, material and texture AObjs only, stops at the
    /// first one and tests `AOBJ_NO_ANIM`. No AObj reads as not ended.
    pub fn material_subtree_ended(&self, bone: usize) -> bool {
        self.first_material_aobj(self.joints[bone])
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0)
    }
    fn first_material_aobj(&self, joint: JObjId) -> Option<&hsd_anim::aobj::AObj> {
        let node = self.tree.get(joint);
        for object in &node.dobj {
            if let Some(material) = &object.mobj {
                if let Some(aobj) = &material.aobj {
                    return Some(aobj);
                }
                if let Some(aobj) = material.textures.iter().find_map(|t| t.animation.as_ref()) {
                    return Some(aobj);
                }
            }
        }
        if node.flags & hsd_anim::jobj::JOBJ_INSTANCE != 0 {
            return None;
        }
        let mut child = self.tree.child(joint);
        while let Some(id) = child {
            if let Some(aobj) = self.first_material_aobj(id) {
                return Some(aobj);
            }
            child = self.tree.next(id);
        }
        None
    }
    fn first_aobj(&self, joint: JObjId) -> Option<&hsd_anim::aobj::AObj> {
        let node = self.tree.get(joint);
        if let Some(aobj) = &node.aobj {
            return Some(aobj);
        }
        for object in &node.dobj {
            if let Some(aobj) = &object.aobj {
                return Some(aobj);
            }
            if let Some(material) = &object.mobj {
                if let Some(aobj) = &material.aobj {
                    return Some(aobj);
                }
                if let Some(aobj) = material.textures.iter().find_map(|t| t.animation.as_ref()) {
                    return Some(aobj);
                }
            }
        }
        if node.flags & hsd_anim::jobj::JOBJ_INSTANCE != 0 {
            return None;
        }
        let mut child = self.tree.child(joint);
        while let Some(id) = child {
            if let Some(aobj) = self.first_aobj(id) {
                return Some(aobj);
            }
            child = self.tree.next(id);
        }
        None
    }
    /// The Ground GObj's JObj: the map-scale wrapper when present.
    fn gobj_joint(&self) -> JObjId {
        self.tree.parent(self.root).unwrap_or(self.root)
    }
    /// `HSD_JObjGetFlags(gobj jobj) & JOBJ_HIDDEN`.
    pub fn hidden(&self) -> bool {
        self.tree.flags(self.gobj_joint()) & hsd_anim::jobj::JOBJ_HIDDEN != 0
    }
    /// `HSD_JObjSetFlagsAll` / `HSD_JObjClearFlagsAll(gobj jobj, JOBJ_HIDDEN)`.
    pub fn set_hidden(&mut self, hidden: bool) {
        let joint = self.gobj_joint();
        if hidden {
            self.tree.set_flags_all(joint, hsd_anim::jobj::JOBJ_HIDDEN);
        } else {
            self.tree.clear_flags_all(joint, hsd_anim::jobj::JOBJ_HIDDEN);
        }
    }
    /// `HSD_JObjGetScaleY(gobj jobj)`.
    pub fn wrapper_scale_y(&self) -> f32 {
        self.tree.scale_y(self.gobj_joint())
    }
    /// `HSD_JObjSetScaleY(gobj jobj, y)`.
    pub fn set_wrapper_scale_y(&mut self, y: f32) {
        let joint = self.gobj_joint();
        self.tree.set_scale_y(joint, y);
    }
    /// `HSD_JObjSetTranslateY(gobj jobj, y)`.
    pub fn set_wrapper_translate_y(&mut self, y: f32) {
        let joint = self.gobj_joint();
        self.tree.set_translate_y(joint, y);
    }
    /// `HSD_JObjSetFlagsAll(gobj jobj, flags)`.
    pub fn set_wrapper_flags_all(&mut self, flags: u32) {
        let joint = self.gobj_joint();
        self.tree.set_flags_all(joint, flags);
    }
    /// `grAnime_801C7A04(gobj, 0, 7, rate)` (0x801C7A04): every joint,
    /// material and texture AObj of the model plays at `rate`
    /// (HSD_AObjSetRate).
    pub fn set_rate(&mut self, rate: f32) {
        for &joint in &self.joints {
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.set_rate(rate);
            }
            if let Some(objects) = self.tree.dobj_mut(joint) {
                for object in objects {
                    if let Some(material) = &mut object.mobj {
                        if let Some(aobj) = &mut material.aobj {
                            aobj.set_rate(rate);
                        }
                        for texture in &mut material.textures {
                            if let Some(aobj) = &mut texture.animation {
                                aobj.set_rate(rate);
                            }
                        }
                    }
                }
            }
        }
    }
    /// `grAnime_801C7B24(gobj, 0, 7, frame)` (0x801C7B24): every joint,
    /// material and texture AObj of the model is requested at `frame`
    /// (HSD_JObjReqAnimAllByFlags).
    pub fn request_frame(&mut self, frame: f32) {
        for &joint in &self.joints {
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.req_anim(frame);
            }
            if let Some(objects) = self.tree.dobj_mut(joint) {
                for object in objects {
                    if let Some(material) = &mut object.mobj {
                        if let Some(aobj) = &mut material.aobj {
                            aobj.req_anim(frame);
                        }
                        for texture in &mut material.textures {
                            if let Some(aobj) = &mut texture.animation {
                                aobj.req_anim(frame);
                            }
                        }
                    }
                }
            }
        }
    }
    /// `HSD_JObjAddRotationZ` on a descendant (Ground_801C3FA4 index).
    pub fn add_rotation_z(&mut self, bone: usize, delta: f32) {
        let joint = self.joints[bone];
        let z = self.tree.rotation_z(joint);
        self.tree.set_rotation_z(joint, z + delta);
    }
    /// Read-only: the JObj behind a Ground_801C3FA4 descendant index.
    pub fn joint_id(&self, bone: usize) -> Option<JObjId> {
        self.joints.get(bone).copied()
    }
    /// The descendant indices of `bone`'s subtree in HSD_JObjWalkTree order
    /// (the bone first, instances not entered).
    pub fn subtree(&self, bone: usize) -> Vec<usize> {
        let mut result = Vec::new();
        self.tree.walk_tree(self.joints[bone], &mut |joint, _| {
            result.push(self.joints.iter().position(|&j| j == joint).unwrap());
        });
        result
    }
    /// `grAnime_801C7A04(gobj, 0, 7, 0.0)` (0x801C7A04): every joint
    /// animation of the model stops advancing. Material and shape clocks
    /// only affect drawing.
    pub fn freeze(&mut self) {
        for &joint in &self.joints {
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.framerate = 0.0;
            }
        }
    }
    /// `Ground_GetStageGObj` (0x801C14D0), ground.c:889-908: map-scale
    /// wrapper above the archive root. Matrix products use audited HSD kernels.
    pub fn set_map_scale(&mut self, scale: f32) {
        let wrapper = self.tree.alloc();
        self.tree
            .set_scale(wrapper, &hsd_types::Vec3::new(scale, scale, scale));
        self.tree.add_child(wrapper, self.root);
    }
    /// Rebuild animation interpreter state through the saved current frame.
    /// `HSD_AObjInterpretAnim` (0x80364190): current frame is already evaluated.
    pub fn restore_frame<T: InverseTrig>(&mut self, frame: f32) {
        assert!(frame >= 0.0 && frame.fract() == 0.0);
        for _ in 0..=frame as usize {
            self.tick::<T>();
        }
    }
    /// grAnime_801C7FF8: replace an animated subtree, preserving siblings.
    pub fn attach_subtree(
        &mut self,
        archive: &Archive,
        bone: usize,
        animation: &hsd_archive::desc::AnimJoint,
        looping: bool,
    ) -> crate::desc::ReadResult<()> {
        let root = self
            .tree
            .bone(self.root, bone)
            .expect("stage animation bone");
        attach_anim_joint(&mut self.tree, root, animation, archive)?;
        if looping {
            let mut joints = Vec::new();
            self.tree
                .walk_tree(root, &mut |joint, _| joints.push(joint));
            for joint in joints {
                if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                    aobj.flags |= hsd_anim::aobj::AOBJ_LOOP;
                }
            }
        }
        self.tree.req_anim_all(root, 0.0);
        // A load-time attachment brings fresh track storage; keep it
        // reserved for the prepared switches that later reuse it.
        self.reserve_events(super::selection::maximum_tracks(
            &self.selections,
            &self.subtrees,
        ));
        Ok(())
    }
    /// `grAnime_801C7FF8(gobj, bone, 7, index, frame, rate)` (0x801C7FF8):
    /// model animation `index`'s joint and material tracks replace those
    /// of `bone`'s subtree (grAnime_801C7C1C with every type), requested at
    /// `frame` (HSD_JObjReqAnimAllByFlags) and played at `rate`
    /// (grAnime_801C752C's HSD_AObjSetRate). Material clocks start at zero:
    /// they only affect drawing.
    pub fn play_subtree(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
        bone: usize,
        index: usize,
        frame: f32,
        rate: f32,
    ) -> crate::desc::ReadResult<()> {
        let subtree = crate::desc::animation_subtree(archive, model, index, bone as u32)?;
        self.attach_subtree(archive, bone, &subtree, model.animation_loops[index])?;
        let root = self.joints[bone];
        self.tree.req_anim_all(root, frame);
        self.play_material_subtree(bone, index);
        self.set_subtree_rate(bone, rate);
        Ok(())
    }
    /// grAnime_801C752C's HSD_AObjSetRate over `bone`'s subtree: its joint,
    /// material and texture clocks. The subtree's descendant indices follow
    /// `bone` in walk order (as in [`Self::play_material_subtree`]).
    pub(super) fn set_subtree_rate(&mut self, bone: usize, rate: f32) {
        let mut count = 0;
        self.tree.walk_tree(self.joints[bone], &mut |_, _| count += 1);
        for part in bone..bone + count {
            let joint = self.joints[part];
            if let Some(aobj) = &mut self.tree.get_mut(joint).aobj {
                aobj.set_rate(rate);
            }
            if let Some(objects) = self.tree.dobj_mut(joint) {
                for object in objects {
                    if let Some(material) = &mut object.mobj {
                        if let Some(aobj) = &mut material.aobj {
                            aobj.set_rate(rate);
                        }
                        for texture in &mut material.textures {
                            if let Some(aobj) = &mut texture.animation {
                                aobj.set_rate(rate);
                            }
                        }
                    }
                }
            }
        }
    }
    /// `grAnime_801C84A4(gobj, bone, 7)` (0x801C84A4): the first AObj of
    /// `bone`'s subtree (grAnime_801C8318) wrapped its loop this frame.
    pub fn subtree_animation_rewound(&self, bone: usize) -> bool {
        self.first_aobj(self.joints[bone])
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_REWINDED != 0)
    }
    /// `lbGetJObjEndFrame` on a bone: the end frame of the first joint
    /// AObj lbFindJObjWithAObj meets (the bone, its children, then its
    /// next siblings), else 0.
    pub fn subtree_joint_end_frame(&self, bone: usize) -> f32 {
        fn find(tree: &JObjTree, joint: Option<JObjId>) -> Option<f32> {
            let joint = joint?;
            if let Some(aobj) = &tree.get(joint).aobj {
                return Some(aobj.end_frame);
            }
            find(tree, tree.child(joint)).or_else(|| find(tree, tree.next(joint)))
        }
        find(&self.tree, Some(self.joints[bone])).unwrap_or(0.0)
    }
    /// Ground_801C2FE0 -> mpLib_80055E9C: bind each model's collision
    /// joint to its animated descendant, preserving hidden state and history.
    pub fn update_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        bindings: &[crate::desc::JointMapping],
    ) {
        for binding in bindings {
            let joint = self
                .tree
                .bone(self.root, binding.extra as usize)
                .expect("collision bone");
            let transform = melee_mp::JobjState {
                mtx: *self.tree.get_mtx(joint),
                hidden: self.tree.flags(joint) & hsd_anim::jobj::JOBJ_HIDDEN != 0,
            };
            map.update_joint_transform(i32::from(binding.joint_index), Some(transform));
        }
    }
    pub fn matrices(&mut self) -> Vec<(usize, Mtx)> {
        let mut matrices = Vec::with_capacity(self.joints.len());
        self.for_each_matrix(|joint, matrix| matrices.push((joint, matrix)));
        matrices
    }
    /// Visit in the same model traversal order without building a per-tick list.
    pub fn for_each_matrix(&mut self, mut visit: impl FnMut(usize, Mtx)) {
        for &joint in &self.joints {
            self.tree.setup_matrix(joint);
            visit(joint.0, self.tree.get(joint).mtx);
        }
    }
    /// Ground_801C1CD0 (ground.c): evaluate the stage animation at s_link 1.
    pub fn tick<T: InverseTrig>(&mut self) -> &[ParticleRequest] {
        self.evaluate::<T>(true)
    }
    /// grAnime_801C8138 evaluates frame zero during Ground creation.
    /// grLib_801C99C0 attaches the JObj pointer without requesting its matrix;
    /// retain the newly loaded cache until the first scheduler update.
    pub fn evaluate_initial_frame<T: InverseTrig>(&mut self) -> &[ParticleRequest] {
        self.evaluate::<T>(false)
    }
    fn evaluate<T: InverseTrig>(&mut self, refresh_matrices: bool) -> &[ParticleRequest] {
        self.tree.anim_all::<T>(self.root);
        self.requests.clear();
        let mut events = std::mem::take(&mut self.tree.events);
        for event in events.drain(..) {
            match event {
                JObjEvent::DPtcl { jobj, lo, hi } => {
                    if refresh_matrices {
                        self.tree.setup_matrix(jobj);
                    }
                    self.requests.push(ParticleRequest {
                        bank: lo as u8,
                        kind: hi as u32,
                        joint: jobj.0,
                        matrix: self.tree.get(jobj).mtx,
                    });
                }
                // A path track without a resolved spline reference: retail's
                // JObjUpdateFunc asserts the AObj's object (jobj.c:369-372).
                // Every stage map animation on the disc names its spline
                // joint (COVERAGE_AUDIT.md, wave 13), so attach_anim_joint
                // always resolves it.
                JObjEvent::Path { .. } => {
                    unreachable!("jobj.c:371: HSD_ASSERT(jp) for an unreferenced path track")
                }
                _ => {}
            }
        }
        self.tree.events = events;
        &self.requests
    }
}

impl Clone for BackgroundAnimation {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            root: self.root,
            joints: hsd_types::storage::clone_vec(&self.joints),
            requests: hsd_types::storage::clone_vec(&self.requests),
            prepared: self
                .prepared
                .iter()
                .map(hsd_types::storage::clone_vec)
                .collect(),
            subtree_ends: hsd_types::storage::clone_vec(&self.subtree_ends),
            prepared_materials: self.prepared_materials.clone(),
            rest: std::sync::Arc::clone(&self.rest),
            selections: std::sync::Arc::clone(&self.selections),
            subtrees: std::sync::Arc::clone(&self.subtrees),
            overlay: self.overlay,
            overlay_script: self.overlay_script,
        }
    }
}

impl BackgroundAnimation {
    /// `lb_8000B804(child of the gobj jobj, the archive joint)` (0x8000B804),
    /// grAnime_801C8138's first step: every joint back to its authored
    /// rotation, scale and translation, quaternion off, matrix dirty.
    /// Flags such as `JOBJ_HIDDEN` stay.
    pub fn reset_pose(&mut self) {
        for &id in &self.joints {
            let source = self.rest.get(id);
            let (rotate, scale, translate) = (source.rotate, source.scale, source.translate);
            let joint = self.tree.get_mut(id);
            joint.rotate = rotate;
            joint.scale = scale;
            joint.translate = translate;
            self.tree.clear_flags(id, hsd_anim::jobj::JOBJ_USE_QUATERNION);
            self.tree.set_flags(id, hsd_anim::jobj::JOBJ_MTX_DIRTY);
        }
    }

    /// Recreate a stage model in its reserved storage. Hierarchy and authored
    /// resources stay fixed; poses, visibility and material values restart.
    pub fn reset_for_creation(&mut self) {
        self.clear_animation();
        self.overlay = Default::default();
        self.overlay_script = None;
        for &id in &self.joints {
            let source = self.rest.get(id);
            let joint = self.tree.get_mut(id);
            joint.flags = source.flags;
            joint.rotate = source.rotate;
            joint.scale = source.scale;
            joint.translate = source.translate;
            joint.mtx = source.mtx;
            joint.scl = source.scl;
            for (object, original) in joint.dobj.iter_mut().zip(&source.dobj) {
                object.flags = original.flags;
                if let (Some(target), Some(source)) = (&mut object.mobj, &original.mobj) {
                    target.mat = source.mat;
                    target.pe = source.pe;
                    target.rendermode = source.rendermode;
                    for (texture, original) in target.textures.iter_mut().zip(&source.textures) {
                        texture.descriptor.clone_from(&original.descriptor);
                        texture.lod_bias = original.lod_bias;
                    }
                }
            }
        }
        self.requests.clear();
        self.tree.events.clear();
    }
    /// `Ground_GetStageGObj` then `grAnime_801C8138(gobj, map, 0)` on the
    /// fresh model, as `load_model` builds it: the new JObjs' matrix caches
    /// are the loaded ones until a scheduler update refreshes them.
    pub fn recreate(
        &mut self,
        archive: &Archive,
        model: &crate::desc::ModelDesc,
    ) -> crate::desc::ReadResult<()> {
        self.reset_for_creation();
        if let Some(animation) = model.animations.first() {
            if !self.attach_prepared(0) {
                self.attach_subtree(archive, 0, animation, model.animation_loops[0])?;
            }
        }
        self.play_materials(0, 0, self.joints.len());
        Ok(())
    }
}

#[cfg(test)]
mod identity_root_tests {
    use super::*;
    #[test]
    fn dynamic_identity_uses_the_map_wrapper_as_its_actual_gobj_root() {
        let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
            || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            std::path::PathBuf::from,
        );
        let path = root.join("harness/roms/files/GrKg.dat");
        assert!(melee_test_support::require_files([&path]));
        let archive = Archive::parse(&std::fs::read(path).unwrap()).unwrap();
        let mut model = BackgroundAnimation::identity_root(&archive, 2.0).unwrap();
        let wrapper = model.tree.parent(model.root).unwrap();
        assert!(model.tree.parent(wrapper).is_none());
        assert_eq!(
            model.tree.scale(model.root),
            hsd_types::Vec3::new(1.0, 1.0, 1.0)
        );
        assert_eq!(
            model.tree.scale(wrapper),
            hsd_types::Vec3::new(2.0, 2.0, 2.0)
        );
        let position = hsd_types::Vec3::new(3.0, 4.0, 5.0);
        let rotation = hsd_types::Vec3::new(0.25, 0.5, 0.75);
        model.set_gobj_translate(position);
        model.set_gobj_rotation(rotation);
        assert_eq!(model.gobj_translation(), position);
        assert_eq!(model.gobj_rotation(), rotation);
        assert_eq!(model.tree.translation(model.root), hsd_types::Vec3::ZERO);
        let matrix = model.gobj_matrix();
        assert_eq!(
            [matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]],
            [3.0, 4.0, 5.0]
        );
        model.tree.setup_matrix(model.root);
        assert_eq!(model.tree.get(model.root).mtx, matrix);
    }
}
