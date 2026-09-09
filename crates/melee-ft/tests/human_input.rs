mod input_support;
use input_support::{common, input_bytes};
use melee_ft::input::{human::apply_deadzone, pad::normalize_stick, *};
use melee_types::PlayerKind;

fn tick(i: &mut FighterInput, pad: PadSample) -> InputEffects {
    update_human_input(i, &pad, &common(), InputContext::default())
}
fn pad(buttons: Buttons, x: f32, y: f32) -> PadSample {
    PadSample {
        buttons,
        stick: Stick { x, y },
        ..PadSample::default()
    }
}

#[test]
fn stick_and_trigger_thresholds_preserve_retail_rounding() {
    let c = common();
    for threshold in [
        c.thresholds.horizontal_stick_deadzone,
        c.thresholds.vertical_stick_deadzone,
    ] {
        let below = f32::from_bits(threshold.to_bits() - 1);
        let above = f32::from_bits(threshold.to_bits() + 1);
        for sign in [-1.0, 1.0] {
            for value in [0.0, below, threshold] {
                assert_eq!(apply_deadzone(sign * value, threshold).to_bits(), 0);
            }
            assert_eq!(
                apply_deadzone(sign * above, threshold).to_bits(),
                (sign * above).to_bits()
            );
        }
    }
    // HSD controller.c normalizes by 80; Fighter deadzone is PlCo +0/+4=.28.
    for (raw, expected) in [
        (0, 0.0_f32),
        (22, 0.0),
        (23, 0.2875),
        (80, 1.0),
        (127, 1.0),
        (-128, -1.0),
    ] {
        let mut i = FighterInput::default();
        tick(
            &mut i,
            PadSample::from_origin_adjusted(Buttons(0), [raw, 0], [0, 0], [0, 0]),
        );
        assert_eq!(i.current.stick.x.to_bits(), expected.to_bits());
    }
    assert_eq!(normalize_stick(80, 80), Stick { x: 0.7, y: 0.7 }); // truncate 56.568 -> 56
    let threshold = c.thresholds.analog_shoulder_deadzone;
    for (value, survives) in [
        (f32::from_bits(threshold.to_bits() - 1), false),
        (threshold, false),
        (f32::from_bits(threshold.to_bits() + 1), true),
    ] {
        let mut i = FighterInput::default();
        tick(
            &mut i,
            PadSample {
                left_trigger: value,
                ..PadSample::default()
            },
        );
        assert_eq!(
            i.current.trigger.to_bits(),
            if survives { value.to_bits() } else { 0 }
        );
        assert_eq!(i.current.held.intersects(Buttons::SHIELD), survives);
    }
    for raw in [0, 42, 43, 140, 255] {
        let sample = PadSample::from_origin_adjusted(Buttons(0), [0; 2], [0; 2], [raw, 0]);
        assert_eq!(
            sample.left_trigger.to_bits(),
            (f32::from(raw.min(140)) / 140.0).to_bits()
        );
    }
}

#[test]
fn buttons_detect_edges_across_ticks_and_preserve_previous_press_ages() {
    let mut i = FighterInput::default();
    for (buttons, pressed, released, age) in [
        (Buttons::A, Buttons::A, Buttons(0), 0),
        (Buttons::A, Buttons(0), Buttons(0), 1),
        (Buttons(0), Buttons(0), Buttons::A, 2),
        (Buttons::B, Buttons::B, Buttons(0), 3),
        (Buttons::A | Buttons::B, Buttons::A, Buttons(0), 0),
    ] {
        tick(&mut i, pad(buttons, 0.0, 0.0));
        assert_eq!(
            (i.pressed, i.released, i.buttons.attack),
            (pressed, released, age)
        );
    }
    assert_eq!(i.buttons.previous_attack, 3);
    tick(&mut i, pad(Buttons::L, 0.0, 0.0));
    assert_eq!(i.current.held, Buttons::L | Buttons::SHIELD);
    assert_eq!(i.current.trigger.to_bits(), 1.0_f32.to_bits());
    assert_eq!(i.buttons.previous_digital_shield, 255);
    tick(&mut i, pad(Buttons::L, 0.0, 0.0));
    tick(&mut i, pad(Buttons::R, 0.0, 0.0));
    assert_eq!(i.buttons.previous_digital_shield, 1);
    assert_eq!(i.pressed, Buttons::R);
    assert_eq!(i.released, Buttons::L);
}

