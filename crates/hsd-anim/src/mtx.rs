//! HSD matrix routines and the Dolphin SDK paired-single matrix/vector kernels
//! they call.
//!
//! Decomp sources:
//! - `src/sysdolphin/baselib/mtx.c`, `mtx.h`: the `HSD_Mtx*` functions (retail
//!   `0x80379310`..`0x8037A54C`). The `HSD_VecAlloc`/`HSD_MtxAlloc` allocator
//!   family is not ported; Rust owns its matrices.
//! - `extern/dolphin/src/dolphin/mtx/{mtx.c,vec.c,mtxvec.c}`: the SDK
//!   `PSMTX*`/`PSVEC*` routines. Melee links the paired-single assembly bodies
//!   (the `MTX*`/`VEC*` macros in `dolphin/mtx.h` resolve to `PS*` in the
//!   retail build, and `config/GALE01/symbols.txt` places `PSMTXConcat` at
//!   `0x80342204`), so those are transcribed instruction by instruction from
//!   the asm, not from the `C_*` reference versions, whose association and
//!   fusion differ.
//!
//! Paired-single transcription key (every op is single precision with one
//! rounding): `ps_mul`/`ps_muls0`/`ps_muls1` -> `*`; `ps_add`/`ps_sub`/
//! `ps_sum0`/`ps_sum1` -> `+`/`-`; `ps_madd`/`ps_madds0`/`ps_madds1` ->
//! [`fmadds`]; `ps_msub` -> [`fmsubs`]; `ps_nmsub` -> [`fnmsubs`]; `ps_nmadd`
//! -> [`fnmadds`]; `ps_neg` -> unary minus; `ps_merge*`, `psq_l`, `psq_st`
//! are data movement only. A `psq_l ..., 1, qr0` load fills the second slot
//! with `1.0`, and that `1.0` takes part in the arithmetic where noted.
//! `fres`/`frsqrte` go through [`gekko_math::estimate`], which is currently
//! an IEEE placeholder (see that module); the lines whose bits depend on it
//! are tagged `ESTIMATE PLACEHOLDER`. Where the asm feeds the double-width
//! `frsqrte` result into `fmuls`, this port rounds the `f64` product to `f32`;
//! the hardware additionally truncates the second operand to 25 mantissa
//! bits, which will matter once the estimate itself is exact.
//!
//! The HSD-level functions have no retail asm in this checkout, so they are
//! transcribed from the decomp C with separate `*` and `+`, and every
//! `a * b + c` shape is marked `// FUSION AUDIT PENDING`.
//!
//! Aliasing: the C lets `dest` alias an input. Rust references cannot, so
//! every function here takes `&Mtx` inputs and a `&mut Mtx` output; a caller
//! that needs the in-place form copies the input first. That is exact for
//! every SDK routine (each loads all of its inputs before its first store)
//! and for `HSD_MtxInverseConcat` / `HSD_MtxInverseTranspose` (they compute
//! into a temporary when aliased). `HSD_MtxInverse(m, m)` reads translation
//! entries it has already overwritten; no caller in the decomp does that and
//! it is not reproduced.

// Constants are spelled exactly as the C source spells them.
#![allow(clippy::excessive_precision)]

use gekko_math::estimate::{fres, frsqrte};
use gekko_math::fma::{fmadds, fmsubs, fnmadds, fnmsubs};
use gekko_math::msl::{cosf, sinf, sqrtf};
use hsd_types::{Mtx, Vec3};

use crate::quat::Quaternion;

/// `M_PI` from `src/MSL/math.h`, spelled as MSL spells it.
#[allow(clippy::approx_constant)]
pub const M_PI: f64 = 3.14159265358979323846;
/// `M_PI_2` from `src/MSL/math.h`: `(M_PI / 2)`.
pub const M_PI_2: f64 = M_PI / 2.0;

/// `EPSILON` in `mtx.c`: `0.0000000001f`.
const EPSILON: f32 = 0.0000000001;
/// `FLOAT_MIN` in `mtx.c`: `1.1754943E-38f`.
const FLOAT_MIN: f32 = 1.1754943E-38;

/// The inverse trigonometric functions Melee supplies from its own
/// `src/melee/lb/lbtrigf.c` (`atan2f`, `asinf`, `acosf`), which live in
/// `melee-lb` and must not be a dependency of this crate.
///
/// [`hsd_mtx_get_rotation`], [`crate::quat::mtx_to_euler`] and
/// [`crate::quat::quat_slerp`] take an implementor as a type parameter.
/// Production code passes the `melee-lb` transcription; tests may pass a
/// `std`-backed or stub implementation.
pub trait InverseTrig {
    /// `atan2f(y, x)`.
    fn atan2f(y: f32, x: f32) -> f32;
    /// `asinf(x)`.
    fn asinf(x: f32) -> f32;
    /// `acosf(x)`.
    fn acosf(x: f32) -> f32;
}

// ---------------------------------------------------------------------------
// Dolphin SDK: vec.c (paired-single bodies)
// ---------------------------------------------------------------------------

/// `PSVECAdd` (retail `0x80342D54`, `extern/dolphin/src/dolphin/mtx/vec.c`).
pub fn vec_add(a: &Vec3, b: &Vec3, c: &mut Vec3) {
    c.x = a.x + b.x;
    c.y = a.y + b.y;
    c.z = a.z + b.z;
}

/// `PSVECSubtract` (retail `0x80342D78`, `vec.c`).
pub fn vec_subtract(a: &Vec3, b: &Vec3, c: &mut Vec3) {
    c.x = a.x - b.x;
    c.y = a.y - b.y;
    c.z = a.z - b.z;
}

/// `PSVECScale` (retail `0x80342D9C`, `vec.c`).
pub fn vec_scale(src: &Vec3, dst: &mut Vec3, mult: f32) {
    dst.x = src.x * mult;
    dst.y = src.y * mult;
    dst.z = src.z * mult;
}

/// `PSVECSquareMag` (retail asm in `vec.c`).
///
/// ```text
/// ps_mul  f3, f2, f2        ; (x*x, y*y)
/// ps_madd f5, f4, f4, f3    ; (z*z + x*x, ...)
/// ps_sum0 f1, f5, f3, f3    ; f5.ps0 + f3.ps1
/// ```
pub fn vec_square_mag(v: &Vec3) -> f32 {
    let xx = v.x * v.x;
    let yy = v.y * v.y;
    fmadds(v.z, v.z, xx) + yy
}

/// `PSVECMag` (retail `0x80342DFC`, `vec.c`).
///
/// Square magnitude as [`vec_square_mag`], then `frsqrte` with one Newton
/// step, `fsel` to substitute the (zero) square magnitude when the estimate
/// is negative or NaN, and a final `sqsum * rsqrt`.
pub fn vec_mag(v: &Vec3) -> f32 {
    let xx = v.x * v.x;
    let yy = v.y * v.y;
    // ps_madd f1, f1, f1, f0 ; ps_sum0 f1, f1, f0, f0
    let sqsum = fmadds(v.z, v.z, xx) + yy;
    // frsqrte f0, f1
    let est = frsqrte(f64::from(sqsum)); // ESTIMATE PLACEHOLDER
                                         // fmuls f2, f0, f0 ; fmuls f0, f0, f4 (f4 = 0.5)
    let f2 = (est * est) as f32; // ESTIMATE PLACEHOLDER
    let f0 = (est * 0.5) as f32; // ESTIMATE PLACEHOLDER
                                 // fnmsubs f2, f2, f1, f3 (f3 = 3.0)
    let f2 = fnmsubs(f2, sqsum, 3.0);
    // fmuls f0, f2, f0
    let f0 = f2 * f0;
    // fsel f0, f0, f0, f1 : f0 >= 0 (including -0) ? f0 : sqsum
    let f0 = if f0 >= 0.0 { f0 } else { sqsum };
    // fmuls f1, f1, f0
    sqsum * f0
}

