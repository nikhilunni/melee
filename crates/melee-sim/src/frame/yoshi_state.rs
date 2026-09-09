//! Extra state evidence for the hooks not covered by the 49-key trace schema.
use super::*;
use crate::{scenario::Scenario, scene_fighter::SceneFighter};
use melee_ft::fighter::MotionData;
use std::{fs, path::Path};

fn replay(name: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    let ledger = scenario.trace_path("ledger.raw.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([ledger.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!("skipping local Yoshi state: {} absent", missing.display());
        return;
    }
    let pads =
        PadScript::from_expected_trace(&scenario.trace_path("tick.expected.jsonl"), true).unwrap();
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut ticks = 0;
    for (tick, row) in fs::read_to_string(ledger).unwrap().lines().enumerate() {
        let row: serde_json::Value = serde_json::from_str(row).unwrap();
        let raw = row["fighters"][0]["bytes"].as_str().unwrap();
        let word =
            |offset: usize| u32::from_str_radix(&raw[offset * 2..offset * 2 + 8], 16).unwrap();
        simulation.tick().unwrap();
        let runtime = simulation.runtime.borrow();
        let SceneFighter::Yoshi(fighter) = &runtime.state.fighters[0] else {
            panic!("Yoshi slot")
        };
        for (field, actual, offset) in [
            ("shield health", fighter.status.shield_health, 0x1998),
            ("double jump armor", fighter.combat.armor, 0x18B4),
        ] {
            assert_eq!(actual.to_bits(), word(offset), "{name} tick {tick} {field}");
        }
        if let MotionData::Guard(guard) = &fighter.state_data {
            for (field, actual, offset) in [
                ("guard elapsed", guard.elapsed, 0x2340),
                ("guard minimum hold", guard.minimum_hold, 0x2350),
                ("reflect frames", guard.reflect_frames, 0x2354),
                ("powershield frames", guard.powershield_frames, 0x2358),
            ] {
                assert_eq!(actual.to_bits(), word(offset), "{name} tick {tick} {field}");
            }
        }
        ticks += 1;
    }
    assert_eq!(ticks, 300);
}
#[test]
fn shield_yoshi_health_and_windows() {
    replay("shield_fd_yoshi");
}
#[test]
fn roll_yoshi_health_and_windows() {
    replay("roll_fd_yoshi");
}
#[test]
fn spotdodge_yoshi_health_and_windows() {
    replay("spotdodge_fd_yoshi");
}
#[test]
fn jump_yoshi_armor() {
    replay("jump_fd_yoshi");
}
#[test]
fn airjumpb_yoshi_armor() {
    replay("airjumpb_fd_yoshi");
}
