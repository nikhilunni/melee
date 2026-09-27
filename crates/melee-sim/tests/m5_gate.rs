use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::path::Path;

#[test]
fn match_fd_foxmarth_6083_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("match_fd_foxmarth", 6083);
}

#[test]
fn match2_fd_foxmarth_10059_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("match2_fd_foxmarth", 10059);
}

#[test]
fn laser_fd_fox_300_ticks_items_and_ordered_particle_draws() {
    combat_gate("laser_fd_fox");
}

#[test]
fn laser_shield_fd_marth_300_ticks_items_and_ordered_particle_draws() {
    combat_gate("laser_shield_fd_marth");
}
#[test]
fn laser_lightshield_fd_marth_300_ticks_items_and_ordered_particle_draws() {
    combat_gate("laser_lightshield_fd_marth");
}
#[test]
fn laser_shield_air_fd_marth_300_ticks_items_and_ordered_particle_draws() {
    combat_gate("laser_shield_air_fd_marth");
}
#[test]
fn laser_shield_deflect_fd_marth_300_ticks_items_and_ordered_particle_draws() {
    combat_gate("laser_shield_deflect_fd_marth");
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
    let compare_items = name.starts_with("cstick_throw_")
        || name.starts_with("laser_reflect_")
        || name.starts_with("corpus_")
        || name.starts_with("sudden_death_")
        || matches!(
            name,
            "illusion_start_landing_fd_fox"
                | "laser_fd_fox"
                | "laser_shield_fd_marth"
                | "laser_lightshield_fd_marth"
                | "laser_shield_air_fd_marth"
                | "laser_shield_deflect_fd_marth"
                | "match_fd_foxmarth"
                | "match2_fd_foxmarth"
        );
    if compare_items {
        trace::gate_items(&scenario).unwrap();
    } else {
        trace::gate(&scenario).unwrap();
    }
    eprintln!(
        "{ticks} ticks, {} keys, 0 divergences",
        if compare_items {
            trace::compared_keys(&scenario).unwrap()
        } else {
            49
        }
    );
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    let ledger = melee_test_support::trace::read_to_string(&ledger_path).unwrap();
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
    let expected = read_trace(
        melee_test_support::trace::open(&scenario.trace_path("tick.expected.jsonl")).unwrap(),
    )
    .unwrap();
    let ledger = melee_test_support::trace::read_to_string(&ledger_path).unwrap();
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

// S3: both Illusion scenes, including item keys and particle order.
#[test]
fn illusion_fd_fox_300_ticks_with_items_and_particle_order() {
    special_gate("illusion_fd_fox");
}

#[test]
fn airillusion_fd_fox_300_ticks_with_items_and_particle_order() {
    special_gate("airillusion_fd_fox");
}

fn special_gate(name: &str) {
    use melee_diff::{first_divergence, read_trace};

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
    let expected =
        read_trace(melee_test_support::trace::open(&scenario.expected_path()).unwrap()).unwrap();
    let raw = melee_test_support::trace::read_to_string(&scenario.expected_path()).unwrap();
    let ledger = melee_test_support::trace::read_to_string(&ledger_path).unwrap();
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

/// A grounded Reflector out of Run leaves the edge and jump-cancels: the
/// aerial jump reads Run's second scratch word (mv.co.run.x4).
#[test]
fn reflector_runedge_jump_fd_fox_120_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("reflector_runedge_jump_fd_fox", 120);
}

/// A neutral air out of PassiveWallJump inherits the wall jump's scratch word.
#[test]
fn walljump_aerial_fd_fox_450_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("walljump_aerial_fd_fox", 450);
}

/// Furafura (the shield-break dizzy) expiring into Wait, and a jab knocking
/// Fox out of it.
#[test]
fn furafura_expire_fd_marth_830_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("furafura_expire_fd_marth", 830);
}

#[test]
fn furafura_hit_fd_marth_460_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("furafura_hit_fd_marth", 460);
}

/// A sub-7% hit on a fighter lying face up: DownDamageU.
#[test]
fn downdamage_up_fd_marth_280_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("downdamage_up_fd_marth", 280);
}

/// Exits from a mature shield (both rolls, spot dodge, jump, grab).
#[test]
fn shield_exits_fd_marth_400_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("shield_exits_fd_marth", 400);
}

/// Jump-cancelled grab and up smash, dash jump, dash grab and boost grab.
#[test]
fn tech_interrupts_fd_marth_360_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("tech_interrupts_fd_marth", 360);
}

/// Interrupts out of a held crouch, then a standing side-B.
#[test]
fn squat_interrupts_fd_marth_380_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("squat_interrupts_fd_marth", 380);
}

/// Marth down-thrown by Fox gets up face up: DownStandU.
#[test]
fn getup_stand_fd_marth_280_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("getup_stand_fd_marth", 280);
}

/// Marth down-thrown by Fox gets up face up: DownFowardU.
#[test]
fn getup_rollf_fd_marth_280_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("getup_rollf_fd_marth", 280);
}

/// Marth down-thrown by Fox gets up face up: DownBackU.
#[test]
fn getup_rollb_fd_marth_280_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("getup_rollb_fd_marth", 280);
}

/// Marth down-thrown by Fox gets up face up: DownAttackU.
#[test]
fn getup_attack_fd_marth_280_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("getup_attack_fd_marth", 280);
}

/// Marth crouch-cancels Fox's jab (DamageN1 back into Squat).
#[test]
fn crouchcancel_victim_fd_marth_200_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("crouchcancel_victim_fd_marth", 200);
}

/// Dancing Blade's up branch, an aerial Dancing Blade landing mid-combo and
/// an aerial Shield Breaker charge landing.
#[test]
fn marth_special_branches_fd_marth_380_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("marth_special_branches_fd_marth", 380);
}

/// Fire Fox aimed at the ledge corner snaps up onto the stage.
#[test]
fn firefox_ledgecorner_fd_fox_200_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("firefox_ledgecorner_fd_fox", 200);
}