/// `PSVECNormalize` (retail `0x80342DB8`, `vec.c`).
///
/// `frsqrte` with one Newton step, then each component scaled. Unlike
/// `C_VECNormalize` there is no zero-magnitude guard: a zero vector yields
/// NaNs, exactly as on hardware.
pub fn vec_normalize(src: &Vec3, dst: &mut Vec3) {
    // ps_mul xx_yy, v1_xy, v1_xy
    let xx = src.x * src.x;
    let yy = src.y * src.y;
    // ps_madd xx_zz, v1_z, v1_z, xx_yy ; ps_sum0 sqsum, xx_zz, v1_z, xx_yy
    let sqsum = fmadds(src.z, src.z, xx) + yy;
    // frsqrte rsqrt, sqsum
    let rsqrt = frsqrte(f64::from(sqsum)); // ESTIMATE PLACEHOLDER
                                           // fmuls nwork0, rsqrt, rsqrt ; fmuls nwork1, rsqrt, c_half
    let nwork0 = (rsqrt * rsqrt) as f32; // ESTIMATE PLACEHOLDER
    let nwork1 = (rsqrt * 0.5) as f32; // ESTIMATE PLACEHOLDER
                                       // fnmsubs nwork0, nwork0, sqsum, c_three
    let nwork0 = fnmsubs(nwork0, sqsum, 3.0);
    // fmuls rsqrt, nwork0, nwork1
    let rsqrt = nwork0 * nwork1;
    // ps_muls0 on (x, y) and z
    dst.x = src.x * rsqrt;
    dst.y = src.y * rsqrt;
    dst.z = src.z * rsqrt;
}

/// `PSVECDotProduct` (retail `0x80342E38`, `vec.c`).
///
/// ```text
/// ps_mul  f2, (a.y, a.z), (b.y, b.z)     ; (ay*by, az*bz)
/// ps_madd f3, (a.x, a.y), (b.x, b.y), f2 ; (ax*bx + ay*by, ...)
/// ps_sum0 f1, f3, f2, f2                 ; f3.ps0 + f2.ps1
/// ```
pub fn vec_dot_product(a: &Vec3, b: &Vec3) -> f32 {
    let yy = a.y * b.y;
    let zz = a.z * b.z;
    fmadds(a.x, b.x, yy) + zz
}

/// `PSVECCrossProduct` (retail `0x80342E58`, `vec.c`).
///
/// ```text
/// ps_mul   f4, (b.x, b.y), a.z            ; (bx*az, by*az)
/// ps_muls0 f7, (b.x, b.y), a.x            ; (bx*ax, by*ax)
/// ps_msub  f5, (a.x, a.y), b.z, f4        ; (ax*bz - bx*az, ay*bz - by*az)
/// ps_msub  f8, (a.x, a.y), (b.y, b.x), f7 ; (ax*by - bx*ax, ay*bx - by*ax)
/// dst.x = f5.ps1 ; (dst.y, dst.z) = -(f5.ps0, f8.ps1)
/// ```
pub fn vec_cross_product(a: &Vec3, b: &Vec3, dst: &mut Vec3) {
    let f4_0 = b.x * a.z;
    let f4_1 = b.y * a.z;
    let f7_1 = b.y * a.x;
    let f5_0 = fmsubs(a.x, b.z, f4_0);
    let f5_1 = fmsubs(a.y, b.z, f4_1);
    let f8_1 = fmsubs(a.y, b.x, f7_1);
    dst.x = f5_1;
    dst.y = -f5_0;
    dst.z = -f8_1;
}

// ---------------------------------------------------------------------------
// Dolphin SDK: mtx.c and mtxvec.c (paired-single bodies)
// ---------------------------------------------------------------------------

/// `PSMTXIdentity` (retail `0x803421A4`, `mtx.c`).
pub fn mtx_identity(m: &mut Mtx) {
    *m = Mtx::IDENTITY;
}

/// `PSMTXCopy` (retail `0x803421D0`, `mtx.c`).
pub fn mtx_copy(src: &Mtx, dst: &mut Mtx) {
    *dst = *src;
}

/// `PSMTXConcat` (retail `0x80342204`, `mtx.c`): `ab = a * b`.
///
/// Per row `i` the asm accumulates, for each column pair, `b[0][j] * a[i][0]`
/// (`ps_muls0`), then `+ b[1][j] * a[i][1]` (`ps_madds1`), then
/// `+ b[2][j] * a[i][2]` (`ps_madds0`), and finally for columns 2 and 3
/// `+ Unit01 * a[i][3]` (`ps_madds1` with `Unit01 = (0, 1)`), so column 3
/// gets `+ 1.0 * a[i][3]` and column 2 gets `+ 0.0 * a[i][3]`. The last step
/// is kept literally: it turns an infinite or NaN translation into NaN in
/// column 2, as the hardware does.
pub fn mtx_concat(a: &Mtx, b: &Mtx, ab: &mut Mtx) {
    let a = a.0;
    let b = b.0;
    const UNIT01: [f32; 2] = [0.0, 1.0];
    let col = |i: usize, j: usize| -> f32 {
        let v = b[0][j] * a[i][0];
        let v = fmadds(b[1][j], a[i][1], v);
        fmadds(b[2][j], a[i][2], v)
    };
    for (i, out_row) in ab.0.iter_mut().enumerate() {
        *out_row = [
            col(i, 0),
            col(i, 1),
            fmadds(UNIT01[0], a[i][3], col(i, 2)),
            fmadds(UNIT01[1], a[i][3], col(i, 3)),
        ];
    }
}

/// `PSMTXTranspose` (retail `0x803422D0`, `mtx.c`): transposes the 3x3 part
/// and zeroes the translation column.
pub fn mtx_transpose(src: &Mtx, xpose: &mut Mtx) {
    let s = src.0;
    xpose.0 = [
        [s[0][0], s[1][0], s[2][0], 0.0],
        [s[0][1], s[1][1], s[2][1], 0.0],
        [s[0][2], s[1][2], s[2][2], 0.0],
    ];
}

/// The shared cofactor/determinant prologue of `PSMTXInverse` and
/// `PSMTXInvXpose` (identical instruction sequences).
///
/// Returns the determinant and the nine unscaled cofactors laid out as the
/// asm's `(f13, f12, f11)` pairs and `(f10, f9, f8)` singles:
/// `[c00, c10], [c01, c11], [c02, c12], c20, c21, c22`, where `c_ij` is the
/// value `PSMTXInverse` stores (after scaling) at `inv[i][j]`. The second
/// slot of the `f10`/`f9`/`f8` pairs mixes in the `1.0` from the `psq_l`
/// loads and is never stored, so it is not computed here.
fn ps_mtx_cofactors(m: &[[f32; 4]; 3]) -> (f32, [[f32; 2]; 3], [f32; 3]) {
    let (m00, m01, m02) = (m[0][0], m[0][1], m[0][2]);
    let (m10, m11, m12) = (m[1][0], m[1][1], m[1][2]);
    let (m20, m21, m22) = (m[2][0], m[2][1], m[2][2]);

    // ps_mul  f11, f3, f6         ; (m11*m02, m12*m00)
    // ps_mul  f13, f5, f7         ; (m21*m12, m22*m10)
    // ps_msub f11, f1, f7, f11    ; (m01*m12 - m11*m02, m02*m10 - m12*m00)
    // ps_mul  f12, f1, f8         ; (m01*m22, m02*m20)
    // ps_msub f13, f3, f8, f13    ; (m11*m22 - m21*m12, m12*m20 - m22*m10)
    // ps_mul  f10, f3, f4         ; (m11*m20, m12*1)
    // ps_msub f12, f5, f6, f12    ; (m21*m02 - m01*m22, m22*m00 - m02*m20)
    // ps_mul  f9,  f0, f5         ; (m00*m21, 1*m22)
    // ps_mul  f8,  f1, f2         ; (m01*m10, m02*1)
    // ps_msub f10, f2, f5, f10    ; (m10*m21 - m11*m20, 1*m22 - m12)
    // ps_msub f9,  f1, f4, f9     ; (m01*m20 - m00*m21, m02*1 - m22)
    // ps_msub f8,  f0, f3, f8     ; (m00*m11 - m01*m10, 1*m12 - m02)
    let f11 = [m11 * m02, m12 * m00];
    let f13 = [m21 * m12, m22 * m10];
    let f11 = [fmsubs(m01, m12, f11[0]), fmsubs(m02, m10, f11[1])];
    let f12 = [m01 * m22, m02 * m20];
    let f13 = [fmsubs(m11, m22, f13[0]), fmsubs(m12, m20, f13[1])];
    let f10 = m11 * m20;
    let f12 = [fmsubs(m21, m02, f12[0]), fmsubs(m22, m00, f12[1])];
    let f9 = m00 * m21;
    let f8 = m01 * m10;
    let f10 = fmsubs(m10, m21, f10);
    let f9 = fmsubs(m01, m20, f9);
    let f8 = fmsubs(m00, m11, f8);

    // ps_mul  f7, f0, f13         ; m00 * c00
    // ps_madd f7, f2, f12, f7     ; + m10 * c01
    // ps_madd f7, f4, f11, f7     ; + m20 * c02
    let det = m00 * f13[0];
    let det = fmadds(m10, f12[0], det);
    let det = fmadds(m20, f11[0], det);
    (det, [f13, f12, f11], [f10, f9, f8])
}

