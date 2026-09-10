mod input_support;
use hsd_archive::Archive;
use input_support::{input_bytes, root};
use melee_ft::input::*;
use melee_types::PlayerKind;
use std::process::Command;

#[test]
fn human_idle_input_600() {
    let root = root();
    for file in [
        "harness/traces/idle_fd_fox.tick.raw.jsonl",
        "harness/traces/idle_fd_fox.tick.expected.jsonl",
        "harness/roms/files/PlCo.dat",
    ] {
        if !melee_test_support::require_files([&root.join(file)]) {
            return;
        }
    }
    let archive =
        Archive::parse(&std::fs::read(root.join("harness/roms/files/PlCo.dat")).unwrap()).unwrap();
    let common = InputCommonData::read(&archive).unwrap();
    let decoded = Command::new("python3")
        .arg(root.join("crates/melee-ft/tests/ref/input/decode_idle.py"))
        .arg(&root)
        .output()
        .expect("python3 trace decoder");
    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    const INPUT_LEN: usize = 0x68c - 0x620;
    const RECORD_LEN: usize = INPUT_LEN + 7 * 8 + 6 + 16;
    assert_eq!(decoded.stdout.len(), 600 * 2 * RECORD_LEN);
    let mut inputs = [FighterInput::default(), FighterInput::default()];
    let mut cpu = [[0_i64; 7]; 2];
    let sample = PadSample::from_origin_adjusted(Buttons(0), [0; 2], [0; 2], [0; 2]);
    assert_eq!(sample, PadSample::default());
    for (ordinal, record) in decoded.stdout.chunks_exact(RECORD_LEN).enumerate() {
        let (tick, player) = (ordinal / 2, ordinal % 2);
        let input = &mut inputs[player];
        let recorded_cpu: Vec<_> = record[INPUT_LEN..INPUT_LEN + 56]
            .chunks_exact(8)
            .map(|word| i64::from_be_bytes(word.try_into().unwrap()))
            .collect();
        let flags = &record[INPUT_LEN + 56..INPUT_LEN + 62];
        if tick == 0 {
            cpu[player].copy_from_slice(&recorded_cpu);
            input.use_current_history = flags[1] & 0x10 != 0;
            input.last_horizontal_positive = flags[4] & 1 != 0;
            input.last_vertical_negative = flags[5] & 0x80 != 0;
        }
        // These are live gate checks; any newly reachable hook fails the test.
        assert_eq!(flags[0] & 4, 0, "hitlag at {tick}/{player}"); // x2219_b5
        assert_eq!(flags[1] & 8, 0, "save/reset at {tick}/{player}"); // x221D_b4
        assert_eq!(flags[2] & 0x18, 0, "disabled/secondary at {tick}/{player}");
        assert_eq!(flags[3] & 0x20, 0, "freeze_sample at {tick}/{player}");
        assert!(
            record[INPUT_LEN + 62..].iter().all(|b| *b == 0),
            "capture/charge/item/jab gate at {tick}/{player}"
        );
        let source = input_source(
            resolve_player_kind(PlayerKind::Human, false, true),
            cpu[player][3] as i32,
        );
        assert_eq!(source, InputSource::Pad);
        run_cpu_input_proc(false, source); // would panic if AI were called; no RNG capability
        let effects = update_input(input, source, &sample, &common, InputContext::default());
        assert_eq!(
            effects,
            InputEffects {
                joystick_count_increments: 0,
                run_input_callback: true
            }
        );
        assert_eq!(
            wait_iasa(
                input,
                &common,
                &WaitContext {
                    facing: if player == 0 { 1.0 } else { -1.0 },
                    ..WaitContext::default()
                }
            ),
            WaitTransition::None,
            "tick {tick} p{player}"
        );
        assert_eq!(
            input_bytes(input),
            record[..INPUT_LEN],
            "input bytes: tick {tick} p{player}"
        );
        assert_eq!(
            cpu[player].as_slice(),
            recorded_cpu,
            "all seven CPU fields: tick {tick} p{player}"
        );
        assert_eq!(input.use_current_history, flags[1] & 0x10 != 0);
        assert_eq!(input.last_horizontal_positive, flags[4] & 1 != 0);
        assert_eq!(input.last_vertical_negative, flags[5] & 0x80 != 0);
    }
}
