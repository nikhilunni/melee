//! Multijump scratch and held-input behavior beyond the 49-key scene schema.
use super::*;
use crate::{scenario::Scenario, scene_fighter::SceneFighter};
use melee_ft::fighter::MotionData;
use std::{fs, path::Path};

fn scenario(name: &str) -> Option<Scenario> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    if let Some(missing) = scenario.required_files().into_iter().find(|p| !p.is_file()) {
        eprintln!("skipping local Puff state: {} absent", missing.display());
        return None;
    }
    Some(scenario)
}

#[test]
fn puff_multijump_turn_and_input_age_match_retail_scratch() {
    for name in ["jump_fd_puff", "airjumpb_fd_puff"] {
        let Some(scenario) = scenario(name) else {
            return;
        };
        let path = scenario.trace_path("tick.raw.jsonl");
        let pads =
            PadScript::from_expected_trace(&scenario.trace_path("tick.expected.jsonl"), true)
                .unwrap();
        let mut simulation = Simulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut jumps = 0;
        for (tick, row) in fs::read_to_string(path).unwrap().lines().enumerate() {
            let row: serde_json::Value = serde_json::from_str(row).unwrap();
            let raw = row["fighters"][0]["bytes"].as_str().unwrap();
            let word =
                |offset: usize| u32::from_str_radix(&raw[offset * 2..offset * 2 + 8], 16).unwrap();
            simulation.tick().unwrap();
            let runtime = simulation.runtime.borrow();
            let SceneFighter::Jigglypuff(fighter) = &runtime.state.fighters[0] else {
                panic!("Puff slot")
            };
            if let MotionData::MultiJump(jump) = &fighter.state_data {
                assert_eq!(
                    jump.turn_remaining as u32,
                    word(0x2340),
                    "{name} tick {tick} turn countdown"
                );
                assert_eq!(
                    jump.retained_drop_timer.to_bits(),
                    word(0x2344),
                    "{name} tick {tick} inherited scratch"
                );
                assert_eq!(
                    fighter.commands.variables,
                    std::array::from_fn(|i| word(0x2200 + i * 4)),
                    "{name} tick {tick} command variables"
                );
                let tilt = u8::from_str_radix(&raw[0x671 * 2..0x672 * 2], 16).unwrap();
                assert_eq!(
                    fighter.input.vertical.tilt, tilt,
                    "{name} tick {tick} tilt age"
                );
                jumps += 1;
            }
        }
        assert!(jumps > 0, "fixture must exercise the multijump path");
    }
}

#[test]
fn puff_held_jump_visits_all_five_states_then_exhausts_air_jumps() {
    use melee_ft::input::{Buttons, PadSample, Stick};
    use melee_types::CommonMotionState;
    let Some(scenario) = scenario("idle_fd_puff") else {
        return;
    };
    let mut initial = InitialState::from_savestate_traces(&scenario).unwrap();
    let SceneFighter::Jigglypuff(fighter) = &mut initial.fighters[0] else {
        panic!("Puff slot")
    };
    let assets = &initial.assets.fighters[0];
    // Isolate Anim/IASA while airborne: the retail scenarios gate physics and
    // collision. Hold X continuously after the first backward aerial jump.
    fighter.leave_ground();
    fighter
        .change_motion_state(CommonMotionState::Fall, assets)
        .unwrap();
    let held = PadSample {
        buttons: Buttons::X,
        stick: Stick { x: -0.8, y: 0.0 },
        ..Default::default()
    };
    fighter.proc_input(assets, &held);
    assert_eq!(
        (fighter.motion_state.action_id, fighter.physics.jumps_used),
        (341, 2)
    );
    assert_eq!(fighter.physics.facing, 1.0);
    let mut actions = vec![341];
    for tick in 1..=200 {
        fighter.proc_anim(assets, &mut initial.rng).unwrap();
        fighter.proc_input(assets, &held);
        if tick < 5 {
            assert_eq!(fighter.physics.facing, 1.0);
        }
        if tick == 5 {
            assert_eq!(fighter.physics.facing, -1.0);
        }
        if actions.last() != Some(&fighter.motion_state.action_id) {
            actions.push(fighter.motion_state.action_id);
        }
    }
    assert_eq!(actions, [341, 342, 343, 344, 345, 32]);
    assert_eq!(fighter.physics.jumps_used, 6);
}