/// `PSMTXInverse` (retail `0x80342320`, `mtx.c`).
///
/// Returns `false` and leaves `inv` untouched when the determinant is exactly
/// zero (`ps_cmpo0` against `0.0`; a NaN determinant is *not* zero and falls
/// through). The reciprocal is `fres` refined by one Newton step:
/// `r = fres(det); r = -(det * (r*r) - (r + r))`.
pub fn mtx_inverse(src: &Mtx, inv: &mut Mtx) -> bool {
    let s = src.0;
    let (det, [f13, f12, f11], [f10, f9, f8]) = ps_mtx_cofactors(&s);
    if det == 0.0 {
        return false;
    }
    // fres f0, f7 ; ps_add f6, f0, f0 ; ps_mul f5, f0, f0 ; ps_nmsub f0, f7, f5, f6
    let r = fres(det); // ESTIMATE PLACEHOLDER
    let f6 = r + r;
    let f5 = r * r;
    let rdet = fnmsubs(det, f5, f6);

    let (s03, s13, s23) = (s[0][3], s[1][3], s[2][3]);

    // ps_muls0 on each cofactor pair.
    let i00 = f13[0] * rdet;
    let i10 = f13[1] * rdet;
    let i01 = f12[0] * rdet;
    let i11 = f12[1] * rdet;
    let i02 = f11[0] * rdet;
    let i12 = f11[1] * rdet;
    let i20 = f10 * rdet;
    let i21 = f9 * rdet;
    let i22 = f8 * rdet;

    // ps_mul   f6, f13, f1     ; (i00*s03, i10*s03)
    // ps_madd  f6, f12, f2, f6 ; (i01*s13 + .., i11*s13 + ..)
    // ps_nmadd f6, f11, f3, f6 ; (-(i02*s23 + ..), -(i12*s23 + ..))
    let i03 = fnmadds(i02, s23, fmadds(i01, s13, i00 * s03));
    let i13 = fnmadds(i12, s23, fmadds(i11, s13, i10 * s03));
    // ps_mul f7, f10, f1 ; ps_madd f7, f9, f2, f7 ; ps_nmadd f7, f8, f3, f7
    let i23 = fnmadds(i22, s23, fmadds(i21, s13, i20 * s03));

    inv.0 = [
        [i00, i01, i02, i03],
        [i10, i11, i12, i13],
        [i20, i21, i22, i23],
    ];
    true
}

/// `PSMTXInvXpose` (retail asm in `mtx.c`): inverse-transpose of the 3x3
/// part with a zero translation column.
///
/// Same prologue as [`mtx_inverse`]; the reciprocal uses `ps_res` and two
/// Newton steps instead of one. Returns `false` and leaves `inv_x` untouched
/// when the determinant is exactly zero.
pub fn mtx_inv_xpose(src: &Mtx, inv_x: &mut Mtx) -> bool {
    let s = src.0;
    let (det, [f13, f12, f11], [f10, f9, f8]) = ps_mtx_cofactors(&s);
    if det == 0.0 {
        return false;
    }
    // ps_res f0, f7
    let r = fres(det); // ESTIMATE PLACEHOLDER
                       // ps_add f6, f0, f0 ; ps_mul f5, f0, f0 ; ps_nmsub f0, f7, f5, f6  (twice)
    let f6 = r + r;
    let f5 = r * r;
    let r = fnmsubs(det, f5, f6);
    let f6 = r + r;
    let f5 = r * r;
    let rdet = fnmsubs(det, f5, f6);

    inv_x.0 = [
        [f13[0] * rdet, f13[1] * rdet, f10 * rdet, 0.0],
        [f12[0] * rdet, f12[1] * rdet, f9 * rdet, 0.0],
        [f11[0] * rdet, f11[1] * rdet, f8 * rdet, 0.0],
    ];
    true
}

