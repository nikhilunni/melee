//! Ports of the Metrowerks Standard Library math routines Melee links.
//!
//! Sources in the decomp submodule:
//! - `src/MSL/math_ppc.h` for `sqrtf` (inline, built on `frsqrte`)
//! - `src/MSL/trigf.c` for `sinf`, `cosf`, `tanf`
//! - `src/MSL/math_1.c`, `src/MSL/math.c` for the rest
//!
//! Every function here must be a literal transcription. Do not "simplify"
//! the arithmetic: the order of operations is the observable behaviour.
//! Where the decomp's C is ambiguous about fusion, consult the retail
//! assembly for the function.

use crate::estimate::frsqrte;

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
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
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
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        (xd * guess) as f32
    } else {
        x
    }
}

/// `sinf` from `src/MSL/trigf.c`. Not yet ported.
pub fn sinf(_x: f32) -> f32 {
    todo!("port MSL sinf from decomp src/MSL/trigf.c (needs __sincos_poly and __sincos_on_quadrant tables from math_data.c)")
}

/// `cosf` from `src/MSL/trigf.c`. Not yet ported.
pub fn cosf(_x: f32) -> f32 {
    todo!("port MSL cosf from decomp src/MSL/trigf.c")
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
}
