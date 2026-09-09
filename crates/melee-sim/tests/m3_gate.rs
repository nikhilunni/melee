use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::path::Path;
fn local_scenario() -> Option<Scenario> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/idle_fd_fox.toml");
    let scenario = Scenario::load(&path).unwrap();
    if let Some(missing) = scenario.required_files().iter().find(|p| !p.is_file()) {
        eprintln!("skipping M3: {} absent", missing.display());
        return None;
    }
    Some(scenario)
}
#[test]
fn idle_fd_fox_600() {
    let Some(scenario) = local_scenario() else {
        return;
    };
    assert_eq!(scenario.frames, 600);
    trace::gate(&scenario).unwrap();
    eprintln!("600 ticks, 49 keys, 0 divergences");
}
#[test]
fn m3_initial_snapshot_schema_coverage() {
    let Some(scenario) = local_scenario() else {
        return;
    };
    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    let record = simulation.tick().unwrap();
    assert_eq!(record.frame, 0);
    assert_eq!(record.phase, "frame_end");
    trace::check_schema(&record).unwrap();
    assert_eq!(record.state.len(), 49);
}
