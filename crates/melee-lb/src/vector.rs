//! lbvector.c helpers. All fusion sites audited with `asm.py
//! lbVector_Normalize lbVector_Angle lbVector_RotateAboutUnitAxis
//! lbVector_CreateEulerMatrix lbVector_Rotate lbVector_WorldToScreen --fused`
//! against the retail DOL.
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::{sqrtf, sqrtf_accurate},
};
use hsd_anim::{mtx, quat::Quaternion};
use hsd_types::{Mtx, Vec3};

pub fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
/// lbVector_Normalize (0x8000D2EC): unfused sum, inline MSL sqrtf.
pub fn length(v: Vec3) -> f32 {
    sqrtf((v.x * v.x + v.y * v.y) + v.z * v.z)
}
pub fn normalize(v: Vec3) -> Vec3 {
    let len = length(v);
    if len == 0.0 {
        return v;
    }
    let inv = 1.0 / len;
    Vec3::new(v.x * inv, v.y * inv, v.z * inv)
}
/// lbVector_NormalizeXY (0x8000D3B0): the XY length through the four-step
/// inline sqrtf (sum unfused), a zero length left alone, then one reciprocal
/// (fdivs) and two fmuls. Z is untouched.
pub fn normalize_xy(v: Vec3) -> Vec3 {
    let len = length_xy(v);
    if len == 0.0 {
        return v;
    }
    let inv = 1.0 / len;
    Vec3::new(v.x * inv, v.y * inv, v.z)
}
/// The XY length as lbVector_NormalizeXY and it_8027781C inline it.
pub fn length_xy(v: Vec3) -> f32 {
    sqrtf_accurate(v.x * v.x + v.y * v.y)
}
/// lbVector_Mirror (0x8000DC6C): reflect the XY of `v` across the line with
/// unit normal `n`; Z is untouched. Retail 8000DC78..9C: fmuls, fmadds for
/// the dot, fmuls by -2, then fmadds per axis.
pub fn mirror(v: Vec3, n: Vec3) -> Vec3 {
    let reflect = -2.0 * fmadds(n.x, v.x, n.y * v.y);
    Vec3::new(fmadds(n.x, reflect, v.x), fmadds(n.y, reflect, v.y), v.z)
}
/// lbVector_CosAngle (0x8000DCA8): the XY lengths through the inline MSL
/// sqrtf (squares summed unfused), their product, and the XY dot (retail
/// 8000DD9C: fmadds, X product outer), divided once.
pub fn cos_angle(a: Vec3, b: Vec3) -> f32 {
    let lengths = sqrtf(a.x * a.x + a.y * a.y) * sqrtf(b.x * b.x + b.y * b.y);
    fmadds(a.x, b.x, a.y * b.y) / lengths
}
/// lbVector_AngleXY (0x8000D790): the angle between the XY projections,
/// from lbVector_Len_xy_accurate lengths (squares summed unfused) and the
/// XY dot (retail 8000D8BC: fmadds, Y product first), clamped to acosf's
/// domain; zero when either projection is empty.
pub fn angle_xy(a: Vec3, b: Vec3) -> f32 {
    let lengths =
        sqrtf_accurate(a.x * a.x + a.y * a.y) * sqrtf_accurate(b.x * b.x + b.y * b.y);
    if lengths == 0.0 {
        return 0.0;
    }
    let cosine = fmadds(a.x, b.x, a.y * b.y) / lengths;
    crate::trigf::acosf(cosine.clamp(-1.0, 1.0))
}
/// lbVector_Angle (0x8000D620): lengths remain unfused; dot is fused.
pub fn angle(a: Vec3, b: Vec3) -> f32 {
    let lengths = length(a) * length(b);
    if lengths > 1e-10 {
        // retail 8000D748/750: fmadds, Y product first.
        let cosine = fmadds(a.z, b.z, fmadds(a.x, b.x, a.y * b.y)) / lengths;
        crate::trigf::acosf(cosine.clamp(-1.0, 1.0))
    } else {
        0.0
    }
}
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    let mut out = Vec3::ZERO;
    mtx::vec_cross_product(&a, &b, &mut out);
    normalize(out)
}

/// Inlined lbvector_sin/cos. Argument reduction promotes to double.
fn polynomial(mut a: f32) -> f32 {
    if f64::from(a) > std::f64::consts::PI {
        a = (f64::from(a) - std::f64::consts::TAU) as f32;
    } else if f64::from(a) < -std::f64::consts::PI {
        a = (f64::from(a) + std::f64::consts::TAU) as f32;
    }
    let cubic = (0.155_271 * a) * a;
    let fifth = (0.005_643 * a) * a;
    let fifth = a * (a * fifth);
    // retail 8000D9D8/D9DC, DA3C/DA40; Euler inlines E5A8/E5AC,
    // E614/E618, E684/E688, E6F0/E6F4, E760/E764, E7D0/E7DC.
    fmadds(a, fifth, fmsubs(0.987_862, a, a * cubic))
}
fn sincos(a: f32) -> (f32, f32) {
    (
        polynomial(a),
        polynomial((f64::from(a) + std::f64::consts::FRAC_PI_2) as f32),
    )
}

