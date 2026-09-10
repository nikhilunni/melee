use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::{fs, path::Path};

#[test]
fn laser_fd_fox_300_ticks_items_and_ordered_particle_draws() {
    combat_gate("laser_fd_fox");
}

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
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([ledger_path.clone()]),
    ) {
        return None;
    }
    assert_eq!(scenario.frames as usize, ticks);
    if name == "laser_fd_fox" {
        trace::gate_items(&scenario).unwrap();
    } else {
        trace::gate(&scenario).unwrap();
    }
    eprintln!(
        "{ticks} ticks, {} keys, 0 divergences",
        if name == "laser_fd_fox" {
            trace::compared_keys(&scenario).unwrap()
        } else {
            49
        }
    );
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
        if name == "laser_fd_fox" {
            let effect_sites: Vec<u32> = row["rng_draws"]
                .as_array()
                .unwrap()
                .iter()
                .map(|draw| draw["lr"].as_u64().unwrap() as u32 - 4)
                .filter(|site| matches!(site, 0x8006_3990 | 0x8007_85CC | 0x8007_85FC))
                .collect();
            assert_eq!(
                simulation.effect_rng_sites(),
                effect_sites,
                "tick {tick} direct hit-effect RNG order"
            );
        }
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
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([ledger_path.clone()]),
    ) {
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

// S1: every recorded ground-attack scene, including the pre-existing Fox up tilt.
#[test]
fn dashattack_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("dashattack_fd_fox");
}

#[test]
fn ftilt_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("ftilt_fd_fox");
}

#[test]
fn ftiltup_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("ftiltup_fd_fox");
}

#[test]
fn ftiltdown_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("ftiltdown_fd_fox");
}

#[test]
fn dtilt_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("dtilt_fd_fox");
}

#[test]
fn fsmashcharge_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("fsmashcharge_fd_fox");
}

#[test]
fn usmash_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("usmash_fd_fox");
}

#[test]
fn dsmash_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("dsmash_fd_fox");
}

#[test]
fn jabcombo_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("jabcombo_fd_fox");
}

#[test]
fn dashattack_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dashattack_fd_marth");
}

#[test]
fn ftilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("ftilt_fd_marth");
}

#[test]
fn ftiltup_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("ftiltup_fd_marth");
}

#[test]
fn ftiltdown_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("ftiltdown_fd_marth");
}

#[test]
fn dtilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dtilt_fd_marth");
}

#[test]
fn fsmashcharge_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("fsmashcharge_fd_marth");
}

#[test]
fn usmash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("usmash_fd_marth");
}

#[test]
fn dsmash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dsmash_fd_marth");
}

#[test]
fn jabcombo_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("jabcombo_fd_marth");
}

#[test]
fn utilt_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("utilt_fd_fox");
}

// S3: full aerial Illusion and the grounded common-pose boundary.
#[test]
fn illusion_fd_fox_first_125_ticks_with_items_and_particle_order() {
    special_gate_prefix("illusion_fd_fox", 125);
}

#[test]
fn airillusion_fd_fox_300_ticks_with_items_and_particle_order() {
    special_gate_prefix("airillusion_fd_fox", 300);
}

fn special_gate_prefix(name: &str, ticks: usize) {
    use melee_diff::{first_divergence, read_trace};
    use std::io::BufReader;
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml")),
    )
    .unwrap();
    let ledger_path = scenario.trace_path("ledger.raw.jsonl");
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([ledger_path.clone()]),
    ) {
        return;
    }
    let expected = read_trace(BufReader::new(
        fs::File::open(scenario.expected_path()).unwrap(),
    ))
    .unwrap();
    let raw = fs::read_to_string(scenario.expected_path()).unwrap();
    let ledger = fs::read_to_string(ledger_path).unwrap();
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    assert_eq!(expected.len(), 300);
    assert_eq!(ledger.lines().count(), 300);
    for (tick, ((expected, raw), ledger)) in expected
        .iter()
        .zip(raw.lines())
        .zip(ledger.lines())
        .take(ticks)
        .enumerate()
    {
        let actual = simulation.tick().unwrap();
        let ledger: serde_json::Value = serde_json::from_str(ledger).unwrap();
        let expected_sites: Vec<u32> = ledger["rng_draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["lr"].as_u64().unwrap() as u32 - 4)
            .filter(|s| (0x8039_8f8c..0x8039_f6cc).contains(s))
            .collect();
        assert_eq!(
            simulation.particle_rng_sites(),
            expected_sites,
            "tick {tick} particle RNG order"
        );
        assert!(
            first_divergence([expected], [&actual]).is_none(),
            "tick {tick}: {:?}",
            first_divergence([expected], [&actual])
        );
        let item_expected =
            melee_sim::trace_items::expected(&serde_json::from_str(raw).unwrap(), tick as u64)
                .unwrap()
                .unwrap();
        let item_actual = simulation.item_snapshot(tick as u64);
        assert!(
            first_divergence([&item_expected], [&item_actual]).is_none(),
            "tick {tick}: {:?}",
            first_divergence([&item_expected], [&item_actual])
        );
    }
}

