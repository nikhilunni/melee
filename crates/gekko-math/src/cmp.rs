//! Float bounds written as the retail comparison.
//!
//! `f32::max`/`f32::min` leave the result for `+0` against `-0` unspecified
//! (aarch64's `fmaxnm` returns `+0`; wasm's lowering depends on argument
//! order) and they discard a NaN operand. Retail clamps with an ordered
//! compare and a branch:
//!
//! ```text
//! fcmpo cr0, f_x, f_floor      fcmpo cr0, f_x, f_ceiling
//! bge   keep                   ble   keep
//! fmr   f_x, f_floor           fmr   f_x, f_ceiling
//! ```
//!
//! which is C's `if (x < floor) x = floor;` and `if (x > ceiling) x = ceiling;`.
//! Both keep `x` when the operands compare equal (so the sign of a zero `x`
//! survives) and when either is NaN (an unordered `fcmpo` takes `bge`/`ble`).
//! The argument order is therefore part of the meaning: the first argument is
//! the value kept on a tie. A C ternary in another order swaps the arguments
//! (`a > b ? a : b` is `max(b, a)`).

/// `if (x < floor) x = floor;` : `fcmpo x, floor` then `bge`.
/// Returns `x` on a tie or when unordered.
#[inline(always)]
pub fn max<T: PartialOrd>(x: T, floor: T) -> T {
    if x < floor { floor } else { x }
}

/// `if (x > ceiling) x = ceiling;` : `fcmpo x, ceiling` then `ble`.
/// Returns `x` on a tie or when unordered.
#[inline(always)]
pub fn min<T: PartialOrd>(x: T, ceiling: T) -> T {
    if x > ceiling { ceiling } else { x }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hint::black_box;

    #[test]
    fn mixed_zeros_keep_the_first_argument() {
        // Expectations are literal bits, never std max/min.
        let cases: [(f32, f32, u32, u32); 4] = [
            (0.0, -0.0, 0x0000_0000, 0x0000_0000),
            (-0.0, 0.0, 0x8000_0000, 0x8000_0000),
            (-0.0, -0.0, 0x8000_0000, 0x8000_0000),
            (0.0, 0.0, 0x0000_0000, 0x0000_0000),
        ];
        for (x, y, want_max, want_min) in cases {
            let (x, y) = black_box((x, y));
            assert_eq!(max(x, y).to_bits(), want_max, "max({x:?}, {y:?})");
            assert_eq!(min(x, y).to_bits(), want_min, "min({x:?}, {y:?})");
        }
        let (x, y) = black_box((-0.0f64, 0.0f64));
        assert_eq!(max(x, y).to_bits(), 0x8000_0000_0000_0000);
        assert_eq!(min(y, x).to_bits(), 0);
    }

    #[test]
    fn ordered_operands_bound_as_expected() {
        let (a, b) = black_box((-1.5f32, 2.0f32));
        assert_eq!(max(a, b), 2.0);
        assert_eq!(max(b, a), 2.0);
        assert_eq!(min(a, b), -1.5);
        assert_eq!(min(b, a), -1.5);
        assert_eq!(max(a, 0.0).to_bits(), 0);
    }

    #[test]
    fn unordered_keeps_the_first_argument() {
        let nan = black_box(f32::NAN);
        assert!(max(nan, 1.0).is_nan());
        assert!(min(nan, 1.0).is_nan());
        assert_eq!(max(1.0, nan), 1.0);
        assert_eq!(min(1.0, nan), 1.0);
    }
}
