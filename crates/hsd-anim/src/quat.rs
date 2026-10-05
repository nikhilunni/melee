//! Quaternion type and the `quatlib.c` routines.
//!
//! Source: `src/sysdolphin/baselib/quatlib.c` / `quatlib.h` (retail
//! `0x8037E708`..`0x8037EF28`). The SDK's `PSMTXQuat` (quaternion to matrix)
//! is in [`crate::mtx::mtx_quat`].
//!
//! Fusion and operand order are audited against the retail DOL; scalar
//! sums of squares remain unfused. Comparisons
//! against `double` literals in the C (`len > 1e-05`) are kept as `f64`
//! comparisons. The C functions return `s32` status codes: those that can
//! only return `0` return `()` here, and [`quat_from_axis_angle`], which
//! returns `-1` on a degenerate axis, returns `bool` (`true` = success).

// Constants are spelled exactly as the C source spells them.
#![allow(clippy::excessive_precision)]

use gekko_math::fma::{fmadds, fmsubs};
use gekko_math::msl::{cosf, fabsf, sinf, sqrtf};
use hsd_types::{Mtx, Vec3};

use crate::mtx::{InverseTrig, M_PI_2};

/// `Quaternion` from `extern/dolphin/include/dolphin/mtx.h`: `{ f32 x, y, z, w; }`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    /// The identity rotation `(0, 0, 0, 1)`.
    pub const IDENTITY: Quaternion = Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Quaternion {
        Quaternion { x, y, z, w }
    }
}

/// `MatToQuat` (retail `0x8037E708`, `quatlib.c`): quaternion from the
/// rotation part of `m`, each column normalised by its own length first.
///
/// The C always returns `0`.
pub fn mat_to_quat(m: &Mtx, q: &mut Quaternion) {
    let m = m.0;
    let nxt: [usize; 3] = [1, 2, 0];

    // retail 0x8037E73C..0x8037E74C, 0x8037E7B4..0x8037E7C8, 0x8037E82C..0x8037E844: fmuls + fadds, not fused.
    let len_col = [
        sqrtf(m[0][0] * m[0][0] + m[1][0] * m[1][0] + m[2][0] * m[2][0]),
        sqrtf(m[0][1] * m[0][1] + m[1][1] * m[1][1] + m[2][1] * m[2][1]),
        sqrtf(m[0][2] * m[0][2] + m[1][2] * m[1][2] + m[2][2] * m[2][2]),
    ];

    let s = m[0][0] / len_col[0] + m[1][1] / len_col[1] + m[2][2] / len_col[2];

    if s > 0.0 {
        let s = sqrtf(1.0 + s);
        q.w = 0.5 * s;
        let scale = 0.5 / s;
        q.x = scale * ((m[2][1] / len_col[1]) - (m[1][2] / len_col[2]));
        q.y = scale * ((m[0][2] / len_col[2]) - (m[2][0] / len_col[0]));
        q.z = scale * ((m[1][0] / len_col[0]) - (m[0][1] / len_col[1]));
    } else {
        let mut i = 0;
        if m[1][1] / len_col[1] > m[0][0] / len_col[0] {
            i = 1;
        }
        if m[2][2] / len_col[2] > m[i][i] / len_col[i] {
            i = 2;
        }
        let j = nxt[i];
        let k = nxt[j];

        let s = sqrtf(
            1.0 + (((m[i][i] / len_col[i]) - (m[j][j] / len_col[j])) - (m[k][k] / len_col[k])),
        );
        let scale = 0.5 / s;
        let mut q3 = [0.0f32; 3];
        q3[i] = 0.5 * s;
        q.w = scale * ((m[k][j] / len_col[j]) - (m[j][k] / len_col[k]));
        q3[j] = scale * ((m[j][i] / len_col[i]) + (m[i][j] / len_col[j]));
        q3[k] = scale * ((m[k][i] / len_col[i]) + (m[i][k] / len_col[k]));
        q.x = q3[0];
        q.y = q3[1];
        q.z = q3[2];
    }
}