/// The coordinate axis of [`rotate_about`]; retail passes 1, 2 and 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// lbVector_Rotate (0x8000DB00): rotate `v` by `angle` about a coordinate
/// axis, with the quintic sine/cosine.
pub fn rotate_about(v: Vec3, axis: Axis, angle: f32) -> Vec3 {
    let (s, c) = sincos(angle);
    match axis {
        // retail 0x8000DC14/DC18.
        Axis::X => Vec3::new(v.x, fmsubs(v.y, c, v.z * s), fmadds(v.y, s, v.z * c)),
        // retail 0x8000DC34/DC38.
        Axis::Y => Vec3::new(fmadds(v.x, c, v.z * s), v.y, fmsubs(v.z, c, v.x * s)),
        // retail 0x8000DC54/DC58.
        Axis::Z => Vec3::new(fmsubs(v.x, c, v.y * s), fmadds(v.x, s, v.y * c), v.z),
    }
}

/// lbVector_WorldToScreen (0x8000E210) with `d != 0`: a fresh look-at
/// matrix, a point behind the near side pushed to 0.01 in front of the eye,
/// then GXProject. Returns the window position (x, y, depth).
pub fn world_to_screen(camera: &hsd_anim::cobj::PerspectiveCamera, position: Vec3) -> Vec3 {
    let view = hsd_anim::cobj::look_at(camera.eye, camera.up_vector(), camera.interest);
    let row = view.0[2];
    // retail 0x8000E498..0x8000E4B8: y product first, two fmadds, then + m23.
    let depth = row[3]
        + fmadds(
            row[2],
            position.z,
            fmadds(row[0], position.x, row[1] * position.y),
        );
    let mut point = position;
    if depth > -0.01 {
        let push = -depth - 0.01;
        // retail 0x8000E4D4..0x8000E4EC.
        point = Vec3::new(
            fmadds(row[0], push, point.x),
            fmadds(row[1], push, point.y),
            fmadds(row[2], push, point.z),
        );
    }
    hsd_anim::cobj::gx_project(
        point,
        &view,
        camera.projection(),
        &camera.viewport_parameters(),
    )
}

/// lbVector_RotateAboutUnitAxis (0x8000D8F4). Do not replace this with
/// Rodrigues' formula: both the polynomial and the rotation order matter.
pub fn rotate(v: Vec3, axis: Vec3, a: f32) -> Vec3 {
    let len = sqrtf(axis.y * axis.y + axis.z * axis.z);
    let (s, c) = sincos(a);
    let (uy, uz) = if len > 1e-10 {
        (axis.z / len, axis.y / len)
    } else {
        (0.0, 0.0)
    };
    // retail 8000DA6C/DA70: fmsubs/fmadds.
    let (y, z) = if len > 1e-10 {
        (fmsubs(v.y, uy, v.z * uz), fmadds(v.y, uz, v.z * uy))
    } else {
        (v.y, v.z)
    };
    // retail 8000DA98/DAA0.
    let z2 = fmadds(v.x, axis.x, z * len);
    let x2 = fmsubs(v.x, len, z * axis.x);
    // retail 8000DAAC/DAB4.
    let x3 = fmsubs(x2, c, y * s);
    let y3 = fmadds(x2, s, y * c);
    // retail 8000DABC/DAC0.
    let x = fmadds(x3, len, z2 * axis.x);
    let z = fmadds(gekko_math::fma::negate_rounded(x3), axis.x, z2 * len);
    if len > 1e-10 {
        // retail 8000DAD8/DADC.
        Vec3::new(
            x,
            fmadds(y3, uy, z * uz),
            fmadds(gekko_math::fma::negate_rounded(y3), uz, z * uy),
        )
    } else {
        Vec3::new(x, y3, z)
    }
}

/// lbVector_CreateEulerMatrix (0x8000E530).
pub fn euler_matrix(rotation: Quaternion) -> Mtx {
    let (sx, cx) = sincos(rotation.x);
    let (sy, cy) = sincos(rotation.y);
    let (sz, cz) = sincos(rotation.z);
    let sxsy = sx * sy;
    let cxsy = cx * sy;
    // retail 8000E7F4/E7F8/E800/E808: fmsubs/fmadds.
    Mtx([
        [
            cy * cz,
            fmsubs(cz, sxsy, cx * sz),
            fmadds(cz, cxsy, sx * sz),
            0.0,
        ],
        [
            cy * sz,
            fmadds(sz, sxsy, cx * cz),
            fmsubs(sz, cxsy, sx * cz),
            0.0,
        ],
        [-sy, sx * cy, cx * cy, 0.0],
    ])
}
