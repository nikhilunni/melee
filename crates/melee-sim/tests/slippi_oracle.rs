//! Local oracle evidence for the two lossy phase/input boundaries.
use melee_ft::input::PadSample;
use melee_sim::inputs::{replay_pad, PadScript};
use serde_json::Value as Json;
use slp::cold::ControllerFrame;
use std::{io::BufRead, path::PathBuf};

fn traces() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/traces")
}
fn lines(path: &std::path::Path) -> Vec<Json> {
    melee_test_support::trace::open(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect()
}
fn word(bytes: &str, offset: usize) -> u32 {
    u32::from_str_radix(&bytes[offset * 2..offset * 2 + 8], 16).unwrap()
}
fn pad_bits(p: PadSample) -> [u32; 7] {
    [
        p.buttons.0,
        p.stick.x.to_bits(),
        p.stick.y.to_bits(),
        p.cstick.x.to_bits(),
        p.cstick.y.to_bits(),
        p.left_trigger.to_bits(),
        p.right_trigger.to_bits(),
    ]
}

#[test]
fn slippi_style_fields_recover_all_1800_recorded_movement_pads() {
    let mut compared = 0;
    for scene in ["walk_fd_fox", "shield_fd_fox", "jump_fd_fox"] {
        let path = traces().join(format!("{scene}.tick.expected.jsonl"));
        let ledger = traces().join(format!("{scene}.ledger.raw.jsonl"));
        if !melee_test_support::require_files([&path, &ledger]) {
            return;
        }
        let script = PadScript::from_expected_trace(&path, true).unwrap();
        let recorded = lines(&ledger);
        assert_eq!(recorded.len(), script.len());
        for (tick, raw) in recorded.iter().enumerate() {
            for port in 0..2 {
                let actual = script.sample(tick as u64, port);
                let bytes = raw["fighters"][port]["bytes"].as_str().unwrap();
                let float = |offset| f32::from_bits(word(bytes, offset));
                // These are the exact Fighter offsets the Slippi recording
                // hook reads. Only the independent GameStatus pad supplies
                // physical buttons/L/R here: MasterStatus was not captured.
                let input = ControllerFrame {
                    frame: tick as u64,
                    port: port as u8,
                    buttons_physical: actual.buttons.0 as u16,
                    buttons_processed: word(bytes, 0x65c),
                    stick: [float(0x620), float(0x624)],
                    cstick: [float(0x638), float(0x63c)],
                    trigger: float(0x650),
                    triggers: [actual.left_trigger, actual.right_trigger],
                    raw_stick: None,
                    raw_cstick: None,
                    raw_stick_x: None,
                };
                assert_eq!(
                    pad_bits(replay_pad(&input).unwrap()),
                    pad_bits(actual),
                    "{scene} tick {tick} port {port}"
                );
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 1800);
}

#[test]
fn ledger_links_tick_end_to_next_scheduler_start_even_with_repeated_vi_frames() {
    let path = traces().join("start_fd_fox.ledger600.raw.jsonl");
    let expected = traces().join("start_fd_fox.tick.expected.jsonl");
    if !melee_test_support::require_files([&path, &expected]) {
        return;
    }
    let rows = lines(&path);
    let expected = lines(&expected);
    assert_eq!(rows.len(), 600);
    assert_eq!(expected.len(), 600);
    let sidecar: serde_json::Value = serde_json::from_slice(
        &std::fs::read(traces().join("../roms/start_fd_fox.sav.json")).unwrap(),
    )
    .unwrap();
    let mut seed = sidecar["seed"].as_u64().unwrap() as u32;
    for tick in 0..rows.len() {
        assert_eq!(rows[tick]["frame"].as_u64(), Some(tick as u64));
        if tick > 0 {
            assert!(
                rows[tick]["vi_frame"].as_u64().unwrap()
                    >= rows[tick - 1]["vi_frame"].as_u64().unwrap(),
                "VI ordinals may repeat but cannot go backwards"
            );
        }
        for draw in rows[tick]["rng_draws"].as_array().unwrap() {
            seed = seed.wrapping_mul(214013).wrapping_add(2531011);
            assert_eq!(
                u64::from(seed),
                draw["seed"].as_u64().unwrap(),
                "tick {tick}"
            );
            // Tick zero includes match-start music selection (grLib_801C26B0);
            // the remaining callers are animation, stage and particle work.
            let lr = draw["lr"].as_u64().unwrap();
            assert!(
                matches!(
                    lr,
                    0x801c26b0
                        | 0x8008a8c0
                        | 0x8009fce0
                        | 0x8009fd04
                        | 0x8009fd28
                        | 0x8021aecc
                        | 0x8021b000
                ) || (0x8039930c..0x8039f258).contains(&lr),
                "unknown RNG caller {lr:#x}"
            );
        }
        assert_eq!(u64::from(seed), rows[tick]["seed"].as_u64().unwrap());
        assert_eq!(rows[tick]["seed"], expected[tick]["state"]["rng.seed"]["v"]);
    }
}

#[test]
fn physical_bits_alone_and_dead_zoned_sticks_are_not_full_pad_copies() {
    let input = ControllerFrame {
        frame: 0,
        port: 0,
        buttons_physical: 0,
        buttons_processed: 0x80000,
        stick: [0.55, 0.0],
        cstick: [0.0, 0.0],
        trigger: 0.0,
        triggers: [0.0, 0.0],
        raw_stick: None,
        raw_cstick: None,
        raw_stick_x: None,
    };
    assert_eq!(replay_pad(&input).unwrap().buttons.0, 0x80000);
    let mut dead_zoned = input.clone();
    dead_zoned.stick = [0.0, 0.0];
    dead_zoned.buttons_processed = 0;
    let mut raw_available = dead_zoned.clone();
    raw_available.raw_stick = Some([1, 0]);
    assert_ne!(
        pad_bits(replay_pad(&dead_zoned).unwrap()),
        pad_bits(replay_pad(&raw_available).unwrap())
    );
    let mut corrupt = input;
    corrupt.triggers[0] = f32::MAX;
    assert!(replay_pad(&corrupt).is_err());
}