/// `HSD_QuatLib_8037EB28` (retail `0x8037EB28`, `quatlib.c`): Euler angles
/// from a rotation matrix, with a gimbal-lock branch when the first column's
/// xy-projection is shorter than `1e-05` (compared in `double`).
///
/// The C always returns `0`.
pub fn mtx_to_euler<T: InverseTrig>(m: &Mtx, euler: &mut Vec3) {
    let m = m.0;

    // retail 0x8037EB50..0x8037EB5C: fmuls + fadds, not fused.
    let len = sqrtf(m[0][0] * m[0][0] + m[1][0] * m[1][0]);
    if f64::from(len) > 1e-05 {
        euler.x = T::atan2f(m[2][1], m[2][2]);
        euler.y = T::atan2f(-m[2][0], len);
        euler.z = T::atan2f(m[1][0], m[0][0]);
    } else {
        euler.x = T::atan2f(-m[1][2], m[1][1]);
        euler.y = T::atan2f(-m[2][0], len);
        euler.z = 0.0;
    }
}

/// `HSD_QuatLib_8037EC4C` (retail `0x8037EC4C`, `quatlib.c`): Hamilton
/// product `out = p * q` (rotation `q` applied after `p`).
///
/// The C always returns `0`.
pub fn quat_mul(p: &Quaternion, q: &Quaternion, out: &mut Quaternion) {
    // retail 0x8037EC94..0x8037ECC4: fmadds / fmsubs; xyz finish with fadds.
    let x = fmadds(q.w, p.x, p.w * q.x) + fmsubs(p.y, q.z, q.y * p.z);
    let y = fmadds(q.w, p.y, p.w * q.y) + fmsubs(q.x, p.z, p.x * q.z);
    let z = fmadds(q.w, p.z, p.w * q.z) + fmsubs(p.x, q.y, q.x * p.y);
    let xy_dot = fmadds(p.x, q.x, p.y * q.y);
    let xyz_dot = fmadds(p.z, q.z, xy_dot);
    let w = fmsubs(p.w, q.w, xyz_dot);

    out.x = x;
    out.y = y;
    out.z = z;
    out.w = w;
}

/// `HSD_QuatLib_8037ECE0` (retail `0x8037ECE0`, `quatlib.c`): quaternion for
/// a rotation of `angle` radians about `axis`.
///
/// Returns `false` (the C's `-1`) without touching `q` when the axis length
/// is below `1.1754944E-38`.
pub fn quat_from_axis_angle(axis: &Vec3, q: &mut Quaternion, angle: f32) -> bool {
    // retail 0x8037ED0C..0x8037ED24: fmuls + fadds, not fused.
    let len = sqrtf(axis.x * axis.x + axis.y * axis.y + axis.z * axis.z);
    if fabsf(len) < 1.1754944E-38 {
        return false;
    }
    let inv_len = 1.0 / len;
    let half_angle = 0.5 * angle;
    q.w = cosf(half_angle);
    let s = sinf(half_angle);
    q.x = s * (inv_len * axis.x);
    q.y = s * (inv_len * axis.y);
    q.z = s * (inv_len * axis.z);
    true
}

/// `EulerToQuat` (retail `0x8037EE0C`, `quatlib.c`).
///
/// The C always returns `0`.
pub fn euler_to_quat(euler: &Vec3, q: &mut Quaternion) {
    let cx = cosf(0.5 * euler.x);
    let cy = cosf(0.5 * euler.y);
    let cz = cosf(0.5 * euler.z);
    let sx = sinf(0.5 * euler.x);
    let sy = sinf(0.5 * euler.y);
    let sz = sinf(0.5 * euler.z);

    let ss = sy * sz;
    let cc = cy * cz;
    // retail 0x8037EED0, 0x8037EED8, 0x8037EEE8, 0x8037EEF0: fmadds / fmsubs.
    q.w = fmadds(cx, cc, sx * ss);
    q.x = fmsubs(sx, cc, cx * ss);
    q.y = fmadds(cz, cx * sy, sz * (sx * cy));
    q.z = fmsubs(sz, cx * cy, cz * (sx * sy));
}

