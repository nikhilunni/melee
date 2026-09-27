//! HSD_CObj (sysdolphin/baselib/cobj.c) for a perspective camera, and the
//! SDK kernels it uses: `C_MTXLookAt`, `MTXPerspective` and `GXProject`.
//!
//! Only the state a gameplay camera needs is kept: the eye and interest
//! positions (their WObjs), the roll, the clip planes, the perspective
//! parameters and the viewport and scissor rectangles. The viewing matrix is
//! derived on demand, which is what `HSD_CObjGetViewingMtxPtr` does whenever
//! a position changed.
use crate::mtx::{mtx_mult_vec_sr, mtx_rot_axis_rad, vec_cross_product, vec_normalize};
use gekko_math::msl::{sqrtf, tanf};
use hsd_types::{Mtx, Vec3};

/// `HSD_RectF32` viewport, in the CObj's field order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub xmin: f32,
    pub xmax: f32,
    pub ymin: f32,
    pub ymax: f32,
}

/// `Scissor` (cobj.h), in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scissor {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

/// A `PROJ_PERSPECTIVE` HSD_CObj.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerspectiveCamera {
    pub viewport: Viewport,
    pub scissor: Scissor,
    pub eye: Vec3,
    pub interest: Vec3,
    /// `u.roll` (the CObj flag 1, an explicit up vector, is not supported).
    pub roll: f32,
    pub near: f32,
    pub far: f32,
    /// Vertical field of view, degrees.
    pub fov: f32,
    pub aspect: f32,
}

/// `FLT_MIN`, the threshold of `vec_normalize_check` (baselib/util.h).
const FLT_MIN: f32 = f32::MIN_POSITIVE;

/// `vec_normalize_check` (baselib/util.h): `None` for a (near) zero vector,
/// else `PSVECNormalize`.
fn normalize_checked(v: Vec3) -> Option<Vec3> {
    if v.x.abs() <= FLT_MIN && v.y.abs() <= FLT_MIN && v.z.abs() <= FLT_MIN {
        return None;
    }
    let mut out = Vec3::ZERO;
    vec_normalize(&v, &mut out);
    Some(out)
}

impl PerspectiveCamera {
    /// `HSD_CObjGetEyeVector` (0x80368E70 area): the unit view direction,
    /// `(0, 0, -1)` when the eye sits on the interest.
    pub fn eye_vector(&self) -> Option<Vec3> {
        // PSVECSubtract: separate ps_sub.
        let d = Vec3::new(
            self.interest.x - self.eye.x,
            self.interest.y - self.eye.y,
            self.interest.z - self.eye.z,
        );
        normalize_checked(d)
    }

    /// `HSD_CObjGetUpVector` through `roll2upvec` (0x80368BC0): the world up
    /// vector for `roll`, `(0, 1, 0)` when the eye vector is degenerate.
    pub fn up_vector(&self) -> Vec3 {
        let Some(eye) = self.eye_vector() else {
            return Vec3::new(0.0, 1.0, 0.0);
        };
        // retail 0x80368CD0: `1.0 - fabs(eye.y) < 0.0001` in double.
        let v0 = if 1.0 - f64::from(eye.y.abs()) < 0.0001 {
            // Looking straight up or down: build the up vector from x.
            let length = sqrtf(eye.y * eye.y + eye.z * eye.z);
            let neg_x = -eye.x;
            Vec3::new(length, eye.y * (neg_x / length), eye.z * (neg_x / length))
        } else {
            let length = sqrtf(eye.x * eye.x + eye.z * eye.z);
            let neg_y = -eye.y;
            Vec3::new(eye.x * (neg_y / length), length, eye.z * (neg_y / length))
        };
        let mut rotation = Mtx::ZERO;
        mtx_rot_axis_rad(&mut rotation, &eye, -self.roll);
        let mut rolled = Vec3::ZERO;
        mtx_mult_vec_sr(&rotation, &v0, &mut rolled);
        let mut up = Vec3::ZERO;
        vec_normalize(&rolled, &mut up);
        up
    }

    /// `HSD_CObjGetViewingMtxPtr`: `C_MTXLookAt(eye, up, interest)`.
    pub fn view_matrix(&self) -> Mtx {
        look_at(self.eye, self.up_vector(), self.interest)
    }

