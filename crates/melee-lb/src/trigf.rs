//! Melee's own trigonometry and exponential routines.
//!
//! Sources in the decomp submodule:
//! - `src/melee/lb/lbtrigf.c`: `atan2f`, `acosf`, `asinf`, `lb_sqrtf`, `atanf`
//!   and the `atanf_lookup` table. These are Melee's code, not MSL's, even
//!   though they carry libm names.
//! - `src/melee/lb/lb_00CE.c`: `expf` and `powf`, Taylor/Gregory series
//!   summed in single precision until the sum stops changing. Also Melee's
//!   own code (the `sdata2_order` hint and the `arg8`/`var_f4` variable
//!   names show it was decompiled from the retail binary, not taken from a
//!   library); self-contained apart from `powf` calling `expf`.
//!
//! Retail addresses are from `config/GALE01/symbols.txt`.
//!
//! Every function is a literal transcription. Operation order and the
//! `f32`/`f64` mix are the observable behaviour; do not simplify.
//!
//! # Estimate instructions
//!
//! `acosf` and `lb_sqrtf` call `__frsqrte` directly (there is no `sqrtf`
//! call anywhere in these two files), then refine with three Newton steps in
//! **single** precision (unlike MSL's `sqrtf`, which refines in double).
//! They go through [`gekko_math::estimate::frsqrte`], which is bit-exact with
//! the captured hardware behaviour (see that module's docs).
//!
//! # Fusion audit
//!
//! No retail assembly was available when this was written. Every `a * b + c`
//! shape MWCC could have contracted is written unfused and marked
//! `// FUSION AUDIT PENDING`; the two `__fnmsubs` intrinsics in `atanf` are
//! explicit in the C and are transcribed with [`gekko_math::fma::fnmsubs`].
//! The transcription is checked bit for bit against the C compiled natively
//! with `-ffp-contract=off` in `tests/ref_oracle.rs`.
//!
//! # Non-terminating inputs
//!
//! `expf` and `powf` loop `while (sum != previous)`. A NaN anywhere in the
//! sum never compares equal, so `expf(NaN)`, `powf(NaN, _)`, `powf(_, NaN)`,
//! `powf(+/-inf, _)` and `powf(-1, 0)` never return. `expf(x)` for
//! `|x|` in roughly `[12.6157, 14.7106]` also never returns: `x^n` and `n!`
//! overflow to infinity in the same step and the term becomes `inf/inf`.
//! The retail game hangs the same way; callers only pass small arguments.

use gekko_math::estimate::frsqrte;
use gekko_math::fma::fnmsubs;

/// `MSL_TrigF_80400770[0]` (`src/MSL/float.c`, retail `0x80400770`): the NaN
/// lbtrigf.c returns, bit pattern `0x7FFFFFFF`.
const NAN: f32 = f32::from_bits(0x7FFF_FFFF);

/// `MSL_TrigF_80400774[0]` (`src/MSL/float.c`, retail `0x80400774`): `+inf`.
const INF: f32 = f32::from_bits(0x7F80_0000);

const SIGN_BIT: u32 = 1 << 31;

/// `(float) M_PI`: `0x40490FDB`.
const M_PI_F: f32 = core::f64::consts::PI as f32;

/// `(float) M_PI_2`: `0x3FC90FDB`, the same bits as `BITWISE_PI_2` below.
const M_PI_2_F: f32 = (core::f64::consts::PI / 2.0) as f32;

/// `BITWISE_PI_2` in the C: the bits of `(float) M_PI_2`.
const BITWISE_PI_2: u32 = 0x3FC9_0FDB;

/// `GET_SIGN_BIT(f)`: the sign bit of the `f32` pattern, in place.
#[inline(always)]
fn sign_bit(f: f32) -> u32 {
    f.to_bits() & SIGN_BIT
}

// ---------------------------------------------------------------------------
// atan2f (lbtrigf.c)
// ---------------------------------------------------------------------------