/// A grounded Reflector's loop sliding off the edge (LwLoop -> AirLwLoop).
#[test]
fn reflector_loop_walkoff_fd_fox_200_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("reflector_loop_walkoff_fd_fox", 200);
}

/// Marth techs a back throw's landing: Passive.
#[test]
fn tech_inplace_victim_fd_marth_240_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("tech_inplace_victim_fd_marth", 240);
}

/// Marth techs a back throw's landing: PassiveStandB.
#[test]
fn tech_rollb_victim_fd_marth_240_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("tech_rollb_victim_fd_marth", 240);
}

/// Marth techs a back throw's landing: PassiveStandF.
#[test]
fn tech_rollf_victim_fd_marth_240_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("tech_rollf_victim_fd_marth", 240);
}

/// Fox's intermediate forward-tilt angles (AttackS3HiS, AttackS3LwS).
#[test]
fn ftilt_angled_fd_fox_140_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("ftilt_angled_fd_fox", 140);
}

/// ftCo_SpecialAir_CheckInput's inclusive bounds: a stick at exactly
/// (0.6, -0.55) takes the down special in the air.
#[test]
fn airspecial_bound_fd_marth_200_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("airspecial_bound_fd_marth", 200);
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
    let expected = read_trace(
        melee_test_support::trace::open(&scenario.trace_path("tick.expected.jsonl")).unwrap(),
    )
    .unwrap();
    assert_eq!(expected.len(), 300);
    let ledger = melee_test_support::trace::read_to_string(&ledger_path).unwrap();
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

// S3 part 2: Fire Fox charge, launch, and special-fall recovery.
#[test]
fn firefox_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("firefox_fd_fox");
}

#[test]
fn airfirefox_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airfirefox_fd_fox");
}

#[test]
fn reflector_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("reflector_fd_fox");
}

#[test]
fn airreflector_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airreflector_fd_fox");
}

#[test]
fn reflectorjc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("reflectorjc_fd_fox");
}

// S3 boundary: damage.rs combo recording needs character special move IDs.
#[test]
fn dolphinslash_fd_marth_300_ticks_and_ordered_particle_draws() {
    special_gate("dolphinslash_fd_marth");
}

#[test]
fn shieldbreaker_fd_marth_300_ticks_and_ordered_particle_draws() {
    special_gate("shieldbreaker_fd_marth");
}

#[test]
fn dancingblade_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dancingblade_fd_marth");
}

#[test]
fn counter_fd_marth_300_ticks_and_ordered_particle_draws() {
    special_gate("counter_fd_marth");
}

// S10: the scripted four-stock match, start to GAME (RebirthWait drop, teeter, four KOs).
#[test]
fn match_fd_marth_scripted_1600_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("match_fd_marth_scripted", 1600);
}

// S9: high-percent KOs. The star KO flies for PlCo +508 frames (the trace covers 90 of
// them); the side exits are DeadRight with the clamped, rotated explosion.
#[test]
fn topko_usmash_fd_fox_230_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("topko_usmash_fd_fox", 230);
}

#[test]
fn hi200_utilt_fd_marth_420_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("hi200_utilt_fd_marth", 420);
}

#[test]
fn hi200_dolphinslash_fd_marth_420_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("hi200_dolphinslash_fd_marth", 420);
}

// S6: shield scenes and the prefix before the S3 Marth special boundary.
#[test]
fn shieldstun_ftilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("shieldstun_ftilt_fd_marth");
}

#[test]
fn shieldtilt_ftilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("shieldtilt_ftilt_fd_marth");
}

#[test]
fn powershield_ftilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("powershield_ftilt_fd_marth");
}

#[test]
fn lightshield_ftilt_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("lightshield_ftilt_fd_marth");
}

#[test]
fn shieldbreak_fd_marth_520_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("shieldbreak_fd_marth", 520);
}

#[test]
fn s6_shield_scenes_before_cross_lane_boundaries() {
    for (name, frames, prefix) in [
        ("powershield_ftilt_fd_marth", 300, 125),
        ("shieldbreak_fd_marth", 520, 118),
    ] {
        s6_shield_prefix(name, frames, prefix);
    }
}

fn s6_shield_prefix(name: &str, frames: usize, prefix: usize) {
    use melee_diff::{first_divergence, read_trace};
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
    assert_eq!(scenario.frames as usize, frames);
    let expected =
        read_trace(melee_test_support::trace::open(&scenario.expected_path()).unwrap()).unwrap();
    let ledger = melee_test_support::trace::read_to_string(&ledger_path).unwrap();
    assert_eq!(expected.len(), frames);
    assert_eq!(ledger.lines().count(), frames);
    assert!(prefix < frames);
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario).unwrap(),
    );
    for (tick, (expected, line)) in expected.iter().zip(ledger.lines()).take(prefix).enumerate() {
        let actual = simulation.tick().unwrap();
        trace::check_schema(&actual).unwrap();
        let divergence = first_divergence([expected], [&actual]);
        assert!(divergence.is_none(), "{name} tick {tick}: {divergence:?}");
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
    eprintln!("{name}: {prefix} ticks, 49 keys, 0 divergences; cross-lane boundary prefix only");
}

// S9 (long): the star KO through the vanish (twinkle 0x42D, stock loss) and the last-stock pause.
#[test]
fn topko_usmash_long_fd_fox_340_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("topko_usmash_long_fd_fox", 340);
}

// S11 pipeline: a human-played scene (P1 Marth from the terminal gamepad, P2 Fox idle).
#[test]
fn human_smoke_fd_marth_600_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("human_smoke_fd_marth", 600);
}

// S7/S8: complete throw, pummel, mash and quick ledge recordings.
#[test]
fn fthrow_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("fthrow_fd_marth");
}
#[test]
fn uthrow_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("uthrow_fd_marth");
}
#[test]
fn dthrow_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dthrow_fd_marth");
}
#[test]
fn pummel_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("pummel_fd_marth");
}
#[test]
fn grabmash_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("grabmash_fd_marth");
}
#[test]
fn ledgeattack_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("ledgeattack_fd_fox");
}
#[test]
fn ledgejump_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("ledgejump_fd_fox");
}
#[test]
fn ledgeroll_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("ledgeroll_fd_fox");
}

