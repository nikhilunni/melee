//! Raw scratch oracle supplements the canonical 49-key scene gates.
//! Only initial state and recorded pads drive the simulation; ledger bytes are assertions.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::{fall::FallState, Fighter, MotionData};
use melee_sim::inputs::PadScript;
use melee_types::CommonMotionState as S;
use serde_json::Value;
use std::{fs, path::Path};

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn check_fall(fall: &FallState, bytes: &[u8]) {
    assert_eq!(
        fall.family.motion(fall.pose) as u32,
        word(bytes, 0x2340),
        "secondary submotion"
    );
    assert_eq!(fall.blend.to_bits(), word(bytes, 0x2344), "fall blend bits");
}
fn compare(f: &Fighter, bytes: &[u8]) -> usize {
    match (&f.state_data, f.motion_state.id) {
        (MotionData::Fall(fall), S::Fall | S::FallAerial) => check_fall(fall, bytes),
        (MotionData::FallSpecial(fall), S::FallSpecial) => {
            check_fall(&fall.animation, bytes);
            assert_eq!(fall.mobility.to_bits(), word(bytes, 0x2348));
            assert_eq!(u32::from(fall.ordinary_gravity), word(bytes, 0x234C));
            assert_eq!(u32::from(fall.force_landing_lag), word(bytes, 0x2350));
            assert_eq!(u32::from(fall.allow_interrupt), word(bytes, 0x2358));
            assert_eq!(fall.landing_lag.to_bits(), word(bytes, 0x2354));
        }
        _ => return 0,
    }
    assert_eq!(f.animation.motion_id as u32, word(bytes, 0x14));
    assert_eq!(f.animation.speed.to_bits(), word(bytes, 0x89C));
    assert_eq!(f.commands.frame.to_bits(), word(bytes, 0x3E8));
    assert_eq!(f.physics.jumps_used, bytes[0x1968]);
    1
}

#[test]
fn falls_match_retail_scratch_and_command_clocks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "ledge_fd_marth",
        "ledgeclimb_fd_marth",
        "ledgeescape_fd_marth",
        "airjumpb_fd_fox",
        "airjumpb_fd_marth",
        "airdodge_fd_marth",
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let ledger_path = scenario.trace_path("ledger.raw.jsonl");
        if !melee_test_support::require_files(
            scenario
                .required_files()
                .into_iter()
                .chain([ledger_path.clone()]),
        ) {
            continue;
        }
        let pads =
            PadScript::from_expected_trace(&scenario.trace_path("tick.expected.jsonl"), true)
                .unwrap();
        let mut simulation = super::TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let ledger = fs::read_to_string(ledger_path).unwrap();
        assert_eq!(ledger.lines().count(), scenario.frames as usize);
        let mut fall_ticks = 0;
        for (tick, line) in ledger.lines().enumerate() {
            let row: Value = serde_json::from_str(line).unwrap();
            simulation.tick().unwrap();
            let bytes: Vec<u8> = row["fighters"][0]["bytes"]
                .as_str()
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
                .collect();
            let runtime = &simulation.runtime;
            assert_eq!(bytes[12], 0, "P0 ledger ordering {name} tick {tick}");
            fall_ticks +=
                crate::scene_fighter::with_fighter!(&runtime.state.fighters[0], |f| compare(
                    f, &bytes
                ));
        }
        assert!(fall_ticks > 0, "{name} must exercise falls");
        eprintln!("{name}: {fall_ticks} fall ticks, scratch and command clocks exact");
    }
}
