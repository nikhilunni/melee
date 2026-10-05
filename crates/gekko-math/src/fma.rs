//! Fused multiply-add in PowerPC operand order.
//!
//! PowerPC spells these `frD = frA * frC + frB`. We keep the (a, c, b)
//! argument order so a line of retail assembly can be transcribed without
//! mentally reordering operands:
//!
//! ```text
//! fmadds f1, f2, f3, f4   =>   fmadds(f2, f3, f4)
//! ```
//!
//! The double-precision forms are one correctly rounded fused operation. The
//! single-precision forms (`fmadds`) on Gekko compute in double and round the
//! final result to single once. Since the inputs are singles, a correctly
//! rounded single-precision FMA gives the same bits.
//!
//! Rust guarantees `mul_add` is correctly rounded, but where the target has no
//! fma instruction it calls libm, and the libms differ: musl's `fmaf` (wasm,
//! via compiler-builtins) computes in double and rounds twice for results in
//! the single subnormal range, and wasi-libc's `fma` drops the sign of an
//! underflowed product when the addend is zero. [`fused_single`] and
//! [`fused_double`] use the hardware instruction where it exists and an exact
//! software path elsewhere, so every target produces the same bits.
//! `tools/wasm-check.sh` runs these tests under wasm.

/// Negate an already rounded single without folding its sign into an FMA.
/// Use for a retail `fneg`/`ps_neg` whose input can be a fused result.
//
// LLVM 22's AArch64 lowering folds fneg(fma(a, c, -b)) into an FMA
// with negated inputs even without fast-math. Exact cancellation then yields
// +0 instead of PPC's -0. A to_bits() XOR of just the sign bit is recognized
// as fneg too. Copy the complemented sign of the *rounded result* instead:
// copysign preserves its magnitude, and the integer complement supplies the
// opposite sign even for either zero and for underflow. This keeps one fused
// operation, with no black_box, volatile memory, or out-of-line call.
#[inline(always)]
pub fn negate_rounded(result: f32) -> f32 {
    result.copysign(f32::from_bits(!result.to_bits()))
}

// Same construction for the double-width instructions.
#[inline(always)]
fn negate_rounded_double(result: f64) -> f64 {
    result.copysign(f64::from_bits(!result.to_bits()))
}

/// Correctly rounded single-precision `a * c + b` on every target.
///
/// With a hardware fma (aarch64; x86_64 built with the `fma` feature)
/// `f32::mul_add` is that one instruction. Elsewhere it would call libm
/// `fmaf`, so compute it exactly instead: the product of two singles is exact
/// in double (48 significant bits, exponent within -298..256), TwoSum gives
/// the double sum and its exact error, and rounding the sum to odd before the
/// single conversion makes the two roundings equal one (53 >= 2 * 24 + 2).
#[cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "fma")
))]
#[inline(always)]
#[allow(clippy::disallowed_methods)] // the one place mul_add is allowed
fn fused_single(a: f32, c: f32, b: f32) -> f32 {
    a.mul_add(c, b)
}

/// See the hardware [`fused_single`].
#[cfg(not(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "fma")
)))]
#[inline(always)]
fn fused_single(a: f32, c: f32, b: f32) -> f32 {
    software_fused_single(a, c, b)
}

/// [`fused_single`] without the fma instruction; compiled on every target so
/// native tests exercise it against the hardware result.
#[cfg_attr(
    any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "fma")
    ),
    allow(dead_code)
)]
#[inline(always)]
fn software_fused_single(a: f32, c: f32, b: f32) -> f32 {
    let product = f64::from(a) * f64::from(c); // exact
    let addend = f64::from(b);
    let sum = product + addend;
    // Infinity or NaN operands: the double result is the fma result. An exact
    // zero sum is exact here too: two signed zeros follow IEEE addition
    // (-0 only for -0 + -0), and nonzero terms cancel to +0, as fma does.
    // A nonzero exact sum cannot round to zero (both terms are multiples of
    // 2^-298), and the sum cannot overflow (|a*c| < 2^256).
    if !sum.is_finite() || sum == 0.0 {
        return sum as f32;
    }
    // TwoSum (Knuth): sum + error == product + addend exactly.
    let addend_part = sum - product;
    let error = (product - (sum - addend_part)) + (addend - addend_part);
    // Round to odd: an inexact sum with an even last bit moves one ulp
    // toward the exact value; the result then brackets it with an odd bit,
    // so the conversion below rounds as if from the exact value. The
    // neighbour has sum's sign (|error| <= half an ulp of sum).
    let bits = sum.to_bits();
    let odd = if error != 0.0 && bits & 1 == 0 {
        let away_from_zero = (error > 0.0) == (sum > 0.0);
        f64::from_bits(if away_from_zero { bits + 1 } else { bits - 1 })
    } else {
        sum
    };
    // Round to nearest even, to infinity past f32::MAX, sign kept on underflow.
    odd as f32
}

