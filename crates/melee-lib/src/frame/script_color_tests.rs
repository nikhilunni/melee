//! A script issued during Input installs its slot now and executes next Animation.
use super::*;
use crate::scenario::Scenario;
use std::{io::BufRead, path::PathBuf};

#[test]
fn getup_attack_installs_color_without_stepping_and_matches_complete_raw_lifecycle() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/getup_attack_fd_marth.toml")).unwrap();
    let raw = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([raw.clone()]),
    ) {
        return;
    }
    assert_eq!(scenario.frames, 280);
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut rows = 0;
    for (frame, line) in melee_test_support::trace::open(&raw)
        .unwrap()
        .lines()
        .enumerate()
    {
        simulation
            .engine
            .set_display_pass(simulation.pads.display_pass(frame as u64));
        simulation.tick_without_snapshot().unwrap();
        let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert_eq!(row["frame"], frame);
        for (port, fighter) in simulation.state().fighters.iter().enumerate() {
            let hex = row["fighters"][port]["bytes"].as_str().unwrap();
            let word =
                |offset: usize| u32::from_str_radix(&hex[offset * 2..offset * 2 + 8], 16).unwrap();
            assert_eq!(
                u32::from(fighter.0.combat.color_overlay.id),
                word(0x430),
                "tick{frame} p{port}: complete primary color ownership"
            );
            if port == 1 && frame == 120 {
                assert_eq!(fighter.0.motion_state.action.0, 222);
                assert_eq!(fighter.0.status.revival_invincibility, 8);
                assert_eq!(fighter.0.combat.color_overlay.id, 9);
                assert!(
                    fighter.0.combat.color_overlay.program.color.is_none(),
                    "throw protection installs after both entry scripts without execution"
                );
                assert_eq!(word(0x408), 0);
            }
            if port == 1 && frame == 121 {
                assert_eq!(fighter.0.combat.color_overlay.id, 9);
                assert!(fighter.0.combat.color_overlay.program.color.is_some());
                assert_eq!(word(0x408), 1);
            }
            if port == 0 && frame == 195 {
                assert_eq!(fighter.0.motion_state.action.0, 187);
                assert_eq!(fighter.0.combat.color_overlay.id, 2);
                assert!(
                    fighter.0.combat.color_overlay.program.color.is_none(),
                    "installed during Input; do not execute its program early"
                );
                assert_eq!(
                    word(0x408),
                    0,
                    "retail newly installed program is unstepped"
                );
                assert_ne!(word(0x410), 0, "retail has a live program pointer");
            }
            if port == 0 && frame == 196 {
                assert_eq!(fighter.0.combat.color_overlay.id, 2);
                assert!(fighter.0.combat.color_overlay.program.color.is_some());
                assert_eq!(word(0x408), 4, "next ordinary Animation step executes it");
            }
            if port == 0 && frame == 230 {
                assert_eq!(fighter.0.combat.color_overlay.id, 0);
                assert_eq!(word(0x410), 0, "retail cleared the program");
            }
        }
        rows += 1;
    }
    assert_eq!(rows, 280);
}

#[test]
fn manual_revival_installs_protection_after_entry_without_stepping() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    for (name, port, entry, action) in [
        ("rebirth_shield_a_fd_fox", 1, 301, 236),
        ("rebirth_analog_shield_a_fd_fox", 1, 301, 65),
        ("rebirth_held_shield_a_fd_fox", 1, 220, 236),
        ("rebirth_rollout_blast_fd_jigglypuff", 0, 400, 355),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let raw = scenario.trace_path("tick.raw.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()]),
        ) {
            continue;
        }
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut rows = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            let hex = row["fighters"][port]["bytes"].as_str().unwrap();
            let word =
                |offset: usize| u32::from_str_radix(&hex[offset * 2..offset * 2 + 8], 16).unwrap();
            let fighter = &simulation.state().fighters[port].0;
            assert_eq!(
                fighter.status.revival_invincibility,
                word(0x1994) as i32,
                "{name} tick{frame}: protection"
            );
            assert_eq!(
                u32::from(fighter.combat.color_overlay.id),
                word(0x430),
                "{name} tick{frame}: primary owner"
            );
            if frame == entry {
                assert_eq!(fighter.motion_state.action.0, action);
                assert_eq!(fighter.status.revival_invincibility, 120);
                assert_eq!(fighter.combat.color_overlay.id, 9);
                assert_eq!(word(0x408), 0);
                assert_ne!(word(0x410), 0);
                assert!(
                    fighter.combat.color_overlay.program.color.is_none(),
                    "{name}: install after entry without execution"
                );
            }
            if frame == entry + 1 {
                assert_eq!(fighter.combat.color_overlay.id, 9);
                assert!(fighter.combat.color_overlay.program.color.is_some());
                assert_eq!(word(0x408), 1);
            }
            rows += 1;
        }
        assert_eq!(rows, scenario.frames as usize);
        assert!(rows > entry + 1);
    }
}
