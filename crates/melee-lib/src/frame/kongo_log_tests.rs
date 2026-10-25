//! Counter semantics from fn_801D7E60, exercised through the real joint queue.
use super::*;
use crate::scenario::Scenario;
use melee_gr::kongo::objects::Log;
use std::path::PathBuf;

fn log(simulation: &Simulation) -> Log {
    let SceneStage::Kongo(stage) = &simulation.state().stage else {
        panic!("Kongo boundary")
    };
    stage.logs[1].expect("the recorded map-2 log")
}
fn tick(simulation: &mut Simulation, pads: &melee_sim::inputs::PadScript) {
    let frame = simulation.frame();
    simulation.set_inputs(pads.samples(frame));
    simulation.set_raw_sticks(pads.raw_sticks(frame));
    simulation.set_recorded_pad_queue_x(pads.recorded_pad_queue_x(frame));
    simulation.set_display(if pads.display_pass(frame) {
        crate::diagnostics::DisplayPass::Rendered
    } else {
        crate::diagnostics::DisplayPass::Skipped
    });
    simulation.tick_without_snapshot().unwrap();
}

#[test]
fn log_events_apply_next_stage_step_once_reset_and_wrap_as_signed_halfwords() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/corpus_v3_kj_fox_marth4_e8be4d273_p2.toml"))
            .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let pads = crate::trace::pad_script(&scenario).unwrap();
    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    for _ in 0..=2638 {
        tick(&mut simulation, &pads);
    }
    let initial = log(&simulation);
    assert_eq!(
        (initial.phase, initial.stood, initial.last_stood),
        (1, 0, 0)
    );
    let fighter = &simulation.state().fighters[0].0;
    assert_eq!(fighter.collision.data.floor.index, 3);
    assert_eq!(
        fighter.physics.ground_or_air,
        melee_types::GroundOrAir::Ground
    );
    let mut coll = fighter.collision.data;
    let mut queue = simulation.state().map.clone();
    assert!(queue
        .take_joint_events(1)
        .iter()
        .any(|event| event.ground_kind == 2));
    // Retail 801D7E68..70 assigns C8+1 on every callback; three calls
    // before the next log proc still represent one standing frame.
    simulation.runtime.state.map.notify_line_joint(&mut coll, 3);
    simulation.runtime.state.map.notify_line_joint(&mut coll, 3);
    assert_eq!(
        (log(&simulation).stood, log(&simulation).last_stood),
        (0, 0)
    );
    tick(&mut simulation, &pads);
    assert_eq!(
        (log(&simulation).stood, log(&simulation).last_stood),
        (1, 1)
    );
    assert_eq!(log(&simulation).age, initial.age + 1);

    // Remove the following frame's contacts, then supply only kind 1:
    // 801D7E60 ignores it, and 801D627C..84 resets the equal counters.
    simulation.runtime.state.map.take_joint_events(1);
    simulation
        .runtime
        .state
        .map
        .notify_floor_joint(&mut coll, 3, true, 0.0);
    tick(&mut simulation, &pads);
    assert_eq!(
        (log(&simulation).stood, log(&simulation).last_stood),
        (0, 0)
    );

    // An actual queued standing callback consumes signed C8 and stores
    // only a halfword: 32767 + 1 becomes -32768 before the next comparison.
    let SceneStage::Kongo(stage) = &mut simulation.runtime.state.stage else {
        unreachable!()
    };
    let log = stage.logs[1].as_mut().unwrap();
    log.stood = i16::MAX;
    log.last_stood = i16::MAX;
    simulation.runtime.state.map.notify_line_joint(&mut coll, 3);
    simulation.runtime.state.map.notify_line_joint(&mut coll, 3);
    tick(&mut simulation, &pads);
    assert_eq!(
        (
            self::log(&simulation).stood,
            self::log(&simulation).last_stood
        ),
        (i16::MIN, i16::MIN)
    );
}
