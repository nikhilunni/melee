//! FD map 4's Ground animation and grLib_801C99C0 particle callback.
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
        let model = &desc.models[4];
        let (mut tree, root) = load_joint_tree(archive, &model.joint)?;
        attach_anim_joint(&mut tree, root, &model.animations[0], archive)?;
        tree.req_anim_all(root, 0.0);
        Ok(Self { tree, root })
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
                JObjEvent::Path { .. } => unimplemented!("FD stage spline"),
                _ => None,
            })
            .collect()
    }
}