/// `PSMTXRotTrig` (retail `0x80342488`, `mtx.c`).
///
/// `axis` is the ASCII axis letter; the asm ORs in `0x20` so upper case is
/// accepted. Any other value leaves `m` untouched (the asm branches to the
/// end without storing).
pub fn mtx_rot_trig(m: &mut Mtx, axis: u8, sin_a: f32, cos_a: f32) {
    let nsin_a = -sin_a;
    match axis | 0x20 {
        b'x' => {
            m.0 = [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, cos_a, nsin_a, 0.0],
                [0.0, sin_a, cos_a, 0.0],
            ];
        }
        b'y' => {
            m.0 = [
                [cos_a, 0.0, sin_a, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [nsin_a, 0.0, cos_a, 0.0],
            ];
        }
        b'z' => {
            m.0 = [
                [cos_a, nsin_a, 0.0, 0.0],
                [sin_a, cos_a, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ];
        }
        _ => {}
    }
}

/// `MTXRotRad` (`mtx.c`): `sinf`/`cosf` of `rad`, then [`mtx_rot_trig`].
pub fn mtx_rot_rad(m: &mut Mtx, axis: u8, rad: f32) {
    let sin_a = sinf(rad);
    let cos_a = cosf(rad);
    mtx_rot_trig(m, axis, sin_a, cos_a);
}

/// `PSMTXTrans` (retail `0x80342634`, `mtx.c`): identity rotation with the
/// given translation.
pub fn mtx_trans(m: &mut Mtx, x_t: f32, y_t: f32, z_t: f32) {
    m.0 = [
        [1.0, 0.0, 0.0, x_t],
        [0.0, 1.0, 0.0, y_t],
        [0.0, 0.0, 1.0, z_t],
    ];
}

/// `PSMTXScale` (retail `0x80342668`, `mtx.c`): diagonal scale, zero
/// translation.
pub fn mtx_scale(m: &mut Mtx, x_s: f32, y_s: f32, z_s: f32) {
    m.0 = [
        [x_s, 0.0, 0.0, 0.0],
        [0.0, y_s, 0.0, 0.0],
        [0.0, 0.0, z_s, 0.0],
    ];
}

/// `PSMTXQuat` (retail `0x80342690`, `mtx.c`): rotation matrix from a
/// quaternion, normalising by `2 / |q|^2`.
///
/// The scale is `s = (z*z + x*x) + (w*w + y*y)` (two `ps_madd` then
/// `ps_sum0`), `r = fres(s)`, one Newton step `r * (2 - s*r)`, times 2.
/// The products feeding each entry follow the asm's slot assignments; see
/// the inline comments.
// `c_zero = c_one - c_one` transcribes `fsubs c_zero, c_one, c_one`.
#[allow(clippy::eq_op)]
pub fn mtx_quat(m: &mut Mtx, q: &Quaternion) {
    let (x, y, z, w) = (q.x, q.y, q.z, q.w);
    let c_one = 1.0f32;
    let c_two = c_one + c_one;
    let c_zero = c_one - c_one;

    // ps_mul  tmp2, (x, y), (x, y)          ; (xx, yy)
    let xx = x * x;
    let yy = y * y;
    // ps_madd tmp4, (z, w), (z, w), tmp2    ; (zz + xx, ww + yy)   fused
    let tmp4_0 = fmadds(z, z, xx);
    let tmp4_1 = fmadds(w, w, yy);
    // ps_mul  tmp3, (z, w), (z, w)          ; (zz, ww)
    let zz = z * z;
    // ps_sum0 scale, tmp4, tmp4, tmp4       ; tmp4.ps0 + tmp4.ps1
    let scale = tmp4_0 + tmp4_1;
    // ps_muls1 tmp7, (y, x), w              ; (y*w, x*w)
    let yw = y * w;
    let xw = x * w;
    // fres tmp9, scale
    let tmp9 = fres(scale); // ESTIMATE PLACEHOLDER
                            // ps_sum1 tmp4, tmp3, tmp4, tmp2        ; (tmp4.ps0, zz + yy)
    let tmp4_1 = zz + yy;
    // ps_nmsub scale, scale, tmp9, c_two    ; 2 - scale*r
    let scale = fnmsubs(scale, tmp9, c_two);
    // ps_muls1 tmp6, (z, w), w              ; (z*w, w*w)
    let zw = z * w;
    // ps_mul scale, tmp9, scale             ; r * (2 - s*r)
    let scale = tmp9 * scale;
    // ps_sum0 tmp2, tmp2, tmp2, tmp2        ; xx + yy
    let tmp2_0 = xx + yy;
    // fmuls scale, scale, c_two
    let scale = scale * c_two;
    // ps_madd tmp8, (x, y), (y, x), tmp6    ; (x*y + z*w, ...)   fused
    let tmp8_0 = fmadds(x, y, zw);
    // ps_msub tmp6, (x, y), (y, x), tmp6    ; (x*y - z*w, ...)   fused
    let tmp6_0 = fmsubs(x, y, zw);
    // ps_nmsub tmp2, tmp2, scale, c_one     ; 1 - (xx+yy)*S
    let tmp2_0 = fnmsubs(tmp2_0, scale, c_one);
    // ps_nmsub tmp4, tmp4, scale, c_one     ; (1 - (zz+xx)*S, 1 - (zz+yy)*S)
    let tmp4_0 = fnmsubs(tmp4_0, scale, c_one);
    let tmp4_1 = fnmsubs(tmp4_1, scale, c_one);
    // ps_mul tmp8, tmp8, scale ; ps_mul tmp6, tmp6, scale
    let tmp8_0 = tmp8_0 * scale;
    let tmp6_0 = tmp6_0 * scale;
    // ps_madds0 tmp5, (x, y), z, tmp7       ; (x*z + y*w, y*z + x*w)   fused
    let tmp5_0 = fmadds(x, z, yw);
    let tmp5_1 = fmadds(y, z, xw);
    // ps_nmsub tmp7, tmp7, c_two, tmp5      ; (tmp5 - 2*yw, tmp5 - 2*xw)   fused
    let tmp7_0 = fnmsubs(yw, c_two, tmp5_0);
    let tmp7_1 = fnmsubs(xw, c_two, tmp5_1);
    // ps_mul tmp5, tmp5, scale ; ps_mul tmp7, tmp7, scale
    let tmp5_0 = tmp5_0 * scale;
    let tmp5_1 = tmp5_1 * scale;
    let tmp7_0 = tmp7_0 * scale;
    let tmp7_1 = tmp7_1 * scale;

    m.0 = [
        [tmp4_1, tmp6_0, tmp5_0, c_zero],
        [tmp8_0, tmp4_0, tmp7_1, c_zero],
        [tmp7_0, tmp5_1, tmp2_0, c_zero],
    ];
}

/// `PSMTXMultVec` (retail `0x80342AA8`, `mtxvec.c`): `dst = m * (src, 1)`.
///
/// Per row: `(m[i][2]*z + m[i][0]*x) + (m[i][3]*1 + m[i][1]*y)`, each half a
/// single `ps_madd`, joined by `ps_sum0`.
pub fn mtx_mult_vec(m: &Mtx, src: &Vec3, dst: &mut Vec3) {
    let (x, y, z) = (src.x, src.y, src.z);
    let one = 1.0f32;
    let row = |r: &[f32; 4]| -> f32 {
        // ps_mul f4, (m0, m1), (x, y) ; ps_madd f5, (m2, m3), (z, 1), f4 ; ps_sum0
        let f4_0 = r[0] * x;
        let f4_1 = r[1] * y;
        fmadds(r[2], z, f4_0) + fmadds(r[3], one, f4_1)
    };
    dst.x = row(&m.0[0]);
    dst.y = row(&m.0[1]);
    dst.z = row(&m.0[2]);
}

/// `PSMTXMultVecSR` (retail `0x80342AFC`, `mtxvec.c`): rotation/scale part
/// only, `dst = m3x3 * src`.
///
/// Per row: `m[i][2]*z + (m[i][0]*x + m[i][1]*y)` with the outer step fused.
pub fn mtx_mult_vec_sr(m: &Mtx, src: &Vec3, dst: &mut Vec3) {
    let (x, y, z) = (src.x, src.y, src.z);
    let row = |r: &[f32; 4]| -> f32 {
        // ps_mul f8, (m0, m1), (x, y) ; ps_sum0 f8 ; ps_madd f9, (m2, m3), (z, 1), f8
        let f8 = r[0] * x + r[1] * y;
        fmadds(r[2], z, f8)
    };
    dst.x = row(&m.0[0]);
    dst.y = row(&m.0[1]);
    dst.z = row(&m.0[2]);
}

// ---------------------------------------------------------------------------
// HSD: sysdolphin/baselib/mtx.c
// ---------------------------------------------------------------------------

/// `HSD_CalcDeterminantMatrix3x4` (static inline in `mtx.c`): determinant of
/// the 3x3 part, evaluated strictly left to right as written.
fn hsd_calc_determinant_3x4(m: &[[f32; 4]; 3]) -> f32 {
    // FUSION AUDIT PENDING (5 sites): each `+ a*b*c` / `- a*b*c` is a madd/msub shape.
    m[0][0] * m[1][1] * m[2][2] + m[0][1] * m[1][2] * m[2][0] + m[0][2] * m[1][0] * m[2][1]
        - m[2][0] * m[1][1] * m[0][2]
        - m[1][0] * m[0][1] * m[2][2]
        - m[0][0] * m[2][1] * m[1][2]
}

/// `fabsf_bitwise` (static inline in `mtx.h`): clears the sign bit.
#[inline]
fn fabsf_bitwise(v: f32) -> f32 {
    f32::from_bits(v.to_bits() & !0x8000_0000)
}

/// `HSD_MtxInverse` (retail `0x80379310`, `mtx.c`).
///
/// Writes the identity when `|det| < 1e-10`. See the module docs on why
/// `src` and `dest` may not alias here.
pub fn hsd_mtx_inverse(src: &Mtx, dest: &mut Mtx) {
    let m = src.0;
    let det = hsd_calc_determinant_3x4(&m);

    if fabsf_bitwise(det) < EPSILON {
        mtx_identity(dest);
        return;
    }

    let det = 1.0 / det;

    // FUSION AUDIT PENDING (9 sites): each `(a*b - c*d) * det` has an msub shape.
    let d00 = (m[1][1] * m[2][2] - m[2][1] * m[1][2]) * det;
    let d01 = -(m[0][1] * m[2][2] - m[2][1] * m[0][2]) * det;
    let d02 = (m[0][1] * m[1][2] - m[1][1] * m[0][2]) * det;
    let d10 = -(m[1][0] * m[2][2] - m[2][0] * m[1][2]) * det;
    let d11 = (m[0][0] * m[2][2] - m[2][0] * m[0][2]) * det;
    let d12 = -(m[0][0] * m[1][2] - m[1][0] * m[0][2]) * det;
    let d20 = (m[1][0] * m[2][1] - m[2][0] * m[1][1]) * det;
    let d21 = -(m[0][0] * m[2][1] - m[2][0] * m[0][1]) * det;
    let d22 = (m[0][0] * m[1][1] - m[1][0] * m[0][1]) * det;

    // FUSION AUDIT PENDING (3 sites): nested msub/nmsub shapes.
    let d03 = -(d02 * m[2][3] - (-d00 * m[0][3] - d01 * m[1][3]));
    let d13 = -(d12 * m[2][3] - (-d10 * m[0][3] - d11 * m[1][3]));
    let d23 = -(d22 * m[2][3] - (-d20 * m[0][3] - d21 * m[1][3]));

    dest.0 = [
        [d00, d01, d02, d03],
        [d10, d11, d12, d13],
        [d20, d21, d22, d23],
    ];
}

/// `HSD_MtxInverseConcat` (retail `0x80379598`, `mtx.c`): `dest = inv^-1 * src`.
///
/// When `|det(inv)| < 1e-10` the result is a copy of `src`.
pub fn hsd_mtx_inverse_concat(inv: &Mtx, src: &Mtx, dest: &mut Mtx) {
    let i = inv.0;
    let s = src.0;
    let det = hsd_calc_determinant_3x4(&i);

    if fabsf_bitwise(det) < EPSILON {
        mtx_copy(src, dest);
        return;
    }

    let det = 1.0 / det;
    // FUSION AUDIT PENDING (9 sites): `(a*b - c*d) * det`, msub shapes.
    let temp1 = ((i[1][1] * i[2][2]) - (i[2][1] * i[1][2])) * det;
    let temp2 = (-((i[0][1] * i[2][2]) - (i[2][1] * i[0][2]))) * det;
    let new_var = i[1][1];
    let temp3 = (-((i[1][0] * i[2][2]) - (i[2][0] * i[1][2]))) * det;
    let temp7 = ((i[0][1] * i[1][2]) - (new_var * i[0][2])) * det;
    let temp4 = ((i[0][0] * i[2][2]) - (i[2][0] * i[0][2])) * det;
    let temp8 = (-((i[0][0] * i[1][2]) - (i[1][0] * i[0][2]))) * det;
    let temp5 = ((i[1][0] * i[2][1]) - (i[2][0] * new_var)) * det;
    let temp6 = (-((i[0][0] * i[2][1]) - (i[2][0] * i[0][1]))) * det;
    let temp9 = ((i[0][0] * i[1][1]) - (i[1][0] * i[0][1])) * det;
    // FUSION AUDIT PENDING (3 sites): nested msub/nmsub shapes.
    let temp10 = -((temp7 * i[2][3]) - (((-temp1) * i[0][3]) - (temp2 * i[1][3])));
    let temp11 = -((temp8 * i[2][3]) - (((-temp3) * i[0][3]) - (temp4 * i[1][3])));
    let new_var = i[0][3];
    let temp12 = -((temp9 * i[2][3]) - (((-temp5) * new_var) - (temp6 * i[1][3])));

    // The aliased branch of the C computes into a temporary and copies; the
    // arithmetic is identical, so one body serves both.
    // FUSION AUDIT PENDING (12 sites): `a*s2 + (b*s0 + c*s1)` madd chains.
    let row = |ta: f32, tb: f32, tc: f32, tt: f32| -> [f32; 4] {
        [
            ta * s[2][0] + (tb * s[0][0] + tc * s[1][0]),
            ta * s[2][1] + (tb * s[0][1] + tc * s[1][1]),
            ta * s[2][2] + (tb * s[0][2] + tc * s[1][2]),
            ta * s[2][3] + (tb * s[0][3] + tc * s[1][3]) + tt,
        ]
    };
    dest.0 = [
        row(temp7, temp1, temp2, temp10),
        row(temp8, temp3, temp4, temp11),
        row(temp9, temp5, temp6, temp12),
    ];
}

/// `HSD_MtxInverseTranspose` (retail `0x80379A20`, `mtx.c`): inverse
/// transpose of the 3x3 part with a zero translation column.
///
/// When `|det| < 1e-10` the result is a copy of `src` (translation included).
pub fn hsd_mtx_inverse_transpose(src: &Mtx, dest: &mut Mtx) {
    let m = src.0;
    let det = hsd_calc_determinant_3x4(&m);

    if fabsf_bitwise(det) < EPSILON {
        mtx_copy(src, dest);
        return;
    }

    let det = 1.0 / det;

    // Same cofactors as HSD_MtxInverse, stored transposed.
    // FUSION AUDIT PENDING (9 sites): `(a*b - c*d) * det`, msub shapes.
    let d00 = ((m[1][1] * m[2][2]) - (m[2][1] * m[1][2])) * det;
    let d10 = -((m[0][1] * m[2][2]) - (m[2][1] * m[0][2])) * det;
    let d20 = ((m[0][1] * m[1][2]) - (m[1][1] * m[0][2])) * det;
    let d01 = -((m[1][0] * m[2][2]) - (m[2][0] * m[1][2])) * det;
    let d11 = ((m[0][0] * m[2][2]) - (m[2][0] * m[0][2])) * det;
    let d21 = -((m[0][0] * m[1][2]) - (m[1][0] * m[0][2])) * det;
    let d02 = ((m[1][0] * m[2][1]) - (m[2][0] * m[1][1])) * det;
    let d12 = -((m[0][0] * m[2][1]) - (m[2][0] * m[0][1])) * det;
    let d22 = ((m[0][0] * m[1][1]) - (m[1][0] * m[0][1])) * det;

    dest.0 = [
        [d00, d01, d02, 0.0],
        [d10, d11, d12, 0.0],
        [d20, d21, d22, 0.0],
    ];
}

/// `calcVal` (static inline in `mtx.c`): `atan2f(y, x)` with a hard
/// `+-pi/2` when `|x|` is at or below `FLOAT_MIN`.
fn calc_val<T: InverseTrig>(x: f32, y: f32) -> f32 {
    if fabsf_bitwise(x) <= FLOAT_MIN {
        if y >= 0.0 {
            (M_PI / 2.0) as f32
        } else {
            (-M_PI / 2.0) as f32
        }
    } else {
        T::atan2f(y, x)
    }
}

/// `HSD_MtxGetRotation` (retail `0x80379C24`, `mtx.c`): Euler angles
/// (x, y, z) from the rotation part, ignoring scale.
///
/// Returns all zeros if any column has a magnitude below `FLOAT_MIN`.
// The C tests `!(len < FLOAT_MIN)`, which proceeds on NaN; `>=` would not.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn hsd_mtx_get_rotation<T: InverseTrig>(m: &Mtx, vec: &mut Vec3) {
    let m = m.0;

    // FUSION AUDIT PENDING: sum of three squares, madd shapes.
    let length0 = sqrtf(m[0][0] * m[0][0] + m[1][0] * m[1][0] + m[2][0] * m[2][0]);
    if !(length0 < FLOAT_MIN) {
        // FUSION AUDIT PENDING
        let length1 = sqrtf(m[0][1] * m[0][1] + m[1][1] * m[1][1] + m[2][1] * m[2][1]);
        if !(length1 < FLOAT_MIN) {
            // FUSION AUDIT PENDING
            let length2 = sqrtf(m[0][2] * m[0][2] + m[1][2] * m[1][2] + m[2][2] * m[2][2]);
            if !(length2 < FLOAT_MIN) {
                let mut test_val_1 = -m[2][0];
                test_val_1 /= length0;

                let val_01 = if test_val_1 >= 1.0 {
                    (M_PI / 2.0) as f32
                } else if test_val_1 <= -1.0 {
                    (-M_PI / 2.0) as f32
                } else {
                    T::asinf(test_val_1)
                };

                vec.y = val_01;

                if cosf(vec.y) >= FLOAT_MIN {
                    let test_val_2_pre = m[2][2] / length2;
                    let test_val_3_pre = m[2][1] / length1;

                    vec.x = calc_val::<T>(test_val_2_pre, test_val_3_pre);
                    vec.z = calc_val::<T>(m[0][0], m[1][0]);
                    return;
                }

                vec.x = calc_val::<T>(m[1][1], m[0][1]);
                vec.z = 0.0;
                return;
            }
        }
    }

    vec.x = 0.0;
    vec.y = 0.0;
    vec.z = 0.0;
}

