//! Modern sticks and triggers as GameCube bytes.
//!
//! A GameCube stick moves inside an octagonal gate: full tilt reaches a
//! notch at the cardinals and diagonals and the flat edges between them.
//! Retail reads origin-adjusted signed bytes and HSD clamps them to an
//! 80-unit circle (`melee_ft::input::normalize_stick`), so full tilt toward
//! a diagonal notch lands near (56, 56): Melee's (0.7, 0.7). A modern stick
//! has a round gate reporting -1..1, so the core reshapes it: a radial
//! deadzone, the remaining travel rescaled to 0..1, then stretched to the
//! octagon's edge in that direction. The gate radii are Dolphin's averages
//! over genuine controllers (87 main, 74 C-stick, at the notches), which
//! keep partial tilts where a GameCube stick would put them.

use gekko_math::msl::sqrtf;

/// Main stick gate radius at the notches, in stick bytes.
pub const MAIN_GATE: f32 = 87.0;
/// C-stick gate radius at the notches.
pub const C_GATE: f32 = 74.0;
/// cos(pi/8) and sin(pi/8): the octagon's edge normals sit 22.5 degrees off
/// the notches.
const COS_PI_8: f32 = 0.923_879_5;
const SIN_PI_8: f32 = 0.382_683_43;
/// Retail's analog trigger full press (HSD clamps to 140).
const TRIGGER_UNITS: f32 = 140.0;

fn byte(value: f32) -> i8 {
    value.round().clamp(-127.0, 127.0) as i8
}

/// A modern stick (`x`, `y` in -1..1, up positive) as GameCube stick bytes
/// inside an octagonal gate of `gate` units, after a radial `deadzone`.
pub fn gate_stick(x: f32, y: f32, deadzone: f32, gate: f32) -> [i8; 2] {
    if !x.is_finite() || !y.is_finite() {
        return [0, 0];
    }
    let length = sqrtf(x * x + y * y);
    if length <= deadzone || length == 0.0 {
        return [0, 0];
    }
    let travel = ((length - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
    // The octagon's edge distance toward (x, y) is gate * cos(pi/8) over the
    // largest edge normal's dot product with the unit direction; scaling the
    // raw vector folds the division by `length` into that dot product.
    let (ax, ay) = (x.abs(), y.abs());
    let (high, low) = if ax >= ay { (ax, ay) } else { (ay, ax) };
    let scale = travel * gate * COS_PI_8 / (COS_PI_8 * high + SIN_PI_8 * low);
    [byte(x * scale), byte(y * scale)]
}

/// A GameCube stick read through a host's standard mapping (-1..1 for the
/// byte's full range): one to one, no gate.
pub fn direct_stick(x: f32, y: f32) -> [i8; 2] {
    let axis = |v: f32| if v.is_finite() { byte(v * 128.0) } else { 0 };
    [axis(x), axis(y)]
}

/// A trigger's travel (0..1) as retail's analog bytes.
pub fn trigger_byte(travel: f32) -> u8 {
    if !travel.is_finite() {
        return 0;
    }
    (travel * TRIGGER_UNITS).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_lib::ControllerState;
    use melee_lib::Buttons;

    /// What the fighter reads: HSD's clamp and scale of the bytes.
    fn melee(bytes: [i8; 2]) -> (f32, f32) {
        let state = ControllerState::from_origin_adjusted(Buttons(0), bytes, [0, 0], [0, 0]);
        (state.stick.x, state.stick.y)
    }

    #[test]
    fn full_tilt_reaches_melees_rim_and_diagonal_notch() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let cases = [
            ((1.0, 0.0), [87, 0], (1.0, 0.0)),
            ((0.0, -1.0), [0, -87], (0.0, -1.0)),
            // The diagonal notch: Melee's (0.7, 0.7).
            ((d, d), [62, 62], (0.7, 0.7)),
            ((-d, d), [-62, 62], (-0.7, 0.7)),
            // Square-gated pads report (1, 1) at the corner: the same notch.
            ((1.0, 1.0), [62, 62], (0.7, 0.7)),
        ];
        for ((x, y), bytes, (mx, my)) in cases {
            let gated = gate_stick(x, y, 0.0, MAIN_GATE);
            assert_eq!(gated, bytes, "({x}, {y})");
            let (sx, sy) = melee(gated);
            assert!((sx - mx).abs() < 0.0125 && (sy - my).abs() < 0.0125, "({x}, {y}) -> ({sx}, {sy})");
        }
    }

    #[test]
    fn full_tilt_between_notches_still_reaches_the_80_unit_rim() {
        // The octagon's flat edge is cos(pi/8) * 87 = 80.4 units out, so a
        // full tilt in any direction clamps to HSD's circle.
        for step in 0..64 {
            let angle = step as f32 * std::f32::consts::TAU / 64.0;
            #[allow(clippy::disallowed_methods)]
            let (x, y) = (angle.cos(), angle.sin());
            let (sx, sy) = melee(gate_stick(x, y, 0.1, MAIN_GATE));
            #[allow(clippy::disallowed_methods)]
            let radius = (sx * sx + sy * sy).sqrt();
            assert!(radius > 0.97 && radius <= 1.0001, "angle {angle}: {radius}");
        }
    }

    #[test]
    fn the_deadzone_rests_at_zero_and_rescales_the_rest() {
        assert_eq!(gate_stick(0.09, -0.05, 0.12, MAIN_GATE), [0, 0]);
        assert_eq!(gate_stick(0.0, 0.0, 0.0, MAIN_GATE), [0, 0]);
        // Just past the deadzone starts near zero, not at the deadzone's value.
        assert_eq!(gate_stick(0.13, 0.0, 0.12, MAIN_GATE), [1, 0]);
        // A quarter of the remaining travel is a quarter of the notch.
        assert_eq!(gate_stick(0.34, 0.0, 0.12, MAIN_GATE), [22, 0]);
        assert_eq!(gate_stick(f32::NAN, 1.0, 0.0, MAIN_GATE), [0, 0]);
        assert_eq!(gate_stick(1.0, 0.0, 0.0, C_GATE), [74, 0]);
    }

    #[test]
    fn gamecube_sticks_and_triggers_map_one_to_one() {
        assert_eq!(direct_stick(0.5, -1.0), [64, -127]);
        assert_eq!(direct_stick(f32::INFINITY, 0.0), [0, 0]);
        assert_eq!(trigger_byte(1.0), 140);
        assert_eq!(trigger_byte(0.5), 70);
        assert_eq!(trigger_byte(-1.0), 0);
        assert_eq!(trigger_byte(f32::NAN), 0);
    }
}
