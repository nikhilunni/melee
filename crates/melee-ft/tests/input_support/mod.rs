#![allow(dead_code)]
use melee_ft::{desc::common::IdleInputAttributes, input::*};
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// NTSC 1.02 PlCo values, common block +9FC0, fields cited by InputCommonData.
/// Synthetic tests remain runnable without the disc; the trace test reads PlCo.
pub fn common() -> InputCommonData {
    InputCommonData {
        thresholds: IdleInputAttributes {
            horizontal_stick_deadzone: 0.28,
            vertical_stick_deadzone: 0.28,
            horizontal_stick_smash_deadzone: 0.25,
            vertical_stick_smash_deadzone: 0.25,
            analog_shoulder_deadzone: 0.3,
            z_press_analog_value: 0.35,
            shield_press_threshold: 0.25,
            walk_stick_threshold: 0.18,
            turn_stick_threshold: -0.25,
            dash_smash_stick_threshold: 0.8,
            dash_smash_window: 2,
            tap_jump_threshold: 0.6625,
            tap_jump_window: 4,
            squat_stick_threshold: 0.6875,
        },
        tilt_angle: f32::from_bits(0x3F5F66F3),
        side_tilt_threshold: 0.25,
        up_tilt_threshold: 0.25,
        down_tilt_threshold: -0.25,
        up_smash_threshold: 0.6625,
        up_smash_window: 4.0,
        down_smash_threshold: -0.6625,
        down_smash_window: 4.0,
        special_side_threshold: 0.6,
        special_vertical_threshold: 0.55,
        powershield_window: 2,
        escape_threshold: -0.7,
        escape_window: 4,
        activity_stick_threshold: 0.5,
        activity_trigger_threshold: 0.5,
        activity_window: 4.0,
    }
}

/// Snapshot encoding for the full input region. Field addresses come from
/// Fighter types.h / fighter.generated.yaml, not Rust's internal layout.
pub fn input_bytes(i: &FighterInput) -> Vec<u8> {
    let mut out = Vec::new();
    for stick in [
        i.current.stick,
        i.previous.stick,
        i.saved.stick,
        i.current.cstick,
        i.previous.cstick,
        i.saved.cstick,
    ] {
        out.extend(stick.x.to_bits().to_be_bytes());
        out.extend(stick.y.to_bits().to_be_bytes());
    }
    for trigger in [i.current.trigger, i.previous.trigger, i.saved.trigger] {
        out.extend(trigger.to_bits().to_be_bytes());
    }
    for buttons in [
        i.current.held,
        i.previous.held,
        i.saved.held,
        i.pressed,
        i.released,
    ] {
        out.extend(buttons.0.to_be_bytes());
    }
    for select in [
        |t: AnalogTimers| t.tilt,
        |t: AnalogTimers| t.held,
        |t: AnalogTimers| t.since_crossing,
        |t: AnalogTimers| t.activity,
    ] {
        for timers in [i.horizontal, i.vertical, i.shoulder] {
            out.push(select(timers));
        }
    }
    let b = &i.buttons;
    out.extend([
        b.attack,
        b.special,
        b.jump_button,
        b.shield,
        b.digital_shield,
        b.taunt,
        b.down,
        b.previous_attack,
        b.previous_digital_shield,
        b.jump,
        b.special_up,
        b.special_down,
        b.special_side,
        b.special_neutral,
        b.previous_jump,
        b.previous_special_up,
    ]);
    assert_eq!(out.len(), 0x68c - 0x620);
    out
}