/// Correctly rounded double-precision `a * c + b` on every target.
///
/// wasi-libc's `fma` returns `a * c + b` when `b` is zero, which turns an
/// underflowed product's `-0` into `+0`. Only that case needs the product's
/// own rounding; the browser target (compiler-builtins) and native hardware
/// are already correct.
#[inline(always)]
#[allow(clippy::disallowed_methods)] // the one place mul_add is allowed
fn fused_double(a: f64, c: f64, b: f64) -> f64 {
    #[cfg(target_os = "wasi")]
    if b == 0.0 {
        let product = a * c;
        if product == 0.0 && a != 0.0 && c != 0.0 {
            // A nonzero exact product rounds to this signed zero, and adding
            // a zero addend leaves it.
            return product;
        }
    }
    a.mul_add(c, b)
}

/// Gekko `fmuls` with double-width register operands. The multiplier is rounded
/// to 25 significant bits (halfway away from zero) before multiplication;
/// the product is then rounded to single. Single-width operands are unchanged.
/// This matters when PSVECMag/Normalize multiply the double `frsqrte` estimate.
#[inline]
pub fn fmuls(a: f64, c: f64) -> f32 {
    let magnitude = c.to_bits() & !(1_u64 << 63);
    let exponent = magnitude >> 52;
    let discard = if exponent == 0 {
        (64 - magnitude.leading_zeros()).saturating_sub(25)
    } else {
        28
    };
    let rounded = if exponent == 0x7ff || discard == 0 {
        c
    } else {
        let quantum = 1_u64 << discard;
        let rounded_magnitude = ((magnitude + quantum / 2) / quantum) * quantum;
        f64::from_bits((c.to_bits() & (1_u64 << 63)) | rounded_magnitude)
    };
    (a * rounded) as f32
}

/// `fmadds frD, frA, frC, frB` : `frA * frC + frB`, single precision.
#[inline(always)]
pub fn fmadds(a: f32, c: f32, b: f32) -> f32 {
    fused_single(a, c, b)
}

/// `fmsubs frD, frA, frC, frB` : `frA * frC - frB`, single precision.
#[inline(always)]
pub fn fmsubs(a: f32, c: f32, b: f32) -> f32 {
    fused_single(a, c, -b)
}

/// `fnmadds frD, frA, frC, frB` : `-(frA * frC + frB)`, single precision.
#[inline(always)]
pub fn fnmadds(a: f32, c: f32, b: f32) -> f32 {
    negate_rounded(fused_single(a, c, b))
}

/// `fnmsubs frD, frA, frC, frB` : `-(frA * frC - frB)`, single precision.
#[inline(always)]
pub fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    negate_rounded(fused_single(a, c, -b))
}

/// `fmadd frD, frA, frC, frB` : `frA * frC + frB`, double precision.
#[inline(always)]
pub fn fmadd(a: f64, c: f64, b: f64) -> f64 {
    fused_double(a, c, b)
}

/// `fmsub frD, frA, frC, frB` : `frA * frC - frB`, double precision.
#[inline(always)]
pub fn fmsub(a: f64, c: f64, b: f64) -> f64 {
    fused_double(a, c, -b)
}

/// `fnmadd frD, frA, frC, frB` : `-(frA * frC + frB)`, double precision.
#[inline(always)]
pub fn fnmadd(a: f64, c: f64, b: f64) -> f64 {
    negate_rounded_double(fused_double(a, c, b))
}

