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

/// Gate every listed scenario with `combat_gate_ticks`, across the
/// machine's cores: the lists hold hundreds of recordings and one thread
/// takes minutes. A failing scenario panics its worker with the usual
/// message and fails the test once the others finish.
fn gate_in_parallel(scenarios: &[(&str, usize)]) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(1, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers.min(scenarios.len()) {
            scope.spawn(|| {
                while let Some(&(name, ticks)) = scenarios.get(next.fetch_add(1, Ordering::Relaxed))
                {
                    combat_gate_ticks(name, ticks);
                }
            });
        }
    });
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
        || name.starts_with("yoshi_upb_")
        || name.starts_with("peach2_")
        || name.starts_with("slope_")
        || name.starts_with("peach3_")
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
                | "slp_bf_fox_falco_phantasm_t300"
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
    // The ledger run's own display clock, as `melee-sim particle-sites`
    // reads it: each Dolphin run renders on its own ticks (host timing), and
    // the particle order the ledger's draws follow is re-sorted only then.
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&ledger_path)
            .unwrap(),
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

// A down tilt repeated out of its own IASA is a new attack instance
// (callUnk 8008BC00 -> ft_800892A0): the third hit's damage shows two stale entries.
#[test]
fn dtilt_repeat_stale_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate("dtilt_repeat_stale_fd_marth");
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

/// DamageFall exits after hitstun: an aerial jump and Fire Fox.
#[test]
fn damagefall_jump_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("damagefall_jump_fd_fox", 300);
}

#[test]
fn damagefall_upb_fd_fox_300_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("damagefall_upb_fd_fox", 300);
}

/// A grounded Reflector turn sliding off the edge (LwTurn -> AirLwTurn).
#[test]
fn reflector_turn_walkoff_fd_fox_200_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("reflector_turn_walkoff_fd_fox", 200);
}

/// A capture over the edge goes high (CaptureWaitHi); a back throw from it.
#[test]
fn capture_hi_edge_throwb_fd_marth_220_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("capture_hi_edge_throwb_fd_marth", 220);
}

/// Pummels in a high capture, then CaptureCut in the air.
#[test]
fn capture_hi_edge_pummel_fd_marth_220_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("capture_hi_edge_pummel_fd_marth", 220);
}

/// Level-1 reactions on Fox (DamageN1, DamageHi1, DamageLw1) from Dancing Blade.
#[test]
fn damage_level1_dancingblade_fd_marth_170_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("damage_level1_dancingblade_fd_marth", 170);
}

/// DamageAir1 on Fox from Dancing Blade.
#[test]
fn damage_air1_dancingblade_fd_marth_170_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("damage_air1_dancingblade_fd_marth", 170);
}

/// A landed Reflector end sliding off the edge (LwEnd -> AirLwEnd).
#[test]
fn reflector_end_walkoff_fd_fox_154_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("reflector_end_walkoff_fd_fox", 154);
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

/// A shield held until it decays and breaks inside the Guard proc: the
/// queued ShieldBreakFly script graphics and burst flush newest-first.
#[test]
fn shieldbreak_hold_fd_marth_700_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("shieldbreak_hold_fd_marth", 700);
}

/// Fox's jab 1 on a crouching Marth's head capsule and a taunting Marth's
/// legs: DamageHi1 and DamageLw1 (the only level-1 Hi/Lw pair on Marth).
#[test]
fn damage_level1_victim_fd_marth_225_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("damage_level1_victim_fd_marth", 225);
}

/// An aerial Counter catching Fox's neutral air ends in the air (AirLwHit -> Fall).
#[test]
fn counter_air_fall_fd_marth_230_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("counter_air_fall_fd_marth", 230);
}

/// Aerial Dancing Blade's fourth hit up (AirS4Hi) and down (AirS4Lw).
#[test]
fn marth_air_dancingblade4_fd_marth_340_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("marth_air_dancingblade4_fd_marth", 340);
}

/// A fully charged Shield Breaker released offstage (AirNEnd1); its 0x16D-0x170
/// generators are standalone AppSRT ones that outlive Marth's KO.
#[test]
fn marth_airn_end1_fd_marth_300_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("marth_airn_end1_fd_marth", 300);
}

/// Fox's push tips Marth's edge-stopped dash attack into Ottotto; then Catch.
#[test]
fn ottotto_marth_attackdash_fd_marth_250_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("ottotto_marth_attackdash_fd_marth", 250);
}

/// Run -> Ottotto at the edge, then OttottoWait -> jab.
#[test]
fn ottotto_marth_run_fd_marth_220_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("ottotto_marth_run_fd_marth", 220);
}

/// RunBrake -> Ottotto at the edge, then OttottoWait -> Catch.
#[test]
fn ottotto_marth_runbrake_fd_marth_220_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("ottotto_marth_runbrake_fd_marth", 220);
}

/// Fox caught in the air offstage cuts free into Fall.
#[test]
fn capture_hi_edge_cut_fall_fd_fox_266_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("capture_hi_edge_cut_fall_fd_fox", 266);
}

/// Marth pulled over the edge and held high takes a pummel (CaptureDamageHi).
#[test]
fn capture_hi_edge_pummel_victim_fd_marth_220_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("capture_hi_edge_pummel_victim_fd_marth", 220);
}

/// Marth at 115% tumbles (DamageFlyRoll) from Fox's forward smash.
#[test]
fn damage_fly_roll_victim_fd_marth_740_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("damage_fly_roll_victim_fd_marth", 740);
}

/// Fox jabs Marth lying face up (DownDamageU).
#[test]
fn downdamage_up_victim_fd_marth_245_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("downdamage_up_victim_fd_marth", 245);
}

/// Marth powershields Fox's forward tilt.
#[test]
fn powershield_ftilt_victim_fd_marth_215_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("powershield_ftilt_victim_fd_marth", 215);
}

/// Marth's slow ledge climb at 300% (CliffClimbSlow).
#[test]
fn sudden_death_ledgeclimb_fd_marth_1385_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("sudden_death_ledgeclimb_fd_marth", 1385);
}

/// Marth's slow ledge roll at 300% (CliffEscapeSlow).
#[test]
fn sudden_death_ledgeroll_fd_marth_1410_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("sudden_death_ledgeroll_fd_marth", 1410);
}

