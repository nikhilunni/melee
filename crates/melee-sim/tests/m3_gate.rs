use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::path::Path;
fn local_scenario_named(name: &str) -> Option<Scenario> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml"));
    let scenario = Scenario::load(&path).unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return None;
    }
    Some(scenario)
}
#[test]
fn idle_fd_fox_600() {
    let Some(scenario) = local_scenario_named("idle_fd_fox") else {
        return;
    };
    assert_eq!(scenario.frames, 600);
    trace::gate(&scenario).unwrap();
    eprintln!("600 ticks, 49 keys, 0 divergences");
}
#[test]
fn m3_initial_snapshot_schema_coverage() {
    let Some(scenario) = local_scenario_named("idle_fd_fox") else {
        return;
    };
    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    let record = simulation.tick().unwrap();
    assert_eq!(record.frame, 0);
    assert_eq!(record.phase, "frame_end");
    trace::check_schema(&record).unwrap();
    assert_eq!(record.state.len(), 49);
}

#[test]
fn start_fd_fox_600() {
    let Some(scenario) = local_scenario_named("start_fd_fox") else {
        return;
    };
    assert_eq!(scenario.frames, 600);
    trace::gate(&scenario).unwrap();
    eprintln!("600 ticks, 49 keys, 0 divergences");
}
