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

#[test]
fn dash_fd_fox_300() {
    movement_gate("dash_fd_fox");
}

#[test]
fn dash_particle_rng_sites_match_the_retail_ledger_in_order() {
    use melee_sim::{frame::Simulation, initial_state::InitialState, inputs::PadScript};
    let Some(scenario) = local_scenario_named("dash_fd_fox") else {
        return;
    };
    let path = scenario.trace_path("ledger.raw.jsonl");
    if !path.exists() {
        eprintln!("skipping: local dash RNG ledger absent");
        return;
    }
    let ledger = std::fs::read_to_string(path).unwrap();
    let pads =
        PadScript::from_expected_trace(&scenario.trace_path("tick.expected.jsonl"), true).unwrap();
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut ticks = 0;
    for (tick, line) in ledger.lines().enumerate() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let expected: Vec<u32> = row["rng_draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|draw| draw["lr"].as_u64().unwrap() as u32 - 4)
            .filter(|site| (0x8039_930C..0x8039_F6CC).contains(site))
            .collect();
        simulation.tick().unwrap();
        assert_eq!(
            simulation.particle_rng_sites(),
            expected,
            "particle RNG order at tick {tick}"
        );
        ticks += 1;
    }
    assert_eq!(ticks, 300);
}
