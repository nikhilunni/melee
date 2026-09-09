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
    tree: JObjTree,
    root: JObjId,
}
/// grLib_801C99C0 (grlib.c:151-158): DPtcl calls hsd_8039EFAC
/// with link 0 and the animation's bank, kind and attachment joint.
pub struct ParticleRequest {
    pub bank: u8,
    pub kind: u32,
    pub joint: usize,
    pub matrix: Mtx,
}
impl BackgroundAnimation {
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
        Ok(Self { tree, root })
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
    pub fn matrices(&mut self) -> Vec<(usize, Mtx)> {
        let mut joints = Vec::new();
        self.tree
            .walk_tree(self.root, &mut |joint, _| joints.push(joint));
        joints
            .into_iter()
            .map(|joint| {
                self.tree.setup_matrix(joint);
                (joint.0, self.tree.get(joint).mtx)
            })
            .collect()
    }
    /// Ground_801C1CD0 (ground.c): evaluate the stage animation at s_link 1.
    pub fn tick<T: InverseTrig>(&mut self) -> Vec<ParticleRequest> {
        self.tree.anim_all::<T>(self.root);
        std::mem::take(&mut self.tree.events)
            .into_iter()
            .filter_map(|event| match event {
                JObjEvent::DPtcl { jobj, lo, hi } => {
                    self.tree.setup_matrix(jobj);
                    Some(ParticleRequest {
                        bank: lo as u8,
                        kind: hi as u32,
                        joint: jobj.0,
                        matrix: self.tree.get(jobj).mtx,
                    })
                }
                JObjEvent::Path { .. } => unimplemented!("jobj.c:287-292: stage spline animation"),
                _ => None,
            })
            .collect()
    }
}