#[test]
fn hi200_uthrow_fd_marth_420_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("hi200_uthrow_fd_marth", 420);
}

#[test]
fn hi200_uthrow2_fd_marth_420_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("hi200_uthrow2_fd_marth", 420);
}

#[test]
fn aircounter_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("aircounter_fd_marth");
}
#[test]
fn aircounter_landing_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("aircounter_landing_fd_marth");
}
#[test]
fn aircounter_hit_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("aircounter_hit_fd_marth");
}

#[test]
fn aircounter_fall_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("aircounter_fall_fd_marth");
}

#[test]
fn fsmash_diagonal_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("fsmash_diagonal_fd_fox");
}

#[test]
fn fsmash_dash_diagonal_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("fsmash_dash_diagonal_fd_fox");
}

#[test]
fn fsmash_diagonal_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("fsmash_diagonal_fd_marth");
}

#[test]
fn fsmash_dash_diagonal_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("fsmash_dash_diagonal_fd_marth");
}

#[test]
fn hitstun_exit_fair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_exit_fair_fd_fox");
}

#[test]
fn hitstun_exit_fair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_exit_fair_fd_marth");
}

#[test]
fn hitstun_exit_nair_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_exit_nair_fd_fox");
}

#[test]
fn hitstun_exit_nair_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_exit_nair_fd_marth");
}

#[test]
fn hitstun_shield_priority_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_shield_priority_fd_fox");
}

#[test]
fn hitstun_shield_priority_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_shield_priority_fd_marth");
}

#[test]
fn hitstun_unbuffered_attack_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("hitstun_unbuffered_attack_fd_marth");
}

#[test]
fn reflectorturn_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("reflectorturn_fd_fox");
}

#[test]
fn reflectorturn_release_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("reflectorturn_release_fd_fox");
}

#[test]
fn airreflectorturn_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airreflectorturn_fd_fox");
}

#[test]
fn airreflectorturn_priority_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airreflectorturn_priority_fd_fox");
}

#[test]
fn airreflectorjc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airreflectorjc_fd_fox");
}

#[test]
fn airreflectortapjc_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airreflectortapjc_fd_fox");
}

#[test]
fn airreflectorturn_landing_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("airreflectorturn_landing_fd_fox");
}

#[test]
fn jumpcancel_upb_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("jumpcancel_upb_fd_fox");
}

#[test]
fn jumpcancel_upb_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("jumpcancel_upb_fd_marth");
}

#[test]
fn jumpcancel_upb_priority_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("jumpcancel_upb_priority_fd_fox");
}

#[test]
fn jumpcancel_upb_priority_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("jumpcancel_upb_priority_fd_marth");
}

#[test]
fn jumpcancel_upb_diagonal_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("jumpcancel_upb_diagonal_fd_fox");
}

#[test]
fn jumpcancel_upb_diagonal_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("jumpcancel_upb_diagonal_fd_marth");
}

#[test]
fn cstick_throw_back_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_back_fd_fox");
}

#[test]
fn cstick_throw_back_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_back_fd_marth");
}

#[test]
fn cstick_throw_down_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_down_fd_fox");
}

#[test]
fn cstick_throw_down_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_down_fd_marth");
}

#[test]
fn cstick_throw_down_pulse_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_down_pulse_fd_fox");
}

#[test]
fn cstick_throw_down_pulse_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_down_pulse_fd_marth");
}

#[test]
fn cstick_throw_forward_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_forward_fd_fox");
}

#[test]
fn cstick_throw_forward_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_forward_fd_marth");
}

#[test]
fn cstick_throw_horizontal_priority_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_horizontal_priority_fd_fox");
}

#[test]
fn cstick_throw_horizontal_priority_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_horizontal_priority_fd_marth");
}

#[test]
fn cstick_throw_main_priority_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_main_priority_fd_fox");
}

#[test]
fn cstick_throw_main_priority_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_main_priority_fd_marth");
}

#[test]
fn cstick_throw_up_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_up_fd_fox");
}

#[test]
fn cstick_throw_up_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("cstick_throw_up_fd_marth");
}

#[test]
fn illusion_start_landing_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("illusion_start_landing_fd_fox");
}

#[test]
fn firefox_charge_landing_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("firefox_charge_landing_fd_fox");
}

#[test]
fn firefox_ground_launch_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("firefox_ground_launch_fd_fox");
}

#[test]
fn firefox_floor_rebound_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("firefox_floor_rebound_fd_fox");
}

#[test]
fn firefox_end_air_landing_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate("firefox_end_air_landing_fd_fox");
}

#[test]
fn common_input_taunt_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("taunt_fd_fox");
}

#[test]
fn common_input_taunt_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("taunt_fd_marth");
}

#[test]
fn common_input_dash_escape_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("dash_escape_fd_fox");
}

#[test]
fn common_input_dash_escape_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("dash_escape_fd_marth");
}

#[test]
fn common_input_dash_shield_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("dash_shield_fd_fox");
}

#[test]
fn common_input_dash_shield_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("dash_shield_fd_marth");
}

#[test]
fn common_input_dash_taunt_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("dash_taunt_fd_fox");
}

#[test]
fn common_input_dash_taunt_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("dash_taunt_fd_marth");
}

#[test]
fn common_input_dash_late_shield_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("dash_late_shield_fd_fox");
}

#[test]
fn common_input_dash_late_shield_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("dash_late_shield_fd_marth");
}

#[test]
fn common_input_shield_grab_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("shield_grab_fd_fox");
}

#[test]
fn common_input_shield_grab_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("shield_grab_fd_marth");
}

#[test]
fn common_input_dash_late_shield_grab_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("dash_late_shield_grab_fd_fox");
}

#[test]
fn common_input_dash_late_shield_grab_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("dash_late_shield_grab_fd_marth");
}