/// `HSD_MtxGetTranslate` (retail `0x80379F6C`, `mtx.c`).
pub fn hsd_mtx_get_translate(mat: &Mtx, vec: &mut Vec3) {
    vec.x = mat.0[0][3];
    vec.y = mat.0[1][3];
    vec.z = mat.0[2][3];
}

/// `HSD_MtxGetScale` (retail `0x80379F88`, `mtx.c`): per-axis scale by
/// Gram-Schmidt over the columns using the SDK vector routines, negated as
/// a whole when the columns form a left-handed frame.
pub fn hsd_mtx_get_scale(arg0: &Mtx, arg1: &mut Vec3) {
    let m = arg0.0;
    let mut vec1 = Vec3::new(m[0][0], m[1][0], m[2][0]);
    let mut vec4 = Vec3::ZERO;

    arg1.x = vec_mag(&vec1);
    let t = vec1;
    vec_normalize(&t, &mut vec1);

    let mut vec2 = Vec3::new(m[0][1], m[1][1], m[2][1]);

    vec_scale(&vec1, &mut vec4, vec_dot_product(&vec1, &vec2));
    let t = vec2;
    vec_subtract(&t, &vec4, &mut vec2);
    arg1.y = vec_mag(&vec2);
    let t = vec2;
    vec_normalize(&t, &mut vec2);

    let mut vec3 = Vec3::new(m[0][2], m[1][2], m[2][2]);

    vec_scale(&vec2, &mut vec4, vec_dot_product(&vec2, &vec3));
    let t = vec3;
    vec_subtract(&t, &vec4, &mut vec3);
    vec_scale(&vec1, &mut vec4, vec_dot_product(&vec1, &vec3));
    let t = vec3;
    vec_subtract(&t, &vec4, &mut vec3);
    arg1.z = vec_mag(&vec3);
    let t = vec3;
    vec_normalize(&t, &mut vec3);
    vec_cross_product(&vec2, &vec3, &mut vec4);

    if vec_dot_product(&vec1, &vec4) < 0.0 {
        // `f64 scale = -1.0; arg1->x *= scale;` : promote, multiply, round.
        let scale: f64 = -1.0;
        arg1.x = (f64::from(arg1.x) * scale) as f32;
        arg1.y = (f64::from(arg1.y) * scale) as f32;
        arg1.z = (f64::from(arg1.z) * scale) as f32;
    }
}

