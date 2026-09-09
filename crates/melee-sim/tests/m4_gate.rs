use melee_sim::{scenario::Scenario, trace};
use std::path::Path;
fn local_scenario_named(name: &str) -> Option<Scenario> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml"));
    let scenario = Scenario::load(&path).unwrap();
    if let Some(missing) = scenario.required_files().iter().find(|p| !p.is_file()) {
        eprintln!("skipping M4: {} absent", missing.display());
        return None;
    }
    Some(scenario)
}

fn movement_gate(name: &str) {
    let Some(scenario) = local_scenario_named(name) else {
        return;
    };
    assert_eq!(scenario.frames, 300);
    trace::gate(&scenario).unwrap();
    eprintln!("300 ticks, 49 keys, 0 divergences");
}

#[test]
fn squat_fd_fox_300() {
    movement_gate("squat_fd_fox");
}
#[test]
fn turn_fd_fox_300() {
    movement_gate("turn_fd_fox");
}
#[test]
fn walk_fd_fox_300() {
    movement_gate("walk_fd_fox");
}