#[test]
fn common_input_shield_cstick_jump_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("shield_cstick_jump_fd_fox");
}

#[test]
fn common_input_shield_cstick_jump_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("shield_cstick_jump_fd_marth");
}

#[test]
fn common_input_shield_delayed_power_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("shield_delayed_power_fd_fox");
}

#[test]
fn common_input_shield_delayed_power_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("shield_delayed_power_fd_marth");
}

#[test]
fn common_input_run_shield_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("run_shield_fd_fox");
}

#[test]
fn common_input_run_shield_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("run_shield_fd_marth");
}

#[test]
fn common_input_run_taunt_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("run_taunt_fd_fox");
}

#[test]
fn common_input_run_taunt_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("run_taunt_fd_marth");
}

#[test]
fn common_input_run_shield_grab_fd_fox_300_ticks_and_ordered_particles() {
    combat_gate("run_shield_grab_fd_fox");
}

#[test]
fn common_input_run_shield_grab_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("run_shield_grab_fd_marth");
}

#[test]
fn capture_revival_rebirth_timeout_fd_fox_900_ticks_and_ordered_particles() {
    combat_gate_ticks("rebirth_timeout_fd_fox", 900);
}

#[test]
fn capture_revival_rebirth_timeout_fd_marth_900_ticks_and_ordered_particles() {
    combat_gate_ticks("rebirth_timeout_fd_marth", 900);
}

#[test]
fn capture_revival_rebirth_shield_a_fd_fox_600_ticks_and_ordered_particles() {
    combat_gate_ticks("rebirth_shield_a_fd_fox", 600);
}

#[test]
fn capture_revival_rebirth_analog_shield_a_fd_fox_600_ticks_and_ordered_particles() {
    combat_gate_ticks("rebirth_analog_shield_a_fd_fox", 600);
}

#[test]
fn capture_revival_rebirth_held_shield_a_fd_fox_600_ticks_and_ordered_particles() {
    combat_gate_ticks("rebirth_held_shield_a_fd_fox", 600);
}

#[test]
fn capture_revival_grab_airborne_fd_foxmarth_600_ticks_and_ordered_particles() {
    combat_gate_ticks("grab_airborne_fd_foxmarth", 600);
}

#[test]
fn capture_revival_grab_airborne_fd_marthfox_600_ticks_and_ordered_particles() {
    combat_gate_ticks("grab_airborne_fd_marthfox", 600);
}

#[test]
fn ledge_input_ledge_cstick_attack_fd_fox_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_attack_fd_fox", 420);
}

#[test]
fn ledge_input_ledge_cstick_attack_fd_marth_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_attack_fd_marth", 420);
}

#[test]
fn ledge_input_ledge_cstick_escape_fd_fox_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_escape_fd_fox", 420);
}

#[test]
fn ledge_input_ledge_cstick_escape_fd_marth_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_escape_fd_marth", 420);
}

#[test]
fn ledge_input_ledge_cstick_drop_fd_fox_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_drop_fd_fox", 420);
}

#[test]
fn ledge_input_ledge_cstick_drop_fd_marth_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_drop_fd_marth", 420);
}

#[test]
fn ledge_input_ledge_cstick_priority_fd_fox_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_priority_fd_fox", 420);
}

#[test]
fn ledge_input_ledge_cstick_priority_fd_marth_420_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_cstick_priority_fd_marth", 420);
}

#[test]
fn ledge_input_ledge_timeout_fd_fox_1200_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_timeout_fd_fox", 1200);
}

#[test]
fn ledge_input_ledge_timeout_fd_marth_1200_ticks_and_ordered_particles() {
    combat_gate_ticks("ledge_timeout_fd_marth", 1200);
}

#[test]
fn fire_contact_firefox_charge_hit_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("firefox_charge_hit_fd_marth");
}

#[test]
fn fire_contact_firefox_travel_hit_fd_marth_300_ticks_and_ordered_particles() {
    combat_gate("firefox_travel_hit_fd_marth");
}

#[test]
fn laser_reflection_fresh_300_ticks_items_and_ordered_particles() {
    combat_gate("laser_reflect_fresh_fd_marth");
}

#[test]
fn laser_reflection_return_300_ticks_items_and_ordered_particles() {
    combat_gate("laser_reflect_return_boundary_fd_marth");
}

#[test]
fn laser_reflection_stale_300_ticks_items_and_ordered_particles() {
    combat_gate("laser_reflect_stale_fd_marth");
}

#[test]
fn laser_reflection_delayed_300_ticks_items_and_ordered_particles() {
    combat_gate("laser_reflect_delayed_timed_fd_marth");
}

#[test]
fn contact_closure_mutual() {
    combat_gate("clank_jab_s74_f122_fd_foxmarth");
}

#[test]
fn contact_closure_priority_fox() {
    combat_gate("clank_priority_fox_spaced_fd_foxmarth");
}

#[test]
fn contact_closure_priority_marth() {
    combat_gate("clank_priority_marth_spaced_fd_foxmarth");
}

#[test]
fn contact_closure_no_rebound() {
    combat_gate("clank_smash_norebound_spaced_fd_foxmarth");
}

#[test]
fn contact_closure_airborne_fox() {
    combat_gate("clank_airborne_fox_spaced_fd_foxmarth");
}

#[test]
fn contact_closure_airborne_marth() {
    combat_gate("clank_airborne_marth_spaced_fd_foxmarth");
}

#[test]
fn contact_closure_overflow() {
    combat_gate_ticks("laser_reflect_overflow_air_timed_fd_marth", 600);
}

#[test]
fn walljump_stopceil_fox_walljump() {
    combat_gate_ticks("walljump_right_underside_fd_fox_candidate", 450);
}

#[test]
fn walljump_stopceil_fox_ceiling() {
    combat_gate_ticks("stopceil_latejump266_fd_fox_candidate", 450);
}

#[test]
fn walljump_stopceil_marth_ceiling() {
    combat_gate_ticks("stopceil_left_latejump270_fd_marth_candidate", 450);
}

