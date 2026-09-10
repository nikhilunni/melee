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
//! `f32::mul_add` and `f64::mul_add` are guaranteed by Rust to be a single
//! correctly rounded fused operation, which is exactly what the hardware does
//! for the double-precision forms. The single-precision forms (`fmadds`) on
//! Gekko compute in double and round the final result to single once. Since the
//! inputs are singles, a correctly rounded single-precision FMA gives the same
//! bits, so `f32::mul_add` is exact here.

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

/// `fmadds frD, frA, frC, frB` : `frA * frC + frB`, single precision.
#[inline(always)]
pub fn fmadds(a: f32, c: f32, b: f32) -> f32 {
    a.mul_add(c, b)
}

/// `fmsubs frD, frA, frC, frB` : `frA * frC - frB`, single precision.
#[inline(always)]
pub fn fmsubs(a: f32, c: f32, b: f32) -> f32 {
    a.mul_add(c, -b)
}

/// `fnmadds frD, frA, frC, frB` : `-(frA * frC + frB)`, single precision.
#[inline(always)]
pub fn fnmadds(a: f32, c: f32, b: f32) -> f32 {
    negate_rounded(a.mul_add(c, b))
}

/// `fnmsubs frD, frA, frC, frB` : `-(frA * frC - frB)`, single precision.
#[inline(always)]
pub fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    negate_rounded(a.mul_add(c, -b))
}

/// `fmadd frD, frA, frC, frB` : `frA * frC + frB`, double precision.
#[inline(always)]
pub fn fmadd(a: f64, c: f64, b: f64) -> f64 {
    a.mul_add(c, b)
}

/// `fmsub frD, frA, frC, frB` : `frA * frC - frB`, double precision.
#[inline(always)]
pub fn fmsub(a: f64, c: f64, b: f64) -> f64 {
    a.mul_add(c, -b)
}

/// `fnmadd frD, frA, frC, frB` : `-(frA * frC + frB)`, double precision.
#[inline(always)]
pub fn fnmadd(a: f64, c: f64, b: f64) -> f64 {
    negate_rounded_double(a.mul_add(c, b))
}

/// `fnmsub frD, frA, frC, frB` : `-(frA * frC - frB)`, double precision.
#[inline(always)]
pub fn fnmsub(a: f64, c: f64, b: f64) -> f64 {
    negate_rounded_double(a.mul_add(c, -b))
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
