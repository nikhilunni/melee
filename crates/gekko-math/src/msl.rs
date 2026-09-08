//! Ports of the Metrowerks Standard Library math routines Melee links.
//!
//! Sources in the decomp submodule:
//! - `src/MSL/math_ppc.h` for `sqrtf` / `sqrtf_accurate` (inline, on `frsqrte`)
//! - `src/MSL/trigf.c` for `sinf`, `cosf`, `tanf` (retail `0x803263D4`,
//!   `0x80326240`, `0x803261BC`)
//! - `src/MSL/math_data.c` for the polynomial and log tables ([`math_data`])
//! - `src/MSL/math_1.c` for `fabsf__Ff` and `frexp` (retail `0x803261B4`,
//!   `0x80326118`)
//! - `src/MSL/math.c` for `logf` (retail `0x803265A8`)
//! - `src/MSL/math.h` for the inline `fmodf`, `__fpclassifyf`, `__fpclassifyd`
//!
//! Not in MSL, despite living in `math.h`'s namespace: `atan2f`, `atanf`,
//! `asinf`, `acosf` are Melee's own (`src/melee/lb/lbtrigf.c`), as are `powf`
//! and `expf` (`src/melee/lb/lb_00CE.c`). They belong in `melee-lb`.
//!
//! Every function here must be a literal transcription. Do not "simplify"
//! the arithmetic: the order of operations is the observable behaviour.
//!
//! # Fusion audit
//!
//! No retail assembly was available when these were written, so the C was
//! transcribed with separate multiplies and adds throughout and checked
//! bit-for-bit against the same C compiled natively with
//! `-ffp-contract=off` (see `tests/ref_oracle.rs`). Every `a * b + c` shape
//! MWCC could have contracted into `fmadds`/`fmsubs`/`fnmsubs` is marked
//! `// FUSION AUDIT PENDING`. When the disc is available, disassemble the
//! retail function, and for each marked line either replace the expression
//! with the matching `crate::fma::*` call or delete the marker.
//!
//! Likewise `// INT CONVERSION AUDIT PENDING` marks the one place an
//! `int -> float` conversion feeds a subtraction and MWCC may or may not have
//! rounded the integer to single precision (`frsp`) before the `fsubs`. It
//! only matters for `|n| >= 2^23`, i.e. `|x| >= ~1.3e7`.

pub mod math_data;

use crate::estimate::frsqrte;
use math_data::{LN_F, ONE_OVER_F, SINCOS_ON_QUADRANT, SINCOS_POLY};

// ---------------------------------------------------------------------------
// Conversions with Gekko semantics
// ---------------------------------------------------------------------------

/// `(int) f` as MWCC compiles it: `fctiwz` then a store/load pair.
///
/// Truncates toward zero and saturates like Rust's `as i32`, but PowerPC
/// converts NaN to the most negative integer where Rust gives 0.
#[inline]
pub fn fctiwz(x: f32) -> i32 {
    if x.is_nan() {
        i32::MIN
    } else {
        x as i32
    }
}

/// `(long long) d` as MWCC compiles it: a call to `__cvt_dbl_usll`
/// (`src/Runtime/runtime.c`, retail `0x80322E54`).
///
/// Transcribed from the asm: exponent below 1023 (`|d| < 1`) gives 0;
/// `|d| >= 2^63`, infinities and NaNs saturate by sign bit; everything else
/// truncates toward zero. This is Rust's `as i64` except that NaN saturates
/// instead of becoming 0.
#[inline]
pub fn cvt_dbl_sll(d: f64) -> i64 {
    if d.is_nan() {
        if d.to_bits() & 0x8000_0000_0000_0000 != 0 {
            i64::MIN
        } else {
            i64::MAX
        }
    } else {
        d as i64
    }
}

/// `(float) ll` as MWCC compiles it: a call to `__cvt_sll_flt`
/// (`src/Runtime/runtime.c`, retail `0x80322DA0`).
///
/// The asm rounds to nearest-even at 53 bits and then `frsp`s, so the
/// result is double-rounded for `|ll| > 2^53`. `as f64` then `as f32`
/// performs exactly those two roundings.
#[inline]
pub fn cvt_sll_flt(ll: i64) -> f32 {
    (ll as f64) as f32
}

// ---------------------------------------------------------------------------
// Intrinsics: fabs / fabsf
// ---------------------------------------------------------------------------

