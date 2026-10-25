//! Actual floor-entry scalars and the existing protection expiry paths.
use super::*;
use crate::scenario::Scenario;
use std::{io::BufRead, path::PathBuf};

const FLOOR_CAPTURE: &str = "corpus_v3_kj_fox_marth4_e0c8363c7_p1";

fn check_raw_protection(name: &str, port: usize, ticks: usize, floor_entry: bool) {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    let raw = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([raw.clone()]),
    ) {
        return;
    }
    assert_eq!(scenario.frames as usize, ticks);
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut rows = 0;
    let mut barrel_holds = 0;
    let mut barrel_launches = 0;
    let mut previous = [0; 2];
    let mut protection_started = false;
    let mut protection_expired = false;
    let mut last_invincibility = 0;
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
        let invincibility = fighter.status.revival_invincibility;
        assert_eq!(
            invincibility,
            word(0x1994) as i32,
            "{name} tick{frame}: protection"
        );
        assert_eq!(
            u32::from(fighter.combat.color_overlay.id),
            word(0x430),
            "{name} tick{frame}: actual primary color program"
        );
        protection_started |= invincibility == 120;
        protection_expired |= last_invincibility == 1 && invincibility == 0;
        last_invincibility = invincibility;
        if floor_entry && frame == ticks - 2 {
            assert_eq!(fighter.motion_state.action.0, 12);
            assert_eq!(invincibility, 0);
            assert_eq!(fighter.combat.color_overlay.id, 0);
        }
        if floor_entry && frame == ticks - 1 {
            assert_eq!(fighter.motion_state.action.0, 14);
            assert_eq!(u32::from(fighter.motion_state.action.0), word(0x10));
            assert_eq!(
                fighter.physics.ground_or_air,
                melee_types::GroundOrAir::Ground
            );
            assert_eq!(word(0xE0), 0);
            let position = fighter.physics.position;
            assert_eq!(
                [
                    position.x.to_bits(),
                    position.y.to_bits(),
                    position.z.to_bits()
                ],
                [word(0xB0), word(0xB4), word(0xB8)]
            );
            assert_eq!(position, Vec3::new(20.0, 65.67595, 0.0));
            assert_eq!(fighter.collision.data.floor.index, 12);
            assert_eq!(fighter.collision.data.floor.index as u32, word(0x83C));
            assert_eq!(fighter.physics.jumps_used, 0);
            assert_eq!(
                u32::from(fighter.physics.jumps_used),
                u32::from_str_radix(&hex[0x1968 * 2..0x1969 * 2], 16).unwrap()
            );
            assert_eq!(fighter.animation.frame, 0.0);
            assert_eq!(
                fighter.physics.self_velocity.y.to_bits(),
                word(0x84),
                "ordinary landing preserves the recorded nonzero vertical velocity"
            );
            assert_ne!(fighter.physics.self_velocity.y, 0.0);
            assert_eq!(fighter.physics.ground_velocity.to_bits(), word(0xEC));
            assert_eq!(invincibility, 120);
            assert_eq!(fighter.combat.color_overlay.id, 9);
            assert_eq!(
                fighter
                    .combat
                    .color_overlay
                    .program
                    .color
                    .map(|(rgba, _)| rgba),
                Some(0xFFFF_FF80)
            );
            assert_eq!(word(0x434), 0xFFFF_FF80);
            assert_eq!(fighter.status.name_tag_timer, 50);
            assert_eq!(
                u32::from(fighter.status.name_tag_timer),
                u32::from_str_radix(&hex[0x209A * 2..0x209C * 2], 16).unwrap()
            );
        }
        for (index, fighter) in simulation.state().fighters.iter().enumerate() {
            let action = fighter.0.motion_state.action.0;
            barrel_holds += usize::from(action == 293);
            barrel_launches += usize::from(previous[index] == 293 && action == 91);
            previous[index] = action;
        }
        rows += 1;
    }
    assert_eq!(rows, ticks);
    assert!(protection_started);
    if floor_entry {
        assert_eq!((barrel_holds, barrel_launches), (47, 2));
        // The last row proves entry only; do not step an invented continuation.
    } else {
        assert!(protection_expired, "exercise the recorded 1 -> 0 expiry");
    }
}

#[test]
fn revival_floor_final_entry_matches_raw_state_and_actual_color() {
    check_raw_protection(FLOOR_CAPTURE, 1, 1679, true);
}

#[test]
fn revival_timeout_protection_and_color_expiry_match_both_complete_captures() {
    check_raw_protection("rebirth_timeout_fd_fox", 1, 900, false);
    check_raw_protection("rebirth_timeout_fd_marth", 0, 900, false);
}
