//! Gekko estimate instructions: `frsqrte` and `fres`.
//!
//! Both return table-driven approximations, not IEEE results, and Melee's
//! `sqrtf` (MSL `math_ppc.h`), `lbtrigf.c` and the paired-single matrix
//! kernels build on them. This module reproduces them bit for bit.
//!
//! # Derivation
//!
//! The tables below were captured by **executing the instructions** in
//! Dolphin (Felk's scripting fork, interpreter core) with a hand-assembled
//! guest program and no game disc, via `harness/gekko_probe`. Every constant
//! here was fitted from input/output bit pairs alone (`analyze.py`), never
//! read from emulator source; the fitted model reproduces 100% of the
//! captured pairs (23317 `frsqrte`, 25045 `fres` on doubles, 25557 `fres` on
//! singles) on both the interpreter and the ARM64 JIT. The decision to commit
//! the fitted tables is recorded in `TRACKER.md` (user decision, 2026-09-08).
//!
//! # Model
//!
//! **`frsqrte`** (double in, double out). The result mantissa depends on the
//! exponent's parity and the top 15 mantissa bits only: a 32-entry table of
//! `(base, slope)` indexed by `(exp & 1, top 4 mantissa bits)` and a linear
//! correction from the next 11 bits,
//! `mant_out = (base - slope * next11) << 26` (26 significant bits), with
//! `exp_out = (0xBFC - exp_in) >> 1`. Denormals are normalised first (the
//! implicit exponent goes negative) and use the same table. `+0 -> +inf`,
//! `-0 -> -inf`, `+inf -> +0`; `-inf` and every negative number, denormals
//! included, give the default quiet NaN `0x7FF8000000000000`.
//!
//! **`fres`** (double in, double out; the hardware op is double-width). The
//! result mantissa depends on the top 15 mantissa bits only, independent of
//! the exponent: 32 entries indexed by the top 5 mantissa bits, linear
//! correction from the next 10 bits with one extra bit of slope precision,
//! `mant_out = ((base - slope * next10) >> 1) << 29` (a 23-bit single
//! mantissa). `exp_out = 0x7FD - exp_in`, clamped to the single range: if
//! `exp_in < 0x37F` (denormals included) the result is `FLT_MAX` as a
//! double, `0x47EFFFFFE0000000`; if `exp_in > 0x47C` it is zero. The sign is
//! preserved for every finite input. `+-0 -> +-inf`, `+-inf -> +-0`.
//!
//! **`fres` on a single** (`lfs` / `fres` / `stfs`): widen to double (single
//! denormals normalise to a normal double), apply the double model, narrow.
//! The narrowing is exact: the result already has a 23-bit mantissa and lies
//! in the single range, or is one of `+-0`, `+-inf`, `FLT_MAX`, NaN.
//!
//! **NaN inputs** pass through unchanged on the interpreter, which is what
//! this module does; a signalling NaN stays signalling. Dolphin's ARM64 JIT
//! instead passes the NaN through with the quiet bit set. That is the only
//! difference observed between the two cores, and no numeric pair differs.
//! Real hardware behaviour for sNaN inputs is unverified.
//!
//! Golden replays: `tests/golden.rs` checks every pair in the committed
//! fixture (`tests/data/`) and, when present, the full captures under
//! `harness/traces/`. The native C twin of this module is
//! `tests/ref/gekko_estimate.h`, shared by the oracle builds of three crates.

/// Whether this module's estimates are bit-exact with hardware.
pub const EXACTNESS: Exactness = Exactness::HardwareExact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exactness {
    /// Matches Gekko hardware bit for bit.
    HardwareExact,
    /// Returns the correctly rounded IEEE value instead of the hardware
    /// table approximation. Good to a few ULPs of the real thing, which is
    /// not good enough for the oracle.
    IeeeApproximation,
}

/// Default quiet NaN produced by invalid operations (`-inf`, negatives).
const DEFAULT_NAN: u64 = 0x7FF8_0000_0000_0000;
/// `FLT_MAX` widened to a double: `fres` result when the input is too small
/// for the reciprocal to fit the single range.
const FLT_MAX_F64: u64 = 0x47EF_FFFF_E000_0000;
const F64_MANT_MASK: u64 = (1 << 52) - 1;
const F64_SIGN: u64 = 1 << 63;