/// `atan2f` (lbtrigf.c), retail `0x80022C30`.
///
/// Branches on the raw sign bits, so `-0.0` is negative here: with both
/// signs set and `x == -0.0` the result is `-pi/2` whatever `y` is, and with
/// `x == +0.0` the result is `+/-pi/2` by the sign bit of `y`. `x == -0.0f`
/// in the C is an IEEE compare, true for either zero.
pub fn atan2f(y: f32, x: f32) -> f32 {
    if sign_bit(x) == sign_bit(y) {
        if sign_bit(x) != 0 {
            return if x == -0.0 {
                -M_PI_2_F
            } else {
                atanf(y / x) - M_PI_F
            };
        }

        return if x != 0.0 { atanf(y / x) } else { M_PI_2_F };
    }

    if x < 0.0 {
        return M_PI_F + atanf(y / x);
    }

    if x != 0.0 {
        return atanf(y / x);
    }

    // `*(u32*) &y = GET_SIGN_BIT(y) + BITWISE_PI_2;` (a 32-bit integer add).
    f32::from_bits(sign_bit(y).wrapping_add(BITWISE_PI_2))
}

// ---------------------------------------------------------------------------
// acosf (lbtrigf.c)
// ---------------------------------------------------------------------------

/// `acosf` (lbtrigf.c), retail `0x80022D1C`.
///
/// Computes `pi/2 - atan(x / sqrt(1 - x^2))` with the reciprocal square
/// root taken from `frsqrte` plus three single-precision Newton steps. For
/// `|x| == 1` the reciprocal is `+inf` and the result is exactly `0` or
/// `pi`; for `|x| > 1` it is the NaN `0x7FFFFFFF` propagated through
/// `atanf`.
pub fn acosf(x: f32) -> f32 {
    // FUSION AUDIT PENDING: `1.0f - x * x` is an fnmsubs shape.
    let mut result = 1.0 - x * x;
    if result > 0.0 {
        // `float guess = __frsqrte(result);` -> double estimate rounded to single.
        let mut guess = frsqrte(result as f64) as f32;
        // FUSION AUDIT PENDING: `3.0f - guess * guess * result` is an fnmsubs shape.
        guess = 0.5 * guess * (3.0 - guess * guess * result);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * result);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * result);
        result = guess;
    } else if result != 0.0 {
        result = NAN;
    } else {
        result = INF;
    }
    M_PI_2_F - atanf(x * result)
}

// ---------------------------------------------------------------------------
// asinf (lbtrigf.c)
// ---------------------------------------------------------------------------

/// `asinf` (lbtrigf.c), retail `0x80022DBC`.
///
/// `atan(x / sqrt(1 - x^2))`, spelled `atanf(x * lb_sqrtf(-(x * x - 1.0f)))`.
pub fn asinf(x: f32) -> f32 {
    // FUSION AUDIT PENDING: `x * x - 1.0f` is an fmsubs shape (fnmsubs with
    // the outer negation).
    atanf(x * lb_sqrtf(-(x * x - 1.0)))
}

// ---------------------------------------------------------------------------
// lb_sqrtf (lbtrigf.c)
// ---------------------------------------------------------------------------

/// `lb_sqrtf` (lbtrigf.c), retail `0x80022DF8`. `static` in the C; public
/// here so the oracle test can reach it.
///
/// Despite the name this is a **reciprocal** square root: `frsqrte` and
/// three single-precision Newton steps, returning `1/sqrt(x)`. Zero gives
/// `+inf` (for either sign of zero), anything else non-positive (negatives,
/// NaN) gives the NaN `0x7FFFFFFF`.
pub fn lb_sqrtf(x: f32) -> f32 {
    if x > 0.0 {
        let mut guess = frsqrte(x as f64) as f32;
        // FUSION AUDIT PENDING: `3.0f - guess * guess * x` is an fnmsubs shape.
        guess = 0.5 * guess * (3.0 - guess * guess * x);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * x);
        // FUSION AUDIT PENDING
        guess = 0.5 * guess * (3.0 - guess * guess * x);
        return guess;
    }

    if x != 0.0 {
        return NAN;
    }

    INF
}

// ---------------------------------------------------------------------------
// atanf (lbtrigf.c)
// ---------------------------------------------------------------------------