/// Sudden Death item holding and the remaining matrix rows, recorded from
/// searched recipes (2026-09-27): Fox holding a Bob-omb through throws,
/// movement, specials, hits and grabs; Marth's air throws; catches; wall
/// jumps, Fire Fox walls, DI/SDI and a wall tech on Marth.
const MATRIX_WITNESSES: [(&str, usize); 431] = [
    ("sudden_death_airdrop_bomb_fd_fox", 1330),
    ("sudden_death_airreflect_bomb_fd_fox", 1330),
    ("sudden_death_airthrow_bomb_fd_fox", 1330),
    ("sudden_death_airthrowb_bomb_fd_fox", 1330),
    ("sudden_death_airthrowb_bomb_fd_marth", 1330),
    ("sudden_death_airthrowb4_bomb_fd_fox", 1330),
    ("sudden_death_airthrowf4_bomb_fd_fox", 1325),
    ("sudden_death_airthrowf4_bomb_fd_marth", 1330),
    ("sudden_death_airthrowhi_bomb_fd_fox", 1330),
    ("sudden_death_airthrowhi_bomb_fd_marth", 1330),
    ("sudden_death_airthrowhi4_bomb_fd_fox", 1330),
    ("sudden_death_airthrowhi4_bomb_fd_marth", 1330),
    ("sudden_death_airthrowlw_bomb_fd_fox", 1325),
    ("sudden_death_airthrowlw_bomb_fd_marth", 1330),
    ("sudden_death_airthrowlw4_bomb_fd_fox", 1320),
    ("sudden_death_airthrowlw4_bomb_fd_marth", 1330),
    ("sudden_death_catchthrown_bomb_fd_fox", 1335),
    ("sudden_death_cstickb4_bomb_fd_fox", 1325),
    ("sudden_death_cstickhi4_bomb_fd_fox", 1330),
    ("sudden_death_csticklw4_bomb_fd_fox", 1325),
    ("sudden_death_dashthrow_bomb_fd_fox", 1300),
    ("sudden_death_fallspecialcatch_bomb_fd_marth", 1335),
    ("sudden_death_grabbedhold_bomb_fd_fox", 1335),
    ("sudden_death_hitholding_bomb_fd_fox", 1310),
    ("sudden_death_knockloose_bomb_fd_fox", 1320),
    ("sudden_death_landhold_bomb_fd_fox", 1335),
    ("sudden_death_ottottohold_bomb_fd_fox", 1335),
    ("sudden_death_reflectorhold_bomb_fd_fox", 1335),
    ("sudden_death_rollthrow_bomb_fd_fox", 1310),
    ("sudden_death_squatthrow_bomb_fd_fox", 1300),
    ("sudden_death_taunthold_bomb_fd_fox", 1335),
    ("sudden_death_throwb_bomb_fd_fox", 1330),
    ("sudden_death_throwf4_bomb_fd_fox", 1330),
    ("sudden_death_throwhi_bomb_fd_fox", 1330),
    ("sudden_death_throwhi4_bomb_fd_fox", 1310),
    ("sudden_death_throwlw_bomb_fd_fox", 1330),
    ("sudden_death_thrownbhold_bomb_fd_fox", 1330),
    ("sudden_death_thrownfhold_bomb_fd_fox", 1330),
    ("sudden_death_thrownhihold_bomb_fd_fox", 1330),
    ("sudden_death_thrownlwhold_bomb_fd_fox", 1330),
    ("sudden_death_turnrunhold_bomb_fd_fox", 1335),
    ("sudden_death_zthrow_bomb_fd_fox", 1315),
    ("sudden_death_shieldlaserbomb_fd_marth", 1275),
    ("di_sdi_upin_fsmash_victim_fd_marth", 430),
    ("firefox_shallow_floor_fd_fox", 137),
    ("firefox_wall_ledge_fd_fox", 195),
    ("firefox_wall_notch_fd_fox", 220),
    ("laser_loop_pushoff_fd_fox", 155),
    ("passivewall_victim_fd_marth", 400),
    ("walljump_left_underside_fd_fox", 182),
    ("walljump_left_vertical_fd_fox", 175),
    ("sudden_death_specialhold_bomb_fd_fox", 1335),
    ("passivewall_downdamage_victim_fd_marth", 216),
    ("passivewall_thrownhi_victim_fd_marth", 230),
    ("sudden_death_catchthrown_bomb_fd_marth", 1312),
    ("sudden_death_firefoxhold_bomb_fd_fox", 1315),
    ("sudden_death_illusionhold_bomb_fd_fox", 1312),
    ("sudden_death_kohold_bomb_fd_fox", 1325),
    ("sudden_death_rollthrow_bomb_fd_marth", 1305),
    ("sudden_death_runshieldthrow_bomb_fd_fox", 1300),
    ("sudden_death_turnthrow_bomb_fd_fox", 1285),
    ("bf_revival_marth_fox4_platform", 480),
    // Guard IASA platform drop (ftCo_8009A080) through Battlefield's top platform.
    ("bf_shielddrop_guard_pass", 150),
    // Reflector Start/Loop platform drop (ftFx_SpecialLw{Start,Loop}_CheckPass).
    ("bf_reflectorpass_start_fox", 230),
    ("bf_reflectorpass_loop_fox", 230),
    ("yoshi_armor_jab_fd_fox4", 260),
    ("yoshi_armor_smash_fd_fox4", 260),
    // ftCo_800DEA28 default arm: AppealSR, turn, then AppealSL.
    ("taunt_start_fd_captainfalcon_fox4", 600),
    ("taunt_start_fd_peach_fox4", 600),
    ("taunt_start_fd_yoshi_fox4", 600),
    ("taunt_start_fd_jigglypuff_fox4", 600),
    // Peach's float (ftpeachfloat.c): entries, release, timeout, and the
    // five float aerials returning to Float or falling after release.
    ("peach_float_release_back", 280),
    ("peach_float_timeout", 480),
    ("peach_float_down_x", 280),
    ("peach_float_nair_return", 400),
    ("peach_float_fair_release", 300),
    ("peach_float_dair_cstick", 320),
    ("peach_float_bair_uair", 420),
    // Falcon Punch: grounded to Wait; aerial landing mid-punch; aerial
    // lunges aimed up and down off stage (efAlt 0x48F, efAsync 0x446).
    ("falcon_punch_fd_falcon", 180),
    ("falcon_airpunch_up_fd_falcon", 240),
    ("falcon_airpunch_down_fd_falcon", 260),
    ("falcon_airpunch_offstage_up_fd_falcon", 200),
    ("falcon_airpunch_offstage_down_fd_falcon", 200),
    // Egg Throw (ftyoshispecialhi.c): grounded, charged, the egg's flight
    // hitbox bursts on Fox; aerial, aimed back, landing in SpecialAirHi
    // after the throw (fn_8012E44C) and the egg bursting on the floor.
    ("yoshi_upb_ground_fd_fox4", 360),
    ("yoshi_upb_air_fd_fox4", 360),
    // Falco's down-throw lasers: light non-captor hits keep the grab, and
    // Fox's hitlag holds Falco frozen past its own (x1A5C / x2219_b7).
    ("captured_hit_falco_dthrow_laser_fd", 330),
    // Yoshi's Egg Lay (ftyoshispecialn.c): the tongue's special grab
    // (category 4), CaptureYoshi on TransN2, the swallow's release
    // (ftCo_800DE2CC) into YoshiEgg, its growth, mash and timer escapes;
    // aerial catches and misses.
    ("yoshi_neutralb_egg_fd_fox4", 400),
    ("yoshi_neutralb_egg_mash_fd_fox4", 400),
    ("yoshi_neutralb_egg_air4_fd_fox4", 500),
    ("yoshi_neutralb_egg_air_fd_fox4", 500),
    ("yoshi_neutralb_egg_air2_fd_fox4", 500),
    ("yoshi_neutralb_egg_air3_fd_fox4", 500),
    ("yoshi_neutralb_whiff_air_fd_fox4", 200),
    // Yoshi Bomb (ftyoshispeciallw.c): grounded and aerial, the descent's hit
    // and the landing stars (it_802B2FC8) hitting Fox.
    ("yoshi_sidedown_bomb_ground", 300),
    ("yoshi_sidedown_bomb_air", 300),
    // Egg Roll (ftyoshispecials.c): hop, aerial and ground loops, turn, the
    // roll's hit on Fox (fn_8012EFF4), ground and aerial breaks, KO rolling.
    ("yoshi_sidedown_roll_ground", 300),
    ("yoshi_sidedown_roll_hit", 300),
    ("yoshi_sidedown_roll_air", 300),
    // Yoshi's mouth hold escaped (ftCo_800DC920's x2226_b2 release from
    // XRotN against Yoshi's CatchCut pose): with a buffered jump (CaptureJump
    // lands at once into Wait) and without (CaptureCut); a hit on the egg
    // (dmg.x182c_behavior damage scale, ftCo_800BC3D0's timer cut, no reaction).
    ("yoshi_hold_mouth_escape_jump", 900),
    ("yoshi_hold_mouth_escape_cut", 900),
    ("yoshi_hold_egg_hit", 420),
    // Peach Parasol (ftpeachspecialhi.c) and the shared parasol states:
    // ground and aerial starts with steering and reversal, the opening and
    // special fall, closing to FallSpecial and reopening, landing, and the
    // parasol knocked out of her hand by a hit.
    ("peach2_upb_float_land", 330),
    ("peach2_upb_close_reopen", 360),
    ("peach2_upb_air_steer", 380),
    ("peach2_upb_hit", 300),
    // Toad (ftpeachspecialn.c): ground and aerial, and the counter against a
    // laser and a dash attack, whose spores hit Fox.
    ("peach2_toad_ground", 220),
    ("peach2_toad_air", 300),
    ("peach2_toad_counter_laser", 260),
    ("peach2_toad_counter_melee", 280),
    // ftPe_AttackS4_Enter: three forward smashes redraw HSD_Randi(3) away
    // from the previous club/pan/racket; one charged, one turning round.
    ("peach2_fsmash_sequence", 360),
    // Falcon Kick: grounded (to 358 and Wait, a hit slowing it through
    // deal_dmg_cb), aerial landings (360), aerial ends in the air (361,
    // the swapped ftCa_SM animation order) and off-stage KOs.
    ("falcon_specials_kick_ground", 160),
    ("falcon_specials_kick_hit_run22", 220),
    ("falcon_specials_kick_hit_run28", 220),
    ("falcon_specials_kick_edge", 200),
    ("falcon_specials_kick_air", 160),
    ("falcon_specials_kick_air_high", 200),
    ("falcon_specials_kick_offstage", 240),
    ("falcon_specials_kick_airhit_run26", 240),
    ("falcon_specials_kick_airhit_run32", 240),
    // Raptor Boost: grounded and aerial startups that miss (Wait, FallSpecial,
    // LandingFallSpecial) and inert-hitbox detections into the lunge.
    ("falcon_specials_boost_hit", 160),
    ("falcon_specials_boost_miss", 200),
    ("falcon_specials_boost_air_miss", 200),
    ("falcon_specials_boost_air_hit", 200),
    ("falcon_specials_boost_offstage", 260),
    ("falcon_specials_boost_hit_run26", 220),
    ("falcon_specials_boost_hit_run32", 220),
    ("falcon_specials_boost_airhit_run26", 240),
    ("falcon_specials_boost_airhit_run32", 240),
    // Falcon Dive without a catch: grounded, reversed, aerial, and dives
    // beside Fox (ftAction_80071784's 26-bit hitbox index in run44).
    ("falcon_specials_dive_ground", 200),
    ("falcon_specials_dive_turn", 200),
    ("falcon_specials_dive_air", 220),
    ("falcon_specials_dive_catch_run36", 240),
    ("falcon_specials_dive_catch_run40", 240),
    ("falcon_specials_dive_catch_run44", 240),
    ("falcon_specials_dive_catch_run57", 300),
    ("falcon_specials_dive_aircatch_run20", 260),
    ("falcon_specials_dive_aircatch_run26", 260),
    ("falcon_specials_dive_aircatch_run36", 300),
    ("falcon_specials_dive_aircatch_run40", 300),
    ("falcon_specials_dive_aircatch_run44", 300),
    // Falcon Dive catches on a grounded Fox: Falcon hangs from the victim
    // (CaptureCaptain), the throw (356) releases and launches him.
    ("falcon_specials_dive_catch_run48", 300),
    ("falcon_specials_dive_catch_run51", 300),
    ("falcon_specials_dive_catch_run54", 300),
    // Yoshi's egg shield damage (344, ftYs_Shield_8012C600, an unanimated
    // row whose frame is ftAnim_8006F3DC's leftover f1): out of the hold into
    // a drain break (ftCo_800925A4 then the shell burst), out of an analog
    // and a powershield startup released during the stun (343), and a hit
    // that breaks the drained egg (ftCo_80098B20).
    ("yoshi_hold_egg_shield_hit", 360),
    ("yoshi_shield_release_stun", 232),
    ("yoshi_shield_powershield_stun", 232),
    // Yoshi's delayed egg powershield (ftYs_Shield_8012C850): a digital press
    // during GuardOn continues GuardOn_1 (345) at GuardOn's frame, against a
    // laser (not reflected) and a melee hit released in and held through
    // the stun; a press at the window's edge stays in GuardOn.
    ("yoshi_shield_delayed_power_laser", 300),
    ("yoshi_shield_delayed_power_laser_late", 300),
    ("yoshi_shield_delayed_power_stun", 232),
    ("yoshi_shield_delayed_power_hold", 300),
    ("yoshi_shield_hit_break", 399),
    // A spot dodge out of the egg's startup (ftCo_80099954, 0x80099954)
    // bursts the shell only while the egg model is selected
    // (x5F4_arr[0].idx == 1): not after a grab out of the egg put the body
    // back (252), and from the held egg it does (340).
    ("yoshi_shield_grab_then_spotdodge", 420),
    // Peach Bomber (ftpeachspecials.c): ground and aerial flights whose inert
    // hitbox touches Fox (unk_gobj -> doAirEnd0) and leaves the PeachExplode
    // blast, a smash-input flight (blast motion 1), one passing through a
    // shield (x221C_b5), and whiffs landing into the grounded recoil.
    ("peach3_bomber_ground_hit", 280),
    ("peach3_bomber_ground_whiff", 220),
    ("peach3_bomber_air_hit", 300),
    ("peach3_bomber_smash_shield", 300),
    ("peach3_bomber_smash_hit", 300),
    ("peach3_bomber_air_whiff", 260),
    // Vegetable (ftpeachspeciallw.c): the pull (HSD_Randi rare-item and face
    // draws, the turnip straight into her hand), then smash, tilt, down-B and
    // aerial down-B throws through the shared item throws, rebounding off
    // Fox and his shield, and meeting the floor; an empty-handed aerial
    // down-B does nothing.
    ("peach3_turnip_throw_hit", 300),
    ("peach3_turnip_downb_throw", 300),
    ("peach3_turnip_tilt_throw", 300),
    ("peach3_turnip_floor", 260),
    ("peach3_turnip_air_downb", 320),
    ("peach3_turnip_shield", 300),
    // Two turnips thrown up side by side clank, forget each other when the
    // rehit timer runs out and clank again every 16 ticks (it_8026FE68's
    // mode 4, 0x8026FEFC / 0x8027006C).
    ("peach_turnip_clank_twice_fd_peach", 400),
    // Vegetable's rare pull (pickVeg): an unlit Bob-omb in hand (state 7 at
    // attribute x0's rate), thrown into Fox (state 9, DmgDealt explosion)
    // and dropped in the air to a hard landing (fn_80280974 detonation).
    ("peach4_pull_bombhei_throw_hit", 700),
    ("peach4_pull_bombhei_air_drop", 700),
    // The Beam Sword pull: SwordSwing1/3/4 (ftswing.c) with the script's
    // blade command, a smash swing hitting Fox, the Z drop (LightThrowDrop,
    // Item_8026AC74), and a thrown sword bouncing off Fox and landing.
    ("peach4_pull_sword_swings", 1400),
    ("peach4_pull_sword_hits", 1700),
    // The Mr. Saturn pull: held, thrown into Fox (the voice's HSD_Randi),
    // landing into the walk, pushed by Fox (it_80271B60) and turning at
    // the edge.
    ("peach4_pull_dosei_throw_hit", 900),
    // Jigglypuff's specials against Fox: Rest's sleep hitbox, Sing's sleep
    // and wake, a full-charge Rollout hit, a Rollout that runs out, an aerial
    // Rollout that bounces and lands rolling, Pound into a shield.
    ("puff_rest_hit_fd_fox4", 480),
    ("puff_sing_sleep_fd_fox4", 425),
    ("puff_rollout_hit_fd_fox4", 360),
    ("puff_rollout_whiff_fd_fox4", 316),
    ("puff_rollout_air_fd_fox4", 360),
    ("puff_pound_shield_fd_fox4", 272),
    // Jigglypuff in each hat costume (ftPr_Init_8013C360; blue and green add
    // hat spring chains, ftCo_8009DC54): Rollout off the edge, KO, respawn,
    // Rest, Sing and Pound; and Rest's hit that KOs Fox.
    ("puff_hat_c1_rollout_ko_fd_fox4", 1320),
    ("puff_hat_c2_rollout_ko_fd_fox4", 1320),
    ("puff_hat_c3_rollout_ko_fd_fox4", 1320),
    ("puff_hat_c4_rollout_ko_fd_fox4", 1320),
    ("puff_hat_c1_rest_hit_fd_fox4", 480),
    ("puff_hat_c2_rest_hit_fd_fox4", 480),
    ("puff_hat_c3_rest_hit_fd_fox4", 480),
    ("puff_hat_c4_rest_hit_fd_fox4", 480),
    // Pikachu against Fox: the repeating jab (a new attack instance per
    // restart), the forward smash's hitlag callbacks and model effect, Skull
    // Bash tapped, charged in full, aerial, and into Fox; Quick Attack's
    // zips along the floor and through the air, second zips (ground to air,
    // air to ground, off the ledge), no second zip, and the stickless zip.
    ("pikachu_jab_fd_fox4", 300),
    ("pikachu_fsmash_fd_fox4", 240),
    ("pikachu_skullbash_fd_fox4", 300),
    ("pikachu_skullbash_charge_fd_fox4", 400),
    ("pikachu_skullbash_air_fd_fox4", 400),
    ("pikachu_sb_hit_fd_fox4", 420),
    ("pikachu_quick_up_fd_fox4", 360),
    ("pikachu_quick_none_fd_fox4", 360),
    ("pikachu_qa_right_up_fd_fox4", 360),
    ("pikachu_qa_diag_down_fd_fox4", 360),
    ("pikachu_qa_left_edge_fd_fox4", 420),
    ("pikachu_qa_air_left_up_fd_fox4", 420),
    ("pikachu_qa_air_down_fd_fox4", 420),
    ("pikachu_qa_hit_fd_fox4", 360),
    ("pikachu_qa_right_same_fd_fox4", 360),
    // Thunder Jolt: grounded, aerial and two at once; the ball rides the
    // crawler's animated joint 6 along the floor until it strikes Fox.
    ("pikachu_jolt_fd_fox4", 420),
    ("pikachu_jolt_air_fd_fox4", 420),
    ("pikachu_jolt2_fd_fox4", 480),
    // Thunder: the cloud chains bolt articles down to Pikachu; the strike
    // model follows the frame-0 script graphics (efAsync order), grounded
    // and in the air.
    ("pikachu_thunder_fd_fox4", 360),
    ("pikachu_thunder_air_fd_fox4", 400),
    // Pikachu replaying Pichu's Quick Attack and tilted Skull Bash witnesses.
    ("pikachu_quick_air_fd_fox4", 400),
    ("pikachu_quick_fd_fox4", 360),
    ("pikachu_quick_late_fd_fox4", 360),
    ("pikachu_skullbash_tilt_fd_fox4", 360),
    // PICHU: Pikachu's witnesses replayed by Pichu. PlPc.dat's scripts
    // hurt Pichu through ftAction_80072BF4 (self-damage: 1% per jolt,
    // Skull Bash and zip, 2% forward smash, 3% Thunder); the articles are
    // the It_Kind_Pichu_* rows with PlPc.dat's own article data.
    ("pichu_jab_fd_fox4", 300),
    ("pichu_fsmash_fd_fox4", 240),
    ("pichu_skullbash_fd_fox4", 300),
    ("pichu_skullbash_charge_fd_fox4", 400),
    ("pichu_skullbash_tilt_fd_fox4", 360),
    ("pichu_skullbash_air_fd_fox4", 400),
    ("pichu_sb_hit_fd_fox4", 420),
    ("pichu_quick_fd_fox4", 360),
    ("pichu_quick_up_fd_fox4", 360),
    ("pichu_quick_none_fd_fox4", 360),
    ("pichu_quick_late_fd_fox4", 360),
    ("pichu_quick_air_fd_fox4", 400),
    ("pichu_qa_right_up_fd_fox4", 360),
    ("pichu_qa_diag_down_fd_fox4", 360),
    ("pichu_qa_left_edge_fd_fox4", 420),
    ("pichu_qa_air_left_up_fd_fox4", 420),
    ("pichu_qa_air_down_fd_fox4", 420),
    ("pichu_qa_hit_fd_fox4", 360),
    ("pichu_qa_right_same_fd_fox4", 360),
    ("pichu_jolt_fd_fox4", 420),
    ("pichu_jolt_air_fd_fox4", 420),
    ("pichu_jolt2_fd_fox4", 480),
    ("pichu_thunder_fd_fox4", 360),
    ("pichu_thunder_air_fd_fox4", 400),
    // Mario Tornado mashed from the ground (row 350 stays grounded) and from a jump
    // (the rise spent, a second Tornado, landing into 349). A grounded tap never lifts
    // 349 off: 975 search candidates (entry 106..130, one tap 2..40 later), none.
    ("mario_tornado_ground_fd_fox4", 300),
    ("mario_tornado_air_fd_fox4", 300),
    // MARIO: the cape (reflector, turnaround on the ground, in the air and on a
    // shield, a whiff) and Fox reflecting the fireball; scenario headers say what retail does.
    ("mario_cape_whiff_fd_fox4", 300),
    ("mario_cape_turn_ground_fd_fox4", 300),
    ("mario_cape_turn_air_fd_fox4", 300),
    ("mario_cape_turn_shield_fd_fox4", 300),
    ("mario_cape_reflect_laser_fd_fox4", 300),
    ("mario_fireball_shine_fd_fox4", 300),
    // ROY: Marth's specials with Roy's data (ft-mars-family). Flare Blade
    // tapped, aerial, into Fox uncharged and charged a level, and held to
    // full charge (the 0x40F explosion and the opcode-51 10% recoil);
    // Double-Edge Dance grounded and aerial; Blazer; Counter whiffed, and
    // catching Fox's jab and laser on the ground and a laser in the air,
    // where only Roy's retaliation rewrites its hitbox damage.
    ("roy_flareblade_fd_fox4", 420),
    ("roy_flareblade_air_fd_fox4", 420),
    ("roy_flareblade_hit_fd_fox4", 231),
    ("roy_flareblade_charged_hit_fd_fox4", 300),
    ("roy_flareblade_full_fd_fox4", 620),
    ("roy_dancingblade_fd_fox4", 420),
    ("roy_dancingblade_air_fd_fox4", 420),
    ("roy_blazer_fd_fox4", 420),
    ("roy_counter_fd_fox4", 300),
    ("roy_counter_hit_fd_fox4", 241),
    ("roy_counter_laser_fd_fox4", 225),
    ("roy_aircounter_laser_fd_fox4", 225),
    // DRMARIO: Dr. Mario runs Mario's specials (ft-mario-family) with his own
    // attributes: the Super Sheet (reflector, turnaround on the ground, in the air
    // and on a shield, a whiff), the Megavitamin (thrown, reflected by Fox, into Fox
    // and his shield), Super Jump Punch, Dr. Tornado and the pill-holding side
    // taunt; scenario headers say what retail does.
    ("drmario_cape_whiff_fd_fox4", 300),
    ("drmario_cape_turn_ground_fd_fox4", 300),
    ("drmario_cape_turn_air_fd_fox4", 300),
    ("drmario_cape_turn_shield_fd_fox4", 300),
    ("drmario_cape_reflect_laser_fd_fox4", 300),
    ("drmario_fireball_shine_fd_fox4", 300),
    ("drmario_megavitamin_fd_fox4", 400),
    ("drmario_megavitamin_hit_fd_fox4", 400),
    ("drmario_super_jump_punch_fd_fox4", 400),
    ("drmario_tornado_ground_fd_fox4", 300),
    ("drmario_tornado_air_fd_fox4", 300),
    ("drmario_taunt_fd_fox4", 400),
    // GANON: the start boundary and its cold construction; the taunt's TopN gust
    // and doubled ftCo_800DEBD0; Captain Falcon's specials with Ganondorf's kind
    // arms (ft-captain-family): Warlock Punch (efSync 0x50B, the wind-up gusts,
    // aerial lunges off stage), Gerudo Dragon (0x50D..0x50F, misses and inert
    // detections into the lunge), Dark Dive (reversal, grounded and aerial
    // catches) and Wizard's Foot (0x50C; hits, landings, 361 and off stage).
    ("taunt_start_fd_ganondorf_fox4", 600),
    ("ganon_punch_ground", 360),
    ("ganon_punch_hit", 441),
    ("ganon_punch_air_up", 560),
    ("ganon_punch_air_down", 560),
    ("ganon_sideb_miss", 320),
    ("ganon_sideb_hit", 380),
    ("ganon_sideb_air_miss", 360),
    ("ganon_sideb_airhit", 390),
    ("ganon_upb_ground", 320),
    ("ganon_upb_air", 360),
    ("ganon_upb_catch", 429),
    ("ganon_upb_aircatch", 431),
    ("ganon_downb_ground", 300),
    ("ganon_downb_hit", 392),
    ("ganon_downb_air", 300),
    ("ganon_downb_airhit", 403),
    ("ganon_downb_air_high", 360),
    ("ganon_downb_offstage", 560),
    // SAMUS: explorer cases from start_fd_samus_fox4 extended by neutral
    // ticks past their first unported boundary; headers say what retail does.
    // The morph-ball roll (ftCo_80099390) and the jump thruster (efAlt 0x487).
    ("samus_x_e1502cb40_p2", 335),
    ("samus_x_edafcfddf_p2", 351),
    ("samus_x_e726cfdde_p1", 335),
    ("samus_x_ec13743d5_p2", 332),
    ("samus_x_e50814092_p2", 409),
    // Screw Attack (ftSs_SpecialHi): grounded take-off, aerial, a KO in
    // special fall and a hit through ftSs_Init_80128428.
    ("samus_x_e3e74affa_p2", 337),
    ("samus_x_e573e2d95_p2", 345),
    ("samus_x_e00f31913_p2", 364),
    ("samus_x_e3e74affa_p1", 486),
    // Missiles (ftSs_SpecialS, itsamusmissile.c): homing and super, grounded
    // and aerial, a homing hit, and Fox's Reflector turning one back; the
    // trail rides the model's grandchild (0 differing particle ticks).
    ("samus_x_e9943b4ab_p0", 349),
    ("samus_x_e573e2d95_p0", 352),
    ("samus_missile_homing_fd_fox4", 260),
    ("samus_missile_super_fd_fox4", 300),
    ("samus_missile_air_fd_fox4", 300),
    ("samus_missile_super_air_fd_fox4", 300),
    ("samus_missile_shine_fd_fox4", 300),
    // Charge Shot (ftSs_SpecialN, itsamuschargeshot.c): charging to full
    // (colour 53 and ftSs_Init_UnkMotionStates4), partial and full shots,
    // the shield cancel keeping the level, an aerial shot's recoil, a roll
    // and a hit dropping the shot, and Fox reflecting a full shot.
    ("samus_x_ec3145eb3_p2", 332),
    ("samus_x_edb2b114a_p1", 344),
    ("samus_charge_fire_fd_fox4", 300),
    ("samus_charge_full_fd_fox4", 400),
    ("samus_charge_air_fd_fox4", 300),
    ("samus_charge_cancel_air_fd_fox4", 330),
    // The hold cancels on the input proc's shield bit, not the digital
    // shoulders (ftSs_SpecialNHold_IASA 0x80129BD8): an analog-only trigger
    // or Z.
    ("samus_charge_cancel_analog_fd_fox4", 300),
    ("samus_charge_cancel_z_fd_fox4", 300),
    ("samus_charge_roll_fd_fox4", 300),
    ("samus_charge_hit_fd_fox4", 214),
    ("samus_charge_shine_fd_fox4", 398),
    // Bomb (ftsamusspeciallw1.c, itsamusbomb.c) and the bomb jump
    // (ftsamusspeciallw0.c): drops from Wait, the air and SquatWait, the
    // bounce to rest, Fox's hit, launches from Wait, the ball and off-centre
    // (landing in 341), the late start from the ball model, and no launch
    // while charging or dodging.
    ("samus_bomb_ground_fd_fox4", 300),
    ("samus_bomb_air_fd_fox4", 300),
    ("samus_bomb_chain_fd_fox4", 360),
    ("samus_bomb_double_fd_fox4", 320),
    ("samus_bomb_edge_fd_fox4", 320),
    ("samus_bomb_fox_fd_fox4", 300),
    ("samus_bomb_crouch_fd_fox4", 300),
    ("samus_bomb_side4_fd_fox4", 320),
    ("samus_bomb_side6_fd_fox4", 320),
    ("samus_bomb_side8_fd_fox4", 320),
    ("samus_bomb_side10_fd_fox4", 320),
    ("samus_bomb_charging_fd_fox4", 300),
    ("samus_bomb_dodge_fd_fox4", 300),
    ("samus_bomb_walk_fd_fox4", 300),
    ("samus_bomb_roll_fd_fox4", 320),
    ("samus_bomb_squat_fd_fox4", 300),
    // The explorer's grounded Bomb (ftData_SpecialDown[Samus]) and 240 neutral ticks.
    ("samus_x_ec3145eb3_p0", 360),
    // Grapple beam (itsamusgrapple.c, ftCo_0D95.c): the grab and dash
    // grab timelines with their RNG sparks, the rope's throw, sag and reel,
    // a catch reeled in to CatchWait, pummels, throws.
    ("samus_grab_whiff_fd_fox4", 260),
    ("samus_grab_dash_fd_fox4", 260),
    ("samus_grab_fox_fd_fox4", 320),
    ("samus_grab_dashfox_fd_fox4", 300),
    ("samus_grab_pummel_fd_fox4", 320),
    ("samus_grab_shined_fd_fox4", 300),
    ("samus_grab_throwf_fd_fox4", 320),
    ("samus_grab_throwb_fd_fox4", 320),
    ("samus_grab_throwd_fd_fox4", 320),
    // The up throw's looped zero-damage hitbox on its victim: no hitlag
    // (dmg.x183C_applied stays 0).
    ("samus_grab_throwu_fd_fox4", 320),
    // The aerial grapple (ftCo_AirCatch.c, ftCo_800C3B10): Z and the air
    // dodge's tether, the throw with the drift, the rope in the air, landing
    // without interrupt, and a whiff off the edge.
    ("samus_zair_fd_fox4", 260),
    ("samus_zair_land_fd_fox4", 260),
    ("samus_zair_dodge_fd_fox4", 260),
    ("samus_zair_twice_fd_fox4", 300),
    ("samus_zair_hit_fd_fox4", 300),
    ("samus_zair_shined_fd_fox4", 300),
    ("samus_zair_reach150_fd_fox4", 300),
    ("samus_grab_edge_fd_fox4", 300),
    // The aerial tether into FD's wall below the ledge (ftCo_800C3CC0,
    // AirCatchHit; itsamusgrapple.c states 6-8, itlinkhookshot.c states
    // 6-8): paying out, the swing and its countdown into DamageFall, and A
    // climbing into CliffCatch or the ledge-less hop (ftCo_8009B390), for
    // Samus, Link and Young Link (0 differing particle ticks).
    ("samus_tether_hang_fd_fox4", 480),
    ("samus_tether_reel_fd_fox4", 480),
    ("samus_tether_late_reel_fd_fox4", 480),
    ("samus_tether_low_hang_fd_fox4", 480),
    ("samus_tether_low_reel_fd_fox4", 480),
    ("samus_tether_low_late_reel_fd_fox4", 480),
    ("links_tether_hang_fd_fox4", 480),
    ("links_tether_reel_fd_fox4", 480),
    ("links_tether_late_reel_fd_fox4", 480),
    ("links_tether_low_hang_fd_fox4", 480),
    ("links_tether_low_reel_fd_fox4", 480),
    ("links_tether_low_late_reel_fd_fox4", 480),
    ("links_yl_tether_hang_fd_fox4", 480),
    ("links_yl_tether_reel_fd_fox4", 480),
    ("links_yl_tether_late_reel_fd_fox4", 480),
    ("sheikzelda_transform_fd_sheik", 360),
    ("sheikzelda_transform_fd_zelda", 360),
    ("sheikzelda_transform_air_fd_sheik", 360),
    ("sheikzelda_transform_twice_fd_zelda", 480),
    ("sheikzelda_vanish_fd_sheik", 300),
    ("sheikzelda_needles_fd_sheik", 300),
    ("sheikzelda_needles_tap_fd_sheik", 240),
    ("sheikzelda_needles_air_fd_sheik", 300),
    // The charge cancels on the input proc's shield bit, not the digital
    // shoulders (ftSk_SpecialNLoop_IASA 0x80112744): an analog-only trigger
    // or Z, on the ground and in the air.
    ("sheikzelda_needles_cancel_analog_fd_sheik", 300),
    ("sheikzelda_needles_cancel_z_fd_sheik", 300),
    ("sheikzelda_needles_cancel_analog_air_fd_sheik", 300),
    ("sheikzelda_chain_fd_sheik", 300),
    ("sheikzelda_chain_hit_fd_sheik", 380),
    ("sheikzelda_chain_swing_fd_sheik", 320),
    ("sheikzelda_chain_air_fd_sheik", 300),
    ("sheikzelda_chain_tap_fd_sheik", 240),
    ("sheikzelda_nayru_fd_zelda", 240),
    ("sheikzelda_zd_nayru_air_fd_zelda", 300),
    ("sheikzelda_zd_nayru_reflect_fd_zelda", 300),
    ("sheikzelda_farore_fd_zelda", 300),
    ("sheikzelda_zd_farore_air_fd_zelda", 360),
    ("sheikzelda_zd_farore_floor_fd_zelda", 300),
    ("sheikzelda_din_fd_zelda", 360),
    ("sheikzelda_zd_din_hold_fd_zelda", 300),
    ("sheikzelda_zd_din_shine_fd_zelda", 300),
    ("sheikzelda_zd_din_hurt_fd_zelda", 300),
    ("sheikzelda_zd_din_air_fd_zelda", 360),
    ("sheikzelda_zd_din_floor_fd_zelda", 300),
    ("sheikzelda_zd_din_high_fd_zelda", 360),
    // LINKS: Spin Attack on the ground (hitting Fox) and in the air, with the
    // swirl models (efSync 0x4BB/0x4BC) and special fall and landing.
    ("links_spin_ground_fd_fox4", 420),
    ("links_spin_air_fd_fox4", 420),
    // LINKS: the boomerang thrown on the ground and in the air, glancing off the
    // floor, striking Fox, shielded, reflected by Fox's down special
    // (it_802A20E8), homing back and caught (SpecialS2 / SpecialAirS2, the catch
    // interrupted by a jump); Young Link's boomerang and Spin Attack.
    ("links_boomerang_fd_fox4", 600),
    ("links_boomerang_air_fd_fox4", 600),
    ("links_boomerang_shield_fd_fox4", 480),
    ("links_boomerang_reflect_fd_fox4", 480),
    ("links_yl_boomerang_fd_fox4", 600),
    ("links_yl_spin_ground_fd_fox4", 420),
    ("links_yl_spin_air_fd_fox4", 420),
    // LINKS: the hookshot (itlinkhookshot.c, run from its thrower's accessory
    // proc): standing and dash grabs whiffing and catching Fox (CatchPull waits
    // on the reel-in), Z-airs (ftLk_MS_AirCatch) whiffing and landing with the
    // chain out, for Link and Young Link; Link's down aerial bounce (lwOnHit /
    // lwOnAnim).
    ("links_grab_fd_fox4", 560),
    ("links_dashgrab_fd_fox4", 560),
    ("links_zair_fd_fox4", 520),
    ("links_zair_hit_fd_fox4", 520),
    ("links_yl_grab_fd_fox4", 560),
    ("links_yl_zair_fd_fox4", 520),
    ("links_dair_fd_fox4", 480),
    // LINKS: the Hylian shield (Wait/SquatWait's kind switch, ftColl_8007B1B8
    // with x221B_b2/b3/b4) against Fox's lasers, far and close, facing and turned
    // away, for Link and Young Link.
    ("links_hylian_fd_fox4", 520),
    ("links_hylian_close_fd_fox4", 600),
    ("links_yl_hylian_close_fd_fox4", 600),
    // Link's and Young Link's bow and arrows: draws, shots, stuck arrows, the fire arrow's flame.
    ("links_bow_fd_fox4", 600),
    ("links_yl_bow_fd_fox4", 600),
    // Link's and Young Link's bombs, the arrow held by a shield and Young Link's taunt milk.
    ("links_bomb_fd_fox4", 900),
    ("links_yl_bomb_fd_fox4", 900),
    ("links_arrow_shield_fd_fox4", 420),
    ("links_yl_milk_fd_fox4", 600),
    // LINKS: explorer cases from start_fd_link_fox4 extended by 240 neutral
    // ticks past their old faults: Spin Attack grounded and aerial, the
    // boomerang, Link shielding as Fox's up special starts, and a hookshot
    // grab on a shielding Fox.
    ("link_x_e3e74affa_p2", 337),
    ("link_x_e4213e6a5_p2", 326),
    ("link_x_ec13743d5_p0", 365),
    ("link_x_e89a89d0e_p2", 326),
    ("link_x_e50f774a0_p2", 340),
    // Stale moves: a down tilt repeated out of a down tilt is a new attack
    // instance (x21EC = 8008BC00 -> ft_800892A0), one stale entry per hit.
    ("stale_dtilt_repeat_fd_marth4", 400),
    // An egg's hit during Yoshi's forward tilt, before its hitbox is made:
    // the hitbox reads the stale table with the egg's entry (ft_80089228).
    ("stale_item_hit_midmove_fd_yoshi_fox4", 420),
    // Two Marth jabs clank: slash against slash draws HSD_Randi(3) for the
    // sword clank sound in hit detection (ftColl_800784B4, 0x800784E4).
    ("clank_sword_jabs_fd_marth_marth4", 260),
    // Samus's angled forward smashes (ftData_MotionStateList[58, 59, 61, 62],
    // doEnter 0x8008C3E0): all five angles whiffing, up and down hitting Fox.
    ("samus_smash_angles_whiff_fd_fox4", 520),
    ("samus_smash_hi_hit_fd_fox4", 260),
    ("samus_smash_lw_hit_fd_fox4", 260),
    // Jigglypuff's multijump lands on a platform with the stick down:
    // ftCo_JumpAerialF1_Coll (0x800D767C) is ft_80082F28, no pass filter.
    ("jigglypuff_multijump_platform_dl_captainfalcon4", 291),
    // Captain Falcon's third jab checks the rapid jab before its interrupt
    // (ftCo_Attack13_IASA, 8008B390): A held from jab 3 enters Attack100Start.
    ("falcon_jab3_rapid_start_fd_captainfalcon_fox4", 420),
    // An electric phantom (Falco's Reflector glancing a crouching Fox) takes
    // the electric hitlag scale too (ftColl_8007A06C, 8007AAF4).
    ("falco_shine_phantom_crouch_start_fd_falco_fox4", 322),
    // A catch box reaching its victim through a wall does not catch
    // (ft_80084CE4): Falcon Dive from under Dream Land's ledge.
    ("falcon_dive_wall_start_dl_captainfalcon_jigglypuff4", 347),
    // A needle an aerial strikes shows the hit spark (efSync 0x3E8, parented
    // to the needle, it_80270E30) and is destroyed in the same tick:
    // Item_8026A8EC's efLib_DestroyAll (0x8005B880) takes the spark's model
    // and generators before the particle pass.
    ("sheikzelda_needle_struck_fd_sheik", 233),
    ("sheikzelda_needle_struck_dair_fd_sheik", 236),
];