/// `frsqrte` table: `[parity][top 4 mantissa bits] = (base, slope)`.
/// `mant_out = (base - slope * next11) << 26`.
#[rustfmt::skip]
const FRSQRTE_TABLE: [[(u32, u32); 16]; 2] = [
    // even exponent
    [
        (0x1A7E800, 0x0568), (0x17CB800, 0x04F3), (0x1552800, 0x048D), (0x130C000, 0x0435),
        (0x10F2000, 0x03E7), (0x0EFF000, 0x03A2), (0x0D2E000, 0x0365), (0x0B7C000, 0x032E),
        (0x09E5000, 0x02FC), (0x0867000, 0x02D0), (0x06FF000, 0x02A8), (0x05AB800, 0x0283),
        (0x046A000, 0x0261), (0x0339800, 0x0243), (0x0218800, 0x0226), (0x0105800, 0x020B),
    ],
    // odd exponent
    [
        (0x3FFA000, 0x07A4), (0x3C29000, 0x0700), (0x38AA000, 0x0670), (0x3572000, 0x05F2),
        (0x3279000, 0x0584), (0x2FB7000, 0x0524), (0x2D26000, 0x04CC), (0x2AC0000, 0x047E),
        (0x2881000, 0x043A), (0x2665000, 0x03FA), (0x2468000, 0x03C2), (0x2287000, 0x038E),
        (0x20C1000, 0x035E), (0x1F12000, 0x0332), (0x1D79000, 0x030A), (0x1BF4000, 0x02E6),
    ],
];

/// `fres` table: `[top 5 mantissa bits] = (base, slope)`.
/// `mant_out = ((base - slope * next10) >> 1) << 29`.
///
/// Fourteen entries have an even slope, so the low bit of their `base` is
/// unobservable (both values reproduce every output); the fitted value is
/// kept as printed by `analyze.py`.
#[rustfmt::skip]
const FRES_TABLE: [(u32, u32); 32] = [
    (0xFFF000, 0x3E1), (0xF07000, 0x3A7), (0xE1D400, 0x371), (0xD41000, 0x340),
    (0xC71000, 0x313), (0xBAC400, 0x2EA), (0xAF2000, 0x2C4), (0xA41000, 0x2A0),
    (0x999000, 0x27F), (0x8F9400, 0x261), (0x861000, 0x245), (0x7D0000, 0x22A),
    (0x745800, 0x212), (0x6C1000, 0x1FB), (0x642800, 0x1E5), (0x5C9400, 0x1D1),
    (0x555000, 0x1BE), (0x4E5800, 0x1AC), (0x47AC00, 0x19B), (0x413C00, 0x18B),
    (0x3B1000, 0x17C), (0x352000, 0x16E), (0x2F5C00, 0x15B), (0x29F000, 0x15B),
    (0x248800, 0x143), (0x1F7C00, 0x143), (0x1A7000, 0x12D), (0x15BC00, 0x12D),
    (0x110800, 0x11A), (0x0CA000, 0x11A), (0x083800, 0x108), (0x041800, 0x106),
];

/// `frsqrte frD, frB` on raw double bits.
#[inline]
pub fn frsqrte_bits(bits: u64) -> u64 {
    let sign = bits & F64_SIGN;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let mant = bits & F64_MANT_MASK;

    if exp == 0x7FF {
        if mant != 0 {
            return bits; // NaN passes through (interpreter behaviour)
        }
        return if sign != 0 { DEFAULT_NAN } else { 0 }; // -inf -> qNaN, +inf -> +0
    }
    if exp == 0 && mant == 0 {
        return sign | (0x7FF << 52); // +-0 -> +-inf
    }
    if sign != 0 {
        return DEFAULT_NAN;
    }

    // Denormal: normalise so the leading one is implicit; the exponent goes
    // negative.
    let (exp, mant) = if exp == 0 {
        let shift = mant.leading_zeros() as i32 - 11;
        (1 - shift, (mant << shift) & F64_MANT_MASK)
    } else {
        (exp, mant)
    };

    let (base, slope) = FRSQRTE_TABLE[(exp & 1) as usize][(mant >> 48) as usize];
    let next11 = ((mant >> 37) & 0x7FF) as u32;
    let mant_out = u64::from(base - slope * next11) << 26;
    let exp_out = ((0xBFC - exp) >> 1) as u64;
    (exp_out << 52) | mant_out
}

/// `frsqrte frD, frB`: reciprocal square root estimate, double precision.
///
/// Accuracy is about 1 part in 2^12. Callers refine with Newton iterations;
/// see `msl::sqrtf` and `melee_lb::trigf`.
#[inline]
pub fn frsqrte(x: f64) -> f64 {
    f64::from_bits(frsqrte_bits(x.to_bits()))
}

