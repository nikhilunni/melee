//! Matrix evaluation scratch, separate from mutable animation and gameplay.
use super::*;

/// Reusable matrix-only copy of a fixed joint tree. Refreshing copies no tracks,
/// display objects, particles, or events, and allocates nothing. This is not a
/// simulation checkpoint; it is only valid for matrix queries.
pub struct MatrixPose {
    tree: JObjTree,
}
impl MatrixPose {
    pub fn new(source: &JObjTree) -> Self {
        let mut pose = Self {
            tree: JObjTree {
                nodes: (0..source.nodes.len()).map(|_| JObj::default()).collect(),
                position_constraints: source.position_constraints.clone(),
                ..JObjTree::default()
            },
        };
        assert!(pose.refresh(source));
        pose
    }
    /// False means the source has a different arena/constraint shape. The caller
    /// must create new scratch; rejection happens before changing any pose data.
    pub fn refresh(&mut self, source: &JObjTree) -> bool {
        if self.tree.nodes.len() != source.nodes.len()
            || !self
                .tree
                .position_constraints
                .keys()
                .eq(source.position_constraints.keys())
        {
            return false;
        }
        for (dst, src) in self.tree.nodes.iter_mut().zip(&source.nodes) {
            dst.flags = src.flags;
            dst.next = src.next;
            dst.parent = src.parent;
            dst.child = src.child;
            dst.rotate = src.rotate;
            dst.scale = src.scale;
            dst.translate = src.translate;
            dst.mtx = src.mtx;
            dst.scl = src.scl;
            dst.envelopemtx = src.envelopemtx;
            dst.path_reference = src.path_reference;
            dst.id = src.id;
            // make_matrix only tests AObj presence for path references. Playback
            // tracks and their clocks are never evaluated by matrix queries.
            dst.aobj = src.aobj.as_ref().map(|_| AObj::default());
        }
        for (dst, src) in self
            .tree
            .position_constraints
            .values_mut()
            .zip(source.position_constraints.values())
        {
            *dst = *src;
        }
        true
    }
    /// HSD_PObjSetupEnvelopeModelMtx and _HSD_mkEnvelopeModelNodeMtx
    /// (pobj.c / displayfunc.c). None indicates missing bind/skeleton data.
    /// A single full-weight influence uses retail's rigid path; weighted palettes
    /// use inverse bind matrices and the owning model node's relative transform.
    pub fn envelope_matrix(&mut self, owner: JObjId, weights: &[(JObjId, f32)]) -> Option<Mtx> {
        let &(first, weight) = weights.first()?;
        let correction = if self.tree.flags(owner) & JOBJ_SKELETON_ROOT != 0 {
            None
        } else {
            let mut ancestor = Some(owner);
            let skeleton = loop {
                let id = ancestor?;
                if self.tree.flags(id) & (JOBJ_SKELETON | JOBJ_SKELETON_ROOT) != 0 {
                    break id;
                }
                ancestor = self.tree.parent(id);
            };
            let mut correction = Mtx::default();
            if skeleton == owner {
                mtx::hsd_mtx_inverse(&self.tree.get(skeleton).envelopemtx?, &mut correction);
            } else {
                let mut basis = self.matrix(skeleton);
                if self.tree.flags(skeleton) & JOBJ_SKELETON_ROOT == 0 {
                    let world = basis;
                    mtx::mtx_concat(&world, &self.tree.get(skeleton).envelopemtx?, &mut basis);
                }
                mtx::hsd_mtx_inverse_concat(&basis, &self.matrix(owner), &mut correction);
            }
            Some(correction)
        };
        let mut matrix = Mtx::default();
        if weight >= 1.0 - f32::EPSILON {
            matrix = self.matrix(first);
            if correction.is_some() {
                let world = matrix;
                mtx::mtx_concat(&world, &self.tree.get(first).envelopemtx?, &mut matrix);
            }
        } else {
            matrix.0 = [[0.0; 4]; 3];
            for &(joint, weight) in weights {
                let mut bound = Mtx::default();
                mtx::mtx_concat(
                    &self.matrix(joint),
                    &self.tree.get(joint).envelopemtx?,
                    &mut bound,
                );
                for (out, row) in matrix.0.iter_mut().zip(bound.0) {
                    for (out, value) in out.iter_mut().zip(row) {
                        // HSD_MtxScaledAdd, retail 0x8037A554..0x8037A604: fmadds.
                        *out = gekko_math::fma::fmadds(weight, value, *out);
                    }
                }
            }
        }
        if let Some(correction) = correction {
            let world = matrix;
            mtx::mtx_concat(&world, &correction, &mut matrix);
        }
        Some(matrix)
    }
    pub fn matrix(&mut self, joint: JObjId) -> Mtx {
        *self.tree.get_mtx(joint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_mesh_retains_its_owner_transform_and_rigid_path() {
        let mut tree = JObjTree::default();
        let root = tree.alloc();
        tree.set_flags(root, JOBJ_SKELETON_ROOT);
        tree.set_translate(root, &Vec3::new(10.0, 0.0, 0.0));
        let bone = tree.alloc();
        tree.add_child(root, bone);
        tree.set_translate(bone, &Vec3::new(2.0, 0.0, 0.0));
        let mut inverse = Mtx::default();
        mtx::mtx_identity(&mut inverse);
        inverse.0[0][3] = -2.0;
        tree.get_mut(bone).envelopemtx = Some(inverse);
        let owner = tree.alloc();
        tree.add_child(root, owner);
        tree.set_translate(owner, &Vec3::new(3.0, 0.0, 0.0));
        let mut pose = MatrixPose::new(&tree);
        assert_eq!(
            pose.envelope_matrix(owner, &[(bone, 1.0)]).unwrap().0[0][3],
            13.0
        );
        assert_eq!(
            pose.envelope_matrix(root, &[(bone, 1.0)]).unwrap().0[0][3],
            12.0
        );
        // Two partial influences exercise inverse binds even for a root owner.
        assert_eq!(
            pose.envelope_matrix(root, &[(bone, 0.5), (bone, 0.5)])
                .unwrap()
                .0[0][3],
            10.0
        );
        assert_eq!(tree.get(root).translate.x, 10.0);
        assert!(pose.envelope_matrix(owner, &[]).is_none());
    }
}