#[test]
fn walljump_stopceil_marth_wall_control() {
    combat_gate_ticks("walljump_left_underside_fd_marth_control", 450);
}

#[test]
fn air_capture_release_fox_up() {
    combat_gate_ticks("capture_jump_up_release_fd_foxmarth_candidate", 450);
}

#[test]
fn air_capture_release_marth_up() {
    combat_gate_ticks("capture_jump_up_release_fd_marthfox_candidate", 450);
}

#[test]
fn air_capture_release_fox_xy() {
    combat_gate_ticks("capture_jump_xy_latch_fd_foxmarth_candidate", 450);
}

#[test]
fn air_capture_release_marth_xy() {
    combat_gate_ticks("capture_jump_xy_latch_fd_marthfox_candidate", 450);
}

#[test]
fn air_capture_release_air_cut() {
    combat_gate_ticks(
        "capture_edge_fox_outward_stop43_jump109_grab107_candidate",
        450,
    );
}

#[test]
fn air_capture_release_air_jump() {
    combat_gate_ticks("capture_edge_fox_air_up_release_candidate", 450);
}

#[test]
fn revival_laser_invincibility() {
    combat_gate_ticks("revival_laser_fd_marth_candidate", 600);
}

#[test]
fn damage_fly_roll_t125() {
    combat_gate_ticks("damage_fly_roll_t125_fd_fox_candidate", 450);
}

#[test]
fn damage_fly_roll_dtilt_t132() {
    combat_gate_ticks("damage_fly_roll_dtilt_t132_fd_fox_candidate", 450);
}

#[test]
fn damage_fly_roll_crouch() {
    combat_gate_ticks("damage_fly_roll_crouch_fd_fox_candidate", 450);
}

/// Generated robustness-corpus matches (melee-replay `explore`, version 2),
/// replayed in retail from their four-stock boundary through the corpus
/// bridge (`harness/replay_to_scenario.py`) and compared from match start to
/// GAME. Each first exposed the retail behaviour named beside it.
const CORPUS_MATCHES: [(&str, usize); 48] = [
    // Phantom contacts beside a real hit; SDI callbacks after a special.
    ("corpus_v2_s0_e2a_p1", 3307),
    // Item hit damage counts, overlay replacement, charge sparkle suppression,
    // grounded mid-animation root-motion velocity.
    ("corpus_v2_s0_e12345678_p0", 2788),
    // TurnRun edge stop; Shield Breaker leaving the ground with one jump;
    // Illusion ghost hitlag against a shield.
    ("corpus_v2_s1_e12345678_p1", 4434),
    // Airborne knockback decay during Fire Fox's charge; a grab puts the
    // Blaster away (ftCommon_8007DB58).
    ("corpus_v2_s0_e49_p1", 5603),
    // Staled charged Shield Breaker; down tilt holding on the squat check.
    ("corpus_v2_s0_e539_p2", 5964),
    // Invincibility flash ownership; staled ledge attack; a screen KO
    // (DeadUpFall, the camera hit and the fall down the screen).
    ("corpus_v2_s0_e80000000_p1_screenko", 5208),
    // Shield SDI/ASDI during shield hitlag.
    ("corpus_v2_s0_e1_p0", 3385),
    // Reflector walking off the stage edge (ground-to-air, one jump), its
    // over-drift air friction, and a double jump inheriting turnFrames.
    ("corpus_v2_s1_edeadbeef_p0", 3563),
    // Knockback while charging a smash attack (PlCo +7C4).
    ("corpus_v2_s0_e12345678_p2", 1342),
    // Grab release only from CaptureWait's own callback; generator insertion
    // after an effect destruction.
    ("corpus_v2_s0_e80000000_p2", 2173),
    // A light hit on a prone fighter (DownDamage); particle lists re-sorted
    // only on retail's display passes (psFrameNum).
    ("corpus_v2_s0_effffffff_p2", 5351),
    // The Blaster is put away on death (death2_cb).
    ("corpus_v2_s1_e2a_p1", 3038),
    // Landing dust and script graphics queued in script order.
    ("corpus_v2_s0_e49_p0", 3791),
    // Boost grab out of a dash attack; effects sealed by a motion change
    // dispatch before the new script's graphics draw.
    ("corpus_v2_s1_e1_p0", 3371),
    // The Blaster's firing accessory is disarmed by leaving the loop.
    ("corpus_v2_s1_e1_p2", 6000),
    // Retail's ft_80084DB0 fast-fall check in the air Blaster and platform
    // drop physics.
    ("corpus_v2_s1_e2a_p2", 6000),
    ("corpus_v2_s1_e12345678_p2", 4750),
    // No extra color step after a shield hit: Fighter_ChangeMotionState owns it.
    ("corpus_v2_s0_effffffff_p1", 3343),
    // Shield out of a forward smash, never a spot dodge (ftCo_AttackS4_IASA);
    // a per-bone hurt state reaches only the bone's first capsule;
    // joint caches refresh only on display passes.
    ("corpus_v2_s1_e49_p2", 6000),
    ("corpus_v2_s0_edeadbeef_p2", 4559),
    ("corpus_v2_s1_e49_p1", 6000),
    // Further generated matches that reach GAME or the explorer's tick cap.
    ("corpus_v2_s0_edeadbeef_p1", 4861),
    ("corpus_v2_s0_e1_p2", 4245),
    ("corpus_v2_s0_e12345678_p1", 3129),
    ("corpus_v2_s0_e2a_p0", 2407),
    ("corpus_v2_s0_e539_p0", 4112),
    ("corpus_v2_s0_e539_p1", 4208),
    ("corpus_v2_s0_edeadbeef_p0", 1692),
    ("corpus_v2_s1_e1_p1", 4619),
    ("corpus_v2_s1_e49_p0", 4239),
    ("corpus_v2_s1_e539_p0", 4182),
    ("corpus_v2_s1_e539_p1", 1753),
    ("corpus_v2_s1_e539_p2", 6000),
    ("corpus_v2_s1_edeadbeef_p1", 3585),
    ("corpus_v2_s1_edeadbeef_p2", 2343),
    ("corpus_v2_s1_effffffff_p0", 2420),
    ("corpus_v2_s1_effffffff_p2", 4248),
    // The gameplay camera: an off-screen fighter's magnifier damage every
    // PlCo +7AC ticks, rendered only on display passes (magnifier before the
    // main camera); the HUD shake lost to a same-tick death and cleared by
    // the revival.
    ("corpus_v2_s1_e80000000_p1", 2989),
    ("corpus_v2_s0_e80000000_p0", 3852),
    ("corpus_v2_s1_e80000000_p0", 5803),
    ("corpus_v2_s1_e80000000_p2", 3000),
    ("corpus_v2_s1_e2a_p0", 2871),
    // Fox's Illusion ghost against fighter defenses: Marth's down smash
    // clanks with the item hitbox (ftColl_80077970) and his Counter catches
    // it as a shield volume (ftColl_80077688, the Counter's own item hitlag).
    ("corpus_v2_s0_e1_p1", 4271),
    ("corpus_v2_s0_effffffff_p0", 6000),
    // Shield recoil decays in the air (Fighter_procUpdate), so Dolphin Slash
    // leaving the ground after a shielded hit carries 0.62, not 0.67.
    ("corpus_v2_s1_e12345678_p0", 6000),
    // DownDamage keeps the prone fighter's facing after the knockback used
    // the hit's (ftCo_8008DCE0's facing argument) and skips ftCommon_8007DB58.
    ("corpus_v2_s0_e49_p2", 6000),
    // An Illusion cut short before s_link 9 never creates its trail: the
    // motion change clears accessory4 (ftFx_SpecialS_CreateGFX).
    ("corpus_v2_s1_effffffff_p1", 6000),
    // Shield Breaker's gusts (lb_800119DC from the hips every 30 charge ticks
    // and on the release's frame 9) push Marth's cape and Fox's tail; the
    // tail's hurtbox then decides a later hit.
    ("corpus_v2_s0_e2a_p2", 4337),
];