/// `HSD_MkRotationMtx` (retail `0x8037A120`, `mtx.c`): rotation matrix from
/// Euler angles (x, then y, then z), zero translation.
pub fn hsd_mk_rotation_mtx(arg0: &mut Mtx, arg1: &Vec3) {
    let sin_x = sinf(arg1.x);
    let cos_x = cosf(arg1.x);
    let sin_y = sinf(arg1.y);
    let cos_y = cosf(arg1.y);
    let sin_z = sinf(arg1.z);
    let cos_z = cosf(arg1.z);

    let temp1 = sin_x * sin_y;
    let m00 = cos_y * cos_z;
    let m10 = cos_y * sin_z;
    let m20 = -sin_y;
    let temp2 = cos_x * sin_y;
    // FUSION AUDIT PENDING (4 sites): `a*b -/+ c*d` msub/madd shapes.
    let m01 = (cos_z * temp1) - (cos_x * sin_z);
    let m11 = (sin_z * temp1) + (cos_x * cos_z);
    let m21 = sin_x * cos_y;
    let m02 = (cos_z * temp2) + (sin_x * sin_z);
    let m12 = (sin_z * temp2) - (sin_x * cos_z);
    let m22 = cos_x * cos_y;

    arg0.0 = [
        [m00, m01, m02, 0.0],
        [m10, m11, m12, 0.0],
        [m20, m21, m22, 0.0],
    ];
}

/// `HSD_MtxQuat` (retail `0x8037A230`, `mtx.c`): thin wrapper over
/// `MTXQuat` -> [`mtx_quat`].
pub fn hsd_mtx_quat(arg0: &mut Mtx, arg1: &Quaternion) {
    mtx_quat(arg0, arg1);
}

/// `HSD_MtxSRT` (retail `0x8037A250`, `mtx.c`): scale * rotation(Euler) *
/// translation, with an optional parent-scale compensation `vec4`.
///
/// `vec1` is the scale, `vec2` the Euler rotation, `vec3` the translation.
/// When `vec4` is `Some`, its reciprocals are formed in `f64`
/// (`f32 temp1 = 1.0 / vec4->x`) and rounded once, as the C does.
pub fn hsd_mtx_srt(m: &mut Mtx, vec1: &Vec3, vec2: &Vec3, vec3: &Vec3, vec4: Option<&Vec3>) {
    let sin_x = sinf(vec2.x);
    let cos_x = cosf(vec2.x);
    let sin_y = sinf(vec2.y);
    let cos_y = cosf(vec2.y);
    let sin_z = sinf(vec2.z);
    let cos_z = cosf(vec2.z);

    let mut vec1x = vec1.x;
    let mut vec1x_1 = vec1x;
    let vec1x_2 = vec1x;
    let mut vec1y = vec1.y;
    let vec1y_1 = vec1y;
    let mut vec1y_2 = vec1y;
    let vec1z = vec1.z;
    let mut vec1z_1 = vec1z;
    let mut vec1z_2 = vec1z;

    if let Some(vec4) = vec4 {
        let temp1 = (1.0f64 / f64::from(vec4.x)) as f32;
        let temp2 = (1.0f64 / f64::from(vec4.y)) as f32;
        let temp3 = (1.0f64 / f64::from(vec4.z)) as f32;

        vec1y_2 *= vec4.y * temp1;
        vec1z_2 *= vec4.z * temp1;
        vec1x_1 *= vec4.x * temp2;
        vec1z_1 *= vec4.z * temp2;
        vec1x *= vec4.x * temp3;
        vec1y *= vec4.y * temp3;
    }

    let m00 = cos_z * (vec1x_2 * cos_y);
    let m10 = sin_z * (vec1x_1 * cos_y);
    let m20 = -vec1x * sin_y;
    // FUSION AUDIT PENDING (4 sites): inner `a*(b*c) -/+ d*e` msub/madd shapes.
    let m01 = vec1y_2 * ((cos_z * (sin_x * sin_y)) - (cos_x * sin_z));
    let m11 = vec1y_1 * ((sin_z * (sin_x * sin_y)) + (cos_x * cos_z));
    let m21 = cos_y * (vec1y * sin_x);
    let m02 = vec1z_2 * ((cos_z * (cos_x * sin_y)) + (sin_x * sin_z));
    let m12 = vec1z_1 * ((sin_z * (cos_x * sin_y)) - (sin_x * cos_z));
    let m22 = cos_y * (vec1z * cos_x);

    m.0 = [
        [m00, m01, m02, vec3.x],
        [m10, m11, m12, vec3.y],
        [m20, m21, m22, vec3.z],
    ];
}

/// `HSD_MtxSRTQuat` (retail `0x8037A43C`, `mtx.c`): scale, then
/// rotation(quat), then translation via SDK concatenation, with an optional
/// parent-scale compensation `arg4` applied as `scale(arg4) ... scale(1/arg4)`.
///
/// Each `MTXConcat(temp, arg0, arg0)` in the C aliases its second input and
/// output; `PSMTXConcat` loads both inputs before storing, so a copy of
/// `arg0` is passed here.
pub fn hsd_mtx_srt_quat(
    arg0: &mut Mtx,
    arg1: &Vec3,
    arg2: &Quaternion,
    arg3: &Vec3,
    arg4: Option<&Vec3>,
) {
    let mut temp = Mtx::ZERO;

    mtx_scale(arg0, arg1.x, arg1.y, arg1.z);

    if let Some(arg4) = arg4 {
        mtx_scale(&mut temp, arg4.x, arg4.y, arg4.z);
        let b = *arg0;
        mtx_concat(&temp, &b, arg0);
    }

    mtx_quat(&mut temp, arg2);
    let b = *arg0;
    mtx_concat(&temp, &b, arg0);

    if let Some(arg4) = arg4 {
        // `MTXScale(temp, 1.0 / arg4->x, ...)`: f64 reciprocal rounded to f32.
        mtx_scale(
            &mut temp,
            (1.0f64 / f64::from(arg4.x)) as f32,
            (1.0f64 / f64::from(arg4.y)) as f32,
            (1.0f64 / f64::from(arg4.z)) as f32,
        );
        let b = *arg0;
        mtx_concat(&temp, &b, arg0);
    }

    mtx_trans(&mut temp, arg3.x, arg3.y, arg3.z);
    let b = *arg0;
    mtx_concat(&temp, &b, arg0);
}

