use super::*;
use crate::input::Stick;
use melee_types::mp::line_flag;

fn fixed(fix: ControllerFix) -> FighterInput {
    FighterInput {
        hardware: HardwareInput {
            fix,
            ..HardwareInput::default()
        },
        ..FighterInput::default()
    }
}

fn turn(frame: f32, facing: f32, popo: bool) -> TurnFacts {
    // Before the natural flip the hook's facing is the turn's destination,
    // and the script frame is the animation's.
    TurnFacts {
        animation_frame: frame,
        script_frame: frame,
        facing_after: facing,
        facing,
        secondary: false,
        popo,
    }
}

/// Feed 0.84's buffer one raw main-stick sample (C-stick neutral).
fn sample(input: &mut FighterInput, stick: [i8; 2]) {
    input.hardware.raw = RawSticks {
        stick,
        cstick: [0, 0],
    };
    update_pad_buffer(input, true, false);
}

#[test]
fn names_round_trip() {
    for (fix, name) in ControllerFix::ALL {
        assert_eq!(ControllerFix::from_name(name), Some(fix));
        assert_eq!(fix.name(), name);
    }
    assert_eq!(ControllerFix::from_name("ucf"), None);
    assert!(ControllerFix::Dween.unsupported().is_none());
}

#[test]
fn smash_turn_needs_a_75_unit_raw_change_on_frame_two() {
    let facts = turn(2.0, -1.0, false);
    let mut input = fixed(ControllerFix::Ucf080);
    input.current.stick = Stick { x: -1.0, y: 0.0 };
    input.horizontal.tilt = 1;
    // (current, two ticks ago, smash turn)
    for (current, earlier, expected) in [
        (-80, 0, true),
        (-76, 0, true),
        (-75, 0, false),
        (-127, -50, true),
        (-80, -10, false),
    ] {
        input.hardware.queue_x = PadQueueX {
            current,
            two_ticks_ago: earlier,
        };
        assert_eq!(
            smash_turn(&input, 0.8, &facts).is_some(),
            expected,
            "{current} {earlier}"
        );
    }
    input.hardware.queue_x = PadQueueX {
        current: -80,
        two_ticks_ago: 0,
    };
    assert_eq!(smash_turn(&input, 0.8, &facts), Some(None));
    let popo = smash_turn(&input, 0.8, &turn(2.0, -1.0, true));
    assert_eq!(
        popo,
        Some(Some(PartnerTurn {
            facing: -1.0,
            stick_x: i8::MIN
        }))
    );
    assert_eq!(smash_turn(&input, 0.8, &turn(3.0, -1.0, false)), None);
    input.horizontal.tilt = 2;
    assert_eq!(smash_turn(&input, 0.8, &facts), None);
    input.horizontal.tilt = 1;
    input.hardware.fix = ControllerFix::Off;
    assert_eq!(smash_turn(&input, 0.8, &facts), None);
}