#[test]
fn z_macro_overrides_digital_shoulder_and_mode_masks_are_in_order() {
    let mut i = FighterInput::default();
    let p = PadSample {
        buttons: Buttons::Z | Buttons::L,
        left_trigger: 0.9,
        cstick: Stick { x: 1.0, y: 0.0 },
        ..PadSample::default()
    };
    tick(&mut i, p);
    assert_eq!(
        i.current.held,
        Buttons::Z | Buttons::L | Buttons::A | Buttons::SHIELD
    );
    assert_eq!(
        i.current.trigger.to_bits(),
        common().thresholds.z_press_analog_value.to_bits()
    );
    update_human_input(
        &mut i,
        &p,
        &common(),
        InputContext {
            suppress_z_macro: true,
            ..InputContext::default()
        },
    );
    assert_eq!(i.current.trigger, 1.0);
    assert!(!i.current.held.intersects(Buttons::A));
    update_human_input(
        &mut i,
        &p,
        &common(),
        InputContext {
            single_button_mode: true,
            suppress_cstick: true,
            ..InputContext::default()
        },
    );
    assert_eq!(i.current.held, Buttons(0));
    assert_eq!(i.current.trigger.to_bits(), 0);
    assert_eq!(i.current.cstick, Stick::default());
}

#[test]
fn hitlag_accumulates_edges_but_defers_action_counters_and_callback() {
    let mut i = FighterInput::default();
    let hitlag = InputContext {
        hitlag: true,
        ..InputContext::default()
    };
    for buttons in [Buttons::A, Buttons(0), Buttons::B] {
        let effect = update_human_input(&mut i, &pad(buttons, 0.0, 0.0), &common(), hitlag);
        assert_eq!(effect, InputEffects::default());
    }
    assert_eq!(i.pressed, Buttons::A | Buttons::B);
    assert_eq!(i.released, Buttons::A);
    assert_eq!(i.buttons.attack, 0); // accumulated A resets it again each hitlag tick
    assert_eq!(i.buttons.special_neutral, 255); // post-input processing skipped
    assert!(tick(&mut i, pad(Buttons::B, 0.0, 0.0)).run_input_callback);
    assert_eq!(i.pressed, Buttons(0));
    assert_eq!(i.buttons.special_neutral, 255);
}

#[test]
fn saved_history_prevents_false_edges_when_input_is_reenabled() {
    let mut i = FighterInput::default();
    let p = pad(Buttons::A, 0.9, 0.0);
    update_human_input(
        &mut i,
        &p,
        &common(),
        InputContext {
            save_and_clear: true,
            ..InputContext::default()
        },
    );
    assert_eq!(i.saved.held, Buttons::A);
    assert_eq!(i.current, InputFrame::default());
    assert!(!i.use_current_history);
    tick(&mut i, p);
    assert_eq!(i.previous, i.saved);
    assert_eq!(i.pressed, Buttons(0));
    let before = i.clone();
    let effect = update_human_input(
        &mut i,
        &PadSample::default(),
        &common(),
        InputContext {
            disabled: true,
            captured: true,
            ..InputContext::default()
        },
    );
    assert_eq!(i, before);
    assert_eq!(effect, InputEffects::default());
    update_human_input(
        &mut i,
        &PadSample::default(),
        &common(),
        InputContext {
            freeze_sample: true,
            ..InputContext::default()
        },
    );
    assert_eq!(i.saved.held, Buttons::A); // frozen pad was never read
    assert_eq!(i.current, InputFrame::default());
    assert_eq!(
        input_bytes(&i)[0x50..],
        input_bytes(&FighterInput::default())[0x50..]
    );
}