/// `fabsf`, which `math.h` maps to the `__fabsf` intrinsic (`fabs` instruction).
///
/// Clears the sign bit and nothing else, so NaN payloads pass through.
/// `fabsf__Ff` in `math_1.c` (retail `0x803261B4`) is this behind a call.
#[inline]
pub fn fabsf(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFF_FFFF)
}

/// `fabs`, which `math.h` maps to the `__fabs` intrinsic.
#[inline]
pub fn fabs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF)
}

// ---------------------------------------------------------------------------
// sqrtf (math_ppc.h)
// ---------------------------------------------------------------------------

/// `sqrtf` from `src/MSL/math_ppc.h`.
///
/// Transcribed from:
/// ```c
/// double guess = __frsqrte((double) x);
/// guess = 0.5 * guess * (3.0 - guess * guess * x);   // x3
/// y = (float) (x * guess);
/// ```
///
/// Fusion: the retail compiler emits `fnmsub`/`fmul` sequences for the
/// Newton steps. TODO(oracle): confirm the exact instruction sequence from
/// the disassembly of an inlined call site and update this transcription
/// before relying on bit-exactness. Tracked by the ignored test below.
pub fn sqrtf(x: f32) -> f32 {
    if x > 0.0 {
        let xd = x as f64;
        let mut guess = frsqrte(xd);
        // FUSION AUDIT PENDING: `3.0 - guess * guess * x` is an fnmsub shape.
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        (xd * guess) as f32
    } else {
        x
    }
}

/// `sqrtf_accurate` from `src/MSL/math_ppc.h`: one extra Newton step.
pub fn sqrtf_accurate(x: f32) -> f32 {
    if x > 0.0 {
        let xd = x as f64;
        let mut guess = frsqrte(xd);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        (xd * guess) as f32
    } else {
        x
    }
}

// ---------------------------------------------------------------------------
// sinf / cosf / tanf (trigf.c)
// ---------------------------------------------------------------------------

/// `__epsilon` from `trigf.c`: below this the reduced argument uses the
/// linear branch. Spelled as MSL spelled it.
#[allow(clippy::excessive_precision)]
const TRIG_EPSILON: f32 = 3.45266983e-4;

/// `(2.0f / (f32) M_PI)` from `trigf.c`, folded by the compiler. Folding in
/// `f32` or via `f64` both give `0x3F22F983`.
const TWO_OVER_PI: f32 = 2.0 / (core::f64::consts::PI as f32);

/// `__four_over_pi_m1` from `trigf.c`: `4/pi - 1` split into four `f32`
/// pieces so the reduction `x * (4/pi - 1)` is computed to extra precision.
/// The C fills it from `tmp_float` in `__sinit_trigf_c`; the values are the
/// same, `f`-suffixed literals parsed straight to single precision.
#[allow(clippy::excessive_precision)]
const FOUR_OVER_PI_M1: [f32; 4] = [0.25, 0.0232393741608, 1.70555722434e-7, 1.86736494323e-11];

/// Shared argument reduction of `sinf`/`cosf` (identical text in both).
///
/// Returns `(n & 3, y)`: the quadrant and the reduced argument in units of
/// `pi/4`, `y = 4x/pi - 2n`.
#[inline(always)]
fn sincos_reduce(x: f32) -> (usize, f32) {
    let z = TWO_OVER_PI * x;
    // `(__HI(x) & 0x80000000)`: the sign bit of the f32 pattern.
    let n = if x.to_bits() & 0x8000_0000 != 0 {
        fctiwz(z - 0.5)
    } else {
        fctiwz(z + 0.5)
    };

    // `y = x - n * 2 + m1[0] * x + m1[1] * x + m1[2] * x + m1[3] * x;`
    // Left-associative: ((((x - n*2) + m1[0]*x) + m1[1]*x) + m1[2]*x) + m1[3]*x.
    // `n * 2` is a 32-bit int multiply (wraps for saturated n), then int -> float.
    // INT CONVERSION AUDIT PENDING: `(float)(n * 2)` before the `fsubs`?
    let n2 = n.wrapping_mul(2) as f32;
    // FUSION AUDIT PENDING (4 sites): each `+ m1[k] * x` is an fmadds shape.
    let y = x - n2
        + FOUR_OVER_PI_M1[0] * x
        + FOUR_OVER_PI_M1[1] * x
        + FOUR_OVER_PI_M1[2] * x
        + FOUR_OVER_PI_M1[3] * x;

    ((n & 3) as usize, y)
}