/// A one-minute match idles into TIME! (the timer runs once GO ends and
/// times out at 0:00 with 59 frames), then Sudden Death after the tie: after
/// twenty seconds Bob-ombs rain (Ground_801C0C2C), spin as they fall, explode
/// on landing, and one KOs Marth at 300%. Marth jabs beside one, and picks
/// another up with A (ftpickupitem_80094790, LightGet) and holds it in his
/// item idle until it blows up in his hand (it_8027429C) and KOs him. His
/// forward smash detonates a falling one (it_802703E8, it_80270E30), and the
/// rain then drops a bomb where his star KO froze his player coordinates.
/// He also throws a held one forward (LightThrowF, ftCo_80095EFC) off the
/// stage, where its fuse runs out.
#[test]
fn timeout_and_sudden_death_match_retail() {
    for (name, ticks) in [
        ("timeout_tie_fd_marth", 3839),
        ("sudden_death_start_fd_marth", 1697),
        ("sudden_death_bombs_fd_marth", 1300),
        ("sudden_death_idle_fd_marth", 1576),
        ("sudden_death_jab_bomb_fd_marth", 1369),
        ("sudden_death_pickup_bomb_fd_marth", 1433),
        ("sudden_death_smash_bomb_fd_marth", 1496),
        ("sudden_death_throw_bomb_fd_marth", 1545),
        // ftCo_80095A30's directed and smash ground throws, a throw after
        // walking, and ftCo_80095328's air throw.
        ("sudden_death_throwb_bomb_fd_marth", 1545),
        ("sudden_death_throwhi_bomb_fd_marth", 1545),
        ("sudden_death_throwlw_bomb_fd_marth", 1394),
        ("sudden_death_throwf4_bomb_fd_marth", 1545),
        ("sudden_death_throwhi4_bomb_fd_marth", 1544),
        ("sudden_death_walkthrow_bomb_fd_marth", 1545),
        ("sudden_death_airthrow_bomb_fd_marth", 1451),
        // Z on the ground throws forward; Z in the air drops the Bob-omb
        // (ftCo_80095744, Item_8026ABD8, it_3F14_Logic6_Dropped); a rain
        // Bob-omb hits Fox and the held one explodes on Marth.
        ("sudden_death_zthrow_bomb_fd_marth", 1545),
        ("sudden_death_airdrop_bomb_fd_marth", 1545),
        ("sudden_death_hitholding_bomb_fd_marth", 1433),
        // Fox's early run changes the rain; his dash attack launches Marth.
        ("sudden_death_foxdash_fd_marth", 1545),
        // Shielding and dashing with the Bob-omb in hand; A in shield and
        // A mid-dash throw it (ftCo_8009515C, LightThrowDash).
        ("sudden_death_shieldhold_bomb_fd_marth", 1433),
        ("sudden_death_shieldthrow_bomb_fd_marth", 1545),
        ("sudden_death_dashhold_bomb_fd_marth", 1437),
        ("sudden_death_dashthrow_bomb_fd_marth", 1545),
        // Turning, crouching, rolling, landing and running with it.
        ("sudden_death_turnhold_bomb_fd_marth", 1545),
        ("sudden_death_crouchhold_bomb_fd_marth", 1433),
        ("sudden_death_rollhold_bomb_fd_marth", 1545),
        ("sudden_death_landhold_bomb_fd_marth", 1545),
        ("sudden_death_runhold_bomb_fd_marth", 1429),
        // Double jumping, a special and a taunt with it; a hit while holding
        // it rolls the drop chance.
        ("sudden_death_airjumphold_bomb_fd_marth", 1482),
        ("sudden_death_specialhold_bomb_fd_marth", 1433),
        ("sudden_death_taunthold_bomb_fd_marth", 1433),
        ("sudden_death_dashintofox_bomb_fd_marth", 1434),
        // LR + A in the air catches it (ftCo_800D7100): from a dash jump,
        // and from a jump out of shield that then lands holding it.
        ("sudden_death_aircatchdash_bomb_fd_marth", 1439),
        ("sudden_death_aircatchshield_bomb_fd_marth", 1513),
        // Fox smashes Marth while he holds it: the drop roll knocks it loose,
        // or he flies holding it, or dies holding it (the item is destroyed).
        ("sudden_death_knockloose_bomb_fd_marth", 1542),
        ("sudden_death_launchhold_bomb_fd_marth", 1545),
        ("sudden_death_kohold_bomb_fd_marth", 1540),
        // A Bob-omb blast on a grab pair (ftCo_8008EC90): both launched, or
        // only the captor, whose second throw record launches the victim.
        ("sudden_death_grabbomb_fd_marth", 1545),
        ("sudden_death_grabbombcaptor_fd_marth", 1471),
        // Only the held fighter launched: ftCo_800DE2F0's PlCo +380 hit on its captor.
        ("sudden_death_releasecaptor_bomb_fd_marth", 1415),
        // A pummel landing as a Bob-omb launches the captor: ftCo_800DE854 swaps
        // in the captor's throw record 1; Fox's DownBound requests a Large quake
        // that the screen KO's camera-space placement sees.
        ("sudden_death_pummelcaptor_bomb_fd_marth", 1471),
        // Fox's slow ledge options at 300% (CliffClimbSlow, CliffEscapeSlow,
        // CliffAttackSlow, CliffJumpSlow1/2) after an edge turn into CliffCatch.
        ("sudden_death_ledgeclimb_fd_fox", 1370),
        ("sudden_death_ledgeroll_fd_fox", 1370),
        ("sudden_death_ledgeattack_fd_fox", 1370),
        ("sudden_death_ledgejump_fd_fox", 1370),
        // A laser meets a thrown Bob-omb and blows it up mid-air.
        ("sudden_death_laserbomb_fd_marth", 1545),
        // Fox's Reflector turns a thrown Bob-omb back (it_80273030).
        ("sudden_death_reflectbomb_fd_marth", 1545),
        // A thrown Bob-omb hits Fox's shield.
        ("sudden_death_shieldbomb_fd_marth", 1472),
        // Marth's aerial Counter triggered by a Bob-omb's blast.
        ("sudden_death_counterbomb_fd_marth", 1471),
        // Fox's Illusion into a thrown Bob-omb: its speed adds contact
        // damage (it_8026B1D4) and the hit points against its velocity.
        ("sudden_death_illusionbomb_fd_marth", 1545),
        // Fox forward-smashes a thrown Bob-omb as it arrives.
        ("sudden_death_smashbomb_fd_fox", 1545),
        // Fox catches a falling Bob-omb in the air, lands and throws it at Marth.
        ("sudden_death_foxcatchthrow_bomb_fd_marth", 1481),
        // A thrown Bob-omb bouncing off the wall under the stage (it_80276FC4).
        ("sudden_death_wallbomb_fd_marth", 1441),
        ("sudden_death_wallbombslope_fd_marth", 1441),
        // TurnRun holding it; a blast knocks it loose, it lands softly
        // (fn_80280974 -> the walk) and a second blast sets it off.
        ("sudden_death_turnrunhold_bomb_fd_marth", 1540),
        // A Bob-omb knocked from a low hand walks (itBombhei_UnkMotion2)
        // until it relights; a walking one's blast sets off a falling one
        // (it_802706D0, item on item).
        ("sudden_death_walkbomb_fd_marth", 1422),
        ("sudden_death_bombchain_fd_marth", 1523),
        // Hanging on the ledge, and jumping from it, holding it.
        ("sudden_death_ledgehold_bomb_fd_marth", 1545),
        ("sudden_death_ledgejump_bomb_fd_marth", 1545),
        // C-stick smashes with it throw it (LightThrowB4/Hi4/Lw4).
        ("sudden_death_cstickb4_bomb_fd_marth", 1420),
        ("sudden_death_cstickhi4_bomb_fd_marth", 1544),
        ("sudden_death_csticklw4_bomb_fd_marth", 1394),
        // An air dodge and its landing holding it; grabbed holding it, the
        // blast launching both with the captured member first.
        ("sudden_death_airdodgehold_bomb_fd_marth", 1433),
        ("sudden_death_grabbedhold_bomb_fd_marth", 1430),
        // Thrown holding it: released before the blast (back, up), or blasted
        // mid-throw (forward, down; ftCo_800DC920's constrained release).
        ("sudden_death_thrownbhold_bomb_fd_marth", 1526),
        ("sudden_death_thrownhihold_bomb_fd_marth", 1545),
        ("sudden_death_thrownfhold_bomb_fd_marth", 1545),
        ("sudden_death_thrownlwhold_bomb_fd_marth", 1431),
        // A smash throw out of Turn.
        ("sudden_death_turnthrow_bomb_fd_marth", 1514),
        // A in a run shield's countdown: LightThrowDash (ftCo_8009515C).
        ("sudden_death_runshieldthrow_bomb_fd_marth", 1531),
    ] {
        combat_gate_ticks(name, ticks);
    }
}

