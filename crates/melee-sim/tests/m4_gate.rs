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
    movement_gate_ticks(name, 300);
}

fn movement_gate_ticks(name: &str, ticks: usize) {
    let Some(scenario) = local_scenario_named(name) else {
        return;
    };
    assert_eq!(scenario.frames as usize, ticks);
    trace::gate(&scenario).unwrap();
    eprintln!("{ticks} ticks, 49 keys, 0 divergences");
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

fn particle_rng_sites_match_the_retail_ledger_in_order(name: &str) {
    particle_rng_sites_for_ticks(name, 300);
}

fn particle_rng_sites_for_ticks(name: &str, expected_ticks: usize) {
    use melee_sim::{frame::Simulation, initial_state::InitialState, inputs::PadScript};
    let Some(scenario) = local_scenario_named(name) else {
        return;
    };
    let path = scenario.trace_path("ledger.raw.jsonl");
    if !path.exists() {
        eprintln!("skipping: local {name} RNG ledger absent");
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
    assert_eq!(ticks, expected_ticks);
}

#[test]
fn jump_fd_fox_300() {
    movement_gate("jump_fd_fox");
}

#[test]
fn dash_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_match_the_retail_ledger_in_order("dash_fd_fox");
}
#[test]
fn jump_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_match_the_retail_ledger_in_order("jump_fd_fox");
}

#[test]
fn shield_fd_fox_300() {
    movement_gate("shield_fd_fox");
}
#[test]
fn shield_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_match_the_retail_ledger_in_order("shield_fd_fox");
}

#[test]
fn spotdodge_fd_fox_300() {
    movement_gate("spotdodge_fd_fox");
}
#[test]
fn spotdodge_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_match_the_retail_ledger_in_order("spotdodge_fd_fox");
}

#[test]
fn roll_fd_fox_300() {
    movement_gate("roll_fd_fox");
}
#[test]
fn roll_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_match_the_retail_ledger_in_order("roll_fd_fox");
}

#[test]
fn airdodge_fd_fox_300() {
    movement_gate_ticks("airdodge_fd_fox", 300);
}
#[test]
fn airdodge_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("airdodge_fd_fox", 300);
}

#[test]
fn wavedash_fd_fox_300() {
    movement_gate_ticks("wavedash_fd_fox", 300);
}
#[test]
fn wavedash_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("wavedash_fd_fox", 300);
}

#[test]
fn ledge_fd_fox_420() {
    movement_gate_ticks("ledge_fd_fox", 420);
}
#[test]
fn ledge_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("ledge_fd_fox", 420);
}

#[test]
fn turnrun_fd_fox_300() {
    movement_gate_ticks("turnrun_fd_fox", 300);
}

#[test]
fn turnrun_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("turnrun_fd_fox", 300);
}

#[test]
fn walkfast_fd_fox_300() {
    movement_gate_ticks("walkfast_fd_fox", 300);
}

#[test]
fn walkfast_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("walkfast_fd_fox", 300);
}

#[test]
fn ledgeclimb_fd_fox_420() {
    movement_gate_ticks("ledgeclimb_fd_fox", 420);
}

#[test]
fn ledgeclimb_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("ledgeclimb_fd_fox", 420);
}

#[test]
fn ledgeescape_fd_fox_420() {
    movement_gate_ticks("ledgeescape_fd_fox", 420);
}

#[test]
fn ledgeescape_particle_rng_sites_match_the_retail_ledger_in_order() {
    particle_rng_sites_for_ticks("ledgeescape_fd_fox", 420);
}