/// The even-indexed (cosine) polynomial from `trigf.c`:
/// `(((p0 * ysq + p2) * ysq + p4) * ysq + p6) * ysq + p8`.
#[inline(always)]
fn sincos_poly_even(ysq: f32) -> f32 {
    // FUSION AUDIT PENDING (4 sites): each `* ysq + p` is an fmadds shape.
    (((SINCOS_POLY[0] * ysq + SINCOS_POLY[2]) * ysq + SINCOS_POLY[4]) * ysq + SINCOS_POLY[6]) * ysq
        + SINCOS_POLY[8]
}

/// The odd-indexed (sine) polynomial from `trigf.c`, before the final `* y`:
/// `(((p1 * ysq + p3) * ysq + p5) * ysq + p7) * ysq + p9`.
#[inline(always)]
fn sincos_poly_odd(ysq: f32) -> f32 {
    // FUSION AUDIT PENDING (4 sites): each `* ysq + p` is an fmadds shape.
    (((SINCOS_POLY[1] * ysq + SINCOS_POLY[3]) * ysq + SINCOS_POLY[5]) * ysq + SINCOS_POLY[7]) * ysq
        + SINCOS_POLY[9]
}

/// `sinf` from `src/MSL/trigf.c` (retail `0x803263D4`).
pub fn sinf(x: f32) -> f32 {
    let (n, y) = sincos_reduce(x);

    if fabsf(y) < TRIG_EPSILON {
        let n = n << 1;
        // FUSION AUDIT PENDING: `q[n] + (q[n+1] * y) * p9` is an fmadds shape.
        return SINCOS_ON_QUADRANT[n] + (SINCOS_ON_QUADRANT[n + 1] * y * SINCOS_POLY[9]);
    }

    let ysq = y * y;
    if n & 1 != 0 {
        let n = n << 1;
        let z = sincos_poly_even(ysq);
        z * SINCOS_ON_QUADRANT[n]
    } else {
        let n = n << 1;
        let z = sincos_poly_odd(ysq) * y;
        z * SINCOS_ON_QUADRANT[n + 1]
    }
}

/// `cosf` from `src/MSL/trigf.c` (retail `0x80326240`).
///
/// Looks like a bug but is not: the small-argument branch returns
/// `q[n+1] - y * q[n]` with no `pi/4` scaling, unlike `sinf`'s
/// `q[n] + q[n+1] * y * p9`. `y` is in units of `pi/4`, so within
/// `__epsilon` of an odd multiple of `pi/2` the result is too large by a
/// factor of `4/pi`: an absolute error of up to `~7.4e-5`. That is what the
/// retail game computes.
pub fn cosf(x: f32) -> f32 {
    let (n, y) = sincos_reduce(x);

    if fabsf(y) < TRIG_EPSILON {
        let n = n << 1;
        // FUSION AUDIT PENDING: `q[n+1] - y * q[n]` is an fnmsubs/fmsubs shape.
        return SINCOS_ON_QUADRANT[n + 1] - y * SINCOS_ON_QUADRANT[n];
    }

    let ysq = y * y;
    if n & 1 != 0 {
        let n = n << 1;
        // `z = -(poly) * y;` : unary minus binds to the polynomial, then `* y`.
        let z = -sincos_poly_odd(ysq) * y;
        z * SINCOS_ON_QUADRANT[n]
    } else {
        let n = n << 1;
        let z = sincos_poly_even(ysq);
        z * SINCOS_ON_QUADRANT[n + 1]
    }
}

/// `tanf` from `src/MSL/trigf.c` (retail `0x803261BC`):
/// `sin__Ff(x) / cos__Ff(x)`, where the `__Ff` wrappers are out-of-line
/// calls to `sinf`/`cosf` with no arithmetic of their own.
pub fn tanf(x: f32) -> f32 {
    sinf(x) / cosf(x)
}

// ---------------------------------------------------------------------------
// frexp (math_1.c)
// ---------------------------------------------------------------------------

/// `lbl_804DE190` from `math_1.c`: `2^54`, used to normalise subnormals.
const FREXP_TWO54: f64 = 1.8014398509481984e+16;