#[test]
fn corpus_v2_matches_through_game() {
    for (name, ticks) in CORPUS_MATCHES {
        combat_gate_ticks(name, ticks);
    }
}

/// Corpus version 3 (`explore <dir> <count>`, seeds from a xorshift of
/// 0x00C0FFEE): the matches that faulted the port. Together they exposed
/// that a motion change ends the attack interaction, that grabs and throws
/// stop at the edge (ft_800841B8 -> ft_800827A0), that a dying grabber
/// releases its victim (ftCo_800DD100), that thrown positioning waits out
/// hitlag (Fighter_CallAcessoryCallbacks_8006C624) and that a motion change
/// drops the Counter volume (fighter.c:1049, `x221B_b0`).
const CORPUS_V3_MATCHES: [(&str, usize); 44] = [
    // Marth grabbed out of Counter takes the pummel as CaptureDamageLw.
    ("corpus_v3_s1_e9943b4ab_p0", 700),
    ("corpus_v3_s1_ec0a10b25_p1", 420),
    ("corpus_v3_s0_e6cc80d32_p1", 957),
    ("corpus_v3_s0_e720659b1_p1", 327),
    ("corpus_v3_s0_e9943b4ab_p2", 3479),
    ("corpus_v3_s0_ed97ea327_p0", 3009),
    ("corpus_v3_s1_e6cc80d32_p2", 4559),
    ("corpus_v3_s1_e720659b1_p0", 3317),
    ("corpus_v3_s1_ee62c6106_p0", 2854),
    // The second batch (seeds 41..80), bridged one tick later to match
    // Match::new's completed boundary tick: Fox landing out of an air
    // Illusion inherits ghostEffectPos[0].x as mv+4; an airborne residual
    // shield recoil (the invisible-ceiling store) decays in Fall.
    ("corpus_v3_s1_e2af099ca_p1", 4519),
    ("corpus_v3_s1_e4068796b_p0", 2877),
    // The v5 sample. Counter catching Fire Fox with a stale powershield
    // window takes ftColl_80076CBC's powershield branch (3400).
    ("corpus_v3_s0_e1cda1301_p0", 6001),
    // A ground Illusion dash leaving the stage the tick it starts drops its
    // accessory4 trail (1038).
    ("corpus_v3_s0_e4f8edfa8_p0", 2972),
    // Marth's FallSpecial cannot catch the ledge Fox holds (3631).
    ("corpus_v3_s0_e6d8e8b19_p0", 5710),
    // The rotating effect bone survives a stock loss (3376); landing dust
    // draws after the motion change dispatched the sealed overlay (3400).
    ("corpus_v3_s0_ec21c8082_p0", 5346),
    // Down tilt ending in SquatWait takes the up special its IASA matched.
    ("corpus_v3_s1_e8be4d273_p0", 1407),
    // DamageFlyTop ends through ftCo_80090780, keeping fast fall (2753).
    ("corpus_v3_s1_e46703f61_p0", 5948),
    // An airborne tumble launch shakes the camera, which delays the
    // magnifier's off-screen pass and its damage (1516).
    ("corpus_v3_s0_e0d368f02_p0", 3534),
    // A throw's damage waits in x1838 until ProcessHit, so crossing the
    // fly-roll percent at release draws no roll (4946).
    ("corpus_v3_s0_e8be4d273_p0", 6001),
    // Marth thrown into the stage's side techs the wall (PassiveWall, 713)
    // and acts out of it once the freeze ends.
    ("corpus_v3_s1_edb4b01fd_p0", 2332),
    // A Reflector started at the stage's edge leaves the ground at once:
    // the motion change uninstalls accessory4, so no start effect (2132).
    ("corpus_v3_s1_ec21c8082_p0", 4629),
    // A multi-hit tumble combo stacks five small camera quakes.
    ("corpus_v3_s0_e46028c49_p0", 4252),
    // A Counter that landed mid-window catches a get-up attack without its
    // minimum hitlag: the transition zeroed shield_unk0 (629).
    ("corpus_v3_s0_e5f386e5e_p0", 6001),
    // A tumbling Fox driven into the stage's wall bounces off it
    // (FlyReflectWall, 4065).
    ("corpus_v3_s0_e0211286e_p1", 5852),
    // Marth meteor-cancels Fox's down throw off the ledge with his double
    // jump (4766); the match ends in its final-KO freeze.
    ("corpus_v3_s1_e8be4d273_p1", 5019),
    // Marth's aerial Counter catches a laser, which bounces off at the
    // fixed counter angle (ftColl_80077688, 3571).
    ("corpus_v3_s0_e6117d326_p2", 3883),
    // A fast-falling MissFoot ends in DamageFall still fast falling (2895).
    ("corpus_v3_s1_e317831ba_p2", 4080),
    // Fox hit off the floor while prone stays on hitstun air physics
    // through DownDamageD (487).
    ("corpus_v3_s0_e0fe4dd03_p1", 6001),
    // Fox rolls back out of shield, waits and reflects: the Reflector's
    // turn word is Guard's tilt (mv.co.guard.x4), kept through the roll.
    ("corpus_v3_s0_e551dd0aa_p0", 2064),
    // Sudden Death explorer (`explore ... sudden-death`): a held Bob-omb
    // leaves the hand while its hand animation is live (ftAnim_80070CC4).
    ("corpus_sd_s1_ea4d5d9ec_p0", 1773),
    ("corpus_sd_s1_ebac668de_p0", 1697),
    // ... with no motion attached (ftAnim_8006FA58's descriptor pose).
    ("corpus_sd_s1_e6b4c2898_p2", 1812),
    ("corpus_sd_s1_efe39c2d1_p0", 1654),
    ("corpus_sd_s1_e98f1f78c_p1", 1739),
    ("corpus_sd_s1_e0c5e5610_p1", 1707),
    ("corpus_sd_s1_eea202b0d_p2", 1532),
    ("corpus_sd_s1_e7d180139_p1", 1703),
    ("corpus_sd_s1_eb1b11f7e_p1", 1581),
    // FlyReflectCeil off FD's underside, and a ceiling tech (PassiveCeil).
    ("corpus_v3_s0_ef4efb740_p1", 5740),
    ("corpus_v3_s0_ef4efb740_p1_ceiltech", 5740),
    // A Reflector inheriting Turn's mv+4 through an up smash and Wait.
    ("corpus_v3_s0_efaccaf3d_p0", 1400),
    // Full generated matches from the 3000+ seeds, one per port layout.
    ("corpus_v3_s0_e035918d1_p1", 6001),
    ("corpus_v3_s1_e42b75250_p1", 6001),
    // A special fall entered on the ground (ftCommon_8007D60C).
    ("corpus_v3_s1_e7d968d2d_p1", 3946),
];

#[test]
fn corpus_v3_matches_retail() {
    for (name, ticks) in CORPUS_V3_MATCHES {
        combat_gate_ticks(name, ticks);
    }
}