/// `fnmsub frD, frA, frC, frB` : `-(frA * frC - frB)`, double precision.
#[inline(always)]
pub fn fnmsub(a: f64, c: f64, b: f64) -> f64 {
    negate_rounded_double(fused_double(a, c, -b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fused_differs_from_separate_rounding() {
        // A case where (a*c) is not representable, so fused and unfused
        // results differ in the last bit. This test exists to catch a build
        // configuration where mul_add silently degrades to mul-then-add.
        let a = 1.0f32 + f32::EPSILON;
        let c = 1.0f32 - f32::EPSILON;
        let b = -1.0f32;
        let fused = fmadds(a, c, b);
        let separate = a * c + b;
        assert_ne!(fused.to_bits(), separate.to_bits());
        // Exact answer is -EPSILON^2, which fused reproduces.
        assert_eq!(fused, -(f32::EPSILON * f32::EPSILON));
    }
}

#[cfg(test)]
mod signed_zero_tests {
    use super::*;
    use std::hint::black_box;

    // IEEE round-to-nearest: unlike zero signs cancel to +0; two negative
    // zero terms sum to -0. PPC negates that rounded result afterward.
    // Literal bit expectations must not be computed through the helpers.
    macro_rules! check_signed_zeros {
        ($name:ident, $float:ident, $add:ident, $sub:ident, $nadd:ident, $nsub:ident) => {
            #[test]
            fn $name() {
                let positive = 0.0 as $float;
                let negative = -0.0 as $float;
                let cases: &[($float, $float, $float, [$float; 4])] = &[
                    (0.051, 0.0, 0.0, [positive, positive, negative, negative]),
                    (0.051, 0.0, -0.0, [positive, positive, negative, negative]),
                    (-0.051, 0.0, 0.0, [positive, negative, negative, positive]),
                    (-0.051, 0.0, -0.0, [negative, positive, positive, negative]),
                    (0.051, -0.0, 0.0, [positive, negative, negative, positive]),
                    (0.051, -0.0, -0.0, [negative, positive, positive, negative]),
                    (-0.051, -0.0, 0.0, [positive, positive, negative, negative]),
                    (-0.051, -0.0, -0.0, [positive, positive, negative, negative]),
                ];
                for &(a, c, b, expected) in cases {
                    // Only obscure inputs: a barrier on the result would hide
                    // the optimizer bug in a caller that consumes float bits.
                    let (a, c, b) = black_box((a, c, b));
                    let actual = [$add(a, c, b), $sub(a, c, b), $nadd(a, c, b), $nsub(a, c, b)];
                    assert_eq!(
                        actual.map($float::to_bits),
                        expected.map($float::to_bits),
                        "a={a:?}, c={c:?}, b={b:?}"
                    );
                }
                // Cancellation of nonzero terms, in both sign directions.
                for (a, c, b) in [(3.0, 2.0, 6.0), (-3.0, 2.0, -6.0)] {
                    let (a, c, b) = black_box((a, c, b));
                    assert_eq!($add(a, c, -b).to_bits(), positive.to_bits());
                    assert_eq!($sub(a, c, b).to_bits(), positive.to_bits());
                    assert_eq!($nadd(a, c, -b).to_bits(), negative.to_bits());
                    assert_eq!($nsub(a, c, b).to_bits(), negative.to_bits());
                }
                // A rounded-to-zero product still carries its nonzero sign.
                for (a, expected, negated) in [
                    ($float::from_bits(1), positive, negative),
                    (-$float::from_bits(1), negative, positive),
                ] {
                    let (a, c, b) = black_box((a, 0.25, 0.0));
                    assert_eq!($add(a, c, b).to_bits(), expected.to_bits());
                    assert_eq!($sub(a, c, b).to_bits(), expected.to_bits());
                    assert_eq!($nadd(a, c, b).to_bits(), negated.to_bits());
                    assert_eq!($nsub(a, c, b).to_bits(), negated.to_bits());
                }
                // Every form must retain fusion, including after negation.
                let (a, c, b) = black_box((1.0 + $float::EPSILON, 1.0 - $float::EPSILON, 1.0));
                let exact = -($float::EPSILON * $float::EPSILON);
                assert_eq!($add(a, c, -b).to_bits(), exact.to_bits());
                assert_eq!($sub(a, c, b).to_bits(), exact.to_bits());
                assert_eq!($nadd(a, c, -b).to_bits(), (-exact).to_bits());
                assert_eq!($nsub(a, c, b).to_bits(), (-exact).to_bits());
            }
        };
    }

    pub(super) use check_signed_zeros;

    check_signed_zeros!(
        single_fused_signed_zeros,
        f32,
        fmadds,
        fmsubs,
        fnmadds,
        fnmsubs
    );
    check_signed_zeros!(double_fused_signed_zeros, f64, fmadd, fmsub, fnmadd, fnmsub);
}

#[cfg(test)]
mod software_single_tests {
    use super::*;
    use std::hint::black_box;

    /// Expected bits are literal: none is computed through mul_add or libm.
    #[test]
    fn rounds_once_at_known_hard_cases() {
        let two_pow_103 = f32::from_bits((127 + 103) << 23);
        let cases: &[(&str, f32, f32, f32, u32)] = &[
            // 2^-75(1+2^-20) * 2^-75(1-2^-20) + (2^19+1)*2^-149: the exact sum
            // is just below a subnormal halfway point. musl's fmaf (wasm)
            // rounds it to the halfway point in double first: 0x00080002.
            (
                "subnormal halfway",
                f32::from_bits((127 - 75) << 23 | 0x8),
                f32::from_bits((126 - 75) << 23 | 0x7f_fff0),
                f32::from_bits(0x0008_0001),
                0x0008_0001,
            ),
            // (1 + 2^-23)(1 - 2^-23) - 1 is exactly -2^-46, lost by separate rounding.
            ("normal cancellation", 1.0 + f32::EPSILON, 1.0 - f32::EPSILON, -1.0, 0xa880_0000),
            // f32::MAX + half an ulp ties to even, which is infinity.
            ("overflow tie", f32::MAX, 1.0, two_pow_103, 0x7f80_0000),
            ("below overflow tie", f32::MAX, 1.0, two_pow_103 * 0.5, 0x7f7f_ffff),
            // 2^128 - 2^104 is exactly f32::MAX though the product overflows.
            (
                "product past MAX",
                f32::from_bits(0x7f00_0000),
                2.0,
                -f32::from_bits((127 + 104) << 23),
                0x7f7f_ffff,
            ),
            ("negative overflow", -f32::MAX, 2.0, 0.0, 0xff80_0000),
            ("infinite addend", 1.0, 1.0, f32::INFINITY, 0x7f80_0000),
            ("infinite product", f32::INFINITY, -1.0, 1.0, 0xff80_0000),
            // A product below the smallest subnormal rounds to signed zero.
            ("underflow to -0", -f32::from_bits(1), f32::from_bits(1), 0.0, 0x8000_0000),
            ("underflow to +0", f32::from_bits(1), f32::from_bits(1), -0.0, 0x0000_0000),
        ];
        for &(name, a, c, b, expected) in cases {
            let (a, c, b) = black_box((a, c, b));
            assert_eq!(software_fused_single(a, c, b).to_bits(), expected, "software: {name}");
            assert_eq!(fmadds(a, c, b).to_bits(), expected, "fmadds: {name}");
        }
        for (a, c, b) in [
            (f32::INFINITY, 0.0, 1.0),
            (f32::INFINITY, 1.0, f32::NEG_INFINITY),
            (f32::NAN, 1.0, 1.0),
        ] {
            let (a, c, b) = black_box((a, c, b));
            assert!(software_fused_single(a, c, b).is_nan());
            assert!(fmadds(a, c, b).is_nan());
        }
    }

    /// Where the target has the instruction, the software path must agree
    /// with it bit for bit (NaNs compared as NaN).
    #[cfg(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "fma")
    ))]
    #[test]
    fn software_matches_the_fma_instruction() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let single = |r: u64| -> f32 {
            let sign = ((r & 1) as u32) << 31;
            let mantissa = (r >> 20) as u32 & 0x7f_ffff;
            let exponent = (r >> 50) as u32;
            match r % 4 {
                0 => f32::from_bits(r as u32 >> 3),     // subnormal and tiny
                1 => f32::from_bits((r >> 32) as u32), // any bits
                // Products near the subnormal range, then near one.
                2 => f32::from_bits(sign | (40 + exponent % 20) << 23 | mantissa),
                _ => f32::from_bits(sign | (100 + exponent % 56) << 23 | mantissa),
            }
        };
        for _ in 0..500_000 {
            let (a, c, b) = (single(next()), single(next()), single(next()));
            let hardware = fused_single(a, c, b);
            let software = software_fused_single(black_box(a), black_box(c), black_box(b));
            if hardware.is_nan() {
                assert!(software.is_nan(), "{a:e} * {c:e} + {b:e}");
            } else {
                assert_eq!(software.to_bits(), hardware.to_bits(), "{a:e} * {c:e} + {b:e}");
            }
        }
    }

    fn soft_fmadds(a: f32, c: f32, b: f32) -> f32 {
        software_fused_single(a, c, b)
    }
    fn soft_fmsubs(a: f32, c: f32, b: f32) -> f32 {
        software_fused_single(a, c, -b)
    }
    fn soft_fnmadds(a: f32, c: f32, b: f32) -> f32 {
        negate_rounded(software_fused_single(a, c, b))
    }
    fn soft_fnmsubs(a: f32, c: f32, b: f32) -> f32 {
        negate_rounded(software_fused_single(a, c, -b))
    }
    super::signed_zero_tests::check_signed_zeros!(
        software_single_signed_zeros,
        f32,
        soft_fmadds,
        soft_fmsubs,
        soft_fnmadds,
        soft_fnmsubs
    );
}