/// `atanf_lookup` (lbtrigf.c), retail `0x803B7300`, 46 `f32`s.
///
/// One flat table indexed at several fixed offsets from a per-interval base
/// `lookup_index` in `-1..=4`:
/// - `[1..=6]`: odd polynomial coefficients for `atan` on `|t| <= sqrt(2)-1`;
/// - `[base+7]`, `[base+13]`, `[base+33]`, `[base+39]`: the Mobius-style
///   reduction `t = 1/(a + x + b)` constants for each interval;
/// - `[base+20]`, `[base+27]`: the interval's `atan` offset, `k*pi/8`, as a
///   high part and a low correction.
///
/// Transcribed by `tools/gen_atanf_lookup.py`; `--check` verifies this
/// block, and the native oracle compares it bit for bit.
#[allow(clippy::excessive_precision, clippy::approx_constant)]
pub const ATANF_LOOKUP: [f32; 46] = [
    1.0,                    // [0] = 0x3F800000
    -0.3333333134651184,    // [1] = 0xBEAAAAAA
    0.1999988704919815,     // [2] = 0x3E4CCC81
    -0.14281649887561798,   // [3] = 0xBE123E7D
    0.11041180044412613,    // [4] = 0x3DE21F95
    -0.08459755778312683,   // [5] = 0xBDAD417C
    0.04714243486523628,    // [6] = 0x3D41186D
    6.828420162200928,      // [7] = 0x40DA826B
    3.239828109741211,      // [8] = 0x404F5958
    2.0,                    // [9] = 0x40000000
    1.4464620351791382,     // [10] = 0x3FB925AB
    1.1715729236602783,     // [11] = 0x3F95F61A
    1.039566159248352,      // [12] = 0x3F851081
    7.1350000325764995e-06, // [13] = 0x36EF692F
    8.200000252145401e-07,  // [14] = 0x355C1DF9
    0.0,                    // [15] = 0x00000000
    6.299999881775875e-07,  // [16] = 0x35291D45
    0.0,                    // [17] = 0x00000000
    0.0,                    // [18] = 0x00000000
    0.0,                    // [19] = 0x00000000
    0.3926900029182434,     // [20] = 0x3EC90EAA
    0.5890486240386963,     // [21] = 0x3F16CBE4
    0.7853981256484985,     // [22] = 0x3F490FDA
    0.9817469716072083,     // [23] = 0x3F7B53C5
    1.1780970096588135,     // [24] = 0x3F96CBE2
    1.3744460344314575,     // [25] = 0x3FAFEDD9
    0.0,                    // [26] = 0x00000000
    9.081698408408556e-06,  // [27] = 0x37185D99
    2.3000000126671694e-08, // [28] = 0x32C59189
    6.30000016599297e-08,   // [29] = 0x33874A9E
    7.040000014058023e-07,  // [30] = 0x353CFA83
    2.499999993688107e-07,  // [31] = 0x348637BD
    7.900000014160469e-07,  // [32] = 0x35541063
    2.414212942123413,      // [33] = 0x401A8277
    1.4966057538986206,     // [34] = 0x3FBF90C7
    1.0,                    // [35] = 0x3F800000
    0.6681786179542542,     // [36] = 0x3F2B0DC1
    0.4142135679721832,     // [37] = 0x3ED413CD
    0.1989123672246933,     // [38] = 0x3E4BAFAF
    5.620000251838064e-07,  // [39] = 0x3516DC59
    0.0,                    // [40] = 0x00000000
    0.0,                    // [41] = 0x00000000
    0.0,                    // [42] = 0x00000000
    0.0,                    // [43] = 0x00000000
    0.0,                    // [44] = 0x00000000
    0.0,                    // [45] = 0x00000000
];

/// `BITWISE_INF`: the exponent mask, used to pick the binade of `|x|`.
const BITWISE_INF: u32 = 0x7F80_0000;
/// Exponent fields of `0.5f`, `1.0f`, `2.0f`.
const BITWISE_0_5: u32 = 0x3F00_0000;
const BITWISE_1_0: u32 = 0x3F80_0000;
const BITWISE_2_0: u32 = 0x4000_0000;

