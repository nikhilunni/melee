//! Gekko estimate instructions: `frsqrte` and `fres`.
//!
//! These return table-driven approximations. The exact tables were
//! reverse-engineered by the Dolphin emulator project and live in
//! `Source/Core/Common/FloatUtils.cpp` there, under GPLv2. Reproducing them
//! here is a licensing decision for the project owner, so this module ships
//! with a **placeholder** that returns the IEEE-exact value and is tagged
//! [`EXACTNESS`] so callers and tests can see the port is not yet bit-exact.
//!
//! Until this is resolved, any function that transitively uses `sqrtf`
//! cannot pass a bit-exact oracle comparison. `melee-diff` scenarios that hit
//! this will report last-bit divergences in positions and velocities.
//!
//! Resolution options, in order of preference:
//! 1. Derive the tables from hardware behaviour documented in the
//!    PowerPC 750CL user manual and verify against traces from the oracle.
//! 2. Extract the tables empirically: run `frsqrte` over the full input
//!    domain in Dolphin (the harness can do this in a single scenario) and
//!    store the resulting table as data, which is not copyrightable code.
//! 3. Port Dolphin's implementation and accept GPLv2 for this crate.

/// Whether this module's estimates are bit-exact with hardware.
pub const EXACTNESS: Exactness = Exactness::IeeeApproximation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exactness {
    /// Matches Gekko hardware bit for bit.
    HardwareExact,
    /// Returns the correctly rounded IEEE value instead of the hardware
    /// table approximation. Good to a few ULPs of the real thing, which is
    /// not good enough for the oracle.
    IeeeApproximation,
}

/// `frsqrte frD, frB`: reciprocal square root estimate, double precision.
///
/// Hardware accuracy is roughly 1 part in 4096. Callers refine with Newton
/// iterations; see `msl::sqrtf`.
#[inline]
pub fn frsqrte(x: f64) -> f64 {
    // PLACEHOLDER. See module docs.
    if x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return if x.is_sign_negative() { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    if x < 0.0 {
        return f64::NAN;
    }
    if x.is_infinite() {
        return 0.0;
    }
    1.0 / x.sqrt()
}

/// `fres frD, frB`: reciprocal estimate, single precision.
#[inline]
pub fn fres(x: f32) -> f32 {
    // PLACEHOLDER. See module docs.
    1.0 / x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        assert!(frsqrte(f64::NAN).is_nan());
        assert!(frsqrte(-1.0).is_nan());
        assert_eq!(frsqrte(0.0), f64::INFINITY);
        assert_eq!(frsqrte(-0.0), f64::NEG_INFINITY);
        assert_eq!(frsqrte(f64::INFINITY), 0.0);
    }

    /// Golden values captured from the oracle go here once the harness can
    /// produce them. Ignored until `EXACTNESS == HardwareExact`.
    #[test]
    #[ignore = "awaiting hardware-exact frsqrte tables; see module docs"]
    fn matches_hardware_golden() {
        assert_eq!(EXACTNESS, Exactness::HardwareExact);
    }
}