/// `HSD_QuatLib_8037EF28` (retail `0x8037EF28`, `quatlib.c`): spherical
/// linear interpolation from `p` (t = 0) to `q` (t = 1).
///
/// Three regimes, as in the C: the general slerp through `acosf`; a linear
/// blend when the quaternions are nearly equal; and, when they are nearly
/// opposite, a blend against the perpendicular `(-p.y, p.x, -p.w, p.z)`
/// split at `t = 0.5`. In that last regime the C stores the perpendicular
/// into `out` and then overwrites it; both stores are kept. The C always
/// returns `0`.
pub fn quat_slerp<T: InverseTrig>(p: &Quaternion, q: &Quaternion, out: &mut Quaternion, t: f32) {
    let mut t = t;

    // retail 0x8037EF7C, 0x8037EF88, 0x8037EF94: fmadds, starting from p.y*q.y.
    let xy = fmadds(p.x, q.x, p.y * q.y);
    let xyz = fmadds(p.z, q.z, xy);
    let cosom = fmadds(p.w, q.w, xyz);

    if (1.0 + cosom) > 1e-10 {
        let sp;
        let sq;
        if (1.0 - cosom) > 1e-10 {
            let theta = T::acosf(cosom);
            let sinom = sinf(theta);
            sp = sinf((1.0 - t) * theta) / sinom;
            sq = sinf(t * theta) / sinom;
        } else {
            sq = t;
            sp = (1.0f64 - f64::from(t)) as f32;
        }
        // retail 0x8037F000..0x8037F03C (stride 0x14): fmadds.
        out.x = fmadds(sp, p.x, sq * q.x);
        out.y = fmadds(sp, p.y, sq * q.y);
        out.z = fmadds(sp, p.z, sq * q.z);
        out.w = fmadds(sp, p.w, sq * q.w);
    } else {
        out.x = -p.y;
        out.y = p.x;
        out.z = -p.w;
        out.w = p.z;

        if t < 0.5 {
            // `sinf((f32) (M_PI_2 * (1.0F - (2.0F * t))))`: f32 inner, f64 product, round.
            let sp = sinf((M_PI_2 * f64::from(1.0 - (2.0 * t))) as f32);
            let sq = sinf((M_PI_2 * f64::from(2.0 * t)) as f32);
            // retail 0x8037F0B4..0x8037F0F0 (stride 0x14): fmadds.
            out.x = fmadds(sp, p.x, sq * q.x);
            out.y = fmadds(sp, p.y, sq * q.y);
            out.z = fmadds(sp, p.z, sq * q.z);
            out.w = fmadds(sp, p.w, sq * q.w);
        } else {
            t -= 0.5;
            let t2 = 2.0 * t;
            let sp = sinf((M_PI_2 * f64::from(1.0 - t2)) as f32);
            let sq = sinf((M_PI_2 * f64::from(t2)) as f32);
            // retail 0x8037F13C..0x8037F178 (stride 0x14): fmadds.
            out.x = fmadds(sp, p.x, sq * q.x);
            out.y = fmadds(sp, p.y, sq * q.y);
            out.z = fmadds(sp, p.z, sq * q.z);
            out.w = fmadds(sp, p.w, sq * q.w);
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // std math is the tolerance reference here
mod tests {
    use super::*;
    use crate::mtx::{hsd_mk_rotation_mtx, mtx_quat};

    struct StdTrig;
    impl InverseTrig for StdTrig {
        fn atan2f(y: f32, x: f32) -> f32 {
            y.atan2(x)
        }
        fn asinf(x: f32) -> f32 {
            x.asin()
        }
        fn acosf(x: f32) -> f32 {
            x.acos()
        }
    }

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    fn quat_approx(a: &Quaternion, b: &Quaternion, tol: f32) {
        assert!(
            approx(a.x, b.x, tol)
                && approx(a.y, b.y, tol)
                && approx(a.z, b.z, tol)
                && approx(a.w, b.w, tol),
            "{a:?} vs {b:?}"
        );
    }

    #[test]
    fn layout() {
        assert_eq!(core::mem::size_of::<Quaternion>(), 16);
        assert_eq!(core::mem::align_of::<Quaternion>(), 4);
        assert_eq!(Quaternion::default(), Quaternion::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn mat_to_quat_identity_is_exact() {
        // Column lengths sqrtf(1) = 1, s = 3, sqrtf(4) = 2: every step exact.
        let mut q = Quaternion::default();
        mat_to_quat(&Mtx::IDENTITY, &mut q);
        assert_eq!(q, Quaternion::IDENTITY);
    }

    #[test]
    fn mat_to_quat_half_turn_takes_diagonal_branch_exactly() {
        // diag(1, -1, -1): a half turn about x. s = -1 <= 0, i = 0, j = 1, k = 2,
        // s = sqrtf(1 + (1 - -1) - -1) = sqrtf(4) = 2 exactly.
        let m = Mtx([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
        ]);
        let mut q = Quaternion::default();
        mat_to_quat(&m, &mut q);
        assert_eq!(q, Quaternion::new(1.0, 0.0, 0.0, 0.0));

        // diag(-1, -1, 1): half turn about z, exercises i = 2 selection.
        let m = Mtx([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]);
        mat_to_quat(&m, &mut q);
        assert_eq!(q, Quaternion::new(0.0, 0.0, 1.0, 0.0));
    }

    #[test]
    fn mat_to_quat_round_trips_through_mtx_quat() {
        let rot = Vec3::new(0.3, -0.7, 1.1);
        let mut m = Mtx::ZERO;
        hsd_mk_rotation_mtx(&mut m, &rot);
        let mut q = Quaternion::default();
        mat_to_quat(&m, &mut q);
        let mut back = Mtx::ZERO;
        mtx_quat(&mut back, &q);
        for (ra, rb) in m.0.iter().zip(back.0.iter()) {
            for (a, b) in ra.iter().zip(rb.iter()) {
                assert!(approx(*a, *b, 2e-6), "{m:?}\n{back:?}");
            }
        }
    }

    #[test]
    fn euler_conversions_round_trip() {
        let rot = Vec3::new(0.3, -0.7, 1.1);
        let mut m = Mtx::ZERO;
        hsd_mk_rotation_mtx(&mut m, &rot);
        let mut e = Vec3::ZERO;
        mtx_to_euler::<StdTrig>(&m, &mut e);
        assert!(
            approx(e.x, rot.x, 1e-5) && approx(e.y, rot.y, 1e-5) && approx(e.z, rot.z, 1e-5),
            "{e:?}"
        );

        // EulerToQuat agrees with MatToQuat of the same rotation, up to sign.
        let mut q1 = Quaternion::default();
        euler_to_quat(&rot, &mut q1);
        let mut q2 = Quaternion::default();
        mat_to_quat(&m, &mut q2);
        if q1.w * q2.w < 0.0 {
            q2 = Quaternion::new(-q2.x, -q2.y, -q2.z, -q2.w);
        }
        quat_approx(&q1, &q2, 2e-6);

        let mut q0 = Quaternion::default();
        euler_to_quat(&Vec3::ZERO, &mut q0);
        quat_approx(&q0, &Quaternion::IDENTITY, 1e-7);

        // Gimbal branch: first column has no xy component.
        let g = Mtx([
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
        ]);
        mtx_to_euler::<StdTrig>(&g, &mut e);
        assert_eq!(e.z, 0.0);
        assert!(approx(e.y, core::f32::consts::FRAC_PI_2, 1e-6));
    }

    #[test]
    fn quat_mul_identity_is_exact() {
        let q = Quaternion::new(0.1, -0.2, 0.3, 0.9);
        let mut out = Quaternion::default();
        quat_mul(&q, &Quaternion::IDENTITY, &mut out);
        assert_eq!(out, q);
        quat_mul(&Quaternion::IDENTITY, &q, &mut out);
        assert_eq!(out, q);
    }

    #[test]
    fn quat_mul_composes_rotations() {
        let a = Vec3::new(0.0, 0.0, 1.0);
        let mut qa = Quaternion::default();
        let mut qb = Quaternion::default();
        assert!(quat_from_axis_angle(&a, &mut qa, 0.5));
        assert!(quat_from_axis_angle(&a, &mut qb, 0.75));
        let mut prod = Quaternion::default();
        quat_mul(&qa, &qb, &mut prod);
        let mut expect = Quaternion::default();
        assert!(quat_from_axis_angle(&a, &mut expect, 1.25));
        quat_approx(&prod, &expect, 2e-7);
    }

    #[test]
    fn axis_angle_degenerate_axis_fails_without_writing() {
        let sentinel = Quaternion::new(9.0, 9.0, 9.0, 9.0);
        let mut q = sentinel;
        assert!(!quat_from_axis_angle(&Vec3::ZERO, &mut q, 1.0));
        assert_eq!(q, sentinel);

        let mut q = Quaternion::default();
        assert!(quat_from_axis_angle(
            &Vec3::new(0.0, 0.0, 2.0),
            &mut q,
            core::f32::consts::PI
        ));
        quat_approx(&q, &Quaternion::new(0.0, 0.0, 1.0, 0.0), 1e-6);
    }

    #[test]
    fn slerp_endpoints_and_midpoint() {
        let axis = Vec3::new(0.0, 1.0, 0.0);
        let mut p = Quaternion::default();
        let mut q = Quaternion::default();
        assert!(quat_from_axis_angle(&axis, &mut p, 0.2));
        assert!(quat_from_axis_angle(&axis, &mut q, 1.2));
        let mut out = Quaternion::default();
        quat_slerp::<StdTrig>(&p, &q, &mut out, 0.0);
        quat_approx(&out, &p, 2e-7);
        quat_slerp::<StdTrig>(&p, &q, &mut out, 1.0);
        quat_approx(&out, &q, 2e-7);
        let mut mid = Quaternion::default();
        assert!(quat_from_axis_angle(&axis, &mut mid, 0.7));
        quat_slerp::<StdTrig>(&p, &q, &mut out, 0.5);
        quat_approx(&out, &mid, 2e-6);

        // Nearly equal: linear branch.
        quat_slerp::<StdTrig>(&p, &p, &mut out, 0.25);
        quat_approx(&out, &p, 2e-7);

        // Exactly opposite: the C's perpendicular stores are dead code, and
        // the result is `sp*p + sq*(-p)`. At t = 0.25 (and t = 0.75 after
        // the `t -= 0.5` rebase) sp == sq, so mathematically every component
        // is 0. Retail computes it as `fmadds(sq, -p, sp*p)` (0x8037F0B4..F0):
        // the first product is rounded, the second is exact inside the fused
        // op, so what remains is the rounding error of `sp*p`, about 1e-8.
        let neg = Quaternion::new(-p.x, -p.y, -p.z, -p.w);
        let zero = Quaternion::new(0.0, 0.0, 0.0, 0.0);
        quat_slerp::<StdTrig>(&p, &neg, &mut out, 0.25);
        quat_approx(&out, &zero, 3e-8);
        quat_slerp::<StdTrig>(&p, &neg, &mut out, 0.75);
        quat_approx(&out, &zero, 3e-8);
        // Elsewhere it is (cos(pi t) - sin(pi t)) * p.
        quat_slerp::<StdTrig>(&p, &neg, &mut out, 0.1);
        let k = (0.1f32 * core::f32::consts::PI).cos() - (0.1f32 * core::f32::consts::PI).sin();
        quat_approx(
            &out,
            &Quaternion::new(k * p.x, k * p.y, k * p.z, k * p.w),
            1e-6,
        );
    }
}