#[test]
fn analog_timers_wrap_before_clamping_and_activity_consumes_crossings() {
    let mut i = FighterInput::default();
    i.horizontal.since_crossing = 255;
    tick(&mut i, PadSample::default());
    assert_eq!(i.horizontal.since_crossing, 0); // stb precedes cmplwi in retail
    for _ in 0..300 {
        tick(&mut i, PadSample::default());
    }
    assert_eq!(i.horizontal.since_crossing, 254);
    assert_eq!(i.buttons.attack, 255);
    let effect = tick(&mut i, pad(Buttons(0), 0.6, 0.0));
    assert_eq!(i.horizontal.tilt, 0);
    assert!(i.last_horizontal_positive);
    assert_eq!(effect.joystick_count_increments, 1);
    assert_eq!(i.horizontal.activity, 254); // ftCommon_8008031C consumes it
    assert_eq!(
        tick(&mut i, pad(Buttons(0), 0.6, 0.0)).joystick_count_increments,
        0
    );
    tick(&mut i, pad(Buttons(0), -0.6, 0.0));
    assert!(!i.last_horizontal_positive);
    assert_eq!(i.horizontal.tilt, 0);
    tick(&mut i, pad(Buttons(0), 0.0, -0.6));
    assert!(i.last_vertical_negative);
    // Both activity events are retained, in retail's stick-then-trigger order.
    tick(&mut i, PadSample::default());
    let effect = tick(
        &mut i,
        PadSample {
            left_trigger: 0.9,
            ..pad(Buttons(0), 0.6, 0.0)
        },
    );
    assert_eq!(effect.joystick_count_increments, 2);
}

#[test]
fn human_slot_cpu_mode_does_not_enable_ai_and_override_mode_reads_pad() {
    for kind in [
        PlayerKind::Human,
        PlayerKind::Demo,
        PlayerKind::Na,
        PlayerKind::Boss,
    ] {
        for mode in [0, 4, 5, 6, 29] {
            assert_eq!(input_source(kind, mode), InputSource::Pad);
            run_cpu_input_proc(false, input_source(kind, mode));
        }
    }
    assert_eq!(input_source(PlayerKind::Cpu, 5), InputSource::Pad);
    assert_eq!(
        input_source(PlayerKind::Cpu, 4),
        InputSource::CpuUnimplemented
    );
    run_cpu_input_proc(true, InputSource::CpuUnimplemented);
    assert_eq!(
        resolve_player_kind(PlayerKind::Human, true, true),
        PlayerKind::Cpu
    );
    assert_eq!(
        resolve_player_kind(PlayerKind::Human, false, true),
        PlayerKind::Human
    );
    assert_eq!(
        resolve_player_kind(PlayerKind::Human, true, false),
        PlayerKind::Human
    );
}

#[test]
#[should_panic(expected = "melee-cpu, M5")]
fn cpu_path_is_explicitly_unimplemented() {
    run_cpu_input_proc(false, InputSource::CpuUnimplemented);
}