/// `frexp` from `src/MSL/math_1.c` (retail `0x80326118`).
///
/// Returns `(mantissa, exponent)` in place of the C out-parameter. `MSL_HI`
/// is the big-endian high word (sign, exponent, top mantissa bits) and
/// `MSL_LO` the low word.
pub fn frexp(x: f64) -> (f64, i32) {
    let mut x = x;
    let mut hx = (x.to_bits() >> 32) as i32;
    let mut ix = 0x7fff_ffff & hx;
    let lx = x.to_bits() as u32 as i32;
    let mut exponent = 0;
    if ix >= 0x7ff0_0000 || (ix | lx) == 0 {
        return (x, exponent); // 0, inf, nan
    }
    if ix < 0x0010_0000 {
        // subnormal
        x *= FREXP_TWO54;
        hx = (x.to_bits() >> 32) as i32;
        ix = hx & 0x7fff_ffff;
        exponent = -54;
    }
    exponent += (ix >> 20) - 1022;
    // `hx = (hx & 0x800fffff) | 0x3fe00000;` : unsigned arithmetic in C.
    let hx = ((hx as u32) & 0x800f_ffff) | 0x3fe0_0000;
    // `MSL_HI(x) = hx;`
    x = f64::from_bits(((hx as u64) << 32) | (x.to_bits() & 0xffff_ffff));
    (x, exponent)
}

// ---------------------------------------------------------------------------
// logf (math.c)
// ---------------------------------------------------------------------------

const EXP_MASK: u32 = 0x7F80_0000;
/// Bit pattern of `1.0f` (the C comment saying `0.0f` is wrong).
const EXP_ZERO: u32 = 0x3F80_0000;
const MANT_MASK: u32 = 0x007F_FFFF;
const SIGN_BIT: u32 = 0x8000_0000;
/// `LN2` from `math.c`: MSL's own rounding of ln 2, kept as spelled.
#[allow(clippy::approx_constant)]
const LN2: f32 = 0.6931472;
/// `MSL_Math_804DE1B0`: the `H^2` coefficient, `-0.500003`.
const LOGF_COEF0: f32 = f32::from_bits(0xBF00_0030);
/// `MSL_Math_804DE1B4`: the `H^3` coefficient, `0.33333`.
const LOGF_COEF1: f32 = f32::from_bits(0x3EAA_AA36);
/// `__float_nan`: `0x7FFFFFFF`, the NaN `logf(-inf)` returns.
const FLOAT_NAN: f32 = f32::from_bits(0x7FFF_FFFF);
/// `__float_huge`: `+inf`.
const FLOAT_HUGE: f32 = f32::from_bits(0x7F80_0000);

/// `logf` from `src/MSL/math.c` (retail `0x803265A8`).
///
/// Table-driven: `ln(x) = E ln2 + ln(1.m) + H + H^2 (c0 + c1 H)` with
/// `m` the top 7 mantissa bits (rounded up on bit 8) and `H = (1.M - 1.m) / 1.m`.
///
/// Note the `default` arm handles every "normal" exponent field including
/// negative inputs, whose sign bit leaks into `E`; and every zero or
/// subnormal returns `-inf`. Both are MSL's behaviour and are preserved.
pub fn logf(x: f32) -> f32 {
    let raw_x = x.to_bits();
    match raw_x & EXP_MASK {
        EXP_MASK => {
            if raw_x & MANT_MASK != 0 {
                x
            } else if raw_x & SIGN_BIT != 0 {
                FLOAT_NAN
            } else {
                FLOAT_HUGE
            }
        }
        0 => -FLOAT_HUGE,
        _ => {
            // x is normal
            let mant_bits = raw_x & MANT_MASK;
            // `F32_UNBIASED_EXPONENT`: `((raw_x & 0xFF800000) >> 23) - 127`
            // in unsigned arithmetic, then stored to `s32`.
            let e = ((raw_x & 0xFF80_0000) >> 23).wrapping_sub(127) as i32;
            // `F32_HIGH_MANTISSA_BITS`: the 7 most significant mantissa bits.
            let mut m = ((raw_x >> 16) & 0x7F) as usize;

            if raw_x & 0xFFFF != 0 {
                let mut raw_fm = (raw_x & 0x7F_0000) | EXP_ZERO;
                let raw_f_upper_m = mant_bits | EXP_ZERO;

                if raw_x & 0x8000 != 0 {
                    raw_fm += 0x10000;
                    m += 1;
                }

                let mut h = f32::from_bits(raw_f_upper_m) - f32::from_bits(raw_fm);
                h *= ONE_OVER_F[m];
                // FUSION AUDIT PENDING: `H * coef1 + coef0` is an fmadds shape.
                let h2_poly = h * h * (h * LOGF_COEF1 + LOGF_COEF0);
                // FUSION AUDIT PENDING: `LN2 * (float) E + __ln_F[m]` is an fmadds shape.
                return (LN2 * e as f32 + LN_F[m]) + (h + h2_poly);
            }
            // FUSION AUDIT PENDING: `LN2 * (float) E + __ln_F[m]` is an fmadds shape.
            LN2 * e as f32 + LN_F[m]
        }
    }
}

