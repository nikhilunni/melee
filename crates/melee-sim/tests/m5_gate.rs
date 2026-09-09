use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::{fs, path::Path};

#[test]
fn jab_fd_marth_300_ticks_and_ordered_particle_draws() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/jab_fd_marth.toml");
    let scenario = Scenario::load(&path).unwrap();
    let ledger_path = scenario.trace_path("ledger.raw.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([ledger_path.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!("skipping M5: {} absent", missing.display());
        return;
    }
    assert_eq!(scenario.frames, 300);
    trace::gate(&scenario).unwrap();
    eprintln!("300 ticks, 49 keys, 0 divergences");
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    let ledger = fs::read_to_string(ledger_path).unwrap();
    assert_eq!(ledger.lines().count(), 300);
    let mut particle_draws = 0;
    for (tick, line) in ledger.lines().enumerate() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let expected: Vec<u32> = row["rng_draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|draw| draw["lr"].as_u64().unwrap() as u32 - 4)
            .filter(|site| (0x8039_930c..0x8039_f6cc).contains(site))
            .collect();
        simulation.tick().unwrap();
        assert_eq!(
            simulation.particle_rng_sites(),
            expected,
            "tick {tick} particle RNG order"
        );
        particle_draws += expected.len();
    }
    assert_eq!(particle_draws, 9373);
}