#[test]
fn matrix_witnesses_match_retail() {
    gate_in_parallel(&MATRIX_WITNESSES);
}

/// Controller-fix Gecko codes (melee_ft::input::controller_fix), each
/// witness recorded in Dolphin without the code (`_off`) and with it
/// (`gecko = [...]`, harness/gecko.py): UCF's dashback (0x800C9A44), shield
/// drop (0x800998A4) and 0.8's tumble wiggle (0x800908F4); the 0.73 beta's
/// dashback, which wants the stick toward the turn (`_away_`: 0.73 keeps the
/// slow turn where 0.74 and 0.8 flip the facing at tick 121); 0.84's pad
/// buffer and 1.0 cardinals (0x8006B460), rim-count shield drop (0x8009A0B8),
/// SDI (0x8008E54C), shield SDI (0x80093294) and squat release (0x800D65EC).
const UCF_WITNESSES: [(&str, usize); 29] = [
    ("ucf_dashback_fd_fox_off", 200),
    ("ucf_dashback_fd_fox_ucf073", 200),
    // A full stick back toward the old facing on the turn's second frame.
    ("ucf_dashback_away_fd_fox_ucf073", 200),
    ("ucf_dashback_away_fd_fox_ucf074", 200),
    ("ucf_dashback_away_fd_fox_ucf08", 200),
    // 0.73 takes Popo's partner from the GX link (gobj+0x10).
    ("ucf_dashback_fd_iceclimbers_ucf073", 200),
    ("ucf_shielddrop_bf_fox_ucf073", 200),
    ("ucf_dashback_fd_fox_ucf074", 200),
    ("ucf_dashback_fd_fox_ucf08", 200),
    // Popo's smash turn rewrites Nana's newest follow sample.
    ("ucf_dashback_fd_iceclimbers_ucf074", 200),
    ("ucf_dashback_fd_iceclimbers_ucf08", 200),
    ("ucf_shielddrop_bf_fox_off", 200),
    ("ucf_shielddrop_bf_fox_ucf074", 200),
    ("ucf_shielddrop_bf_fox_ucf08", 200),
    ("ucf_tumble_fd_fox_off", 290),
    ("ucf_tumble_fd_fox_ucf08", 290),
    ("ucf084_dashback_fd_fox_ucf084", 200),
    ("ucf084_shielddrop_bf_fox_ucf084", 200),
    ("ucf084_tumble_fd_fox_ucf084", 290),
    ("ucf084_cardinal_run_fd_fox_off", 200),
    ("ucf084_cardinal_run_fd_fox_ucf084", 200),
    ("ucf084_shielddrop_rim_bf_fox_off", 200),
    ("ucf084_shielddrop_rim_bf_fox_ucf084", 200),
    ("ucf084_sdi_rest_fd_fox_off", 240),
    ("ucf084_sdi_rest_fd_fox_ucf084", 240),
    ("ucf084_shieldsdi_pound_fd_fox_off", 272),
    ("ucf084_shieldsdi_pound_fd_fox_ucf084", 272),
    ("ucf084_squatrv_fd_fox_off", 200),
    ("ucf084_squatrv_fd_fox_ucf084", 200),
];