// ---------------------------------------------------------------------------
// fmodf, fpclassify (math.h inlines)
// ---------------------------------------------------------------------------

/// The inline `fmodf` from `src/MSL/math.h`.
///
/// ```c
/// long long quotient;
/// if (fabsf(b) > fabsf(a)) return a;
/// quotient = a / b;
/// return a - b * quotient;
/// ```
///
/// `a / b` is a single-precision divide whose result is converted to
/// `long long` through `__cvt_dbl_usll`, and `quotient` comes back to
/// single through `__cvt_sll_flt`. Inlined at every call site, so each
/// caller's retail asm must be audited separately for fusion.
pub fn fmodf(a: f32, b: f32) -> f32 {
    if fabsf(b) > fabsf(a) {
        return a;
    }
    let quotient = cvt_dbl_sll((a / b) as f64);
    // FUSION AUDIT PENDING: `a - b * quotient` is an fnmsubs shape.
    a - b * cvt_sll_flt(quotient)
}

/// `enum FloatType` from `src/MSL/math.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FloatType {
    Nan = 1,
    Infinite = 2,
    Zero = 3,
    Normal = 4,
    Subnormal = 5,
}

/// `__fpclassifyf` from `src/MSL/math.h`.
pub fn fpclassifyf(x: f32) -> FloatType {
    let bits = x.to_bits();
    match bits & EXP_MASK {
        EXP_MASK => {
            if bits & MANT_MASK != 0 {
                FloatType::Nan
            } else {
                FloatType::Infinite
            }
        }
        0 => {
            if bits & MANT_MASK != 0 {
                FloatType::Subnormal
            } else {
                FloatType::Zero
            }
        }
        _ => FloatType::Normal,
    }
}