/// `HSD_MtxScaledAdd` (retail `0x8037A54C`, `mtx.c`): `arg2 = arg1 + arg3 * arg0`
/// element-wise over all twelve entries.
pub fn hsd_mtx_scaled_add(arg0: &Mtx, arg1: &Mtx, arg2: &mut Mtx, arg3: f32) {
    for (r2, (r1, r0)) in arg2.0.iter_mut().zip(arg1.0.iter().zip(arg0.0.iter())) {
        for (e2, (e1, e0)) in r2.iter_mut().zip(r1.iter().zip(r0.iter())) {
            // FUSION AUDIT PENDING: `b + s*a` is a madd shape.
            *e2 = *e1 + (arg3 * *e0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `std`-backed inverse trig for tolerance tests only.
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

    fn mtx_approx(a: &Mtx, b: &Mtx, tol: f32) {
        for (i, (ra, rb)) in a.0.iter().zip(b.0.iter()).enumerate() {
            for (j, (x, y)) in ra.iter().zip(rb.iter()).enumerate() {
                assert!(approx(*x, *y, tol), "[{i}][{j}]: {x} vs {y}\n{a:?}\n{b:?}");
            }
        }
    }

    fn sample() -> Mtx {
        Mtx([
            [0.5, -1.25, 2.0, 3.5],
            [1.5, 0.75, -0.5, -2.0],
            [-0.25, 2.5, 1.0, 4.0],
        ])
    }

    fn sample_b() -> Mtx {
        Mtx([
            [1.1, 0.3, -0.7, 0.9],
            [-2.2, 1.7, 0.4, -0.1],
            [0.6, -1.3, 2.9, 1.2],
        ])
    }

    #[test]
    fn identity_concat_is_identity_bit_for_bit() {
        let m = sample();
        let mut out = Mtx::ZERO;
        mtx_concat(&Mtx::IDENTITY, &m, &mut out);
        assert_eq!(out, m);
        mtx_concat(&m, &Mtx::IDENTITY, &mut out);
        // Column 2 picks up `+ 0.0 * a[i][3]`, exact for finite input.
        assert_eq!(out, m);
    }

    #[test]
    fn concat_matches_hand_evaluated_fused_chain() {
        let a = sample();
        let b = sample_b();
        let mut out = Mtx::ZERO;
        mtx_concat(&a, &b, &mut out);
        // Row 1, column 0 and column 3, evaluated as the asm does.
        let v = b.0[0][0] * a.0[1][0];
        let v = fmadds(b.0[1][0], a.0[1][1], v);
        let v = fmadds(b.0[2][0], a.0[1][2], v);
        assert_eq!(out.0[1][0].to_bits(), v.to_bits());
        let t = b.0[0][3] * a.0[1][0];
        let t = fmadds(b.0[1][3], a.0[1][1], t);
        let t = fmadds(b.0[2][3], a.0[1][2], t);
        let t = fmadds(1.0, a.0[1][3], t);
        assert_eq!(out.0[1][3].to_bits(), t.to_bits());
    }

    #[test]
    fn concat_infinite_translation_poisons_column_two() {
        let mut a = Mtx::IDENTITY;
        a.0[0][3] = f32::INFINITY;
        let mut out = Mtx::ZERO;
        mtx_concat(&a, &Mtx::IDENTITY, &mut out);
        assert!(out.0[0][2].is_nan(), "0 * inf must poison column 2");
        assert_eq!(out.0[0][3], f32::INFINITY);
    }

    #[test]
    fn ps_inverse_round_trip() {
        let m = sample();
        let mut inv = Mtx::ZERO;
        assert!(mtx_inverse(&m, &mut inv));
        let mut prod = Mtx::ZERO;
        mtx_concat(&m, &inv, &mut prod);
        mtx_approx(&prod, &Mtx::IDENTITY, 2e-6);
        mtx_concat(&inv, &m, &mut prod);
        mtx_approx(&prod, &Mtx::IDENTITY, 2e-6);
    }

    #[test]
    fn ps_inverse_singular_leaves_output_untouched() {
        let mut m = sample();
        m.0[2] = m.0[1];
        let sentinel = Mtx([[7.0; 4]; 3]);
        let mut inv = sentinel;
        assert!(!mtx_inverse(&m, &mut inv));
        assert_eq!(inv, sentinel);
        assert!(!mtx_inv_xpose(&m, &mut inv));
        assert_eq!(inv, sentinel);
    }

    #[test]
    fn ps_inverse_of_scale_translate_is_exact() {
        // det = 64; the fres placeholder gives 1/64 exactly and the Newton
        // step is then a no-op, so every entry is exactly representable.
        let m = Mtx([
            [2.0, 0.0, 0.0, 1.0],
            [0.0, 4.0, 0.0, 2.0],
            [0.0, 0.0, 8.0, 3.0],
        ]);
        let mut inv = Mtx::ZERO;
        assert!(mtx_inverse(&m, &mut inv));
        let expect = Mtx([
            [0.5, 0.0, 0.0, -0.5],
            [0.0, 0.25, 0.0, -0.5],
            [0.0, 0.0, 0.125, -0.375],
        ]);
        // Some zero entries are -0.0 (from `-(0 + 0)`); PartialEq treats them as 0.
        assert_eq!(inv, expect);
        let mut ix = Mtx::ZERO;
        assert!(mtx_inv_xpose(&m, &mut ix));
        assert_eq!(
            ix,
            Mtx([
                [0.5, 0.0, 0.0, 0.0],
                [0.0, 0.25, 0.0, 0.0],
                [0.0, 0.0, 0.125, 0.0],
            ])
        );
    }

    #[test]
    fn hsd_inverse_of_scale_translate_is_exact() {
        let m = Mtx([
            [2.0, 0.0, 0.0, 1.0],
            [0.0, 4.0, 0.0, 2.0],
            [0.0, 0.0, 8.0, 3.0],
        ]);
        let mut inv = Mtx::ZERO;
        hsd_mtx_inverse(&m, &mut inv);
        let expect = Mtx([
            [0.5, 0.0, 0.0, -0.5],
            [0.0, 0.25, 0.0, -0.5],
            [0.0, 0.0, 0.125, -0.375],
        ]);
        assert_eq!(inv, expect);

        let mut it = Mtx::ZERO;
        hsd_mtx_inverse_transpose(&m, &mut it);
        assert_eq!(
            it,
            Mtx([
                [0.5, 0.0, 0.0, 0.0],
                [0.0, 0.25, 0.0, 0.0],
                [0.0, 0.0, 0.125, 0.0],
            ])
        );
    }

    #[test]
    fn hsd_inverse_round_trip_and_singular_identity() {
        let m = sample();
        let mut inv = Mtx::ZERO;
        hsd_mtx_inverse(&m, &mut inv);
        let mut prod = Mtx::ZERO;
        mtx_concat(&m, &inv, &mut prod);
        mtx_approx(&prod, &Mtx::IDENTITY, 2e-6);

        let mut sing = sample();
        sing.0[2] = sing.0[1];
        hsd_mtx_inverse(&sing, &mut inv);
        assert_eq!(inv, Mtx::IDENTITY);

        // InverseConcat(m, m) ~ identity; singular inv -> copy of src.
        let mut out = Mtx::ZERO;
        hsd_mtx_inverse_concat(&m, &m, &mut out);
        mtx_approx(&out, &Mtx::IDENTITY, 2e-6);
        hsd_mtx_inverse_concat(&sing, &m, &mut out);
        assert_eq!(out, m);
        hsd_mtx_inverse_transpose(&sing, &mut out);
        assert_eq!(out, sing);
    }

    #[test]
    fn hsd_inverse_concat_matches_inverse_then_concat_closely() {
        let a = sample();
        let b = sample_b();
        let mut ic = Mtx::ZERO;
        hsd_mtx_inverse_concat(&a, &b, &mut ic);
        let mut inv = Mtx::ZERO;
        hsd_mtx_inverse(&a, &mut inv);
        let mut ref_out = Mtx::ZERO;
        mtx_concat(&inv, &b, &mut ref_out);
        mtx_approx(&ic, &ref_out, 1e-4);
    }

    #[test]
    fn quat_identity_gives_identity_matrix() {
        let mut m = Mtx::ZERO;
        mtx_quat(&mut m, &Quaternion::IDENTITY);
        // `-(0*2 - 0)` yields -0.0 at [1][2] and [2][0]; PartialEq treats it as 0.
        assert_eq!(m, Mtx::IDENTITY);
        assert_eq!(m.0[1][2].to_bits(), (-0.0f32).to_bits());
        let mut h = Mtx::ZERO;
        hsd_mtx_quat(&mut h, &Quaternion::IDENTITY);
        assert_eq!(h, m);
    }

    #[test]
    fn quat_quarter_turn_about_z() {
        // q = (0, 0, sin(pi/4), cos(pi/4)) is a 90 degree turn about z.
        let s = core::f32::consts::FRAC_1_SQRT_2;
        let q = Quaternion::new(0.0, 0.0, s, s);
        let mut m = Mtx::ZERO;
        mtx_quat(&mut m, &q);
        let expect = Mtx([
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]);
        mtx_approx(&m, &expect, 2e-7);
    }

    #[test]
    fn vector_kernels() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(-4.0, 0.5, 2.0);
        assert_eq!(vec_dot_product(&a, &b), -4.0 + 1.0 + 6.0);
        let mut c = Vec3::ZERO;
        vec_cross_product(&a, &b, &mut c);
        assert_eq!(
            c,
            Vec3::new(
                2.0 * 2.0 - 3.0 * 0.5,
                3.0 * -4.0 - 1.0 * 2.0,
                1.0 * 0.5 - 2.0 * -4.0
            )
        );
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        vec_cross_product(&x, &y, &mut c);
        assert_eq!(c, Vec3::new(0.0, 0.0, 1.0));
        vec_add(&a, &b, &mut c);
        assert_eq!(c, Vec3::new(-3.0, 2.5, 5.0));
        vec_subtract(&a, &b, &mut c);
        assert_eq!(c, Vec3::new(5.0, 1.5, 1.0));
        vec_scale(&a, &mut c, 0.5);
        assert_eq!(c, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(vec_square_mag(&a), 14.0);

        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!(approx(vec_mag(&v), 5.0, 1e-6));
        assert_eq!(vec_mag(&Vec3::ZERO), 0.0, "fsel path on zero vector");
        let mut n = Vec3::ZERO;
        vec_normalize(&v, &mut n);
        assert!(approx(n.x, 0.6, 1e-6) && approx(n.y, 0.8, 1e-6));
        assert_eq!(n.z, 0.0);
    }

    #[test]
    fn mult_vec_and_sr() {
        let m = Mtx([
            [2.0, 0.0, 0.0, 10.0],
            [0.0, 3.0, 0.0, 20.0],
            [0.0, 0.0, 4.0, 30.0],
        ]);
        let v = Vec3::new(1.0, 2.0, 3.0);
        let mut out = Vec3::ZERO;
        mtx_mult_vec(&m, &v, &mut out);
        assert_eq!(out, Vec3::new(12.0, 26.0, 42.0));
        mtx_mult_vec_sr(&m, &v, &mut out);
        assert_eq!(out, Vec3::new(2.0, 6.0, 12.0));
    }

    #[test]
    fn builders() {
        let mut m = Mtx::ZERO;
        mtx_trans(&mut m, 1.0, 2.0, 3.0);
        let mut t = Vec3::ZERO;
        hsd_mtx_get_translate(&m, &mut t);
        assert_eq!(t, Vec3::new(1.0, 2.0, 3.0));
        mtx_scale(&mut m, 2.0, 3.0, 4.0);
        assert_eq!(m.0[1][1], 3.0);
        let mut s = Vec3::ZERO;
        hsd_mtx_get_scale(&m, &mut s);
        assert!(approx(s.x, 2.0, 1e-6) && approx(s.y, 3.0, 1e-6) && approx(s.z, 4.0, 1e-6));
        // A left-handed frame flips the sign of all three.
        mtx_scale(&mut m, -2.0, 3.0, 4.0);
        hsd_mtx_get_scale(&m, &mut s);
        assert!(approx(s.x, -2.0, 1e-6) && approx(s.y, -3.0, 1e-6) && approx(s.z, -4.0, 1e-6));

        let mut tr = Mtx::ZERO;
        mtx_transpose(&sample(), &mut tr);
        assert_eq!(tr.0[0][1], sample().0[1][0]);
        assert_eq!(tr.0[2][3], 0.0);

        mtx_rot_trig(&mut m, b'Z', 1.0, 0.0);
        assert_eq!(m.0[0][1], -1.0);
        assert_eq!(m.0[1][0], 1.0);
        let before = m;
        mtx_rot_trig(&mut m, b'q', 1.0, 0.0);
        assert_eq!(m, before, "unknown axis must not store");
        mtx_rot_rad(&mut m, b'x', 0.0);
        mtx_approx(&m, &Mtx::IDENTITY, 1e-7);

        let mut c = Mtx::ZERO;
        mtx_copy(&sample(), &mut c);
        assert_eq!(c, sample());
        mtx_identity(&mut c);
        assert_eq!(c, Mtx::IDENTITY);
    }

    #[test]
    fn rotation_round_trip_through_get_rotation() {
        let rot = Vec3::new(0.3, -0.7, 1.1);
        let mut m = Mtx::ZERO;
        hsd_mk_rotation_mtx(&mut m, &rot);
        let mut back = Vec3::ZERO;
        hsd_mtx_get_rotation::<StdTrig>(&m, &mut back);
        assert!(approx(back.x, rot.x, 1e-5), "{back:?}");
        assert!(approx(back.y, rot.y, 1e-5), "{back:?}");
        assert!(approx(back.z, rot.z, 1e-5), "{back:?}");

        // A zero column gives zeros.
        let mut z = Mtx::ZERO;
        z.0[1][1] = 1.0;
        hsd_mtx_get_rotation::<StdTrig>(&z, &mut back);
        assert_eq!(back, Vec3::ZERO);

        // Gimbal branch: y pinned at +pi/2, z forced to zero.
        let mut g = Mtx::IDENTITY;
        g.0[2][0] = -1.0;
        g.0[0][0] = 0.0;
        hsd_mtx_get_rotation::<StdTrig>(&g, &mut back);
        assert_eq!(back.y, (M_PI / 2.0) as f32);
        assert_eq!(back.z, 0.0);
    }

    #[test]
    fn srt_and_srt_quat_agree_on_pure_scale_translate() {
        let scale = Vec3::new(2.0, 3.0, 4.0);
        let rot = Vec3::ZERO;
        let trans = Vec3::new(5.0, 6.0, 7.0);
        let mut a = Mtx::ZERO;
        hsd_mtx_srt(&mut a, &scale, &rot, &trans, None);
        let expect = Mtx([
            [2.0, 0.0, 0.0, 5.0],
            [0.0, 3.0, 0.0, 6.0],
            [0.0, 0.0, 4.0, 7.0],
        ]);
        mtx_approx(&a, &expect, 1e-6);

        let mut b = Mtx::ZERO;
        hsd_mtx_srt_quat(&mut b, &scale, &Quaternion::IDENTITY, &trans, None);
        mtx_approx(&b, &expect, 1e-6);

        // Parent-scale compensation with a unit vector is a no-op up to rounding.
        let one = Vec3::new(1.0, 1.0, 1.0);
        hsd_mtx_srt(&mut a, &scale, &rot, &trans, Some(&one));
        mtx_approx(&a, &expect, 1e-6);
        hsd_mtx_srt_quat(&mut b, &scale, &Quaternion::IDENTITY, &trans, Some(&one));
        mtx_approx(&b, &expect, 1e-6);
    }

    #[test]
    fn srt_matches_mk_rotation_for_unit_scale() {
        let rot = Vec3::new(0.4, 0.9, -0.2);
        let one = Vec3::new(1.0, 1.0, 1.0);
        let mut r = Mtx::ZERO;
        hsd_mk_rotation_mtx(&mut r, &rot);
        let mut s = Mtx::ZERO;
        hsd_mtx_srt(&mut s, &one, &rot, &Vec3::ZERO, None);
        mtx_approx(&s, &r, 2e-7);
    }

    #[test]
    fn scaled_add_is_exact_on_representable_values() {
        let a = Mtx([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
        ]);
        let b = Mtx([[0.5; 4]; 3]);
        let mut out = Mtx::ZERO;
        hsd_mtx_scaled_add(&a, &b, &mut out, 0.25);
        for (ro, ra) in out.0.iter().zip(a.0.iter()) {
            for (o, x) in ro.iter().zip(ra.iter()) {
                assert_eq!(*o, 0.5 + 0.25 * *x);
            }
        }
    }
}