#[test]
fn ucf_controller_fix_witnesses_match_retail() {
    gate_in_parallel(&UCF_WITNESSES);
}

/// Slippi replays played back on retail (`harness/slippi_to_scenario.py`):
/// tournament inputs from a boundary with the replay's ports, timer, codes
/// and seed. Each is gated through its cold twin, with items and the
/// ledger's particle draw order.
const SLIPPI_REPLAY_WITNESSES: [(&str, usize); 12] = [
    // A shield in hitlag keeps its cached position: Sheik's second needle
    // strikes where Marth's shield was a tick before (148, 149).
    ("slp_bf_sheik_marth_t400_cold", 400),
    // Fox's up throw draws its voice in his input proc, ahead of Yoshi's
    // Story's puff timer draw in the same tick (1817).
    ("slp_ys_fox_falco_t1900_cold", 1900),
    // Pikachu's second Quick Attack zip flushes its cheek spark in the
    // animation proc, ahead of the other fighter's link-9 effects (2244).
    ("slp_bf_captainfalcon_pikachu_t2900_cold", 2900),
    // A turnip that clanked with Marth clanks with him again once its
    // rehit timer forgets him (5858, 5891).
    ("slp_bf_marth_peach_t6500_cold", 6500),
    // Pokemon Stadium on ports 1 and 4 up to the first transformation.
    ("slp_ps_fox_falco_t4000_cold", 4000),
    // A smash launch off Stadium's side shakes the camera; the magnifier's
    // damage follows retail's display passes (3045, 3105), which the
    // console's replay (3043) cannot show.
    ("slp_ps_falco_marth_ns_t3200_cold", 3200),
    // Fox's entry warp is queued after EntryStart's motion change and waits
    // for his link-9 flush, behind Randall's puff of the same tick (15).
    ("slp_ys_jigglypuff_fox_t300_cold", 300),
    // Peach's down smash, entered in the input proc, flushes her queued
    // shine spark before the new script's smash voice draws (1048).
    ("slp_dl_peach_fox_t1300_cold", 1300),
    // Popo's unlaunched ice block meets Battlefield's top platform (121);
    // Nana's edge-guard test leaves a ledge target below her alone (1781).
    ("slp_bf_falco_iceclimbers_t2050_cold", 2050),
    // Final Destination from a boundary created from the replay's Game
    // Start seed: the background's accelerations are the console's, so its
    // limit draws (grLast_8021ADD0: 206, 309, 606, ... twelve by 3000) fall
    // on the console's ticks.
    ("slp_fd_marth_marth_t3000_cold", 3000),
    // Fountain of Dreams likewise: the platforms' first waits are the
    // console's, so grIzumi_801CC358 draws again on its ticks (858, 913,
    // 931, ... nine by 3000).
    ("slp_fod_fox_falco_t3000_cold", 3000),
    // The Yoshi Bomb catches a ledge twice (ftCliffCommon_80081298, then
    // ftCliffCommon_80081370 at 0x8012E9E8): two ledge flashes (828).
    ("slp_bf_yoshi_samus_t860_cold", 860),
];

#[test]
fn slippi_replay_inputs_match_retail() {
    for (name, ticks) in SLIPPI_REPLAY_WITNESSES {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../harness/scenarios/{name}.toml"));
        let scenario = Scenario::load(&path).unwrap();
        let ledger = scenario.trace_path("ledger.raw.jsonl");
        if !melee_test_support::require_files(scenario.required_files().into_iter().chain([ledger]))
        {
            continue;
        }
        assert_eq!(scenario.frames as usize, ticks);
        trace::gate_items(&scenario).unwrap();
        let differing =
            trace::particle_site_diff(&scenario, 0, scenario.frames - 1, "ledger").unwrap();
        assert!(differing.is_empty(), "{name}: {}", differing[0]);
    }
}

/// ftCo_8008E908 (0x8008E908): a hit that lands during a ledge state
/// (x221D_b7) starts the ledge cooldown. Marth, hit out of his ledge attack
/// and slid off the edge in MissFoot, falls through the grab range (retail
/// tick 195 is where the port used to catch) and regrabs after a jump.
#[test]
fn hit_out_of_a_ledge_state_starts_the_ledge_cooldown() {
    combat_gate_ticks("ledge_hit_cooldown_fd_marth", 330);
}

/// Final Destination's whole background cycle and the start of the second
/// (grLast_8021B920, 0x8021B920): the star field's generators outlive their
/// map when phase 17 retires it (Ground_801C4A08, tick 13015) and emit from
/// the orphaned joint for their remaining life.
#[test]
fn final_destination_background_cycle_matches_retail() {
    combat_gate_ticks("fd_background_cycle_marth_marth4", 13600);
}

/// Battlefield's background swaps (grBattle_BG_Callback2, 0x8021A3BC) run
/// three full cycles from the start boundary: transition animation, color
/// overlays, mid-match background creation with its particle keys, and the
/// faded map's retirement while its generators keep emitting.
#[test]
fn battlefield_background_swap_cycles_match_retail() {
    combat_gate_ticks("bf_transition_cycle_marth_fox4", 11000);
}

/// Dream Land from the Fox/Marth start boundary: Whispy Woods' blink, turn
/// and blow cycles (grOldPupupu_802113E0) with the tie vote, left and right
/// gusts pushing fighters (fn_802112F4), and the Bronto Burt flyby's draws
/// when the background timer expires (grOldPupupu_80210D10).
#[test]
fn dream_land_wind_and_flyby_match_retail() {
    combat_gate_ticks("stage_dl_idle_fox_marth4", 5000);
    combat_gate_ticks("stage_dl_windright_fox_marth4", 2400);
}

/// Fountain of Dreams from the Fox/Marth start boundary: both fighters ride
/// the side platforms (grIzumi_801CC358) through waits, random steps up and
/// down, a submerge below the stage and the resurfacing to the rest height.
#[test]
fn fountain_of_dreams_platforms_match_retail() {
    combat_gate_ticks("stage_fod_idle_fox_marth4", 6000);
}

/// Yoshi's Story's Shy Guys (itheiho.c) from the Fox/Marth start boundary:
/// a Fire Fox knocks one spinning away (state 2) and Fox then walks the
/// stage's terrain; a light nair stuns one and it flees (states 3 and 4);
/// Marth's up-air knocks one away.
#[test]
fn yoshis_story_shy_guys_match_retail() {
    combat_gate_ticks("stage_ys_shyguy_firefox", 660);
    combat_gate_ticks("stage_ys_shyguy_nair", 780);
    combat_gate_ticks("stage_ys_shyguy_marth", 620);
}

