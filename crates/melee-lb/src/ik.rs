//! Ground-contact two-joint IK, lb/lb_020A.c:244-450 (80021410).
//! Saved local rotations belong to the caller; the solver dirties the chain.
use gekko_math::{
    fma::{fmadds, fnmadds, fnmsubs},
    msl::sqrtf,
};
use hsd_anim::{
    jobj::{JObjId, JObjTree, JOBJ_USE_QUATERNION},
    mtx, quat,
};
use hsd_types::{Mtx, Vec3};

/// World-space inputs and scratch from retail IKState, with the extended foot
/// distinguished from the actual foot joint and the floor-adjusted target.
#[derive(Clone, Copy, Debug)]
pub struct TwoJointIk {
    pub hip: Vec3,
    pub knee: Vec3,
    pub foot: Vec3,
    pub extended_foot: Vec3,
    pub target: Vec3,
    pub upper_length: f32,
    pub lower_length: f32,
}

/// lbVector_Len / Normalize: 8000D2F8..D310, unfused z*z + (x*x + y*y).
pub fn length(v: Vec3) -> f32 {
    sqrtf(v.z * v.z + (v.x * v.x + v.y * v.y))
}

pub fn normalize(v: Vec3) -> Vec3 {
    let magnitude = length(v);
    if magnitude == 0.0 {
        return v;
    }
    let reciprocal = 1.0 / magnitude;
    Vec3::new(v.x * reciprocal, v.y * reciprocal, v.z * reciprocal)
}

fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    // retail 800215C0/CC and lbVector_Angle 8000D748/750: fmadds.
    fmadds(a.z, b.z, fmadds(a.x, b.x, a.y * b.y))
}

/// lbVector_Angle (8000D620): scalar lengths, fused dot, Melee acosf.
fn angle(a: Vec3, b: Vec3) -> f32 {
    let product = length(a) * length(b);
    if product > 1e-10 {
        crate::trigf::acosf((dot(a, b) / product).clamp(-1.0, 1.0))
    } else {
        0.0
    }
}

fn project(point: &mut Vec3, axis: Vec3, plane: f32) {
    // retail 800214E4/F4/F8/FC, 80021530..3C, 80021570..7C.
    let distance = -(plane + dot(*point, axis));
    // retail 80021500/0C/18, 80021540/4C/58, 80021580/8C/98: fmadds.
    point.x = fmadds(distance, axis.x, point.x);
    point.y = fmadds(distance, axis.y, point.y);
    point.z = fmadds(distance, axis.z, point.z);
}

impl TwoJointIk {
    /// Numerical half of lbBgFlash_80021410. Returns hip/knee angle deltas;
    /// axis is the normalized third column of the knee's world matrix.
    #[allow(clippy::assign_op_pattern)] // Preserve retail fmuls operand order.
    pub fn angles(&mut self, axis: Vec3) -> [f32; 2] {
        // retail 800214D8 fmadds, 800214EC fnmadds (preserve signed zero).
        let plane = fnmadds(
            axis.z,
            self.knee.z,
            fmadds(axis.x, self.knee.x, axis.y * self.knee.y),
        );
        project(&mut self.target, axis, plane);
        project(&mut self.hip, axis, plane);
        project(&mut self.knee, axis, plane);
        let perpendicular = dot(axis, difference(self.hip, self.knee));
        // retail 800215D0 fnmsubs; this scratch is overwritten below, as retail.
        self.upper_length *= sqrtf(fnmsubs(perpendicular, perpendicular, 1.0));
        let upper = difference(self.knee, self.hip);
        let old_hip = angle(difference(self.target, self.hip), upper);
        // retail 800216B8 double fsub, 800216C0 frsp.
        let old_knee = (std::f64::consts::PI
            - f64::from(angle(difference(self.foot, self.knee), upper)))
            as f32;
        let mut reach = length(difference(self.hip, self.target));
        self.upper_length = length(difference(self.hip, self.knee));
        self.lower_length = length(difference(self.knee, self.extended_foot));
        let a = self.upper_length;
        let c = self.lower_length;
        // retail 80021844..C8: every product/sum/division is separate.
        // Soft extension is an eleventh/tenth-power ratio, not a hard clamp.
        let soft_reach = (10.0 * (a + c)) / 11.0;
        let mut soft_power = soft_reach * (soft_reach * soft_reach);
        for _ in 0..8 {
            soft_power = soft_reach * soft_power;
        }
        let mut reach_power = reach * (reach * (reach * reach));
        for _ in 0..6 {
            reach_power = reach * reach_power;
        }
        if reach > soft_reach {
            reach = (11.0 * soft_reach) / 10.0 + (-soft_power / (10.0 * reach_power));
        }
        let a2 = a * a;
        let b2 = reach * reach;
        let c2 = c * c;
        let twice_a = 2.0 * a;
        let hip_cos = ((a2 + b2) - c2) / (twice_a * reach);
        let knee_cos = ((a2 + c2) - b2) / (twice_a * c);
        let hip_angle = crate::trigf::acosf(hip_cos.clamp(-1.0, 1.0));
        let mut knee_angle = crate::trigf::acosf(knee_cos.clamp(-1.0, 1.0));
        // Retail @488: ten degrees, @489: PI minus ten degrees. Preserve the
        // double arithmetic and both intermediate frsp at 80021984/8C.
        const MIN_BEND: f64 = 0.1745329201221466;
        const STRAIGHT_LIMIT: f64 = 2.9670597334676465;
        let remainder = std::f64::consts::PI - f64::from(knee_angle);
        if remainder < MIN_BEND {
            let ratio = (remainder.abs() / MIN_BEND) as f32;
            let correction = (f64::from(ratio) * (f64::from(knee_angle) - STRAIGHT_LIMIT)) as f32;
            knee_angle = (STRAIGHT_LIMIT + f64::from(correction)) as f32;
        }
        [hip_angle - old_hip, knee_angle - old_knee]
    }

