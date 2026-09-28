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
    rest: std::sync::Arc<JObjTree>,
    pub(super) tree: JObjTree,
    pub(super) root: JObjId,
    // Animation replacement preserves the model hierarchy and these identities.
    pub(super) joints: Vec<JObjId>,
    requests: Vec<ParticleRequest>,
    pub(super) prepared: Vec<Vec<Option<hsd_anim::aobj::AObj>>>,
    pub(super) prepared_materials: Vec<Vec<Vec<hsd_anim::material_playback::PreparedMaterial>>>,
    pub(super) subtree_ends: Vec<usize>,
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
        };
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
        self.attach_subtree(
            archive,
            0,
            &model.animations[index],
            model.animation_loops[index],
        )?;
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
    /// `grAnime_801C83D0(gobj, 0, 7)` (0x801C83D0): `HSD_ForeachAnim` over
    /// every animation type from the archive root stops at the first AObj
    /// (JObjForeachAnim order: joint, then its DObjs' own, material and
    /// texture AObjs, then children) and tests `AOBJ_NO_ANIM`. No AObj reads
    /// as not ended. The loaded stage models carry no RObj or shape AObjs.
    pub fn first_animation_ended(&self) -> bool {
        self.first_aobj(self.root)
            .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0)
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
        Ok(())
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
                JObjEvent::Path { .. } => unimplemented!("jobj.c:287-292: stage spline animation"),
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
            overlay: self.overlay,
            overlay_script: self.overlay_script,
        }
    }
}

impl BackgroundAnimation {
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
            self.attach_subtree(archive, 0, animation, model.animation_loops[0])?;
        }
        self.play_materials(0, 0, self.joints.len());
        Ok(())
    }
}