#[test]
fn wait_iasa_order_table_uses_real_input_predicates() {
    use WaitPredicate as P;
    use WaitTransition as T;
    let cases = [
        ("neutral", pad(Buttons(0), 0.0, 0.0), T::None, P::Walk),
        (
            "side before up special",
            pad(Buttons::A | Buttons::B, 0.8, 0.8),
            T::Special,
            P::SpecialSide,
        ),
        (
            "up special",
            pad(Buttons::B, 0.0, 0.8),
            T::Special,
            P::SpecialUp,
        ),
        (
            "neutral special before grab",
            pad(Buttons::B | Buttons::Z, 0.0, 0.0),
            T::Special,
            P::SpecialNeutral,
        ),
        (
            "down special",
            pad(Buttons::B, 0.0, -0.8),
            T::Special,
            P::SpecialDown,
        ),
        (
            "grab before smash",
            pad(Buttons::Z, 1.0, 0.0),
            T::Grab,
            P::Grab,
        ),
        (
            "side smash before up smash",
            pad(Buttons::A, 0.9, 0.9),
            T::Attack,
            P::SmashSide,
        ),
        (
            "up smash before jump",
            pad(Buttons::A, 0.0, 0.9),
            T::Attack,
            P::SmashUp,
        ),
        (
            "down smash before squat",
            pad(Buttons::A, 0.0, -0.9),
            T::Attack,
            P::SmashDown,
        ),
        (
            "side tilt",
            pad(Buttons::A, 0.4, 0.0),
            T::Attack,
            P::TiltSide,
        ),
        ("up tilt", pad(Buttons::A, 0.0, 0.4), T::Attack, P::TiltUp),
        (
            "down tilt",
            pad(Buttons::A, 0.0, -0.4),
            T::Attack,
            P::TiltDown,
        ),
        (
            "jab before taunt",
            pad(Buttons::A | Buttons::UP, 0.0, 0.0),
            T::Attack,
            P::Jab,
        ),
        (
            "escape before shield",
            pad(Buttons::L, 0.0, -0.9),
            T::Escape,
            P::Escape,
        ),
        (
            "shield before jump",
            pad(Buttons::L | Buttons::X, 0.0, 0.0),
            T::Shield,
            P::Shield,
        ),
        (
            "taunt before jump",
            pad(Buttons::UP | Buttons::X, 0.0, 0.0),
            T::Taunt,
            P::Taunt,
        ),
        (
            "jump before dash",
            pad(Buttons::X, 0.9, 0.0),
            T::Jump,
            P::Jump,
        ),
        (
            "dash before squat",
            pad(Buttons(0), 0.9, -0.9),
            T::Dash,
            P::Dash,
        ),
        (
            "squat before turn",
            pad(Buttons(0), -0.4, -0.9),
            T::Squat,
            P::Squat,
        ),
        ("turn", pad(Buttons(0), -0.4, 0.0), T::Turn, P::Turn),
        ("walk", pad(Buttons(0), 0.4, 0.0), T::Walk, P::Walk),
        (
            "back smash enters turn",
            pad(Buttons(0), -0.9, 0.0),
            T::Turn,
            P::Dash,
        ),
    ];
    for (name, sample, want, last) in cases {
        let mut input = FighterInput::default();
        tick(&mut input, sample);
        let mut visited = Vec::new();
        let result = wait_iasa_observe(&input, &common(), &WaitContext::default(), |p| {
            visited.push(p)
        });
        assert_eq!(result, want, "{name}");
        assert_eq!(visited.last(), Some(&last), "{name}");
        assert_eq!(
            visited,
            WAIT_PREDICATES[..visited.len()],
            "must stop at the first matching predicate: {name}"
        );
    }
    let mut i = FighterInput::default();
    tick(
        &mut i,
        PadSample {
            cstick: Stick { x: 1.0, y: 0.0 },
            ..PadSample::default()
        },
    );
    assert_eq!(wait_iasa(&i, &common(), &WaitContext::default()), T::Attack);
    tick(
        &mut i,
        PadSample {
            cstick: Stick { x: 1.0, y: 0.0 },
            ..PadSample::default()
        },
    );
    assert_eq!(wait_iasa(&i, &common(), &WaitContext::default()), T::None);
    tick(&mut i, pad(Buttons::DOWN, 0.0, 0.0));
    tick(&mut i, pad(Buttons::UP, 0.0, 0.0));
    let mut visited = Vec::new();
    let ctx = WaitContext {
        fox_taunt_available: true,
        ..WaitContext::default()
    };
    assert_eq!(
        wait_iasa_observe(&i, &common(), &ctx, |p| visited.push(p)),
        T::Taunt
    );
    assert_eq!(visited.last(), Some(&P::FoxTaunt));
}

#[test]
#[should_panic(expected = "capture hook")]
fn nonidle_capture_hook_is_not_silently_skipped() {
    update_human_input(
        &mut FighterInput::default(),
        &PadSample::default(),
        &common(),
        InputContext {
            captured: true,
            ..InputContext::default()
        },
    );
}