#[test]
fn ucf073_smash_turn_needs_the_stick_toward_the_turn() {
    // A slow turn to the left on its second frame, the raw stick 80 units
    // from where it was two ticks ago.
    let facts = turn(2.0, -1.0, false);
    // (stick x, raw now, raw two ticks ago, 0.73, 0.74)
    for (x, current, earlier, beta, release) in [
        // Toward the turn: both versions.
        (-1.0, -80, 0, true, true),
        (-0.8, -80, 0, true, true),
        // Back toward the old facing: only 0.74 takes the absolute value.
        (1.0, 80, 0, false, true),
        (0.8, 64, -20, false, true),
        // Short of the dash threshold, or a slow stick: neither.
        (-0.7875, -80, 0, false, false),
        (-1.0, -80, -5, false, false),
    ] {
        for (fix, expected) in [
            (ControllerFix::Ucf073, beta),
            (ControllerFix::Ucf074, release),
        ] {
            let mut input = fixed(fix);
            input.current.stick = Stick { x, y: 0.0 };
            input.horizontal.tilt = 1;
            input.hardware.queue_x = PadQueueX {
                current,
                two_ticks_ago: earlier,
            };
            assert_eq!(
                smash_turn(&input, 0.8, &facts).is_some(),
                expected,
                "{fix:?} {x} {current} {earlier}"
            );
        }
    }
    let mut input = fixed(ControllerFix::Ucf073);
    input.current.stick = Stick { x: -1.0, y: 0.0 };
    input.horizontal.tilt = 1;
    input.hardware.queue_x = PadQueueX {
        current: -80,
        two_ticks_ago: 0,
    };
    // Popo writes his partner's sample: left unless the facing is 1.0.
    for (facing, stick_x) in [(-1.0, i8::MIN), (1.0, 0x7F)] {
        input.current.stick.x = facing;
        assert_eq!(
            smash_turn(&input, 0.8, &turn(2.0, facing, true)),
            Some(Some(PartnerTurn { facing, stick_x }))
        );
    }
    input.current.stick.x = -1.0;
    // The frame test reads the script frame's high half: [2.0, 2.0078125).
    for (script_frame, expected) in [
        (2.0, true),
        (f32::from_bits(0x4000_FFFF), true),
        (f32::from_bits(0x4001_0000), false),
        (1.0, false),
        (3.0, false),
    ] {
        let facts = TurnFacts {
            script_frame,
            ..turn(2.0, -1.0, false)
        };
        assert_eq!(
            smash_turn(&input, 0.8, &facts).is_some(),
            expected,
            "{script_frame}"
        );
    }
    // Held two ticks, or Nana: no smash turn.
    input.horizontal.tilt = 2;
    assert_eq!(smash_turn(&input, 0.8, &facts), None);
    input.horizontal.tilt = 1;
    let nana = TurnFacts {
        secondary: true,
        ..turn(2.0, -1.0, false)
    };
    assert_eq!(smash_turn(&input, 0.8, &nana), None);
}

#[test]
fn ucf084_smash_turn_reads_its_own_buffer_and_the_new_facing() {
    let mut input = fixed(ControllerFix::Ucf084);
    for stick in [[0, 0], [-40, 0], [-80, 0]] {
        sample(&mut input, stick);
    }
    input.current.stick = Stick { x: -1.0, y: 0.0 };
    input.horizontal.tilt = 1;
    // Any kind names its second fighter; the stick follows the sign bit.
    let turned = smash_turn(&input, 0.8, &turn(2.0, -1.0, false));
    assert_eq!(
        turned,
        Some(Some(PartnerTurn {
            facing: -1.0,
            stick_x: i8::MIN
        }))
    );
    // The stick must point where the fighter now faces.
    assert_eq!(smash_turn(&input, 0.8, &turn(2.0, 1.0, false)), None);
    // The HSD queue is not read.
    input.hardware.queue_x = PadQueueX::default();
    assert!(smash_turn(&input, 0.8, &turn(2.0, -1.0, false)).is_some());
}

#[test]
fn shield_drop_blocks_rim_stick_held_sideways() {
    let facts = SpotDodgeFacts {
        escape_threshold: -0.7,
        walk_fast_threshold: 0.8,
        roll_window: 4,
        floor: SurfaceData {
            index: 3,
            flags: line_flag::PLATFORM,
            ..SurfaceData::default()
        },
    };
    for fix in [
        ControllerFix::Ucf073,
        ControllerFix::Ucf074,
        ControllerFix::Ucf084,
    ] {
        let mut input = fixed(fix);
        input.horizontal.tilt = 4;
        // A down-right notch on the rim: 0.7 each way reaches the circle.
        input.current.stick = Stick { x: 0.7, y: -0.7 };
        assert!(blocks_spot_dodge(&input, &facts), "{fix:?}");
        // Straight down past -0.8 spot dodges.
        input.current.stick = Stick { x: 0.3, y: -0.8125 };
        assert!(!blocks_spot_dodge(&input, &facts));
        // Inside the circle spot dodges.
        input.current.stick = Stick { x: 0.5, y: -0.7 };
        assert!(!blocks_spot_dodge(&input, &facts));
        // Freshly moved sideways spot dodges.
        input.current.stick = Stick { x: 0.7, y: -0.7 };
        input.horizontal.tilt = 3;
        assert!(!blocks_spot_dodge(&input, &facts));
        // The C-stick always spot dodges.
        input.horizontal.tilt = 4;
        input.current.cstick.y = -0.7;
        assert!(!blocks_spot_dodge(&input, &facts));
    }
    // 0.84 blocks only on a platform.
    let mut input = fixed(ControllerFix::Ucf084);
    input.horizontal.tilt = 4;
    input.current.stick = Stick { x: 0.7, y: -0.7 };
    let ground = SpotDodgeFacts {
        floor: SurfaceData::default(),
        ..facts
    };
    assert!(!blocks_spot_dodge(&input, &ground));
}

