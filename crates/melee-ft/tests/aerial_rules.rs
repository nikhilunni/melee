mod input_support;
use melee_ft::{
    fighter::{attack::aerial, landing},
    input::{Buttons, FighterInput, Stick},
};
use melee_types::CommonMotionState as S;

#[test]
fn aerial_direction_respects_facing_and_strict_thresholds() {
    let common = input_support::common();
    let mut input = FighterInput::default();
    for (x, y, facing, expected) in [
        (0.0, 0.0, 1.0, S::AttackAirN),
        (1.0, 0.0, 1.0, S::AttackAirF),
        (1.0, 0.0, -1.0, S::AttackAirB),
        (-1.0, 0.0, -1.0, S::AttackAirF),
        (0.0, 1.0, 1.0, S::AttackAirHi),
        (0.0, -1.0, 1.0, S::AttackAirLw),
        (common.aerial_horizontal_threshold, 0.0, 1.0, S::AttackAirF),
    ] {
        input.current.stick = Stick { x, y };
        assert_eq!(aerial::select(&input, &common, facing), expected);
    }
}

#[test]
fn cstick_crossing_overrides_a_but_held_cstick_does_not_retrigger() {
    let common = input_support::common();
    let mut input = FighterInput::default();
    input.current.stick = Stick { x: 0.0, y: -1.0 };
    input.current.cstick = Stick { x: 1.0, y: 0.0 };
    input.pressed = Buttons::A;
    assert_eq!(aerial::select(&input, &common, 1.0), S::AttackAirF);
    input.previous.cstick = input.current.cstick;
    assert_eq!(aerial::select(&input, &common, 1.0), S::AttackAirLw);
    input.pressed = Buttons::default();
    assert!(!aerial::requested(&input, &common));
    input.current.cstick.y = -1.0;
    assert!(aerial::requested(&input, &common));
}

#[test]
fn lcancel_excludes_window_boundary_and_truncates_before_minimum() {
    let mut common = input_support::common();
    common.l_cancel_window = 7;
    common.l_cancel_divisor = 2.0;
    for (lag, age, expected) in [
        (15.0, 6, 7.0f32),
        (15.0, 7, 15.0),
        (1.0, 0, 1.0),
        (16.0, 0, 8.0),
    ] {
        assert_eq!(
            landing::cancelled_lag(lag, age, &common).to_bits(),
            expected.to_bits()
        );
    }
    common.l_cancel_window = 3;
    common.l_cancel_divisor = 3.0;
    assert_eq!(landing::cancelled_lag(17.0, 2, &common), 5.0);
    assert_eq!(landing::cancelled_lag(17.0, 3, &common), 17.0);
}