/// `fres frD, frB` on raw double bits.
#[inline]
pub fn fres_bits(bits: u64) -> u64 {
    let sign = bits & F64_SIGN;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let mant = bits & F64_MANT_MASK;

    if exp == 0x7FF {
        if mant != 0 {
            return bits; // NaN passes through (interpreter behaviour)
        }
        return sign; // +-inf -> +-0
    }
    if exp == 0 && mant == 0 {
        return sign | (0x7FF << 52); // +-0 -> +-inf
    }
    // Single-range clamp. Denormal inputs (exp == 0) fall under the first
    // rule, so no normalisation is needed.
    if exp < 0x37F {
        return sign | FLT_MAX_F64;
    }
    if exp > 0x47C {
        return sign;
    }

    let (base, slope) = FRES_TABLE[(mant >> 47) as usize];
    let next10 = ((mant >> 37) & 0x3FF) as u32;
    let mant_out = u64::from((base - slope * next10) >> 1) << 29;
    let exp_out = (0x7FD - exp) as u64;
    sign | (exp_out << 52) | mant_out
}

/// `fres frD, frB` on a double: the hardware operation, which is
/// double-width even though the result is single-precision accurate.
#[inline]
pub fn fres_f64(x: f64) -> f64 {
    f64::from_bits(fres_bits(x.to_bits()))
}

/// `lfs`: widen single bits to double bits. Single denormals become normal
/// doubles; NaN payloads shift up and stay as they are.
#[inline]
fn widen_bits(bits: u32) -> u64 {
    let sign = u64::from(bits >> 31) << 63;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = u64::from(bits & 0x7F_FFFF);
    if exp == 0xFF {
        return sign | (0x7FF << 52) | (mant << 29);
    }
    if exp == 0 {
        if mant == 0 {
            return sign;
        }
        let shift = mant.leading_zeros() as i32 - 40;
        let mant = (mant << shift) & 0x7F_FFFF;
        return sign | (((1 - shift + 0x380) as u64) << 52) | (mant << 29);
    }
    sign | (((exp + 0x380) as u64) << 52) | (mant << 29)
}

/// `stfs` of a `fres` result: narrow double bits to single bits. The `fres`
/// model only produces values that are exactly single-representable (see
/// the module docs), so this never rounds.
#[inline]
fn narrow_fres_bits(bits: u64) -> u32 {
    let sign = ((bits >> 63) as u32) << 31;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let mant = ((bits & F64_MANT_MASK) >> 29) as u32;
    debug_assert_eq!(
        bits & ((1 << 29) - 1),
        0,
        "fres result not single-representable"
    );
    if exp == 0x7FF {
        return sign | (0xFF << 23) | mant;
    }
    if exp == 0 {
        return sign;
    }
    debug_assert!(
        (0x381..=0x47E).contains(&exp),
        "fres result exponent {exp:#x} outside single range"
    );
    sign | (((exp - 0x380) as u32) << 23) | mant
}