/// `__fpclassifyd` from `src/MSL/math.h`.
pub fn fpclassifyd(x: f64) -> FloatType {
    let bits = x.to_bits();
    let hi = (bits >> 32) as u32;
    let lo = bits as u32;
    let frac_nonzero = (hi & 0x000f_ffff) != 0 || lo != 0;
    match hi & 0x7ff0_0000 {
        0x7ff0_0000 => {
            if frac_nonzero {
                FloatType::Nan
            } else {
                FloatType::Infinite
            }
        }
        0 => {
            if frac_nonzero {
                FloatType::Subnormal
            } else {
                FloatType::Zero
            }
        }
        _ => FloatType::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrtf_basic_shape() {
        assert_eq!(sqrtf(0.0), 0.0);
        assert_eq!(sqrtf(-4.0), -4.0, "MSL returns negative inputs unchanged");
        assert!((sqrtf(4.0) - 2.0).abs() < 1e-6);
        assert!((sqrtf(2.0) - core::f32::consts::SQRT_2).abs() < 1e-6);
    }

    #[test]
    #[ignore = "needs hardware-exact frsqrte and oracle golden values"]
    fn sqrtf_matches_oracle_goldens() {
        // Populate from harness/goldens/msl_sqrtf.jsonl once recorded.
    }

    #[test]
    fn folded_constants_match_c() {
        assert_eq!(TWO_OVER_PI.to_bits(), 0x3F22_F983);
        assert_eq!(SINCOS_POLY[8], 1.0);
        assert_eq!(
            SINCOS_POLY[9].to_bits(),
            core::f32::consts::FRAC_PI_4.to_bits()
        );
        let sum: f64 = FOUR_OVER_PI_M1.iter().map(|&v| v as f64).sum();
        assert!((sum - (4.0 / core::f64::consts::PI - 1.0)).abs() < 1e-9);
        assert_eq!(FREXP_TWO54, 2f64.powi(54));
    }

    /// `std` is a tolerance reference only. Bit-exactness is checked against
    /// the native C build in `tests/ref_oracle.rs`.
    #[test]
    fn sinf_cosf_tanf_within_tolerance_of_std() {
        let steps = 200_000;
        let two_pi = 2.0 * core::f64::consts::PI;
        let mut worst_sin = 0.0f64;
        let mut worst_cos = 0.0f64;
        for i in 0..=steps {
            let x = (-two_pi + 2.0 * two_pi * (i as f64) / (steps as f64)) as f32;
            let ds = (sinf(x) as f64 - (x as f64).sin()).abs();
            let dc = (cosf(x) as f64 - (x as f64).cos()).abs();
            worst_sin = worst_sin.max(ds);
            assert!(
                ds < 1e-6,
                "sinf({x}) = {} vs std {}",
                sinf(x),
                (x as f64).sin()
            );
            // cosf's linear branch omits the pi/4 factor (see its doc comment),
            // so within ~2.7e-4 rad of an odd multiple of pi/2 it is off by up
            // to ~7.4e-5. Everywhere else it is as accurate as sinf.
            if (x as f64).cos().abs() < 3e-4 {
                assert!(
                    dc < 1e-4,
                    "cosf({x}) = {} vs std {}",
                    cosf(x),
                    (x as f64).cos()
                );
            } else {
                worst_cos = worst_cos.max(dc);
                assert!(
                    dc < 1e-6,
                    "cosf({x}) = {} vs std {}",
                    cosf(x),
                    (x as f64).cos()
                );
            }
            // tan blows up near odd multiples of pi/2; check it where |cos| is sane.
            if (x as f64).cos().abs() > 0.1 {
                let dt = (tanf(x) as f64 - (x as f64).tan()).abs();
                assert!(
                    dt < 1e-4,
                    "tanf({x}) = {} vs std {}",
                    tanf(x),
                    (x as f64).tan()
                );
            }
        }
        eprintln!("worst |sinf - sin| = {worst_sin:e}, worst |cosf - cos| away from zeros = {worst_cos:e}");
    }

    #[test]
    fn sinf_cosf_special_values() {
        assert_eq!(sinf(0.0).to_bits(), 0.0f32.to_bits());
        // -0.0: n = (int)(-0.0 - 0.5) = 0, y = -0.0, linear branch:
        // 0.0 + (1 * -0.0 * p9) = 0.0 + -0.0 = +0.0. MSL loses the sign.
        assert_eq!(sinf(-0.0).to_bits(), 0.0f32.to_bits());
        assert_eq!(cosf(0.0), 1.0);
        assert_eq!(cosf(-0.0), 1.0);
        assert!(sinf(f32::NAN).is_nan());
        assert!(cosf(f32::NAN).is_nan());
        assert!(tanf(f32::NAN).is_nan());
        assert_eq!(sinf(core::f32::consts::FRAC_PI_2), 1.0);
        assert_eq!(cosf(core::f32::consts::PI), -1.0);
    }

    #[test]
    fn logf_shape() {
        assert_eq!(logf(1.0), 0.0);
        assert!((logf(core::f32::consts::E) - 1.0).abs() < 1e-6);
        assert!((logf(10.0) - core::f32::consts::LN_10).abs() < 1e-6);
        assert!((logf(0.5) + core::f32::consts::LN_2).abs() < 1e-6);
        assert_eq!(logf(0.0), f32::NEG_INFINITY);
        assert_eq!(logf(-0.0), f32::NEG_INFINITY);
        assert_eq!(
            logf(1e-40),
            f32::NEG_INFINITY,
            "MSL treats subnormals as zero"
        );
        assert_eq!(logf(f32::INFINITY), f32::INFINITY);
        assert_eq!(logf(f32::NEG_INFINITY).to_bits(), 0x7FFF_FFFF);
        assert_eq!(
            logf(f32::NAN).to_bits(),
            f32::NAN.to_bits(),
            "NaN input is returned as-is"
        );
        for i in 1..=4096 {
            let x = i as f32 * (1.0 / 128.0);
            let d = (logf(x) as f64 - (x as f64).ln()).abs();
            assert!(
                d < 2e-6,
                "logf({x}) = {} vs std {}",
                logf(x),
                (x as f64).ln()
            );
        }
    }

    #[test]
    fn frexp_shape() {
        assert_eq!(frexp(8.0), (0.5, 4));
        assert_eq!(frexp(-8.0), (-0.5, 4));
        assert_eq!(frexp(1.0), (0.5, 1));
        assert_eq!(frexp(0.75), (0.75, 0));
        assert_eq!(frexp(0.0), (0.0, 0));
        assert_eq!(frexp(-0.0).0.to_bits(), (-0.0f64).to_bits());
        assert_eq!(frexp(f64::INFINITY), (f64::INFINITY, 0));
        assert!(frexp(f64::NAN).0.is_nan());
        // Subnormal: 2^-1074 = 0.5 * 2^-1073
        let (m, e) = frexp(f64::from_bits(1));
        assert_eq!((m, e), (0.5, -1073));
        for &x in &[3.0, 1e-300, 1e300, 123456.789, -2.5e-310] {
            let (m, e) = frexp(x);
            assert!(m.abs() >= 0.5 && m.abs() < 1.0, "{x}: {m}");
            // Scale back in two steps so 2^e does not under/overflow on its own.
            let half = e / 2;
            assert_eq!(m * 2f64.powi(half) * 2f64.powi(e - half), x);
        }
    }

    #[test]
    fn fmodf_shape() {
        assert_eq!(fmodf(5.5, 2.0), 1.5);
        assert_eq!(fmodf(-5.5, 2.0), -1.5);
        assert_eq!(fmodf(1.0, 3.0), 1.0, "|b| > |a| returns a");
        assert_eq!(fmodf(6.0, 3.0), 0.0);
        assert_eq!(fmodf(-0.0, 1.0).to_bits(), (-0.0f32).to_bits());
        // a / 0 = inf, (long long) inf saturates to i64::MAX, 0 * that = 0,
        // so MSL's fmodf(a, 0) is a. Not NaN.
        assert_eq!(fmodf(1.0, 0.0), 1.0);
        assert_eq!(fmodf(-7.5, 0.0), -7.5);
    }

    #[test]
    fn fpclassify_shape() {
        assert_eq!(fpclassifyf(f32::NAN), FloatType::Nan);
        assert_eq!(fpclassifyf(f32::INFINITY), FloatType::Infinite);
        assert_eq!(fpclassifyf(-0.0), FloatType::Zero);
        assert_eq!(fpclassifyf(1e-40), FloatType::Subnormal);
        assert_eq!(fpclassifyf(1.0), FloatType::Normal);
        assert_eq!(fpclassifyd(f64::NAN), FloatType::Nan);
        assert_eq!(fpclassifyd(f64::NEG_INFINITY), FloatType::Infinite);
        assert_eq!(fpclassifyd(0.0), FloatType::Zero);
        assert_eq!(fpclassifyd(f64::from_bits(1)), FloatType::Subnormal);
        assert_eq!(fpclassifyd(f64::from_bits(1 << 32)), FloatType::Subnormal);
        assert_eq!(fpclassifyd(-1.0), FloatType::Normal);
    }

    #[test]
    fn gekko_conversions() {
        assert_eq!(fctiwz(f32::NAN), i32::MIN);
        assert_eq!(fctiwz(1e30), i32::MAX);
        assert_eq!(fctiwz(-1e30), i32::MIN);
        assert_eq!(fctiwz(-2.7), -2);
        assert_eq!(fctiwz(2.7), 2);
        assert_eq!(cvt_dbl_sll(f64::NAN), i64::MAX);
        assert_eq!(cvt_dbl_sll(-f64::NAN), i64::MIN);
        assert_eq!(cvt_dbl_sll(0.999), 0);
        assert_eq!(cvt_dbl_sll(-0.999), 0);
        assert_eq!(cvt_dbl_sll(-3.9), -3);
        assert_eq!(cvt_dbl_sll(9.3e18), i64::MAX);
        assert_eq!(cvt_dbl_sll(-9.3e18), i64::MIN);
        assert_eq!(cvt_sll_flt(1 << 40), 1099511627776.0);
        assert_eq!(fabsf(-0.0).to_bits(), 0);
        assert_eq!(
            fabsf(f32::from_bits(0xFFC0_0001)).to_bits(),
            0x7FC0_0001,
            "payload preserved"
        );
        assert_eq!(fabs(-1.5), 1.5);
    }
}