#[test]
fn tumble_wiggle_second_tick_needs_the_raw_change() {
    let mut input = fixed(ControllerFix::Ucf080);
    // Retail PlCo: threshold 0.8 (+0x210), window 1 (+0x214).
    for tilt in [0u8, 1, 2] {
        input.horizontal.tilt = tilt;
        input.hardware.fix = ControllerFix::Off;
        assert_eq!(tumble_wiggle(&input, 0.8, 1), tilt == 0);
        input.hardware.fix = ControllerFix::Ucf074;
        assert_eq!(tumble_wiggle(&input, 0.8, 1), tilt == 0);
    }
    input.hardware.fix = ControllerFix::Ucf080;
    input.horizontal.tilt = 0;
    assert!(tumble_wiggle(&input, 0.8, 1));
    input.horizontal.tilt = 2;
    assert!(!tumble_wiggle(&input, 0.8, 1));
    input.horizontal.tilt = 1;
    input.hardware.queue_x = PadQueueX {
        current: 80,
        two_ticks_ago: 0,
    };
    assert!(tumble_wiggle(&input, 0.8, 1));
    input.previous.stick.x = 0.8;
    assert!(!tumble_wiggle(&input, 0.8, 1));
    input.previous.stick.x = 0.5;
    input.hardware.queue_x.two_ticks_ago = 10;
    assert!(!tumble_wiggle(&input, 0.8, 1));
}

#[test]
fn ucf084_snaps_cardinals_to_exactly_one() {
    let mut input = fixed(ControllerFix::Ucf084);
    // (raw stick, processed before, processed after)
    for (raw, before, after) in [
        ([80, 6], (0.9875, 0.075), (1.0, 0.0)),
        ([-127, -6], (-0.9875, -0.0375), (-1.0, 0.0)),
        ([3, -90], (0.0375, -0.9875), (0.0, -1.0)),
        ([79, 0], (0.9875, 0.0), (0.9875, 0.0)),
        ([80, 7], (0.9875, 0.0875), (0.9875, 0.0875)),
    ] {
        input.current.stick = Stick {
            x: before.0,
            y: before.1,
        };
        input.hardware.raw = RawSticks {
            stick: raw,
            cstick: raw,
        };
        input.current.cstick = input.current.stick;
        update_pad_buffer(&mut input, true, false);
        let expected = Stick {
            x: after.0,
            y: after.1,
        };
        assert_eq!(input.current.stick, expected, "{raw:?}");
        assert_eq!(input.current.cstick, expected, "{raw:?}");
    }
    // CPU fighters and the exempt Zelda motion keep HSD's values.
    input.current.stick = Stick { x: 0.9875, y: 0.0 };
    input.hardware.raw.stick = [80, 0];
    update_pad_buffer(&mut input, false, false);
    update_pad_buffer(&mut input, true, true);
    assert_eq!(input.current.stick.x, 0.9875);
}

