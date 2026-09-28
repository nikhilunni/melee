//! REFTYPE_JOBJ subtype 4 orientation constraints (lb_8000C290), resolved by
//! resolveCnsOrientation (robj.c:343, retail 8037B7B0): the constrained
//! joint takes the target joint's world axes, each at the length of its own.
use crate::jobj::{JObjId, JObjTree, JOBJ_CLASSICAL_SCALE};
use crate::mtx;
use hsd_types::{Mtx, Vec3};

/// The deepest ancestor chain the classical-scale branch walks.
pub const MAX_ORIENTATION_CHAIN: usize = 24;

/// resolveCnsOrientation's view of the target joint, captured once its
/// matrices are set up (8037B8B4 / 8037BA18 HSD_JObjSetupMatrix).
// A plain value: the tick path allocates nothing, so the chain stays inline.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrientationTarget {
    /// A target without JOBJ_CLASSICAL_SCALE, or without a parent
    /// (robj.c:361-378): its world matrix.
    World(Mtx),
    /// A classical-scale target with a parent (robj.c:379-434): its matrix
    /// relative to its parent (HSD_MtxInverseConcat), and each ancestor's
    /// own relative matrix with unit columns, the parent first and the
    /// root's world matrix last.
    Chain {
        local: Mtx,
        ancestors: [Mtx; MAX_ORIENTATION_CHAIN],
        count: usize,
    },
}

/// HSD_MtxColVec: column `column`'s x, y and z.
fn column_of(m: &Mtx, column: usize) -> Vec3 {
    Vec3::new(m.0[0][column], m.0[1][column], m.0[2][column])
}

fn set_column(m: &mut Mtx, column: usize, v: Vec3) {
    m.0[0][column] = v.x;
    m.0[1][column] = v.y;
    m.0[2][column] = v.z;
}

/// VECMag (PSVECMag) and its guarded reciprocal: a magnitude at or below
/// 1e-10 is kept as it is (8037B914..1C: fcmpo, fdivs).
fn inverse_magnitude(v: &Vec3) -> f32 {
    let magnitude = mtx::vec_mag(v);
    if magnitude > 1e-10 {
        1.0 / magnitude
    } else {
        magnitude
    }
}

/// HSD_MtxColMag, inlined at 8037B920..88: (y*y + x*x) + z*z in separate
/// fmuls/fadds, then the frsqrte sqrtf.
fn column_magnitude(m: &Mtx, column: usize) -> f32 {
    let (x, y, z) = (m.0[0][column], m.0[1][column], m.0[2][column]);
    gekko_math::msl::sqrtf(z * z + (y * y + x * x))
}

/// `v` scaled by `scale`, one fmuls per component.
fn scaled(v: Vec3, scale: f32) -> Vec3 {
    Vec3::new(v.x * scale, v.y * scale, v.z * scale)
}

/// Column `column` of `source` at unit length times the constrained
/// joint's own column length (robj.c:368-376, 385-398).
fn fitted_column(source: &Mtx, own: &Mtx, column: usize) -> Vec3 {
    let v = column_of(source, column);
    let scale = inverse_magnitude(&v) * column_magnitude(own, column);
    scaled(v, scale)
}

impl OrientationTarget {
    /// The constrained joint's world axes, from its current matrix `own`
    /// (JObjUpdateFunc types 0x32..0x34 store them as its columns).
    pub fn axes(&self, own: &Mtx) -> [Vec3; 3] {
        match self {
            Self::World(target) => std::array::from_fn(|i| fitted_column(target, own, i)),
            Self::Chain {
                local,
                ancestors,
                count,
            } => {
                let mut basis = *local;
                for column in 0..3 {
                    let v = fitted_column(local, own, column);
                    set_column(&mut basis, column, v);
                }
                // PSMTXConcat(mtx0, mtx1, mtx1) up the chain.
                for ancestor in &ancestors[..*count] {
                    let mut product = Mtx::default();
                    mtx::mtx_concat(ancestor, &basis, &mut product);
                    basis = product;
                }
                std::array::from_fn(|i| column_of(&basis, i))
            }
        }
    }
}

impl JObjTree {
    /// The orientation target `id` presents to resolveCnsOrientation, its
    /// matrix and its ancestors' set up first.
    pub fn orientation_target(&mut self, id: JObjId) -> OrientationTarget {
        self.setup_matrix(id);
        let node = self.get(id);
        let world = node.mtx;
        let Some(parent) = node
            .parent
            .filter(|_| node.flags & JOBJ_CLASSICAL_SCALE != 0)
        else {
            return OrientationTarget::World(world);
        };
        let mut local = Mtx::default();
        mtx::hsd_mtx_inverse_concat(&self.get(parent).mtx, &world, &mut local);
        let mut ancestors = [Mtx::default(); MAX_ORIENTATION_CHAIN];
        let mut count = 0;
        let mut joint = Some(parent);
        while let Some(current) = joint {
            let node = self.get(current);
            let mut relative = node.mtx;
            if let Some(up) = node.parent {
                mtx::hsd_mtx_inverse_concat(&self.get(up).mtx, &node.mtx, &mut relative);
            }
            // robj.c:410-424: unit columns, no length of its own.
            for column in 0..3 {
                let v = column_of(&relative, column);
                let v = scaled(v, inverse_magnitude(&v));
                set_column(&mut relative, column, v);
            }
            assert!(
                count < MAX_ORIENTATION_CHAIN,
                "orientation target chain too deep"
            );
            ancestors[count] = relative;
            count += 1;
            joint = node.parent;
        }
        OrientationTarget::Chain {
            local,
            ancestors,
            count,
        }
    }
}