/// `fres frD, frB` on a single: `lfs` / `fres` / `stfs`, i.e. widen, apply
/// [`fres_f64`], narrow (exactly).
#[inline]
pub fn fres(x: f32) -> f32 {
    f32::from_bits(narrow_fres_bits(fres_bits(widen_bits(x.to_bits()))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frsqrte_special_values() {
        assert_eq!(frsqrte_bits(f64::NAN.to_bits()), f64::NAN.to_bits());
        assert_eq!(frsqrte_bits(0x7FF0_0000_0000_0001), 0x7FF0_0000_0000_0001); // sNaN unchanged
        assert_eq!(frsqrte_bits(0x7FF8_0000_0000_BEEF), 0x7FF8_0000_0000_BEEF);
        assert_eq!(frsqrte(-1.0).to_bits(), DEFAULT_NAN);
        assert_eq!(frsqrte(f64::NEG_INFINITY).to_bits(), DEFAULT_NAN);
        assert_eq!(frsqrte(-f64::from_bits(1)).to_bits(), DEFAULT_NAN);
        assert_eq!(frsqrte(0.0), f64::INFINITY);
        assert_eq!(frsqrte(-0.0), f64::NEG_INFINITY);
        assert_eq!(frsqrte(f64::INFINITY).to_bits(), 0);
    }

    #[test]
    fn frsqrte_captured_examples() {
        // From harness/traces/frsqrte_probe.jsonl (see module docs).
        assert_eq!(frsqrte_bits(0x0000_0000_0000_0001), 0x617F_FE80_0000_0000);
        assert_eq!(frsqrte_bits(0x0000_0000_0000_0002), 0x6176_9FA0_0000_0000);
        assert_eq!(frsqrte_bits(0x0008_0000_0000_0000), 0x5FE6_9FA0_0000_0000);
        // 1.0: odd exponent (0x3FF), index 0, next11 = 0.
        assert_eq!(
            frsqrte_bits(1.0f64.to_bits()),
            (0x3FEu64 << 52) | (0x3FFA000u64 << 26)
        );
    }

    #[test]
    fn fres_special_values() {
        assert_eq!(fres_bits(0x7FF0_0000_0000_0001), 0x7FF0_0000_0000_0001);
        assert_eq!(fres_f64(0.0), f64::INFINITY);
        assert_eq!(fres_f64(-0.0), f64::NEG_INFINITY);
        assert_eq!(fres_f64(f64::INFINITY).to_bits(), 0);
        assert_eq!(fres_f64(f64::NEG_INFINITY).to_bits(), F64_SIGN);
        // Clamp: too small -> +-FLT_MAX, too large -> +-0. Denormals count as small.
        assert_eq!(fres_bits(0x0000_0000_0000_0001), FLT_MAX_F64);
        assert_eq!(fres_bits(0x8000_0000_0000_0001), F64_SIGN | FLT_MAX_F64);
        assert_eq!(fres_bits(0x37E << 52), FLT_MAX_F64);
        assert_eq!(fres_bits(0x47D << 52), 0);
        assert_eq!(fres_bits(F64_SIGN | (0x47D << 52)), F64_SIGN);
        // Captured pairs.
        assert_eq!(fres_bits(0xBFF0_0000_0000_0000), 0xBFEF_FF00_0000_0000);
        assert_eq!(fres_bits(0xC004_0000_0000_0000), 0xBFD9_9900_0000_0000);
        assert_eq!(fres_bits(0x908D_8C20_C06E_7E5F), 0xC7EF_FFFF_E000_0000);
    }

    #[test]
    fn fres_single_path() {
        assert_eq!(fres(1.0).to_bits(), 0x3F7F_F800);
        assert_eq!(fres(0.0), f32::INFINITY);
        assert_eq!(fres(-0.0), f32::NEG_INFINITY);
        assert_eq!(fres(f32::INFINITY).to_bits(), 0);
        assert_eq!(fres(f32::from_bits(1)), f32::MAX);
        assert_eq!(fres(f32::from_bits(0x8000_0001)), f32::MIN);
        assert_eq!(fres(f32::MAX).to_bits(), 0);
        assert_eq!(fres(f32::from_bits(0x7F80_0001)).to_bits(), 0x7F80_0001);
        assert_eq!(fres(f32::from_bits(0xFFC0_BEEF)).to_bits(), 0xFFC0_BEEF);
        // The single path is exactly widen -> fres_f64 -> narrow.
        for bits in [
            0x3F80_0000u32,
            0x4220_0000,
            0x0080_0000,
            0x0000_0001,
            0x7F7F_FFFF,
        ] {
            let x = f32::from_bits(bits);
            assert_eq!(fres(x), fres_f64(f64::from(x)) as f32);
        }
    }

    #[test]
    fn widen_matches_std_for_every_class() {
        for bits in [
            0u32,
            0x8000_0000,
            1,
            0x0040_0000,
            0x007F_FFFF,
            0x0080_0000,
            0x3F80_0000,
            0x7F7F_FFFF,
            0xFF80_0000,
            0x7F80_0000,
            0x7FC0_0000,
            0xFFC0_BEEF,
        ] {
            assert_eq!(
                widen_bits(bits),
                f64::from(f32::from_bits(bits)).to_bits(),
                "{bits:#x}"
            );
        }
        // sNaN payload placement (std may quiet it; the widen must not).
        assert_eq!(widen_bits(0x7F80_0001), 0x7FF0_0000_2000_0000);
    }

    #[test]
    fn matches_hardware_golden() {
        assert_eq!(EXACTNESS, Exactness::HardwareExact);
        // The full replay of the captured pairs lives in tests/golden.rs.
        assert_eq!(frsqrte_bits(0x3FF0_0000_0000_0000), 0x3FEF_FE80_0000_0000);
        assert_eq!(fres_bits(0x3FF0_0000_0000_0000), 0x3FEF_FF00_0000_0000);
    }
}