/// The ground pose's body tilt (ft_80089B08, ft_0899.c:180-232) on Yoshi's
/// Story's side slopes: Fox's and Marth's smashes pitch the root joint to the
/// floor (0.2054 rad, either facing); standing, walking, crouching and
/// landing there do not. The tilt shows only in the joints, so the retail
/// bone dumps are compared too.
#[test]
fn slope_body_tilt_matches_retail_bones() {
    for (name, ticks) in [
        ("slope_ys_walk_fox_marth", 400),
        ("slope_ys_smash_fox_marth", 680),
    ] {
        if combat_gate_ticks(name, ticks).is_none() {
            continue;
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../harness/scenarios/{name}.toml"));
        let scenario = Scenario::load(&path).unwrap();
        if !melee_test_support::require_files([
            scenario.trace_path("bones.jsonl"),
            scenario.trace_path("bones.raw.jsonl"),
        ]) {
            continue;
        }
        let report = melee_sim::bones::bones_diff(&scenario, 12, None).unwrap();
        assert!(report.is_empty(), "{name}: {report:#?}");
    }
}

/// Jigglypuff's costume hats are no fighter part and their spring chains move
/// only the hat's joints, so every part keeps retail's pose in each costume
/// through Rollout, a KO and respawn, and Rest.
#[test]
fn jigglypuff_hat_costumes_match_retail_bones() {
    for costume in 1..=4 {
        for (kind, ticks) in [("rollout_ko", 1320), ("rest_hit", 480)] {
            let name = format!("puff_hat_c{costume}_{kind}_fd_fox4");
            if combat_gate_ticks(&name, ticks).is_none() {
                continue;
            }
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../harness/scenarios/{name}.toml"));
            let scenario = Scenario::load(&path).unwrap();
            if !melee_test_support::require_files([
                scenario.trace_path("bones.jsonl"),
                scenario.trace_path("bones.raw.jsonl"),
            ]) {
                continue;
            }
            let report = melee_sim::bones::bones_diff(&scenario, 12, None).unwrap();
            assert!(report.is_empty(), "{name}: {report:#?}");
        }
    }
}

/// Yoshi's forward smash from Wait (facing forward, turned by the stick or
/// the C-stick) and turned out of a walk: frame 0's dust (0x3F3 on bone 57,
/// ftCo_8009F834's three offset draws) precedes the smash voice's Randi
/// (ftAction_80071CCC -> ft_800889F4) in the script. Only the dust's
/// position shows the draw order, so the retail particle dumps are compared.
#[test]
fn yoshi_forward_smash_dust_positions_match_retail() {
    for name in [
        "yoshi_fsmash_forward_cstick",
        "yoshi_fsmash_turn_cstick",
        "yoshi_fsmash_turn_stick",
        "yoshi_fsmash_walk_turn_cstick",
    ] {
        if combat_gate_ticks(name, 240).is_none() {
            continue;
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../harness/scenarios/{name}.toml"));
        let scenario = Scenario::load(&path).unwrap();
        if !melee_test_support::require_files([scenario.trace_path("particles.jsonl")]) {
            continue;
        }
        let report = trace::particle_state_diff_with(&scenario, 0, 240, false, false).unwrap();
        assert!(report.is_empty(), "{name}: {report:#?}");
    }
}

/// Pokemon Stadium from the Fox/Marth start boundary: the screen's mode
/// cycle (grStadium_801D2A60) with its close-up framing, and the
/// transformation controller (grStadium_801D4548) sinking the base arena
/// under idle fighters, raising each form (a short walk steers the first
/// choice) with its collision and particles, and returning to the base.
#[test]
fn pokemon_stadium_transformations_match_retail() {
    combat_gate_ticks("stage_ps_idle_fox_marth4", 7200);
    combat_gate_ticks("stage_ps_fire_fox_marth4", 7500);
    combat_gate_ticks("stage_ps_water_fox_marth4", 7500);
    combat_gate_ticks("stage_ps_grass_fox_marth4", 7500);
    // A second form read, faster than any first read (12 polls): replayed
    // from the trace's recorded read completion (docs/ORACLE.md).
    combat_gate_ticks("stage_ps_second_fox_marth4", 11000);
}

/// Slippi's Stadium codes (melee_lib::slippi), recorded in Dolphin with the
/// Gecko codes (`gecko = [...]`, harness/gecko.py) over the idle witness.
/// Preload (0x801D45EC and five more): the form is drawn on the first
/// waiting tick and announced the tick the base duration ends, with no read
/// poll; the next is drawn the tick after the base arena settles again.
/// Frozen (0x801D45FC) on top of it: the same early draw, and no
/// transformation.
#[test]
fn pokemon_stadium_slippi_codes_match_retail() {
    combat_gate_ticks("slippi_ps_preload_fox_marth4", 7500);
    combat_gate_ticks("slippi_ps_frozen_fox_marth4", 4600);
}

/// "Frozen Stages" (slippi-ssbm-asm External/Frozen All/Core, the code
/// tournament consoles ran beside Slippi), recorded with its Gecko text:
/// no Shy Guy spawner (0x801E3348), no Final Destination phase update
/// (0x8021AAE4; the background keeps its first phase past the star field's
/// tick), no Stadium controller (0x801D1548; not even the preload code's
/// form draw) and Whispy's cycle waiting where it would blow (0x803E67E0).
#[test]
fn frozen_stages_code_matches_retail() {
    combat_gate_ticks("frozen_stages_ys_fox_marth4", 2400);
    combat_gate_ticks("frozen_stages_fd_marth_marth4", 13600);
    combat_gate_ticks("frozen_stages_ps_fox_marth4", 7500);
    combat_gate_ticks("frozen_stages_dl_fox_marth4", 5000);
}

/// Pokemon Stadium's jumbotron close-up ends when the player's camera bone
/// leaves the main CObj as last rendered (grStadium_801D32D0): a throw's
/// graphics 0x514 shakes it with a Medium quake (efAsync kind 8 ->
/// Camera_RequestQuake), and a tick without a display pass keeps the
/// previous tick's CObj. The form reads' completion ticks come from the
/// traces (docs/PORT_NOTES/POKEMON_STADIUM.md).
#[test]
fn pokemon_stadium_close_up_follows_the_rendered_camera() {
    combat_gate_ticks("ps_match_screen_e9943b4ab_p0", 4100);
    combat_gate_ticks("ps_match_screen_ef89b3e70_p2", 4100);
    combat_gate_ticks("ps_match_screen_edb2b114a_p1", 4100);
}

/// Fox dashes into a wall on the risen rock form: ftCo_8009EDA4 enters
/// StopWall (ftCo_8009EE30), which returns to Wait at its animation's end.
#[test]
fn stop_wall_on_pokemon_stadium_rock_form_matches_retail() {
    combat_gate_ticks("stopwall_ps_fox_dash_rock", 4800);
}

/// Marth dizzy after a decay break until Furafura wears off (831).
#[test]
fn furafura_expire_victim_fd_marth_900_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("furafura_expire_victim_fd_marth", 900);
}

/// Fox dash-attacks a dizzy Marth (Furafura -> damage).
#[test]
fn furafura_hit_victim_fd_marth_600_ticks_and_ordered_particle_draws() {
    combat_gate_ticks("furafura_hit_victim_fd_marth", 600);
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
        // Fox loses a stock, then the timer runs out with Marth ahead.
        ("timeout_decisive_fd_marth", 3839),
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
        // A reflecting Reflector sliding off the edge (LwHit -> AirLwHit).
        ("sudden_death_reflecthit_walkoff_fd_marth", 1343),
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
    gate_in_parallel(&CORPUS_MATCHES);
}

/// Corpus version 3 (`explore <dir> <count>`, seeds from a xorshift of
/// 0x00C0FFEE): the matches that faulted the port. Together they exposed
/// that a motion change ends the attack interaction, that grabs and throws
/// stop at the edge (ft_800841B8 -> ft_800827A0), that a dying grabber
/// releases its victim (ftCo_800DD100), that thrown positioning waits out
/// hitlag (Fighter_CallAcessoryCallbacks_8006C624) and that a motion change
/// drops the Counter volume (fighter.c:1049, `x221B_b0`).
const CORPUS_V3_MATCHES: [(&str, usize); 490] = [
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
    // Holding a Bob-omb through FallAerial after an aerial jump.
    ("corpus_sd_s1_e09bbd0e1_p0", 1418),
    // Teetering at the edge while holding a Bob-omb.
    ("corpus_sd_s1_e1f29a5c7_p2", 1360),
    // A catch cut by the captor's separation on its CatchWait entry tick:
    // the release's motion changes flush the entry's CaptureFlash.
    ("corpus_v3_s1_e19a3b12e_p2", 1902),
    // A jump-squat up smash throwing a Bob-omb upward past a rain bomb in
    // its creation intangibility (item.c foobar).
    ("corpus_sd_s1_e00088bc5_p0", 1758),
    // A forward smash (AttackS4S) whose motion row the old per-state load list
    // omitted: every authored motion row now loads (ftData_80085CD8).
    ("corpus_v3_fd_falco_fox4_e9943b4ab_p2", 88),
    ("corpus_v3_fd_captainfalcon_fox4_e9943b4ab_p2", 88),
    // A Battlefield revival at the player's own marker (stage_info.unk8C.b4).
    ("corpus_v3_bf_marth_fox4_e9943b4ab_p0", 255),
    // Fire Fox aimed down from a Battlefield platform skips it (ftCo_8009A134).
    ("corpus_v3_bf_marth_fox4_e75fb4a9a_p2", 133),
    // Shielding on a Battlefield platform with the stick held down since the
    // air: ftCommonData+0x468, the platform-drop input window, is a float.
    ("corpus_v3_bf_marth_fox4_edb2b114a_p0", 87),
    // Reflector started with a fresh down tap on a Battlefield platform drops
    // through it (ftFx_SpecialLwStart_CheckPass, 800E87D4).
    ("corpus_v3_bf_marth_fox4_e50814092_p1", 179),
    // Yoshi's egg shield: GuardHold swaps capsule 0 for the grabbable egg
    // capsule (ftYs_Init_8012BDA0) while Fox attacks it, and a grab out of it.
    ("corpus_v3_fd_yoshi_fox4_e89a89d0e_p2", 105),
    ("corpus_v3_fd_yoshi_fox4_e726cfdde_p1", 127),
    ("yoshi_egg_fd_fox4_e1502cb40_p2", 164),
    ("yoshi_egg_fd_fox4_e726cfdde_p2", 133),
    // Falco's taunt: ftCo_800DEA28's default arm (common AppealS).
    ("corpus_v3_fd_falco_fox4_e89a89d0e_p1", 102),
    // Peach floats out of Fall on the final tick (ftPe_8011BAD8 -> ftPe_8011BB6C).
    ("corpus_v3_fd_peach_fox4_e00f31913_p2", 125),
    // Peach's subaction effect 0x3F4 (ftCo_09F7.c block_70, generator 0x48).
    ("corpus_v3_fd_peach_fox4_edb2b114a_p2", 137),
    // Exact on arrival after the wave-A merges (animation loader, taunt, float).
    ("corpus_v3_fd_falco_fox4_e0dee256e_p2", 86),
    ("corpus_v3_fd_peach_fox4_e3e74affa_p1", 110),
    // Captain Falcon's Falcon Punch out of Landing (ftCa_SpecialN_Enter).
    ("corpus_v3_fd_captainfalcon_fox4_e75fb4a9a_p1", 92),
    // A Reflector held into a platform that the f32 +0x468 window keeps on the platform.
    ("corpus_v3_bf_marth_fox4_e19f8579b_p2", 150),
    // A 3376-tick Battlefield match through a background swap and revivals.
    ("corpus_v3_bf_marth_fox4_e573e2d95_p0", 3376),
    // Marth lands from a grounded Dolphin Slash entered after a whiffed grab:
    // LandingFallSpecial inherits mv+4 through Catch and the special (neither
    // writes it; ftCo_800D8C54, ftMars_MotionVars).
    ("corpus_v3_bf_marth_fox4_e3e74affa_p1", 425),
    // Battlefield's map 6 proc ticks the Shield Breaker gust (lb_800115F4), so
    // Fox's tail hangs into Marth's grab a few frames into a jump.
    ("corpus_v3_bf_marth_fox4_e75fb4a9a_p0", 3376),
    // A 4072-tick Battlefield match (Fox P1), exact end to end.
    ("corpus_v3_bf_fox_marth4_ee133b82f_p0", 4072),
    // Yoshi's Egg Throw entry, aerial and grounded (ftYs_SpecialHi_Enter).
    ("corpus_v3_fd_yoshi_fox4_ec13743d5_p0", 126),
    ("corpus_v3_fd_yoshi_fox4_e3e74affa_p2", 97),
    // Subaction opcode 12, hitbox damage adjustment (ftAction_8007162C).
    ("corpus_v3_fd_yoshi_fox4_e7ff378da_p1", 170),
    // Fox back-throws Yoshi, who authors no ThrownB animation of his own.
    ("corpus_v3_fd_yoshi_fox4_e0dee256e_p0", 176),
    // Yoshi's tongue grab (fn_800D9CE8 skips ahead in CatchPull), mouth hold
    // (ftCo_800DB368/800DB464) and forward throw of a victim he authors no
    // ThrownF animation for: its CaptureWait AObjs keep playing.
    ("corpus_v3_fd_yoshi_fox4_e005a4f43_p0", 256),
    // Falco's down-throw laser hits the thrown Fox: a light non-captor hit
    // keeps the grab and freezes the captor (ftCo_8008EC90 inlineB1/B2).
    ("corpus_v3_fd_fox_falco4_ec3145eb3_p0", 238),
    // Yoshi's grounded Egg Lay entry (ftYs_SpecialN_Enter), and a whiffed
    // one that returns to Wait before the double-jump armor case.
    ("corpus_v3_fd_yoshi_fox4_ec3145eb3_p1", 93),
    ("yoshi_armor_fd_fox4_ef89b3e70_p2", 187),
    // Fox hits Yoshi's egg shield: Yoshi's own shield damage row (344,
    // ftYs_Shield_8012C600 via ftCo_80092E50's kind branch).
    ("corpus_v3_fd_yoshi_fox4_e00f160ee_p1", 185),
    // Dolphin Slash falls through Battlefield's top platform with the stick
    // held down (ftCo_80096CC8 against PlCo +25C).
    ("corpus_v3_bf_marth_fox4_ee133b82f_p0", 3139),
    // Up smash out of a down tilt, then Dolphin Slash: mv+4 carries through
    // AttackLw3, which writes only +2340.
    ("corpus_v3_bf_marth_fox4_e50f774a0_p2", 380),
    // Long wave5 samples, exact end to end (FD Fox P1 5415 ticks; BF Fox P1 3623).
    ("corpus_v3_s0_ee98155f3_p0", 5415),
    ("corpus_v3_bf_fox_marth4_efd4a7dd2_p2", 3623),
    // Yoshi's side and down specials: the Egg Roll hop and the Yoshi Bomb's
    // grounded and aerial starts.
    ("corpus_v3_fd_yoshi_fox4_eeda0d0fc_p0", 88),
    ("corpus_v3_fd_yoshi_fox4_ec3145eb3_p0", 120),
    ("corpus_v3_fd_yoshi_fox4_e7ff378da_p0", 96),
    // Fox grabs Falco: CatchWait's flash (fn_800DA1D8) is queued before the
    // color program's effect when that step runs after the entry callback.
    ("corpus_v3_fd_fox_falco4_eb8786a38_p1", 5788),
    // Fox mashes out of Yoshi's mouth hold (ftCo_800DC920's x2226_b2 release).
    ("corpus_v3_fd_yoshi_fox4_eb8786a38_p0", 829),
    // Fox is hit inside Yoshi's egg (ftCo_800BC3D0, x1828 = 4).
    ("corpus_v3_fd_yoshi_fox4_e12d92447_p1", 258),
    // Peach's up special draws the parasol (item 103) from joint 109.
    ("corpus_v3_fd_peach_fox4_e3e74affa_p2", 124),
    // Peach's neutral special draws Toad (item 104) from joint 109.
    ("corpus_v3_fd_peach_fox4_eeda0d0fc_p1", 115),
    // Peach's forward smash picks club, pan or racket (ftPe_AttackS4_Enter).
    ("corpus_v3_fd_peach_fox4_ea4d5d9ec_p2", 108),
    ("corpus_v3_fd_peach_fox4_e9943b4ab_p2", 88),
    // Captain Falcon's Raptor Boost, Falcon Dive and Falcon Kick entries.
    ("corpus_v3_fd_captainfalcon_fox4_ee133b82f_p0", 104),
    ("corpus_v3_fd_captainfalcon_fox4_e4213e6a5_p2", 86),
    ("corpus_v3_fd_captainfalcon_fox4_e2b9e1400_p2", 174),
    ("corpus_v3_fd_captainfalcon_fox4_e09db17e3_p1", 173),
    ("corpus_v3_fd_captainfalcon_fox4_e50f475b4_p2", 258),
    ("corpus_v3_fd_captainfalcon_fox4_ee62c6106_p1", 342),
    // Falcon Dive catches an airborne Fox (the victim hangs, ftCo_800DB464).
    ("corpus_v3_fd_captainfalcon_fox4_ed5eb74a7_p1", 300),
    ("corpus_v3_fd_captainfalcon_fox4_ec48387e0_p0", 355),
    // Raptor Boost's inert hitbox touching a Fox laser (ftColl_8007925C).
    ("corpus_v3_fd_captainfalcon_fox4_e317831ba_p0", 181),
    ("corpus_v3_fd_captainfalcon_fox4_edea229bd_p2", 236),
    // Falcon Dive grabbing the ledge (ftCliffCommon_80081370 twice).
    ("corpus_v3_fd_captainfalcon_fox4_e4f8edfa8_p2", 840),
    ("corpus_v3_fd_captainfalcon_fox4_e0211286e_p0", 1680),
    // Clean explorer samples: whole Falcon-Fox matches with every special.
    ("corpus_v3_fd_captainfalcon_fox4_e8d30e8ed_p0", 5791),
    ("corpus_v3_fd_captainfalcon_fox4_e45a17231_p0", 6001),
    ("corpus_v3_fd_captainfalcon_fox4_e188c0a9b_p0", 4464),
    ("corpus_v3_fd_captainfalcon_fox4_e1a1346bb_p0", 4432),
    // Clean explorer samples: whole Ganondorf-Fox matches, the Dark hit
    // spark (hit_effect_ids[HitElement_Dark], generator 0x196) and every special.
    ("corpus_v3_fd_ganondorf_fox4_ef89b3e70_p0", 6001),
    ("corpus_v3_fd_ganondorf_fox4_e1502cb40_p0", 2812),
    ("corpus_v3_fd_ganondorf_fox4_e573e2d95_p0", 3834),
    ("corpus_v3_fd_ganondorf_fox4_edafcfddf_p0", 4314),
    ("corpus_v3_fd_ganondorf_fox4_ee133b82f_p0", 4394),
    ("corpus_v3_fd_ganondorf_fox4_ee62c6106_p0", 6001),
    ("corpus_v3_fd_ganondorf_fox4_e0fcf0c70_p0", 6001),
    ("corpus_v3_fd_ganondorf_fox4_ee98155f3_p0", 4896),
    ("corpus_v3_fd_ganondorf_fox4_e0ac13e8e_p2", 6001),
    ("corpus_v3_fd_ganondorf_fox4_eb4935276_p0", 2576),
    ("corpus_v3_fd_ganondorf_fox4_e3ef41ca4_p2", 5248),
    ("corpus_v3_fd_ganondorf_fox4_e46703f61_p0", 5249),
    ("corpus_v3_fd_ganondorf_fox4_e0d368f02_p1", 3754),
    ("corpus_v3_fd_ganondorf_fox4_e255c070a_p1", 6001),
    // Dark Dive: the throw's second collision (ft_80083B68 after a landing
    // check) loads its ECB from the pre-collision joint pose; the release's
    // damage goes to percentTemp before the fly-roll draw; the damage trail
    // queues behind the entry's pending Dark graphic.
    ("corpus_v3_fd_ganondorf_fox4_eae52f0ea_p0", 6001),
    ("corpus_v3_fd_ganondorf_fox4_eecfcf1ea_p1", 5257),
    // Dream Land: Whispy's gust samples each fighter after its velocity
    // (ftColl_GetWindOffsetVec), and its dynamics gusts (lb_80011A50) swing
    // Fox's tail hurtbox and switch Marth's cape to the solver.
    ("corpus_v3_dl_fox_marth4_edb2b114a_p2", 2534),
    ("corpus_v3_dl_fox_marth4_e75fb4a9a_p0", 6001),
    // Yoshi's Story: whole Fox-Marth matches whose smashes tilt the body on
    // the side slopes (ft_80089B08's body tilt; formerly unported).
    // A Shy Guy's removal walks the generator list (efLib_DestroyAll ->
    // hsd_8039D688) before Fox's dust, so the dust inserts after the head.
    ("slope_ys_e75fb4a9a_p1", 1670),
    ("slope_ys_e1502cb40_p0", 2026),
    ("slope_ys_e75fb4a9a_p2", 2131),
    ("slope_ys_ec3145eb3_p2", 2184),
    ("slope_ys_ec13743d5_p0", 2526),
    ("slope_ys_e9943b4ab_p0", 2658),
    // Fountain of Dreams: whole Fox-Marth matches on the moving platforms
    // (grIzumi_801CC358) and the fountain's water, whose terrain row splashes
    // stage-bank effects on footsteps, landings and bounds (mpLib_803BD7A0).
    ("corpus_v3_fod_fox_marth4_e9943b4ab_p0", 2067),
    ("corpus_v3_fod_fox_marth4_e75fb4a9a_p2", 2659),
    ("corpus_v3_fod_fox_marth4_edb2b114a_p0", 2778),
    ("corpus_v3_fod_fox_marth4_e9943b4ab_p1", 3045),
    ("corpus_v3_fod_fox_marth4_e89a89d0e_p0", 3205),
    ("corpus_v3_fod_fox_marth4_ef89b3e70_p0", 3715),
    // Peach Bomber's ground and aerial entries (ftPe_SpecialS_Enter / AirS).
    ("corpus_v3_fd_peach_fox4_e255c070a_p0", 110),
    ("corpus_v3_fd_peach_fox4_e9a8e5048_p2", 106),
    ("corpus_v3_fd_peach_fox4_e4068796b_p0", 210),
    ("corpus_v3_fd_peach_fox4_e41b2c2b8_p2", 211),
    // Peach's down special pulls a turnip (ftPe_SpecialLw_Enter).
    ("corpus_v3_fd_peach_fox4_e4068796b_p1", 989),
    ("corpus_v3_fd_peach_fox4_e98e8f6a6_p2", 87),
    // Jigglypuff's specials enter (ftData_SpecialN/S/Hi/Lw[Purin]): Rollout,
    // Pound, Sing (the notes, efSync 1238) and Rest, ground and air.
    ("corpus_v3_fd_fox_jigglypuff4_e00f31913_p0", 88),
    ("corpus_v3_fd_fox_jigglypuff4_e1502cb40_p1", 103),
    ("corpus_v3_fd_fox_jigglypuff4_e726cfdde_p0", 268),
    ("corpus_v3_fd_fox_jigglypuff4_e75fb4a9a_p2", 168),
    ("corpus_v3_fd_fox_jigglypuff4_e89a89d0e_p0", 252),
    ("corpus_v3_fd_fox_jigglypuff4_e89a89d0e_p2", 86),
    ("corpus_v3_fd_fox_jigglypuff4_e9943b4ab_p0", 92),
    ("corpus_v3_fd_fox_jigglypuff4_ef89b3e70_p1", 260),
    ("corpus_v3_fd_jigglypuff_fox4_e00f31913_p0", 224),
    ("corpus_v3_fd_jigglypuff_fox4_e19f8579b_p1", 203),
    ("corpus_v3_fd_jigglypuff_fox4_e726cfdde_p0", 144),
    ("corpus_v3_fd_jigglypuff_fox4_e726cfdde_p1", 209),
    ("corpus_v3_fd_jigglypuff_fox4_e89a89d0e_p0", 138),
    ("corpus_v3_fd_jigglypuff_fox4_ec13743d5_p0", 125),
    ("corpus_v3_fd_jigglypuff_fox4_ec13743d5_p2", 87),
    ("corpus_v3_fd_jigglypuff_fox4_edb2b114a_p1", 104),
    // Puff's later aerial jumps out of damage need held X/Y (ftCo_800D730C),
    // not the press ftCo_Damage_IASA/inlineC0 synthesize.
    ("corpus_v3_fd_fox_jigglypuff4_e1611c835_p0", 3979),
    ("corpus_v3_fd_fox_jigglypuff4_e3e74affa_p0", 4757),
    ("corpus_v3_fd_jigglypuff_fox4_e2b9e1400_p0", 1951),
    // Sing's sleep (ftCo_800C318C..DamageSongRv); the wake seals the old
    // script's bubbles before DamageSongRv's stars (efAsync_QueueFlush).
    ("corpus_v3_fd_fox_jigglypuff4_e4068796b_p0", 3339),
    ("corpus_v3_fd_fox_jigglypuff4_e8f0de8c6_p0", 6001),
    // Sing's zero-damage hit on a revival-invincible Fox sets no x1914.
    ("corpus_v3_fd_fox_jigglypuff4_eb17a6598_p0", 5481),
    // A Rollout into a shield: x1924, not x1914, so no deal_dmg_cb bounce.
    ("corpus_v3_fd_jigglypuff_fox4_e4f8edfa8_p0", 5061),
    // Dash into GuardOn/GuardReflect (Ft_MF_SkipAnim): the outgoing root
    // motion clamps gr_vel to dash speed (fighter.c:1363-1368).
    ("corpus_v3_fd_fox_jigglypuff4_eae52f0ea_p0", 6001),
    ("corpus_v3_fd_fox_jigglypuff4_e4f8edfa8_p0", 2307),
    // Pound into Fox's shield.
    ("corpus_v3_fd_jigglypuff_fox4_e0fcbde24_p0", 4618),
    ("corpus_v3_fd_fox_jigglypuff4_edb4b01fd_p0", 5128),
    // Rollouts that turn, run out, bounce off Fox (ftPr_SpecialS_8013D764).
    ("corpus_v3_fd_jigglypuff_fox4_e7ff378da_p0", 5108),
    ("corpus_v3_fd_fox_jigglypuff4_e3ef41ca4_p0", 5135),
    ("corpus_v3_fd_jigglypuff_fox4_efc6a328f_p0", 3728),
    // Full explorer matches from the four hat-costume boundaries
    // (ftPr_Init_8013C360; the hat never reaches simulation state).
    ("corpus_v3_fd_jigglypuff_c1_fox4_ef89b3e70_p0", 6001),
    ("corpus_v3_fd_jigglypuff_c2_fox4_ef89b3e70_p0", 6001),
    ("corpus_v3_fd_jigglypuff_c3_fox4_ef89b3e70_p0", 6001),
    ("corpus_v3_fd_jigglypuff_c4_fox4_ef89b3e70_p0", 6001),
    // A roll or spot dodge out of Dash or Landing writes only mv.co.escape.x0
    // (ftCo_80099314), so a later special inherits the predecessor's mv+4.
    ("corpus_v3_fod_fox_marth4_e00f31913_p0", 3028),
    ("corpus_v3_fod_fox_marth4_e3e74affa_p0", 5546),
    // A hit landing on the thrown fighter the tick Jigglypuff's ThrowF
    // releases it: the throw's damage joins x1838_percentTemp (ftColl_80076640)
    // for that hit's knockback and ProcessHit's single percent addition.
    ("corpus_v3_fd_fox_jigglypuff4_e0fcbde24_p0", 6001),
    ("corpus_v3_fd_jigglypuff_fox4_e7254fba4_p0", 5310),
    // Pikachu from its start boundary. A charged forward smash installs the
    // charge color over the attack's electric program (ftCo_800DF0D0) and
    // clears it on release (ftCo_800C0200).
    ("corpus_v3_fd_pikachu_fox4_ec13743d5_p1", 6001),
    ("corpus_v3_fd_pikachu_fox4_ecad2716f_p0", 6001),
    ("corpus_v3_fd_pikachu_fox4_eeda0d0fc_p0", 6001),
    ("corpus_v3_fd_pikachu_fox4_edb2b114a_p2", 5126),
    ("corpus_v3_fd_pikachu_fox4_e1502cb40_p2", 6001),
    ("corpus_v3_fd_pikachu_fox4_e573e2d95_p0", 6001),
    // Thunder's bolt chain is one item hit group (xAC4_ignoreItemID):
    // one bolt striking Fox marks him for every bolt (it_8026FAC4).
    ("corpus_v3_fd_pikachu_fox4_e1502cb40_p0", 4287),
    // efLib_DestroyAll walks the fighter tree with hsd_8039D688 even with
    // no generator on it, parking the insertion cursor at the tail.
    ("corpus_v3_fd_pikachu_fox4_e4213e6a5_p0", 6001),
    // Pikachu's electric pummel flashes the captured Fox at reaction level 0:
    // the CaptureDamage entry's Fighter_ChangeMotionState clears kb_applied.
    ("corpus_v3_fd_pikachu_fox4_e1502cb40_p1", 6001),
    ("corpus_v3_fd_pikachu_fox4_e75fb4a9a_p0", 5192),
    // PICHU from its start boundary (Pikachu's specials, PlPc.dat's data and
    // self-damage scripts). A Thunder bolt ending while Pichu is in Landing
    // writes 3 into the common state's mv+4 (ftPk_SpecialLw_SetState_Unk0).
    ("corpus_v3_fd_pichu_fox4_e2af099ca_p1", 860),
    ("corpus_v3_fd_pichu_fox4_ef89b3e70_p0", 4406),
    ("corpus_v3_fd_pichu_fox4_e4213e6a5_p0", 5321),
    // PICHU: a Thunder bolt's end writes 3 into mv+4 of Run, Squat,
    // JumpF and JumpB (ftPk_SpecialLw_SetState_Unk0), which they read as
    // their own word; Thunder's bolts re-hit Fox once their rehit timer
    // (x40_b4, 8 ticks for Pichu) runs out (ftColl_80077C60 mode 5).
    ("corpus_v3_fd_pichu_fox4_e83c73abe_p0", 652),
    ("corpus_v3_fd_pichu_fox4_e633c44d1_p1", 308),
    ("corpus_v3_fd_pichu_fox4_e6e4bdd56_p0", 468),
    ("corpus_v3_fd_pichu_fox4_e7be527d7_p0", 3295),
    ("corpus_v3_fd_pichu_fox4_eca3cc46f_p1", 799),
    ("corpus_v3_fd_pichu_fox4_e79162ccc_p2", 1961),
    ("corpus_v3_fd_pichu_fox4_eb91a3d7f_p2", 2309),
    // PICHU: a reflected Thunder Jolt crawler turns at its joint 6
    // (it_2725_Logic107_Reflected); two clean samples.
    ("corpus_v3_fd_pichu_fox4_e1005f1f4_p2", 136),
    ("corpus_v3_fd_pichu_fox4_eb6a96fbd_p1", 612),
    ("corpus_v3_fd_pichu_fox4_e936421d6_p0", 1202),
    ("corpus_v3_fd_pichu_fox4_e173083b8_p1", 2972),
    // PICHU: a Thunder bolt's end in Walk writes 3 over mv.co.walk.msid;
    // Walk's IASA (ftWalkCommon_800DFEC8) then re-enters Walk.
    ("corpus_v3_fd_pichu_fox4_e793fae16_p0", 655),
    ("corpus_v3_fd_pichu_fox4_e7bd97a61_p0", 1527),
    // PICHU thrown by Fox: a throw laser's first hitbox only glances her
    // (phantom); its second still lands a full hit (ftColl_8007925C).
    ("corpus_v3_fd_pichu_fox4_e68e14c22_p1", 2133),
    // Mario's Fireball (ftMr_SpecialN, it_8029B6F8): the item's DPtcl trail
    // (1002) before the hand flash (efAlt 0x47A), bounces (efAlt 0x47B);
    // forward smash flame 0x411; landing flash 0x423 (ftCo_8009F834 block_67).
    ("corpus_v3_fd_mario_fox4_e75fb4a9a_p1", 5087),
    ("corpus_v3_fd_mario_fox4_edb2b114a_p1", 671),
    ("corpus_v3_fd_mario_fox4_e00f31913_p1", 893),
    ("corpus_v3_fd_mario_fox4_ef89b3e70_p2", 855),
    ("corpus_v3_fd_mario_fox4_e9943b4ab_p2", 212),
    ("corpus_v3_fd_mario_fox4_e19f8579b_p0", 2789),
    ("corpus_v3_fd_mario_fox4_e19f8579b_p1", 2946),
    ("corpus_v3_fd_mario_fox4_ef89b3e70_p1", 2241),
    ("corpus_v3_fd_mario_fox4_e89a89d0e_p2", 2033),
    // Super Jump Punch (ftMr_SpecialHi): ft_80085154's steered rise, coin hits
    // (hit_effect_ids[HitElement_Coin] = efAlt 0x479, generator 1010).
    ("corpus_v3_fd_mario_fox4_e00f31913_p2", 470),
    // Mario's up throw of Fox: the throw input's captor and victim motion
    // changes flush their efAsync queues (the CatchWait flash first) inside
    // the captor's input proc; Fox's back throw flips facing after its
    // graphics commands queued the old one (ftCo_09F7.c copies facing_dir).
    ("corpus_v3_fd_mario_fox4_ec13743d5_p0", 6001),
    ("corpus_v3_fd_mario_fox4_e89a89d0e_p0", 4067),
    ("corpus_v3_fd_mario_fox4_edb2b114a_p0", 4005),
    ("corpus_v3_fd_mario_fox4_e726cfdde_p2", 2418),
    ("corpus_v3_fd_mario_fox4_e1502cb40_p2", 915),
    ("corpus_v3_fd_mario_fox4_e9943b4ab_p1", 3970),
    ("corpus_v3_fd_mario_fox4_e19f8579b_p2", 2646),
    ("corpus_v3_fd_mario_fox4_e726cfdde_p1", 829),
    // Mario Tornado (ftMr_SpecialLw): both entries in the aerial row 350, landing
    // into 349 (doAirCollIfUnk), efAlt 0x47C with efLib_Cb_ftMr_SpecialLw's tilt.
    ("corpus_v3_fd_mario_fox4_e1502cb40_p0", 3513),
    ("corpus_v3_fd_mario_fox4_e726cfdde_p0", 2068),
    // MARIO: Mario vs Fox explorer cases through the cape (turnaround, blocked
    // turnaround in catch/throw states, shield push), fireball destroy order and Fox's
    // Fire Fox x21F8 after a cape turn.
    ("corpus_v3_fd_mario_fox4_e00f31913_p0", 4770),
    ("corpus_v3_fd_mario_fox4_e75fb4a9a_p0", 4348),
    ("corpus_v3_fd_mario_fox4_e9943b4ab_p0", 6001),
    ("corpus_v3_fd_mario_fox4_ec13743d5_p1", 5655),
    ("corpus_v3_fd_mario_fox4_ef89b3e70_p0", 4235),
    ("corpus_v3_fd_mario_fox4_e89a89d0e_p1", 6001),
    ("corpus_v3_fd_mario_fox4_e573e2d95_p0", 2081),
    ("corpus_v3_fd_mario_fox4_ec3145eb3_p0", 6001),
    ("corpus_v3_fd_mario_fox4_e50814092_p0", 6001),
    ("corpus_v3_fd_mario_fox4_e6af4a7bb_p0", 6001),
    ("corpus_v3_fd_mario_fox4_e3e74affa_p0", 4489),
    ("corpus_v3_fd_mario_fox4_edafcfddf_p0", 4839),
    ("corpus_v3_fd_mario_fox4_eeda0d0fc_p0", 2937),
    ("corpus_v3_fd_mario_fox4_e005a4f43_p0", 2785),
    ("corpus_v3_fd_mario_fox4_e7ff378da_p0", 4594),
    ("corpus_v3_fd_mario_fox4_e0dee256e_p0", 6001),
    // STAGE-PS: full Pokemon Stadium explorer matches with the form reads'
    // completion replayed from the trace (22 or 23 polls; the default
    // policy says 21 for rock, 23 for water); moving floors in hitlag
    // (Fighter_procUpdate's mpGetSpeed), the water form's splashes
    // (mpLib_803BD850) and Fire Fox skipping the rock form's dynamic
    // platform (mpColl_IsOnPlatform reads the line).
    ("corpus_v3_ps_fox_marth4_e75fb4a9a_p1", 6001),
    ("corpus_v3_ps_fox_marth4_e89a89d0e_p2", 6001),
    ("corpus_v3_ps_fox_marth4_e9943b4ab_p0", 6001),
    ("corpus_v3_ps_fox_marth4_edb2b114a_p1", 5572),
    ("corpus_v3_ps_fox_marth4_ef89b3e70_p2", 6001),
    // STAGE-PS: exact samples of a 10-seed batch (seeds after 20) from
    // start_ps_fox_marth4, through the first transformation.
    ("corpus_v3_ps_fox_marth4_ee133b82f_p0", 5491),
    ("corpus_v3_ps_fox_marth4_ecdf8887e_p1", 5085),
    ("corpus_v3_ps_fox_marth4_e2b9e1400_p2", 4222),
    // ROY: exact samples of a 128-seed batch (seeds after 64) from
    // start_fd_roy_fox4 (no faults in 384 cases); particle sites exact.
    ("corpus_v3_fd_roy_fox4_e9a8e5048_p0", 6001),
    ("corpus_v3_fd_roy_fox4_ef17f3433_p2", 4045),
    ("corpus_v3_fd_roy_fox4_e721bca4f_p1", 6001),
    ("corpus_v3_fd_roy_fox4_ebf1d3d3e_p0", 4460),
    ("corpus_v3_fd_roy_fox4_e4f8edfa8_p2", 5837),
    ("corpus_v3_fd_roy_fox4_e3b84663f_p1", 4673),
    ("corpus_v3_fd_roy_fox4_e67c8e917_p0", 3367),
    ("corpus_v3_fd_roy_fox4_e7af4edc6_p2", 4187),
    ("corpus_v3_fd_roy_fox4_ebd36234b_p1", 6001),
    ("corpus_v3_fd_roy_fox4_ed5076307_p0", 6001),
    ("corpus_v3_fd_roy_fox4_ea33f1478_p2", 6001),
    ("corpus_v3_fd_roy_fox4_e6aa92c64_p1", 4286),
    // DRMARIO: Dr. Mario vs Fox explorer cases up to the first graphics 0x41C
    // (ftCo_8009F834 block_70, efAsync kind 2: generator 0x5D).
    ("corpus_v3_fd_drmario_fox4_e0dee256e_p2", 144),
    ("corpus_v3_fd_drmario_fox4_e67c8e917_p2", 144),
    // DRMARIO: exact samples of a 100-seed batch (300 cases, none faulted) from
    // start_fd_drmario_fox4: every row 341..350, thrown and taunt Megavitamins,
    // the Super Sheet, Bob-ombs and Fox's lasers.
    ("corpus_v3_fd_drmario_fox4_ef89b3e70_p0", 3393),
    ("corpus_v3_fd_drmario_fox4_e19f8579b_p1", 4229),
    ("corpus_v3_fd_drmario_fox4_eeda0d0fc_p2", 3228),
    ("corpus_v3_fd_drmario_fox4_ee62c6106_p0", 2934),
    ("corpus_v3_fd_drmario_fox4_ec0a10b25_p1", 5965),
    ("corpus_v3_fd_drmario_fox4_ea4d5d9ec_p2", 4035),
    ("corpus_v3_fd_drmario_fox4_eb17a6598_p0", 5084),
    ("corpus_v3_fd_drmario_fox4_ee32d950f_p1", 6001),
    ("corpus_v3_fd_drmario_fox4_ea7e2e7e9_p2", 6001),
    ("corpus_v3_fd_drmario_fox4_ee8d2a62f_p0", 6001),
    ("corpus_v3_fd_drmario_fox4_e2bf046d4_p1", 6001),
    ("corpus_v3_fd_drmario_fox4_e0d368f02_p2", 6001),
    // LUIGI: Luigi vs Fox explorer cases through every special: Fireball
    // (it_802C01AC, efSync 0x507/0x508, shield bounce itColl_BounceOffShield),
    // Green Missile with its misfire draw (ftLg_SpecialS_SetVars) and the
    // launch frame's effects flushed by the flight's motion change,
    // Super Jump Punch, Cyclone (efSync 0x509) and item hits on an
    // intangible fighter reaching clank and shield first (ftcoll.c:2285).
    ("corpus_v3_fd_luigi_fox4_e0ac13e8e_p0", 5646),
    ("corpus_v3_fd_luigi_fox4_e0fcf0c70_p2", 1193),
    ("corpus_v3_fd_luigi_fox4_e121faf54_p2", 4121),
    ("corpus_v3_fd_luigi_fox4_e1b05b110_p1", 6001),
    ("corpus_v3_fd_luigi_fox4_e24fdee66_p2", 6001),
    ("corpus_v3_fd_luigi_fox4_e27acb822_p0", 2844),
    ("corpus_v3_fd_luigi_fox4_e50f774a0_p1", 3634),
    ("corpus_v3_fd_luigi_fox4_e6af4a7bb_p1", 5880),
    ("corpus_v3_fd_luigi_fox4_e7f8dc4fa_p2", 6001),
    ("corpus_v3_fd_luigi_fox4_eae52f0ea_p2", 616),
    ("corpus_v3_fd_luigi_fox4_eb4935276_p1", 6001),
    ("corpus_v3_fd_luigi_fox4_ebfeefc0e_p1", 4173),
    ("corpus_v3_fd_luigi_fox4_ec13743d5_p2", 4042),
    ("corpus_v3_fd_luigi_fox4_edea229bd_p0", 4093),
    ("corpus_v3_fd_luigi_fox4_ee133b82f_p0", 5850),
    ("corpus_v3_fd_luigi_fox4_ef5188d7f_p1", 3916),
    ("corpus_v3_fd_luigi_fox4_ef89b3e70_p0", 5660),
    ("corpus_v3_fd_luigi_fox4_ef9e858d4_p0", 5340),
    ("corpus_v3_fd_luigi_fox4_efc6a328f_p2", 5007),
    // CROSS: explorer matches from the non-Fox cross-matchup boundaries.
    // DamageFly/DamageFall never float (ftCo_DamageFall_IASA); Peach's parasol.
    ("corpus_v3_fd_marth_peach4_ef89b3e70_p0", 3948),
    ("corpus_v3_fd_marth_peach4_e75fb4a9a_p1", 2848),
    ("corpus_v3_fd_marth_peach4_e573e2d95_p1", 1526),
    ("corpus_v3_fd_marth_peach4_eeda0d0fc_p0", 4737),
    // Peach's down throw: its zero-damage hitbox on the thrown victim
    // freezes neither member (dmg.x183C_applied stays 0).
    ("corpus_v3_fd_marth_peach4_e8f0de8c6_p1", 6001),
    ("corpus_v3_fd_marth_peach4_e6cc80d32_p0", 6001),
    // Yoshi's knockback eyes, inverted down bound and airborne DownBoundD; the down roll's floor projection; the inert Peach Bomber on a Shy Guy.
    ("corpus_v3_ys_peach_yoshi4_edb2b114a_p0", 1760),
    ("corpus_v3_ys_peach_yoshi4_e726cfdde_p0", 920),
    ("corpus_v3_ys_peach_yoshi4_eeda0d0fc_p0", 1907),
    ("corpus_v3_bf_yoshi_pikachu4_e726cfdde_p1", 3132),
    // Invincible hurt capsules and grabs; Peach's float aerial with a turnip; Wait takes Peach's parasol; a turnip destroyed by her damage callback empties the hand.
    ("corpus_v3_ys_peach_yoshi4_e89a89d0e_p1", 1810),
    ("corpus_v3_ys_peach_yoshi4_e6af4a7bb_p0", 1911),
    ("corpus_v3_ys_peach_yoshi4_e0fcf0c70_p1", 4660),
    ("corpus_v3_ys_peach_yoshi4_ecad2716f_p0", 1219),
    ("corpus_v3_bf_yoshi_pikachu4_ee133b82f_p2", 2228),
    // Item throw accessory scoped to the throw states, and its mv+4; Peach's up special with a turnip stowed under the parasol.
    ("corpus_v3_fd_peach_falco4_e00f31913_p1", 4395),
    ("corpus_v3_fd_peach_falco4_ec3145eb3_p0", 154),
    // mv+4 through Sing and the shield-break chain; Thunder's bolt end in Quick Attack and common states; the jolt crawler reflected by the cape.
    ("corpus_v3_ps_marth_jigglypuff4_e573e2d95_p1", 5239),
    ("corpus_v3_fod_pikachu_mario4_e50814092_p2", 1555),
    ("corpus_v3_bf_yoshi_pikachu4_ec13743d5_p2", 1619),
    ("corpus_v3_fod_pikachu_mario4_e89a89d0e_p1", 2703),
    // Item hitboxes clank with each other (it_8026FE68); Yoshi's egg bounces off a shield.
    ("corpus_v3_bf_yoshi_pikachu4_ecad2716f_p1", 679),
    // Exact cross-matchup samples.
    ("corpus_v3_fd_falco_captainfalcon4_e89a89d0e_p2", 5461),
    ("corpus_v3_fd_falco_captainfalcon4_eeda0d0fc_p1", 6001),
    // Mario's fireball bounces off a shield (itColl_BounceOffShield).
    ("corpus_v3_fod_pikachu_mario4_eae52f0ea_p2", 1366),
    // Peach drops through a platform holding a turnip (Pass keeps the item).
    ("corpus_v3_ys_peach_yoshi4_ea4d5d9ec_p1", 2354),
    // Yoshi's delayed egg powershield (ftYs_Shield_8012C850).
    ("corpus_v3_ys_peach_yoshi4_e2af099ca_p2", 87),
    ("corpus_v3_ys_peach_yoshi4_ee133b82f_p2", 2223),
    // More exact cross-matchup matches.
    ("corpus_v3_fd_peach_falco4_ec0a10b25_p0", 1668),
    ("corpus_v3_fod_pikachu_mario4_ec0a10b25_p0", 1658),
    ("corpus_v3_fd_peach_falco4_e6cc80d32_p2", 6001),
    ("corpus_v3_dl_captainfalcon_jigglypuff4_e6cc80d32_p2", 6001),
    ("corpus_v3_dl_mario_falco4_e6cc80d32_p0", 6001),
    // Mario's cape reflects Falco's laser back into him, then the reversed laser
    // hits him mid cape turn: damage without a reaction (x2220_b4, ftCo_8008EC90).
    ("corpus_v3_dl_mario_falco4_e8f0de8c6_p1", 2905),
    // ICECLIMBERS: Popo and Nana never hit or grab each other (ftLib_80086FD4,
    // ftcoll.c:1664-1676); a thrown fighter's hit on its thrower's partner;
    // Nana's CPU follow through smashes, grabs and hits.
    ("corpus_v3_fd_iceclimbers_fox4_e005a4f43_p0", 207),
    ("corpus_v3_fd_iceclimbers_fox4_e005a4f43_p2", 105),
    ("corpus_v3_fd_iceclimbers_fox4_e09db17e3_p0", 237),
    ("corpus_v3_fd_iceclimbers_fox4_e50f475b4_p1", 88),
    ("corpus_v3_fd_iceclimbers_fox4_ecdf8887e_p2", 239),
    // ICECLIMBERS: Ice Shot (ftPp_SpecialN, it_802C1590/it_802C16F8) by both climbers, grounded
    // and aerial; the ice block slides, falls off its own motion and melts.
    ("iceclimbers_iceshot_fd_fox4", 400),
    // ICECLIMBERS: an unlaunched ice block drops onto Battlefield's side
    // platform under a rising Popo and bounces (it_8026E15C, it_80276FC4, 159).
    ("iceclimbers_iceshot_platform_bf_fox4", 330),
    // ICECLIMBERS: Nana, fallen to the stage while Popo teeters on the side
    // platform above, turns and jumps back up to him (ftCo_800AB224's
    // off-island arm, ftCo_800A0148; 224, 236).
    ("iceclimbers_nana_island_bf_fox4", 420),
    // ICECLIMBERS: Nana, knocked down, picks how she gets up (behaviour 5,
    // ftCo_800AC7D4).
    ("corpus_v3_fd_iceclimbers_fox4_e2b9e1400_p1", 268),
    // ICECLIMBERS: explorer cases through Ice Shot and Nana's close attacks
    // (ftCo_800B8A9C/800B4AB0), holds, mashes and tumble steering.
    ("corpus_v3_fd_iceclimbers_fox4_e19f8579b_p0", 92),
    ("corpus_v3_fd_iceclimbers_fox4_e75fb4a9a_p1", 86),
    ("corpus_v3_fd_iceclimbers_fox4_e89a89d0e_p2", 97),
    ("corpus_v3_fd_iceclimbers_fox4_e9f348009_p1", 240),
    ("corpus_v3_fd_iceclimbers_fox4_e9f348009_p2", 155),
    ("corpus_v3_fd_iceclimbers_fox4_eae52f0ea_p2", 114),
    ("corpus_v3_fd_iceclimbers_fox4_ec0a10b25_p1", 240),
    ("corpus_v3_fd_iceclimbers_fox4_ec3145eb3_p1", 93),
    ("corpus_v3_fd_iceclimbers_fox4_ec3145eb3_p2", 98),
    ("corpus_v3_fd_iceclimbers_fox4_ecdf8887e_p0", 118),
    ("corpus_v3_fd_iceclimbers_fox4_edb2b114a_p2", 92),
    // ICECLIMBERS: Squall Hammer (ftPp_SpecialS*, ftnanaspecials.c) entered on
    // the ground and in the air with Nana linked (x1A5C, SpecialS_0/_1).
    ("corpus_v3_fd_iceclimbers_fox4_e4213e6a5_p2", 86),
    ("corpus_v3_fd_iceclimbers_fox4_e50814092_p2", 164),
    ("corpus_v3_fd_iceclimbers_fox4_e573e2d95_p1", 203),
    ("corpus_v3_fd_iceclimbers_fox4_ec13743d5_p2", 87),
    // ICECLIMBERS: Squall Hammer witnesses: B-press lifts and landings, stick
    // steering, running off the edge, the wall rebound, Nana joining from the
    // other ground state, Popo alone (Nana mid-jab), hits on Fox sharing hitlag
    // and the player's stale table, and a trade that unlinks the pair.
    ("iceclimbers_squall_mash_fd_fox4", 300),
    ("iceclimbers_squall_steer_fd_fox4", 240),
    ("iceclimbers_squall_air_fd_fox4", 270),
    ("iceclimbers_squall_edge_fd_fox4", 250),
    ("iceclimbers_squall_wall_fd_fox4", 184),
    ("iceclimbers_squall_land_fd_fox4", 240),
    ("iceclimbers_squall_hop_fd_fox4", 260),
    ("iceclimbers_squall_catch_fd_fox4", 280),
    ("iceclimbers_squall_solo_fd_fox4", 260),
    ("iceclimbers_squall_hit_fd_fox4", 300),
    ("iceclimbers_squall_trade_fd_fox4", 222),
    // ICECLIMBERS: Belay (ftPp_SpecialHi, it_802C27D4's rope) with Nana joining
    // (ftNn_Init_8012300C) or not, on the ground, in the air, off stage and
    // into a ledge grab; Nana's Belay recovery CPU (ftCo_800A8DE4,
    // behaviour 4's ftCo_800A9904). Explorer cases end at the entry.
    ("iceclimbers_belay_fd_fox4", 520),
    ("iceclimbers_belay_solo_fd_fox4", 400),
    ("iceclimbers_belay_offstage_fd_fox4", 400),
    ("iceclimbers_belay_ledge_fd_fox4", 225),
    ("corpus_v3_fd_iceclimbers_fox4_e3e74affa_p2", 97),
    ("corpus_v3_fd_iceclimbers_fox4_e50f774a0_p1", 123),
    ("corpus_v3_fd_iceclimbers_fox4_e573e2d95_p2", 105),
    ("corpus_v3_fd_iceclimbers_fox4_ec13743d5_p0", 125),
    // ICECLIMBERS: Blizzard (ftPp_SpecialLw, fn_80122D2C, itClimbersBlizzard_*) by both
    // climbers, grounded, aerial and landing; the partner joins facing away (ftCo_800B0AF4);
    // an item phantom credits its owner's stale table and combo push.
    ("corpus_v3_fd_iceclimbers_fox4_e7ff378da_p0", 96),
    ("corpus_v3_fd_iceclimbers_fox4_ee133b82f_p1", 219),
    ("corpus_v3_fd_iceclimbers_fox4_eeda0d0fc_p2", 195),
    ("corpus_v3_fd_iceclimbers_fox4_ec3145eb3_p0", 120),
    ("corpus_v3_fd_iceclimbers_fox4_e89a89d0e_p0", 92),
    ("corpus_v3_fd_iceclimbers_fox4_ecad2716f_p0", 235),
    ("corpus_v3_fd_iceclimbers_fox4_edafcfddf_p1", 92),
    ("iceclimbers_blizzard_fd_fox4", 400),
    ("iceclimbers_blizzard_fd_fox4_ec3145eb3_p0", 220),
    ("iceclimbers_blizzard_fd_fox4_ecad2716f_p0", 335),
    ("iceclimbers_blizzard_fd_fox4_e89a89d0e_p0", 192),
    ("iceclimbers_blizzard_fd_fox4_e7ff378da_p0", 196),
    ("iceclimbers_blizzard_fd_fox4_ee133b82f_p1", 319),
    // ICECLIMBERS: deaths. Nana costs no stock; Popo's revival vanishes a dying Nana
    // (ftCo_800D4F24) and revives her beside him (ftCo_800BFD9C, Player_80032070).
    ("iceclimbers_ko_both_fd_fox4", 700),
    ("iceclimbers_ko_nana_jump158_fd_fox4", 600),
    ("iceclimbers_ko_nana_jump152_fd_fox4", 600),
    ("iceclimbers_ko_nana2_fd_fox4", 700),
    // ICECLIMBERS: Nana dies alone and sleeps (ftCo_800BFD9C, fn_8016719C(slot, 1));
    // Popo's later revival brings her back.
    ("iceclimbers_ko_nana_alone_fd_fox4", 700),
    ("iceclimbers_ko_nana_alone_then_popo_fd_fox4", 1000),
    // Sheik vs Fox: a thrown fighter against an owned item (it_802703E8),
    // Vanish's landing flash (efAsync 0x3FA), the smash charge's colour
    // program (ftColl_8007B62C on a body status), Zelda's up-air particle
    // 418, and the explorer's clean samples.
    ("corpus_v3_fd_sheik_fox4_ef89b3e70_p1", 492),
    ("corpus_v3_fd_sheik_fox4_ee62c6106_p1", 3311),
    ("corpus_v3_fd_sheik_fox4_e26ea92b1_p2", 1452),
    ("corpus_v3_fd_sheik_fox4_e1b05b110_p1", 1660),
    ("corpus_v3_fd_sheik_fox4_e121faf54_p0", 232),
    // Sheik to Zelda and back (ftSk SpecialLw 361, SpecialLw2 362): the
    // explorer's pre-merge fault on the return (e255c070a_p0), the same
    // inputs run 330 ticks past it (_ext), and Zelda playing on for 2000
    // ticks after the swap (_long); Zelda's script sparkles 0x43A/0x501
    // reached from Sheik's boundary (e4213e6a5_p0, e98e8f6a6_p2).
    ("corpus_v3_fd_sheik_fox4_e255c070a_p0", 1170),
    ("corpus_v3_fd_sheik_fox4_e255c070a_p0_ext", 1500),
    ("corpus_v3_fd_sheik_fox4_e255c070a_p0_long", 3037),
    ("corpus_v3_fd_sheik_fox4_e4213e6a5_p0", 855),
    ("corpus_v3_fd_sheik_fox4_e98e8f6a6_p2", 183),
    // ZELDA: explorer samples from start_fd_zelda_fox4: her script
    // sparkles (efSync 0x500-0x502), 0x40D and the empty 0x43A animlist
    // id, and Farore's Wind's end returning the body flash.
    ("corpus_v3_fd_zelda_fox4_e0dee256e_p1", 1780),
    ("corpus_v3_fd_zelda_fox4_ee62c6106_p0", 1855),
    ("corpus_v3_fd_zelda_fox4_e50814092_p2", 4070),
    ("corpus_v3_fd_zelda_fox4_edafcfddf_p2", 4299),
    ("corpus_v3_fd_zelda_fox4_e9943b4ab_p1", 5637),
    ("corpus_v3_fd_zelda_fox4_e89a89d0e_p2", 3118),
    ("corpus_v3_fd_zelda_fox4_e9943b4ab_p2", 2102),
    ("corpus_v3_fd_zelda_fox4_e005a4f43_p1", 5823),
    // Link from its start boundary: the forward smash's second hit
    // (ftCo_800CED30, ftLk_MS_AttackS42).
    ("corpus_v3_fd_link_fox4_e9943b4ab_p2", 110),
    // Link explorer matches with hookshot grabs and Z-airs, a boomerang
    // bouncing off the floor and a wall (its release sweep's box, it_80275D5C),
    // and Link hit with the hookshot out (death1_cb, it_802A7AAC).
    ("corpus_v3_fd_link_fox4_e2b9e1400_p2", 3928),
    ("corpus_v3_fd_link_fox4_e1b05b110_p1", 3553),
    ("corpus_v3_fd_link_fox4_ecdf8887e_p2", 2847),
    ("corpus_v3_fd_link_fox4_ed97ea327_p1", 2793),
    ("corpus_v3_fd_link_fox4_e266e1150_p0", 2688),
    ("corpus_v3_fd_link_fox4_e2b9e1400_p0", 2682),
    // Link standing with his back to Fox stops a laser on the Hylian shield.
    ("corpus_v3_fd_link_fox4_edafcfddf_p2", 2737),
    // Link: the bow and arrow (SpecialN), fixed with its port.
    ("corpus_v3_fd_link_fox4_e50814092_p1", 3861),
    ("corpus_v3_fd_link_fox4_e75fb4a9a_p1", 86),
    // Link / Young Link explorer cases: bombs held, thrown, dropped and exploding in hand;
    // item hitboxes following the JObj; the arrow at the archer's scale and in shields;
    // thrown/dropped hitboxes placed past link 11; self-hits leave the stale queue alone;
    // timed shield victims; Fox's blaster callbacks per motion; the taunt milk.
    ("corpus_v3_fd_link_fox4_e7ff378da_p0", 96),
    ("corpus_v3_fd_link_fox4_ec3145eb3_p0", 120),
    ("corpus_v3_fd_link_fox4_ecad2716f_p0", 2933),
    ("corpus_v3_fd_link_fox4_ef17f3433_p1", 5943),
    ("corpus_v3_fd_link_fox4_ef17f3433_p2", 6001),
    ("corpus_v3_fd_link_fox4_ef5188d7f_p0", 6001),
    ("corpus_v3_fd_link_fox4_ef5188d7f_p1", 6001),
    ("corpus_v3_fd_link_fox4_ef5188d7f_p2", 6001),
    ("corpus_v3_fd_link_fox4_ef5e3e043_p1", 6001),
    ("corpus_v3_fd_link_fox4_ef6b5a67f_p0", 6001),
    ("corpus_v3_fd_link_fox4_ef6b5a67f_p1", 6001),
    ("corpus_v3_fd_link_fox4_ef6b5a67f_p2", 6001),
    ("corpus_v3_fd_link_fox4_ef9e858d4_p1", 6001),
    ("corpus_v3_fd_link_fox4_efc6a328f_p1", 6001),
    ("corpus_v3_fd_link_fox4_efc6a328f_p2", 6001),
    ("corpus_v3_fd_younglink_fox4_eeda0d0fc_p0", 6001),
    ("corpus_v3_fd_younglink_fox4_eeda0d0fc_p1", 5788),
    ("corpus_v3_fd_younglink_fox4_ef89b3e70_p2", 6001),
    ("corpus_v3_fd_younglink_fox4_ef9b6d16d_p0", 6001),
    ("corpus_v3_fd_younglink_fox4_ef9b6d16d_p1", 6001),
    ("corpus_v3_fd_younglink_fox4_ef9b6d16d_p2", 6001),
    // Jigglypuff explorer matches from both port layouts, through her
    // special rows (341-372: Rollout, Pound, Sing, Rest), exact since the
    // specials' port (ft-purin).
    ("corpus_v3_fd_fox_jigglypuff4_e0fcf0c70_p0", 6001),
    ("corpus_v3_fd_fox_jigglypuff4_e121faf54_p0", 3084),
    ("corpus_v3_fd_fox_jigglypuff4_e2b9e1400_p0", 2336),
    ("corpus_v3_fd_fox_jigglypuff4_e4ef0d3e0_p0", 3255),
    ("corpus_v3_fd_fox_jigglypuff4_e573e2d95_p0", 4900),
    ("corpus_v3_fd_fox_jigglypuff4_e5f386e5e_p0", 6001),
    ("corpus_v3_fd_fox_jigglypuff4_e7254fba4_p0", 3213),
    ("corpus_v3_fd_fox_jigglypuff4_e7ff378da_p0", 5879),
    ("corpus_v3_fd_fox_jigglypuff4_e8be4d273_p0", 5574),
    ("corpus_v3_fd_fox_jigglypuff4_e8c6ff530_p0", 5850),
    ("corpus_v3_fd_fox_jigglypuff4_ebfeefc0e_p0", 3869),
    ("corpus_v3_fd_fox_jigglypuff4_ee98155f3_p0", 4052),
    ("corpus_v3_fd_fox_jigglypuff4_efc6a328f_p0", 2741),
    ("corpus_v3_fd_jigglypuff_fox4_e0fcf0c70_p0", 4782),
    ("corpus_v3_fd_jigglypuff_fox4_e121faf54_p0", 6001),
    ("corpus_v3_fd_jigglypuff_fox4_e1611c835_p0", 3106),
    ("corpus_v3_fd_jigglypuff_fox4_e3e74affa_p0", 3668),
    ("corpus_v3_fd_jigglypuff_fox4_e3ef41ca4_p0", 1502),
    ("corpus_v3_fd_jigglypuff_fox4_e4068796b_p0", 2650),
    ("corpus_v3_fd_jigglypuff_fox4_e4ef0d3e0_p0", 4865),
    ("corpus_v3_fd_jigglypuff_fox4_e573e2d95_p0", 2975),
    ("corpus_v3_fd_jigglypuff_fox4_e5f386e5e_p0", 6001),
    ("corpus_v3_fd_jigglypuff_fox4_e8be4d273_p0", 6001),
    ("corpus_v3_fd_jigglypuff_fox4_e8c6ff530_p0", 4820),
    ("corpus_v3_fd_jigglypuff_fox4_e8f0de8c6_p0", 5332),
    ("corpus_v3_fd_jigglypuff_fox4_eae52f0ea_p0", 3503),
    ("corpus_v3_fd_jigglypuff_fox4_eb17a6598_p0", 4495),
    ("corpus_v3_fd_jigglypuff_fox4_ebfeefc0e_p0", 5660),
    ("corpus_v3_fd_jigglypuff_fox4_edb4b01fd_p0", 3938),
    ("corpus_v3_fd_jigglypuff_fox4_ee98155f3_p0", 4285),
    // Clean Fox-Marth explorer samples (normal from both port layouts and
    // Sudden Death), bridged alongside the fault witnesses above.
    ("corpus_sd_s1_e2726590c_p0", 1683),
    ("corpus_v3_s0_e01a74e09_p0", 6001),
    ("corpus_v3_s0_e1f186101_p2", 6001),
    ("corpus_v3_s0_e37053c42_p0", 4151),
    ("corpus_v3_s0_e42b75250_p1", 4763),
    ("corpus_v3_s0_e74315b3d_p2", 2447),
    ("corpus_v3_s0_e8500e42f_p1", 4994),
    ("corpus_v3_s0_ec36a7713_p1", 6001),
    ("corpus_v3_s0_ec72b9942_p2", 6001),
    ("corpus_v3_s1_e035918d1_p1", 5803),
    ("corpus_v3_s1_e1f186101_p2", 3011),
    ("corpus_v3_s1_e37053c42_p0", 5863),
    ("corpus_v3_s1_e74315b3d_p2", 2641),
    ("corpus_v3_s1_e8500e42f_p1", 6001),
    ("corpus_v3_s1_ec36a7713_p1", 5266),
    ("corpus_v3_s1_ec72b9942_p2", 5633),
    // Peach pulls a Beam Sword (it_802BD4AC) and swings it.
    ("corpus_v3_fd_peach_falco4_efc6a328f_p0", 286),
];

#[test]
fn corpus_v3_matches_retail() {
    gate_in_parallel(&CORPUS_V3_MATCHES);
}

/// Hits found by tournament replays (Slippi wave). A shield takes a hit
/// while ledge intangibility lasts: ftColl_80078C70 tests the shield
/// (0x80079074) before the x1988/x198C checks (0x800790B4) that skip only the
/// hurt capsules.
#[test]
fn intangible_shield_takes_the_hit() {
    combat_gate_ticks("ledgedash_shield_intangible_fd_falco_fox4", 300);
}

/// Falco's Phantasm article keeps the command size as its capsule radius at
/// scl 1.1 (it_8029CFF0's it_80274594, 0x8029D070), which reaches a falling
/// Fox on its first frame: 300 ticks of a console game (Slippi wave) fed from
/// a neutral-spawn UCF boundary.
#[test]
fn phantasm_capsule_keeps_the_command_size() {
    combat_gate_ticks("slp_bf_fox_falco_phantasm_t300", 300);
}

/// Fox's up throw on Fountain of Dreams' main floor, at Marth's weight rate.
#[test]
fn up_throw_on_fountain_of_dreams_floor_matches_retail() {
    combat_gate_ticks("throwhi_floor_fod_fox_marth4", 330);
}

/// Fox grabs and up-throws Marth on a Fountain of Dreams side platform while
/// it descends. The captured fighter rides the floor in Fighter_procUpdate's
/// tail (mpGetSpeed, 0x8006BE48) before the accessory pins it to the captor
/// again, so the tick's end state does not show the ride; Slippi replays older
/// than 3.4.0 (Post Frame at the map proc, 0x8006C5D8) do, and two of the
/// corpus's FoD games check it. The recording samples retail there too
/// (`after_map`), and the gate compares it: thrown Marth's y is 18.675087 at
/// tick 660, one platform step under where he stands without the ride.
#[test]
fn capture_and_throw_on_a_moving_platform_match_retail() {
    let name = "capture_moving_platform_fod_fox_marth4";
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml"));
    assert!(
        Scenario::load(&path).unwrap().after_map,
        "the witness is its after-map sample"
    );
    combat_gate_ticks(name, 740);
}