/// Interval thresholds compared as signed 32-bit patterns (the sign bit has
/// already been cleared, so this is an ordinary `f32` compare).
const BITWISE_THRESHOLD_0: i32 = 0x3F08_D5B9; // 0.534511148929596f
const BITWISE_THRESHOLD_1: i32 = 0x3F52_1801; // 0.8206787705421448f
const BITWISE_THRESHOLD_2: i32 = 0x3F9B_F7EC; // 1.218503475189209f
const BITWISE_THRESHOLD_3: i32 = 0x3FEF_789E; // 1.870868444442749f

/// `atanf` (lbtrigf.c), retail `0x80022E68`.
///
/// Works on `|x|` and restores the sign at the end. Three ranges:
/// - `|x| >= 1 + sqrt(2)`: evaluates `atan(1/|x|)` and subtracts `pi/2`;
/// - `sqrt(2) - 1 < |x| < 1 + sqrt(2)`: picks one of five sub-intervals by
///   the bit pattern, reduces with a fused rational step, and adds the
///   interval's `k*pi/8` offset from the table;
/// - otherwise evaluates the odd polynomial on `|x|` directly.
///
/// The polynomial is evaluated with `lookup_index == -1` for the outer
/// ranges, so the offset reads hit `atanf_lookup[26]` and `[19]`, both zero.
pub fn atanf(x: f32) -> f32 {
    #[allow(clippy::excessive_precision)]
    const SILVER_RATIO: f32 = 2.4142136573791504;
    #[allow(clippy::excessive_precision)]
    const SILVER_RATIO_CONJUGATE: f32 = 0.4142135679721832;

    let mut result: f32;
    let mut lookup_index: i32 = -1;
    let mut x_ge_ratio = false;
    let sign_bit_x = x.to_bits() & SIGN_BIT;

    // `BITWISE(x) &= ~SIGN_BIT;`
    let x = f32::from_bits(x.to_bits() & !SIGN_BIT);

    if x >= SILVER_RATIO {
        x_ge_ratio = true;
        result = 1.0 / x;
    } else if SILVER_RATIO_CONJUGATE < x {
        lookup_index = 0;
        // `SIGNED_BITWISE(x)`: the pattern as a signed int. The C spells the
        // tests `!(bits < THRESHOLD)`; for integers that is `bits >= THRESHOLD`.
        let bits = x.to_bits() as i32;
        match x.to_bits() & BITWISE_INF {
            BITWISE_0_5 => {
                if bits >= BITWISE_THRESHOLD_0 {
                    lookup_index = 1;
                }

                if bits >= BITWISE_THRESHOLD_1 {
                    lookup_index += 1;
                }
            }
            BITWISE_1_0 => {
                lookup_index = 2;
                if bits >= BITWISE_THRESHOLD_2 {
                    lookup_index = 3;
                }

                if bits >= BITWISE_THRESHOLD_3 {
                    lookup_index += 1;
                }
            }
            BITWISE_2_0 => {
                lookup_index = 4;
            }
            _ => {}
        }
        {
            // `lookup_ptr = &atanf_lookup[lookup_index];`
            let offset_39 = lut(lookup_index, 39);
            let offset_33 = lut(lookup_index, 33);

            result = 1.0 / (offset_33 + (x + offset_39));
            // Explicit `__fnmsubs` intrinsics in the C: `-(a * b - c)`.
            result = fnmsubs(result, lut(lookup_index, 7), offset_33)
                + fnmsubs(result, lut(lookup_index, 13), offset_39);
        }
    } else {
        result = x;
    }

    {
        let result_squared = result * result;

        // `result * result_squared * (...) + result`, Horner on
        // `result_squared` inside. Left-associative as the C parses it:
        // `(result * result_squared) * (...)`.
        // FUSION AUDIT PENDING (6 sites): each `result_squared * (...) + c`
        // is an fmadds shape, as is the outer `... * (...) + result`.
        result = result
            * result_squared
            * (result_squared
                * (result_squared
                    * (result_squared
                        * (result_squared
                            * (result_squared * ATANF_LOOKUP[6] + ATANF_LOOKUP[5])
                            + ATANF_LOOKUP[4])
                        + ATANF_LOOKUP[3])
                    + ATANF_LOOKUP[2])
                + ATANF_LOOKUP[1])
            + result;

        result += lut(lookup_index, 27);
        result += lut(lookup_index, 20);
    }

    if x_ge_ratio {
        result -= M_PI_2_F;
        return if sign_bit_x != 0 { result } else { -result };
    }

    // `BITWISE(result) |= sign_bit_x;`
    f32::from_bits(result.to_bits() | sign_bit_x)
}

