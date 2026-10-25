//! Saved live AObj clocks must precede the barrel controller's first rate write.
use super::*;
use crate::scenario::Scenario;
use std::path::PathBuf;

#[test]
fn saved_barrel_clock_advances_before_controller_installs_its_rate() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/corpus_v3_kj_fox_marth4_ef5e3e043_p0.toml"))
            .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let initial = InitialState::from_savestate_traces(&scenario).unwrap();
    let SceneStage::Kongo(stage) = &initial.stage else {
        panic!("Kongo boundary")
    };
    assert_eq!(stage.barrel.rate, 0.5);
    let clock = |state: &InitialState| {
        let model = &state.stage_animations[&melee_gr::kongo::BARREL];
        let aobj = model
            .pose_tree()
            .get(hsd_anim::jobj::JObjId(0))
            .aobj
            .as_ref()
            .unwrap();
        (aobj.curr_frame, aobj.framerate)
    };
    // Raw boundary: authored root 0x80C3E160, AObj 0x80C3F6C0.
    assert_eq!(clock(&initial), (0.0, 1.0));
    let mut simulation =
        TestSimulation::with_inputs(initial, crate::trace::pad_script(&scenario).unwrap());
    simulation.tick_without_snapshot().unwrap();
    // The saved partial tick is already past the stage animation proc.
    assert_eq!(clock(simulation.state()), (0.0, 1.0));
    simulation.tick_without_snapshot().unwrap();
    // Link 1 advances at 1.0; only then does link 4 install gp+E8.
    assert_eq!(clock(simulation.state()), (1.0, 0.5));
    simulation.tick_without_snapshot().unwrap();
    assert_eq!(clock(simulation.state()), (1.5, 0.5));
}

#[test]
fn moving_barrel_hidden_cooldown_and_launch_state_match_raw_retail() {
    use std::io::BufRead;
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/corpus_v3_kj_fox_marth4_ef5e3e043_p0.toml"))
            .unwrap();
    let raw = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([raw.clone()]),
    ) {
        return;
    }
    let pads = crate::trace::pad_script(&scenario).unwrap();
    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    let mut held_ticks = 0;
    let mut launches = 0;
    let mut previous = [0; 2];
    let mut rows = 0;
    for (frame, line) in melee_test_support::trace::open(&raw)
        .unwrap()
        .lines()
        .enumerate()
    {
        simulation.set_inputs(pads.samples(frame as u64));
        simulation.set_raw_sticks(pads.raw_sticks(frame as u64));
        simulation.set_recorded_pad_queue_x(pads.recorded_pad_queue_x(frame as u64));
        simulation.set_display(if pads.display_pass(frame as u64) {
            crate::diagnostics::DisplayPass::Rendered
        } else {
            crate::diagnostics::DisplayPass::Skipped
        });
        simulation.tick_without_snapshot().unwrap();
        let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert_eq!(row["frame"], frame);
        for (index, fighter) in simulation.state().fighters.iter().enumerate() {
            let hex = row["fighters"][index]["bytes"].as_str().unwrap();
            let word =
                |offset: usize| u32::from_str_radix(&hex[offset * 2..offset * 2 + 8], 16).unwrap();
            // Fighter.bury_timer_2 at232C, initialized by ftCo_8009EC70.
            assert_eq!(
                fighter.0.status.barrel_cooldown,
                word(0x232C),
                "cooldown frame{frame} p{index}"
            );
            let action = u32::from(fighter.0.motion_state.action.0);
            if action == 293 || previous[index] == 293 {
                assert_eq!(action, word(0x10), "hold/launch frame{frame} p{index}");
                let position = fighter.0.physics.position;
                assert_eq!(
                    [
                        position.x.to_bits(),
                        position.y.to_bits(),
                        position.z.to_bits()
                    ],
                    [word(0xB0), word(0xB4), word(0xB8)],
                    "held/launch position frame{frame} p{index}"
                );
                assert_eq!(
                    fighter.0.physics.percent.to_bits(),
                    word(0x1830),
                    "damage frame{frame} p{index}"
                );
            }
            held_ticks += usize::from(action == 293);
            launches += usize::from(previous[index] == 293 && action == 91);
            previous[index] = action;
        }
        rows += 1;
    }
    assert_eq!(rows, scenario.frames as usize);
    assert_eq!((held_ticks, launches), (18, 3));
}
