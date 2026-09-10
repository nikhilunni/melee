use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::{fs, path::Path};

#[test]
fn jab_fd_marth_300_ticks_and_ordered_particle_draws() {
    if let Some(draws) = combat_gate("jab_fd_marth") {
        assert!(
            draws > 0,
            "the per-tick ledger comparison must cover particle draws"
        );
    }
}

#[test]
fn jab_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("jab_fd_fox");
}

#[test]
fn utilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("utilt_fd_marth");
}

#[test]
fn shieldhit_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("shieldhit_fd_marth");
}

#[test]
fn grab_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("grab_fd_marth");
}

#[test]
fn tech_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("tech_fd_marth");
}

#[test]
fn ko_fd_marth_480_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("ko_fd_marth", 480);
}

fn combat_gate(name: &str) -> Option<usize> {
    combat_gate_ticks(name, 300)
}
fn combat_gate_ticks(name: &str, ticks: usize) -> Option<usize> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml"));
    let scenario = Scenario::load(&path).unwrap();
    let ledger_path = scenario.trace_path("ledger.raw.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([ledger_path.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!("skipping M5: {} absent", missing.display());
        return None;
    }
    assert_eq!(scenario.frames as usize, ticks);
    trace::gate(&scenario).unwrap();
    eprintln!("{ticks} ticks, 49 keys, 0 divergences");
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    let ledger = fs::read_to_string(ledger_path).unwrap();
    assert_eq!(ledger.lines().count(), ticks);
    let mut particle_draws = 0;
    for (tick, line) in ledger.lines().enumerate() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let expected: Vec<u32> = row["rng_draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|draw| draw["lr"].as_u64().unwrap() as u32 - 4)
            .filter(|site| (0x8039_8f8c..0x8039_f6cc).contains(site))
            .collect();
        simulation.tick().unwrap();
        assert_eq!(
            simulation.particle_rng_sites(),
            expected,
            "tick {tick} particle RNG order"
        );
        particle_draws += expected.len();
    }
    Some(particle_draws)
}

/// Retain the focused catch-entry regression alongside the full throw gate.
#[test]
fn grab_fd_marth_catch_startup_127_ticks_and_ordered_particle_draws() {
    use melee_diff::{first_divergence, read_trace};
    use std::io::BufReader;
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/grab_fd_marth.toml");
    let scenario = Scenario::load(&path).unwrap();
    let ledger_path = scenario.trace_path("ledger.raw.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([ledger_path.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!("skipping grab startup: {} absent", missing.display());
        return;
    }
    let expected = read_trace(BufReader::new(
        fs::File::open(scenario.trace_path("tick.expected.jsonl")).unwrap(),
    ))
    .unwrap();
    let ledger = fs::read_to_string(ledger_path).unwrap();
    assert_eq!(expected.len(), 300);
    assert_eq!(ledger.lines().count(), 300);
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    for (tick, (expected, line)) in expected.iter().zip(ledger.lines()).take(127).enumerate() {
        let actual = simulation.tick().unwrap();
        trace::check_schema(&actual).unwrap();
        assert!(
            first_divergence([expected], [&actual]).is_none(),
            "tick {tick}"
        );
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let draws: Vec<u32> = row["rng_draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|draw| draw["lr"].as_u64().unwrap() as u32 - 4)
            .filter(|site| (0x8039_8f8c..0x8039_f6cc).contains(site))
            .collect();
        assert_eq!(
            simulation.particle_rng_sites(),
            draws,
            "tick {tick} particle RNG order"
        );
    }
    eprintln!("127 ticks, 49 keys, 0 divergences; linked capture at tick 127 remains unsupported");
}