/// `lookup_ptr[k]` where `lookup_ptr = &atanf_lookup[lookup_index]` and
/// `lookup_index` may be `-1`.
#[inline(always)]
fn lut(lookup_index: i32, k: i32) -> f32 {
    ATANF_LOOKUP[(lookup_index + k) as usize]
}

// ---------------------------------------------------------------------------
// expf / powf (lb_00CE.c)
// ---------------------------------------------------------------------------

/// `expf` (lb_00CE.c), retail `0x8000CE50`.
///
/// Taylor series of `e^|x|` summed in single precision until adding the
/// next term leaves the sum unchanged, then reciprocated for negative `x`.
/// The factorial is accumulated as an `f32` and overflows to `+inf` at
/// `35!`, which is where the non-terminating band described in the module
/// docs comes from. Variable names follow the decomp.
pub fn expf(arg8: f32) -> f32 {
    let mut var_f1: f32 = arg8;
    let mut var_r5: i32 = 2;
    let mut var_f6: f32 = 1.0;
    let var_r4: i32;
    if var_f1 < 0.0 {
        var_f1 = -var_f1;
        var_r4 = 1;
    } else {
        var_r4 = 0;
    }
    let mut var_f4: f32 = var_f1;
    let mut var_f3: f32 = 1.0 + var_f1;
    loop {
        let temp_r3: i32 = var_r5;
        var_f4 *= var_f1;
        let temp_f5: f32 = var_f3;
        // Signed overflow is UB in the C; it is only reachable in the
        // non-terminating cases, so wrap rather than trap there.
        var_r5 = var_r5.wrapping_add(1);
        // `(f32) temp_r3`: exact, the counter never reaches 2^24 in a
        // terminating call.
        var_f6 *= temp_r3 as f32;
        var_f3 += var_f4 / var_f6;
        // `while (var_f3 != temp_f5)`: a NaN never compares equal.
        if var_f3 == temp_f5 {
            break;
        }
    }
    if var_r4 != 0 {
        var_f3 = 1.0 / var_f3;
    }
    var_f3
}