#[test]
fn ucf084_rim_count_starts_with_a_downward_flick_and_passes_the_drop() {
    let mut input = fixed(ControllerFix::Ucf084);
    let rim = Stick { x: 0.7, y: -0.7 };
    // Neutral, neutral, then a flick to the down-right rim (y 0 -> -56).
    sample(&mut input, [0, 0]);
    sample(&mut input, [0, 0]);
    input.current.stick = rim;
    input.vertical.tilt = 0;
    sample(&mut input, [56, -56]);
    assert!(!platform_drop_stick(&input, 0.8));
    input.vertical.tilt = 1;
    sample(&mut input, [56, -56]);
    assert!(platform_drop_stick(&input, 0.8));
    // Leaving the lower rim resets it; a slow slide never starts it.
    input.current.stick = Stick { x: 0.7, y: -0.5 };
    sample(&mut input, [56, -40]);
    assert!(!platform_drop_stick(&input, 0.8));
    input.current.stick = rim;
    input.vertical.tilt = 1;
    sample(&mut input, [56, -56]);
    sample(&mut input, [56, -56]);
    assert!(!platform_drop_stick(&input, 0.8));
    // Other versions keep retail's stick test.
    input.hardware.fix = ControllerFix::Ucf080;
    assert!(!platform_drop_stick(&input, 0.8));
    input.current.stick.y = -0.8;
    assert!(platform_drop_stick(&input, 0.8));
}

#[test]
fn ucf084_sdi_taps_on_a_62_unit_jump_from_the_center() {
    let mut input = fixed(ControllerFix::Ucf084);
    input.vertical.tilt = 5;
    // Fresh holds (the timers' defaults are 0xFE).
    input.horizontal.held = 0;
    input.vertical.held = 0;
    sample(&mut input, [0, 0]);
    sample(&mut input, [0, 0]);
    sample(&mut input, [45, 45]);
    // (45, 45) moved sqrt(4050) > 62 raw units from the center.
    assert!(sdi_vertical_tap(&input, 3, 0.7));
    // The previous tick's stick must lie inside the SDI radius.
    input.previous.stick = Stick { x: 0.7, y: 0.0 };
    assert!(!sdi_vertical_tap(&input, 3, 0.7));
    input.previous.stick = Stick::default();
    // Both axes held past a tick fail.
    input.horizontal.held = 2;
    input.vertical.held = 2;
    assert!(!sdi_vertical_tap(&input, 3, 0.7));
    input.vertical.tilt = 2;
    assert!(sdi_vertical_tap(&input, 3, 0.7));
    input.hardware.fix = ControllerFix::Ucf080;
    input.vertical.tilt = 5;
    input.horizontal.held = 0;
    assert!(!sdi_vertical_tap(&input, 3, 0.7));
}

#[test]
fn ucf084_shield_sdi_reads_the_signed_previous_x() {
    let mut input = fixed(ControllerFix::Ucf084);
    input.horizontal.tilt = 5;
    input.horizontal.held = 1;
    sample(&mut input, [0, 0]);
    sample(&mut input, [0, 0]);
    sample(&mut input, [-70, 0]);
    assert!(shield_sdi_tap(&input, 3, 0.7));
    // Last tick far left still counts: the compare is signed.
    input.previous.stick.x = -0.9;
    assert!(shield_sdi_tap(&input, 3, 0.7));
    input.previous.stick.x = 0.7;
    assert!(!shield_sdi_tap(&input, 3, 0.7));
}

#[test]
fn ucf084_squat_release_drops_to_059_on_a_fresh_rim_tap() {
    let mut input = fixed(ControllerFix::Ucf084);
    input.current.stick = Stick { x: 0.7, y: -0.7 };
    input.horizontal.tilt = 0;
    assert_eq!(squat_release_threshold(&input, 0.6875), 0.59);
    input.horizontal.tilt = 1;
    assert_eq!(squat_release_threshold(&input, 0.6875), 0.6875);
    input.horizontal.tilt = 0;
    input.current.stick = Stick { x: 0.5, y: -0.5 };
    assert_eq!(squat_release_threshold(&input, 0.6875), 0.6875);
}