// S2: every recorded aerial, L-cancel and autocancel scene.
#[test]
fn nair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("nair_fd_fox");
}

#[test]
fn fair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("fair_fd_fox");
}

#[test]
fn bair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("bair_fd_fox");
}

#[test]
fn uair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("uair_fd_fox");
}

#[test]
fn dair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("dair_fd_fox");
}

#[test]
fn nairlc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("nairlc_fd_fox");
}

#[test]
fn fairlc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("fairlc_fd_fox");
}

#[test]
fn bairlc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("bairlc_fd_fox");
}

#[test]
fn uairlc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("uairlc_fd_fox");
}

#[test]
fn dairlc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("dairlc_fd_fox");
}

#[test]
fn nair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("nair_fd_marth");
}

#[test]
fn fair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("fair_fd_marth");
}

#[test]
fn bair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("bair_fd_marth");
}

#[test]
fn uair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("uair_fd_marth");
}

#[test]
fn dair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dair_fd_marth");
}

#[test]
fn dairlc_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dairlc_fd_marth");
}

#[test]
fn fairlc_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("fairlc_fd_marth");
}

// The full S2 gates above remain mandatory. These additional prefix gates retain
// exact evidence through the four former floor-endpoint pose boundaries.
#[test]
fn s2_marth_landings_before_floor_endpoint_pose_correction() {
    for (name, ticks) in [
        ("fair_fd_marth", 154),
        ("dair_fd_marth", 166),
        ("dairlc_fd_marth", 150),
        ("fairlc_fd_marth", 142),
    ] {
        s2_prefix_gate(name, ticks);
    }
}

fn s2_prefix_gate(name: &str, ticks: usize) {
    use melee_diff::{first_divergence, read_trace};
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml"));
    let scenario = Scenario::load(&path).unwrap();
    let ledger_path = scenario.trace_path("ledger.raw.jsonl");
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([ledger_path.clone()]),
    ) {
        return;
    }
    let expected = read_trace(std::io::BufReader::new(
        fs::File::open(scenario.trace_path("tick.expected.jsonl")).unwrap(),
    ))
    .unwrap();
    assert_eq!(expected.len(), 300);
    let ledger = fs::read_to_string(ledger_path).unwrap();
    assert_eq!(ledger.lines().count(), 300);
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    for (tick, (expected, line)) in expected.iter().zip(ledger.lines()).take(ticks).enumerate() {
        let actual = simulation.tick().unwrap();
        trace::check_schema(&actual).unwrap();
        assert!(
            first_divergence([expected], [&actual]).is_none(),
            "{name} tick {tick}"
        );
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let sites: Vec<u32> = row["rng_draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|draw| draw["lr"].as_u64().unwrap() as u32 - 4)
            .filter(|site| (0x8039_8f8c..0x8039_f6cc).contains(site))
            .collect();
        assert_eq!(simulation.particle_rng_sites(), sites, "{name} tick {tick}");
    }
    eprintln!("{name}: {ticks} ticks, 49 keys, 0 divergences; former pose boundary prefix");
}

// S5: full hit-reaction scenes, both fighters and ordered particle draws.
#[test]
fn di_upaway_fsmash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("di_upaway_fsmash_fd_marth");
}

#[test]
fn di_downin_fsmash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("di_downin_fsmash_fd_marth");
}

#[test]
fn sdi_fsmash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("sdi_fsmash_fd_marth");
}

#[test]
fn cc_ftilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cc_ftilt_fd_marth");
}

#[test]
#[ignore = "blocked on S3 part 2: Marth SpecialHi (Dolphin Slash) entry at tick 119; un-ignore when it lands"]
fn tumbledi_dolphinslash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("tumbledi_dolphinslash_fd_marth");
}

#[test]
fn getupattack_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("getupattack_fd_fox");
}

#[test]
fn getupstand_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("getupstand_fd_fox");
}

#[test]
fn getuproll_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("getuproll_fd_fox");
}
