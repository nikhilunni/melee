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
    -a.mul_add(c, b)
}

/// `fnmsubs frD, frA, frC, frB` : `-(frA * frC - frB)`, single precision.
#[inline(always)]
pub fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -a.mul_add(c, -b)
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

/// `fnmsub frD, frA, frC, frB` : `-(frA * frC - frB)`, double precision.
#[inline(always)]
pub fn fnmsub(a: f64, c: f64, b: f64) -> f64 {
    -a.mul_add(c, -b)
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