    /// The `GXProject` projection parameters of `MTXPerspective`, as
    /// `lbVector_WorldToScreen` extracts them.
    pub fn projection(&self) -> Projection {
        let m = perspective(self.fov, self.aspect, self.near, self.far);
        Projection::Perspective {
            scale_x: m[0][0],
            offset_x: m[0][2],
            scale_y: m[1][1],
            offset_y: m[1][2],
            scale_z: m[2][2],
            offset_z: m[2][3],
        }
    }

    /// The `GXProject` viewport vector: origin, size and the unit depth range.
    pub fn viewport_parameters(&self) -> [f32; 6] {
        let v = self.viewport;
        [v.xmin, v.ymin, v.xmax - v.xmin, v.ymax - v.ymin, 0.0, 1.0]
    }
}

/// `C_MTXLookAt` (0x80342734): camera basis from the eye, up and target.
/// The translations are separate products and sums, then a negation.
pub fn look_at(eye: Vec3, up: Vec3, target: Vec3) -> Mtx {
    let look = Vec3::new(eye.x - target.x, eye.y - target.y, eye.z - target.z);
    let mut look_unit = Vec3::ZERO;
    vec_normalize(&look, &mut look_unit);
    let mut right = Vec3::ZERO;
    vec_cross_product(&up, &look_unit, &mut right);
    let unnormalized = right;
    vec_normalize(&unnormalized, &mut right);
    let mut camera_up = Vec3::ZERO;
    vec_cross_product(&look_unit, &right, &mut camera_up);
    // retail 0x803427E8..0x80342804: fmuls x3, fadds (x + y) then z, fneg.
    let translate = |axis: Vec3| -> f32 {
        let (x, y, z) = (eye.x * axis.x, eye.y * axis.y, eye.z * axis.z);
        -(z + (x + y))
    };
    Mtx([
        [right.x, right.y, right.z, translate(right)],
        [camera_up.x, camera_up.y, camera_up.z, translate(camera_up)],
        [look_unit.x, look_unit.y, look_unit.z, translate(look_unit)],
    ])
}

/// `MTXPerspective` (0x80342BEC): a 4x4 projection matrix. No fused sites.
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    // retail 0x80342C20: (0.5 * fov) then * (pi / 180), before MSL tanf.
    let angle = DEGREES_TO_RADIANS * (0.5 * fov_y);
    let cotangent = 1.0 / tanf(angle);
    let depth = 1.0 / (far - near);
    [
        [cotangent / aspect, 0.0, 0.0, 0.0],
        [0.0, cotangent, 0.0, 0.0],
        [0.0, 0.0, -near * depth, depth * -(far * near)],
        [0.0, 0.0, -1.0, 0.0],
    ]
}

/// `MTXDegToRad`'s constant as MWCC rounds it (`0.017453292f`).
pub const DEGREES_TO_RADIANS: f32 = 0.017_453_292;

/// The projection parameters `GXProject` takes (`projection[1..7]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    Perspective {
        scale_x: f32,
        offset_x: f32,
        scale_y: f32,
        offset_y: f32,
        scale_z: f32,
        offset_z: f32,
    },
}

/// `GXProject` (0x80341148): model-view transform, projection and viewport
/// mapping, all unfused. Returns window coordinates `(x, y, z)`.
pub fn gx_project(
    point: Vec3,
    model_view: &Mtx,
    projection: Projection,
    viewport: &[f32; 6],
) -> Vec3 {
    let m = &model_view.0;
    let row = |r: &[f32; 4]| -> f32 {
        // retail 0x80341154..0x803411C8: (m0 x + m1 y), then m2 z + that, then m3 + that.
        let xy = r[0] * point.x + r[1] * point.y;
        r[3] + (r[2] * point.z + xy)
    };
    let (ex, ey, ez) = (row(&m[0]), row(&m[1]), row(&m[2]));
    let Projection::Perspective {
        scale_x,
        offset_x,
        scale_y,
        offset_y,
        scale_z,
        offset_z,
    } = projection;
    // retail 0x803411D0..0x80341210: w = 1 / -z.
    let w = 1.0 / -ez;
    let cx = ex * scale_x + ez * offset_x;
    let cy = ey * scale_y + ez * offset_y;
    let cz = offset_z + ez * scale_z;
    let [vx, vy, vw, vh, vnear, vfar] = *viewport;
    // retail 0x80341258..0x803412B0.
    let sx = vw * 0.5 + (vx + w * ((cx * vw) * 0.5));
    let sy = vh * 0.5 + (vy + w * ((-cy * vh) * 0.5));
    let sz = vfar + w * (cz * (vfar - vnear));
    Vec3::new(sx, sy, sz)
}