/// `powf` (lb_00CE.c), retail `0x8000CEE0`.
///
/// `exp(y * ln(x))` with `ln(x)` from the series
/// `2 * (u + u^3/3 + u^5/5 + ...)`, `u = (x - 1) / (x + 1)`, summed in
/// single precision until it stops changing. `powf(0, y)` is `0` for every
/// `y`, including `0` (the C returns before looking at `y`). Negative bases
/// diverge the series to `-inf` and yield `expf(-inf * y)`. Variable names
/// follow the decomp.
pub fn powf(arg0: f32, arg1: f32) -> f32 {
    if arg0 == 0.0 {
        return 0.0;
    }
    let mut var_r4: i32 = 1;
    let mut var_f6: f32 = (arg0 - 1.0) / (1.0 + arg0);
    let mut var_f4: f32 = var_f6;
    let temp_f5: f32 = var_f6 * var_f6;
    loop {
        // Signed overflow is UB in the C; only reachable when the loop
        // never terminates anyway.
        var_r4 = var_r4.wrapping_add(2);
        var_f6 *= temp_f5;
        let temp_f1: f32 = var_f4;
        // `(f32) var_r4`: exact below 2^24, which the slowest terminating
        // series (u = +/-1, about 2^21 steps) stays under.
        var_f4 += var_f6 / (var_r4 as f32);
        // `while (var_f4 != temp_f1)`
        if var_f4 == temp_f1 {
            break;
        }
    }
    expf(arg1 * (2.0 * var_f4))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PI: f32 = core::f32::consts::PI;
    const FRAC_PI_2: f32 = core::f32::consts::FRAC_PI_2;
    const FRAC_PI_4: f32 = core::f32::consts::FRAC_PI_4;

    fn assert_close(what: &str, got: f32, want: f32, tol: f32) {
        assert!(
            (got - want).abs() <= tol,
            "{what}: got {got:e}, want {want:e}, |diff| = {:e} > {tol:e}",
            (got - want).abs()
        );
    }

    #[test]
    fn constants_have_the_c_bit_patterns() {
        assert_eq!(M_PI_2_F.to_bits(), BITWISE_PI_2);
        assert_eq!(M_PI_F.to_bits(), 0x4049_0FDB);
        assert_eq!(NAN.to_bits(), 0x7FFF_FFFF);
        assert_eq!(INF, f32::INFINITY);
        // The thresholds are the C's `0.534511148929596f` .. `1.870868444442749f`.
        assert_eq!(
            BITWISE_THRESHOLD_0 as u32,
            (0.534511148929596f64 as f32).to_bits()
        );
        assert_eq!(
            BITWISE_THRESHOLD_1 as u32,
            (0.8206787705421448f64 as f32).to_bits()
        );
        assert_eq!(
            BITWISE_THRESHOLD_2 as u32,
            (1.218503475189209f64 as f32).to_bits()
        );
        assert_eq!(
            BITWISE_THRESHOLD_3 as u32,
            (1.870868444442749f64 as f32).to_bits()
        );
    }

    #[test]
    fn atanf_within_tolerance_of_std() {
        // Sweep all three ranges plus the interval boundaries.
        for i in -60_000..=60_000 {
            let x = i as f32 / 10_000.0;
            assert_close(&format!("atanf({x})"), atanf(x), x.atan(), 2e-6);
        }
        for &x in &[1e-3f32, 10.0, 100.0, 1e4, 1e10, 3.4e38] {
            assert_close(&format!("atanf({x})"), atanf(x), x.atan(), 2e-6);
            assert_close(&format!("atanf(-{x})"), atanf(-x), -x.atan(), 2e-6);
        }
        assert_eq!(atanf(0.0).to_bits(), 0.0f32.to_bits());
        assert_eq!(
            atanf(-0.0).to_bits(),
            (-0.0f32).to_bits(),
            "sign bit is restored"
        );
        assert_eq!(atanf(f32::INFINITY), FRAC_PI_2);
        assert_eq!(atanf(f32::NEG_INFINITY), -FRAC_PI_2);
        assert!(atanf(f32::NAN).is_nan());
        assert_close("atanf(1)", atanf(1.0), FRAC_PI_4, 1e-6);
    }

    #[test]
    fn atan2f_within_tolerance_of_std() {
        for i in -200..=200 {
            for j in -200..=200 {
                let (y, x) = (i as f32 / 40.0, j as f32 / 40.0);
                if x == 0.0 && y == 0.0 {
                    continue;
                }
                assert_close(&format!("atan2f({y}, {x})"), atan2f(y, x), y.atan2(x), 3e-6);
            }
        }
        // Both zero: the game's convention, by sign bits.
        assert_eq!(atan2f(0.0, 0.0), FRAC_PI_2);
        assert_eq!(atan2f(-0.0, 0.0), -FRAC_PI_2);
        assert_eq!(atan2f(0.0, -0.0), FRAC_PI_2);
        assert_eq!(atan2f(-0.0, -0.0), -FRAC_PI_2);
        // Axes.
        assert_eq!(atan2f(1.0, 0.0), FRAC_PI_2);
        assert_eq!(atan2f(-1.0, 0.0), -FRAC_PI_2);
        assert_eq!(atan2f(0.0, 1.0), 0.0);
        assert_close("atan2f(0, -1)", atan2f(0.0, -1.0), PI, 1e-6);
        assert_close("atan2f(-0, -1)", atan2f(-0.0, -1.0), -PI, 1e-6);
        assert_eq!(atan2f(1.0, -0.0), FRAC_PI_2, "x == -0.0 with positive y");
        assert_eq!(atan2f(-1.0, -0.0), -FRAC_PI_2, "x == -0.0 with negative y");
    }

    #[test]
    fn asinf_acosf_within_tolerance_of_std() {
        for i in -10_000..=10_000 {
            let x = i as f32 / 10_000.0;
            // Precision degrades as |x| -> 1 because 1 - x^2 is formed in
            // single precision; std computes it differently.
            let tol = if x.abs() > 0.999 {
                2e-3
            } else if x.abs() > 0.99 {
                2e-4
            } else {
                5e-6
            };
            assert_close(&format!("asinf({x})"), asinf(x), x.asin(), tol);
            assert_close(&format!("acosf({x})"), acosf(x), x.acos(), tol);
        }
        assert_eq!(asinf(1.0), FRAC_PI_2);
        assert_eq!(asinf(-1.0), -FRAC_PI_2);
        assert_eq!(acosf(1.0), 0.0);
        assert_close("acosf(-1)", acosf(-1.0), PI, 1e-6);
        assert_eq!(asinf(0.0), 0.0);
        assert_eq!(acosf(0.0), FRAC_PI_2);
        assert!(asinf(1.5).is_nan());
        assert!(acosf(-1.5).is_nan());
        assert!(asinf(f32::NAN).is_nan());
        assert!(acosf(f32::NAN).is_nan());
    }

    #[test]
    fn lb_sqrtf_is_a_reciprocal_square_root() {
        for &x in &[1e-30f32, 1e-6, 0.25, 1.0, 2.0, 4.0, 1e6, 1e30] {
            let want = 1.0 / x.sqrt();
            assert!(
                ((lb_sqrtf(x) - want) / want).abs() < 4e-7,
                "lb_sqrtf({x}) = {}, want {want}",
                lb_sqrtf(x)
            );
        }
        assert_eq!(lb_sqrtf(0.0), f32::INFINITY);
        assert_eq!(lb_sqrtf(-0.0), f32::INFINITY);
        assert_eq!(lb_sqrtf(-1.0).to_bits(), 0x7FFF_FFFF);
        assert_eq!(lb_sqrtf(f32::NAN).to_bits(), 0x7FFF_FFFF);
        // frsqrte(inf) is 0, and the first Newton step forms 0 * (3 - 0 * 0 * inf).
        assert!(lb_sqrtf(f32::INFINITY).is_nan());
    }

    #[test]
    fn expf_within_tolerance_of_std() {
        for i in -1_200..=1_200 {
            let x = i as f32 / 100.0;
            let want = x.exp();
            let rel = ((expf(x) - want) / want).abs();
            assert!(
                rel < 1e-5,
                "expf({x}) = {}, want {want}, rel err {rel:e}",
                expf(x)
            );
        }
        assert_eq!(expf(0.0), 1.0);
        assert_eq!(expf(-0.0), 1.0);
        assert_eq!(expf(f32::INFINITY), f32::INFINITY);
        assert_eq!(expf(f32::NEG_INFINITY), 0.0);
        assert_eq!(expf(100.0), f32::INFINITY, "overflows past the series");
        assert_eq!(expf(-100.0), 0.0);
    }

    #[test]
    fn powf_within_tolerance_of_std() {
        for i in 1..=200 {
            for j in -20..=20 {
                let (base, exp) = (i as f32 / 10.0, j as f32 / 8.0);
                let want = base.powf(exp);
                let rel = ((powf(base, exp) - want) / want).abs();
                assert!(
                    rel < 5e-5,
                    "powf({base}, {exp}) = {}, want {want}, rel err {rel:e}",
                    powf(base, exp)
                );
            }
        }
        assert_eq!(powf(0.0, 2.0), 0.0);
        assert_eq!(
            powf(0.0, 0.0),
            0.0,
            "retail returns 0 for a zero base whatever the exponent"
        );
        assert_eq!(powf(-0.0, 3.0), 0.0);
        assert_eq!(powf(1.0, 1e10), 1.0);
        assert_eq!(
            powf(-1.0, 1.0),
            0.0,
            "negative base: series diverges to -inf, exp(-inf)"
        );
        assert_eq!(powf(-1.0, -1.0), f32::INFINITY);
        assert_eq!(powf(2.0, 200.0), f32::INFINITY);
    }
}
