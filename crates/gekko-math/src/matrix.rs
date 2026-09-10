//! Audited HSD inverse (80379310) and PSMTXMultVec (80342AA8).
use crate::fma::{fmadds, fmsubs, fnmsubs};
const EPSILON: f32 = 1e-10;
pub fn transform_point(m: &[[f32; 4]; 3], src: [f32; 3]) -> [f32; 3] {
    let (x, y, z) = (src[0], src[1], src[2]);
    let one = 1.0f32;
    let row = |r: &[f32; 4]| -> f32 {
        // ps_mul f4, (m0, m1), (x, y) ; ps_madd f5, (m2, m3), (z, 1), f4 ; ps_sum0
        let f4_0 = r[0] * x;
        let f4_1 = r[1] * y;
        fmadds(r[2], z, f4_0) + fmadds(r[3], one, f4_1)
    };
    [row(&m[0]), row(&m[1]), row(&m[2])]
}
fn hsd_calc_determinant_3x4(m: &[[f32; 4]; 3]) -> f32 {
    // retail 0x80379368..0x80379384: two fmadds, then three fnmsubs.
    // Also inlined at 0x803795F4..0x80379614 and 0x80379A70..0x80379A8C.
    let second_term = m[2][0] * (m[0][1] * m[1][2]);
    let det = fmadds(m[2][2], m[0][0] * m[1][1], second_term);
    let det = fmadds(m[2][1], m[0][2] * m[1][0], det);
    let det = fnmsubs(m[0][2], m[2][0] * m[1][1], det);
    let det = fnmsubs(m[2][2], m[1][0] * m[0][1], det);
    fnmsubs(m[1][2], m[0][0] * m[2][1], det)
}

/// `fabsf_bitwise` (static inline in `mtx.h`): clears the sign bit.
#[inline]
fn fabsf_bitwise(v: f32) -> f32 {
    f32::from_bits(v.to_bits() & !0x8000_0000)
}

pub fn inverse(src: &[[f32; 4]; 3]) -> [[f32; 4]; 3] {
    let m = *src;
    let det = hsd_calc_determinant_3x4(&m);

    if fabsf_bitwise(det) < EPSILON {
        return [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
    }

    let det = 1.0 / det;

    // retail 0x803793EC..0x803794EC: fmsubs / fnmsubs, then fmuls.
    let d00 = fmsubs(m[1][1], m[2][2], m[2][1] * m[1][2]) * det;
    let d01 = fnmsubs(m[0][1], m[2][2], m[2][1] * m[0][2]) * det;
    let d02 = fmsubs(m[0][1], m[1][2], m[1][1] * m[0][2]) * det;
    let d10 = fnmsubs(m[1][0], m[2][2], m[2][0] * m[1][2]) * det;
    let d11 = fmsubs(m[0][0], m[2][2], m[2][0] * m[0][2]) * det;
    let d12 = fnmsubs(m[0][0], m[1][2], m[1][0] * m[0][2]) * det;
    let d20 = fmsubs(m[1][0], m[2][1], m[2][0] * m[1][1]) * det;
    let d21 = fnmsubs(m[0][0], m[2][1], m[2][0] * m[0][1]) * det;
    let d22 = fmsubs(m[0][0], m[1][1], m[1][0] * m[0][1]) * det;

    // retail 0x80379518..0x80379574: fmsubs + fnmsubs for each row.
    let d03 = fnmsubs(d02, m[2][3], fmsubs(-d00, m[0][3], d01 * m[1][3]));
    let d13 = fnmsubs(d12, m[2][3], fmsubs(-d10, m[0][3], d11 * m[1][3]));
    let d23 = fnmsubs(d22, m[2][3], fmsubs(-d20, m[0][3], d21 * m[1][3]));

    [
        [d00, d01, d02, d03],
        [d10, d11, d12, d13],
        [d20, d21, d22, d23],
    ]
}