    pub fn solve(&mut self, tree: &mut JObjTree, hip: JObjId, knee: JObjId) {
        let matrix = *tree.get_mtx(knee);
        let axis = normalize(Vec3::new(matrix.0[0][2], matrix.0[1][2], matrix.0[2][2]));
        let [hip_angle, knee_angle] = self.angles(axis);
        rotate_about_world_axis(tree, hip, axis, hip_angle);
        rotate_about_world_axis(tree, knee, axis, knee_angle);
    }
}

struct RetailTrig;
impl mtx::InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        crate::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        crate::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        crate::trigf::acosf(x)
    }
}

/// fn_80020AEC: remove inherited scale while accumulating parent rotations.
fn axis_transform(tree: &mut JObjTree, joint: JObjId) -> Mtx {
    let parent = tree.parent(joint).expect("IK joint requires a parent");
    // retail 80020B44/68: demand the child matrix, then the parent's, even
    // when a matrix-independent joint did not demand its parent recursively.
    let world = *tree.get_mtx(joint);
    let parent_world = *tree.get_mtx(parent);
    let mut result = Mtx::default();
    mtx::hsd_mtx_inverse_concat(&parent_world, &world, &mut result);
    for column in 0..3 {
        let mut vector = Vec3::new(
            result.0[0][column],
            result.0[1][column],
            result.0[2][column],
        );
        let mut magnitude = mtx::vec_mag(&vector);
        if magnitude > 1e-10 {
            magnitude = 1.0 / magnitude;
        }
        // retail 80020BD0..BE4: separate y*y + x*x, then z*z + sum.
        let x = world.0[0][column];
        let y = world.0[1][column];
        let z = world.0[2][column];
        let factor = magnitude * sqrtf(z * z + (y * y + x * x));
        vector.x *= factor;
        vector.y *= factor;
        vector.z *= factor;
        result.0[0][column] = vector.x;
        result.0[1][column] = vector.y;
        result.0[2][column] = vector.z;
    }
    let mut ancestor = Some(parent);
    while let Some(joint) = ancestor {
        let world = *tree.get_mtx(joint);
        let mut local = world;
        if let Some(parent) = tree.parent(joint) {
            let parent_world = *tree.get_mtx(parent);
            mtx::hsd_mtx_inverse_concat(&parent_world, &world, &mut local);
        }
        for column in 0..3 {
            let vector = Vec3::new(local.0[0][column], local.0[1][column], local.0[2][column]);
            let mut magnitude = mtx::vec_mag(&vector);
            if magnitude > 0.00001 {
                magnitude = 1.0 / magnitude;
            }
            local.0[0][column] = vector.x * magnitude;
            local.0[1][column] = vector.y * magnitude;
            local.0[2][column] = vector.z * magnitude;
        }
        let previous = result;
        mtx::mtx_concat(&local, &previous, &mut result);
        ancestor = tree.parent(joint);
    }
    result
}

/// fn_8002113C (8002113C): transform world axis, then concatenate local rotation.
fn rotate_about_world_axis(tree: &mut JObjTree, joint: JObjId, axis: Vec3, angle: f32) {
    tree.setup_matrix(joint);
    let transform = axis_transform(tree, joint);
    let mut transpose = Mtx::default();
    mtx::mtx_transpose(&transform, &mut transpose);
    let mut local_axis = Vec3::ZERO;
    mtx::mtx_mult_vec(&transpose, &axis, &mut local_axis);
    let mut rotation = Mtx::default();
    mtx::mtx_rot_axis_rad(&mut rotation, &local_axis, -angle);
    let mut current = Mtx::default();
    let saved = tree.rotation(joint);
    let quaternion = tree.flags(joint) & JOBJ_USE_QUATERNION != 0;
    if quaternion {
        mtx::mtx_quat(&mut current, &saved);
    } else {
        mtx::hsd_mk_rotation_mtx(&mut current, &Vec3::new(saved.x, saved.y, saved.z));
    }
    let mut result = Mtx::default();
    mtx::mtx_concat(&current, &rotation, &mut result);
    let mut rotated = saved;
    if quaternion {
        quat::mat_to_quat(&result, &mut rotated);
    } else {
        let mut euler = Vec3::ZERO;
        quat::mtx_to_euler::<RetailTrig>(&result, &mut euler);
        rotated.x = euler.x;
        rotated.y = euler.y;
        rotated.z = euler.z;
    }
    tree.set_rotation(joint, &rotated);
}
