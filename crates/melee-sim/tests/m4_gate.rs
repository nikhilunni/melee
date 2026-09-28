use melee_sim::{scenario::Scenario, trace};
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
    particle_rng_sites_with_ledger(name, expected_ticks, "ledger");
}
fn particle_rng_sites_with_ledger(name: &str, expected_ticks: usize, ledger_suffix: &str) {
    use melee_sim::{frame::Simulation, initial_state::InitialState, inputs::PadScript};
    let Some(scenario) = local_scenario_named(name) else {
        return;
    };
    let path = scenario.trace_path(&format!("{ledger_suffix}.raw.jsonl"));
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let ledger = melee_test_support::trace::read_to_string(&path).unwrap();
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
            .filter(|site| *site == 0x8039_9114 || (0x8039_930C..0x8039_F6CC).contains(site))
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

#[test]
fn idle_fd_marth_600() {
    movement_gate_ticks("idle_fd_marth", 600);
}
#[test]
fn start_fd_marth_600() {
    movement_gate_ticks("start_fd_marth", 600);
}

#[test]
fn idle_marth_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_marth", 600, "ledger600");
}
#[test]
fn start_marth_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_marth", 600, "ledger600");
}

#[test]
fn squat_fd_marth_300() {
    movement_gate_ticks("squat_fd_marth", 300);
}

#[test]
fn turn_fd_marth_300() {
    movement_gate_ticks("turn_fd_marth", 300);
}

#[test]
fn walk_fd_marth_300() {
    movement_gate_ticks("walk_fd_marth", 300);
}

#[test]
fn dash_fd_marth_300() {
    movement_gate_ticks("dash_fd_marth", 300);
}

#[test]
fn jump_fd_marth_300() {
    movement_gate_ticks("jump_fd_marth", 300);
}

#[test]
fn shield_fd_marth_300() {
    movement_gate_ticks("shield_fd_marth", 300);
}

#[test]
fn spotdodge_fd_marth_300() {
    movement_gate_ticks("spotdodge_fd_marth", 300);
}

#[test]
fn roll_fd_marth_300() {
    movement_gate_ticks("roll_fd_marth", 300);
}

#[test]
fn airdodge_fd_marth_300() {
    movement_gate_ticks("airdodge_fd_marth", 300);
}

#[test]
fn wavedash_fd_marth_300() {
    movement_gate_ticks("wavedash_fd_marth", 300);
}

#[test]
fn ledge_fd_marth_420() {
    movement_gate_ticks("ledge_fd_marth", 420);
}

#[test]
fn turnrun_fd_marth_300() {
    movement_gate_ticks("turnrun_fd_marth", 300);
}

#[test]
fn walkfast_fd_marth_300() {
    movement_gate_ticks("walkfast_fd_marth", 300);
}

#[test]
fn ledgeclimb_fd_marth_420() {
    movement_gate_ticks("ledgeclimb_fd_marth", 420);
}

#[test]
fn ledgeescape_fd_marth_420() {
    movement_gate_ticks("ledgeescape_fd_marth", 420);
}

#[test]
fn airjumpb_fd_marth_300() {
    movement_gate_ticks("airjumpb_fd_marth", 300);
}

#[test]
fn airjumpb_fd_fox_300() {
    movement_gate("airjumpb_fd_fox");
}

#[test]
fn squat_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("squat_fd_marth", 300);
}

#[test]
fn turn_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("turn_fd_marth", 300);
}

#[test]
fn walk_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("walk_fd_marth", 300);
}

#[test]
fn dash_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("dash_fd_marth", 300);
}

#[test]
fn jump_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("jump_fd_marth", 300);
}

#[test]
fn shield_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("shield_fd_marth", 300);
}

#[test]
fn spotdodge_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("spotdodge_fd_marth", 300);
}

#[test]
fn roll_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("roll_fd_marth", 300);
}

#[test]
fn airdodge_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("airdodge_fd_marth", 300);
}

#[test]
fn wavedash_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("wavedash_fd_marth", 300);
}

#[test]
fn ledge_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("ledge_fd_marth", 420);
}

#[test]
fn turnrun_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("turnrun_fd_marth", 300);
}

#[test]
fn walkfast_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("walkfast_fd_marth", 300);
}

#[test]
fn ledgeclimb_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("ledgeclimb_fd_marth", 420);
}

#[test]
fn ledgeescape_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("ledgeescape_fd_marth", 420);
}

#[test]
fn airjumpb_marth_particle_draw_order() {
    particle_rng_sites_for_ticks("airjumpb_fd_marth", 300);
}

#[test]
fn airjumpb_fox_particle_draw_order() {
    particle_rng_sites_for_ticks("airjumpb_fd_fox", 300);
}

#[test]
fn idle_bf_fox_600() {
    movement_gate_ticks("idle_bf_fox", 600);
}
#[test]
fn start_bf_fox_600() {
    movement_gate_ticks("start_bf_fox", 600);
}
#[test]
fn battlefield_idle_particle_rng_order() {
    particle_rng_sites_with_ledger("idle_bf_fox", 600, "ledger600");
}
#[test]
fn battlefield_start_particle_rng_order() {
    particle_rng_sites_with_ledger("start_bf_fox", 600, "ledger600");
}

#[test]
fn idle_fd_falco_600() {
    movement_gate_ticks("idle_fd_falco", 600);
}

#[test]
fn idle_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_falco", 600, "ledger600");
}

#[test]
fn start_fd_falco_600() {
    movement_gate_ticks("start_fd_falco", 600);
}

#[test]
fn start_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_falco", 600, "ledger600");
}

#[test]
fn squat_fd_falco_300() {
    movement_gate_ticks("squat_fd_falco", 300);
}

#[test]
fn squat_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("squat_fd_falco", 300, "ledger");
}

#[test]
fn turn_fd_falco_300() {
    movement_gate_ticks("turn_fd_falco", 300);
}

#[test]
fn turn_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("turn_fd_falco", 300, "ledger");
}

#[test]
fn walk_fd_falco_300() {
    movement_gate_ticks("walk_fd_falco", 300);
}

#[test]
fn walk_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("walk_fd_falco", 300, "ledger");
}

#[test]
fn walkfast_fd_falco_300() {
    movement_gate_ticks("walkfast_fd_falco", 300);
}

#[test]
fn walkfast_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("walkfast_fd_falco", 300, "ledger");
}

#[test]
fn dash_fd_falco_300() {
    movement_gate_ticks("dash_fd_falco", 300);
}

#[test]
fn dash_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("dash_fd_falco", 300, "ledger");
}

#[test]
fn turnrun_fd_falco_300() {
    movement_gate_ticks("turnrun_fd_falco", 300);
}

#[test]
fn turnrun_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("turnrun_fd_falco", 300, "ledger");
}

#[test]
fn jump_fd_falco_300() {
    movement_gate_ticks("jump_fd_falco", 300);
}

#[test]
fn jump_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("jump_fd_falco", 300, "ledger");
}

#[test]
fn airjumpb_fd_falco_300() {
    movement_gate_ticks("airjumpb_fd_falco", 300);
}

#[test]
fn airjumpb_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("airjumpb_fd_falco", 300, "ledger");
}

#[test]
fn shield_fd_falco_300() {
    movement_gate_ticks("shield_fd_falco", 300);
}

#[test]
fn shield_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("shield_fd_falco", 300, "ledger");
}

#[test]
fn spotdodge_fd_falco_300() {
    movement_gate_ticks("spotdodge_fd_falco", 300);
}

#[test]
fn spotdodge_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("spotdodge_fd_falco", 300, "ledger");
}

#[test]
fn roll_fd_falco_300() {
    movement_gate_ticks("roll_fd_falco", 300);
}

#[test]
fn roll_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("roll_fd_falco", 300, "ledger");
}

#[test]
fn airdodge_fd_falco_300() {
    movement_gate_ticks("airdodge_fd_falco", 300);
}

#[test]
fn airdodge_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("airdodge_fd_falco", 300, "ledger");
}

#[test]
fn wavedash_fd_falco_300() {
    movement_gate_ticks("wavedash_fd_falco", 300);
}

#[test]
fn wavedash_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("wavedash_fd_falco", 300, "ledger");
}

#[test]
fn ledge_fd_falco_420() {
    movement_gate_ticks("ledge_fd_falco", 420);
}

#[test]
fn ledge_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("ledge_fd_falco", 420, "ledger");
}

#[test]
fn ledgeclimb_fd_falco_420() {
    movement_gate_ticks("ledgeclimb_fd_falco", 420);
}

#[test]
fn ledgeclimb_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeclimb_fd_falco", 420, "ledger");
}

#[test]
fn ledgeescape_fd_falco_420() {
    movement_gate_ticks("ledgeescape_fd_falco", 420);
}

#[test]
fn ledgeescape_falco_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeescape_fd_falco", 420, "ledger");
}

#[test]
fn platform_bf_fox_300() {
    movement_gate("platform_bf_fox");
}
#[test]
fn battlefield_platform_particle_rng_order() {
    particle_rng_sites_for_ticks("platform_bf_fox", 300);
}

#[test]
fn idle_ys_fox_600() {
    movement_gate_ticks("idle_ys_fox", 600);
}
#[test]
fn story_idle_particle_rng_order() {
    particle_rng_sites_with_ledger("idle_ys_fox", 600, "ledger600");
}

#[test]
fn idle_fd_falcon_600() {
    movement_gate_ticks("idle_fd_falcon", 600);
}

#[test]
fn idle_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_falcon", 600, "ledger600");
}

#[test]
fn start_fd_falcon_600() {
    movement_gate_ticks("start_fd_falcon", 600);
}

#[test]
fn start_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_falcon", 600, "ledger600");
}

#[test]
fn squat_fd_falcon_300() {
    movement_gate_ticks("squat_fd_falcon", 300);
}

#[test]
fn squat_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("squat_fd_falcon", 300, "ledger");
}

#[test]
fn turn_fd_falcon_300() {
    movement_gate_ticks("turn_fd_falcon", 300);
}

#[test]
fn turn_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("turn_fd_falcon", 300, "ledger");
}

#[test]
fn walk_fd_falcon_300() {
    movement_gate_ticks("walk_fd_falcon", 300);
}

#[test]
fn walk_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("walk_fd_falcon", 300, "ledger");
}

#[test]
fn walkfast_fd_falcon_300() {
    movement_gate_ticks("walkfast_fd_falcon", 300);
}

#[test]
fn walkfast_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("walkfast_fd_falcon", 300, "ledger");
}

#[test]
fn dash_fd_falcon_300() {
    movement_gate_ticks("dash_fd_falcon", 300);
}

#[test]
fn dash_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("dash_fd_falcon", 300, "ledger");
}

#[test]
fn turnrun_fd_falcon_300() {
    movement_gate_ticks("turnrun_fd_falcon", 300);
}

#[test]
fn turnrun_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("turnrun_fd_falcon", 300, "ledger");
}

#[test]
fn jump_fd_falcon_300() {
    movement_gate_ticks("jump_fd_falcon", 300);
}

#[test]
fn jump_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("jump_fd_falcon", 300, "ledger");
}

#[test]
fn airjumpb_fd_falcon_300() {
    movement_gate_ticks("airjumpb_fd_falcon", 300);
}

#[test]
fn airjumpb_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("airjumpb_fd_falcon", 300, "ledger");
}

#[test]
fn shield_fd_falcon_300() {
    movement_gate_ticks("shield_fd_falcon", 300);
}

#[test]
fn shield_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("shield_fd_falcon", 300, "ledger");
}

#[test]
fn spotdodge_fd_falcon_300() {
    movement_gate_ticks("spotdodge_fd_falcon", 300);
}

#[test]
fn spotdodge_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("spotdodge_fd_falcon", 300, "ledger");
}

#[test]
fn roll_fd_falcon_300() {
    movement_gate_ticks("roll_fd_falcon", 300);
}

#[test]
fn roll_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("roll_fd_falcon", 300, "ledger");
}

#[test]
fn airdodge_fd_falcon_300() {
    movement_gate_ticks("airdodge_fd_falcon", 300);
}

#[test]
fn airdodge_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("airdodge_fd_falcon", 300, "ledger");
}

#[test]
fn wavedash_fd_falcon_300() {
    movement_gate_ticks("wavedash_fd_falcon", 300);
}

#[test]
fn wavedash_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("wavedash_fd_falcon", 300, "ledger");
}

#[test]
fn ledge_fd_falcon_420() {
    movement_gate_ticks("ledge_fd_falcon", 420);
}

#[test]
fn ledge_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("ledge_fd_falcon", 420, "ledger");
}

#[test]
fn ledgeclimb_fd_falcon_420() {
    movement_gate_ticks("ledgeclimb_fd_falcon", 420);
}

#[test]
fn ledgeclimb_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeclimb_fd_falcon", 420, "ledger");
}

#[test]
fn ledgeescape_fd_falcon_420() {
    movement_gate_ticks("ledgeescape_fd_falcon", 420);
}

#[test]
fn ledgeescape_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeescape_fd_falcon", 420, "ledger");
}

#[test]
fn idle_fd_peach_600() {
    movement_gate_ticks("idle_fd_peach", 600);
}

#[test]
fn idle_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_peach", 600, "ledger600");
}

#[test]
fn start_fd_peach_600() {
    movement_gate_ticks("start_fd_peach", 600);
}

#[test]
fn start_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_peach", 600, "ledger600");
}

#[test]
fn squat_fd_peach_300() {
    movement_gate_ticks("squat_fd_peach", 300);
}

#[test]
fn squat_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("squat_fd_peach", 300, "ledger");
}

#[test]
fn turn_fd_peach_300() {
    movement_gate_ticks("turn_fd_peach", 300);
}

#[test]
fn turn_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("turn_fd_peach", 300, "ledger");
}

#[test]
fn walk_fd_peach_300() {
    movement_gate_ticks("walk_fd_peach", 300);
}

#[test]
fn walk_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("walk_fd_peach", 300, "ledger");
}

#[test]
fn walkfast_fd_peach_300() {
    movement_gate_ticks("walkfast_fd_peach", 300);
}

#[test]
fn walkfast_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("walkfast_fd_peach", 300, "ledger");
}

#[test]
fn dash_fd_peach_300() {
    movement_gate_ticks("dash_fd_peach", 300);
}

#[test]
fn dash_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("dash_fd_peach", 300, "ledger");
}

#[test]
fn turnrun_fd_peach_300() {
    movement_gate_ticks("turnrun_fd_peach", 300);
}

#[test]
fn turnrun_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("turnrun_fd_peach", 300, "ledger");
}

#[test]
fn jump_fd_peach_300() {
    movement_gate_ticks("jump_fd_peach", 300);
}

#[test]
fn jump_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("jump_fd_peach", 300, "ledger");
}

#[test]
fn airjumpb_fd_peach_300() {
    movement_gate_ticks("airjumpb_fd_peach", 300);
}

#[test]
fn airjumpb_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("airjumpb_fd_peach", 300, "ledger");
}

#[test]
fn shield_fd_peach_300() {
    movement_gate_ticks("shield_fd_peach", 300);
}

#[test]
fn shield_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("shield_fd_peach", 300, "ledger");
}

#[test]
fn spotdodge_fd_peach_300() {
    movement_gate_ticks("spotdodge_fd_peach", 300);
}

#[test]
fn spotdodge_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("spotdodge_fd_peach", 300, "ledger");
}

#[test]
fn roll_fd_peach_300() {
    movement_gate_ticks("roll_fd_peach", 300);
}

#[test]
fn roll_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("roll_fd_peach", 300, "ledger");
}

#[test]
fn airdodge_fd_peach_300() {
    movement_gate_ticks("airdodge_fd_peach", 300);
}

#[test]
fn airdodge_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("airdodge_fd_peach", 300, "ledger");
}

#[test]
fn wavedash_fd_peach_300() {
    movement_gate_ticks("wavedash_fd_peach", 300);
}

#[test]
fn wavedash_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("wavedash_fd_peach", 300, "ledger");
}

#[test]
fn ledge_fd_peach_420() {
    movement_gate_ticks("ledge_fd_peach", 420);
}

#[test]
fn ledge_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("ledge_fd_peach", 420, "ledger");
}

#[test]
fn ledgeclimb_fd_peach_420() {
    movement_gate_ticks("ledgeclimb_fd_peach", 420);
}

#[test]
fn ledgeclimb_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeclimb_fd_peach", 420, "ledger");
}

#[test]
fn ledgeescape_fd_peach_420() {
    movement_gate_ticks("ledgeescape_fd_peach", 420);
}

#[test]
fn ledgeescape_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeescape_fd_peach", 420, "ledger");
}

#[test]
fn start_ys_fox_600() {
    movement_gate_ticks("start_ys_fox", 600);
}
#[test]
fn start_ys_fox_cold_600() {
    movement_gate_ticks("start_ys_fox_cold", 600);
}
#[test]
fn story_start_particle_rng_order() {
    particle_rng_sites_with_ledger("start_ys_fox", 600, "ledger600");
}

#[test]
fn idle_fd_yoshi_600() {
    movement_gate_ticks("idle_fd_yoshi", 600);
}

#[test]
fn idle_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_yoshi", 600, "ledger600");
}

#[test]
fn start_fd_yoshi_600() {
    movement_gate_ticks("start_fd_yoshi", 600);
}

#[test]
fn start_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_yoshi", 600, "ledger600");
}

#[test]
fn squat_fd_yoshi_300() {
    movement_gate_ticks("squat_fd_yoshi", 300);
}

#[test]
fn squat_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("squat_fd_yoshi", 300, "ledger");
}

#[test]
fn turn_fd_yoshi_300() {
    movement_gate_ticks("turn_fd_yoshi", 300);
}

#[test]
fn turn_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("turn_fd_yoshi", 300, "ledger");
}

#[test]
fn walk_fd_yoshi_300() {
    movement_gate_ticks("walk_fd_yoshi", 300);
}

#[test]
fn walk_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("walk_fd_yoshi", 300, "ledger");
}

#[test]
fn walkfast_fd_yoshi_300() {
    movement_gate_ticks("walkfast_fd_yoshi", 300);
}

#[test]
fn walkfast_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("walkfast_fd_yoshi", 300, "ledger");
}

#[test]
fn dash_fd_yoshi_300() {
    movement_gate_ticks("dash_fd_yoshi", 300);
}

#[test]
fn dash_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("dash_fd_yoshi", 300, "ledger");
}

#[test]
fn turnrun_fd_yoshi_300() {
    movement_gate_ticks("turnrun_fd_yoshi", 300);
}

#[test]
fn turnrun_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("turnrun_fd_yoshi", 300, "ledger");
}

#[test]
fn jump_fd_yoshi_300() {
    movement_gate_ticks("jump_fd_yoshi", 300);
}

#[test]
fn jump_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("jump_fd_yoshi", 300, "ledger");
}

#[test]
fn airjumpb_fd_yoshi_300() {
    movement_gate_ticks("airjumpb_fd_yoshi", 300);
}

#[test]
fn airjumpb_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("airjumpb_fd_yoshi", 300, "ledger");
}

#[test]
fn shield_fd_yoshi_300() {
    movement_gate_ticks("shield_fd_yoshi", 300);
}

#[test]
fn shield_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("shield_fd_yoshi", 300, "ledger");
}

#[test]
fn spotdodge_fd_yoshi_300() {
    movement_gate_ticks("spotdodge_fd_yoshi", 300);
}

#[test]
fn spotdodge_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("spotdodge_fd_yoshi", 300, "ledger");
}

#[test]
fn roll_fd_yoshi_300() {
    movement_gate_ticks("roll_fd_yoshi", 300);
}

#[test]
fn roll_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("roll_fd_yoshi", 300, "ledger");
}

#[test]
fn airdodge_fd_yoshi_300() {
    movement_gate_ticks("airdodge_fd_yoshi", 300);
}

#[test]
fn airdodge_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("airdodge_fd_yoshi", 300, "ledger");
}

#[test]
fn wavedash_fd_yoshi_300() {
    movement_gate_ticks("wavedash_fd_yoshi", 300);
}

#[test]
fn wavedash_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("wavedash_fd_yoshi", 300, "ledger");
}

#[test]
fn ledge_fd_yoshi_420() {
    movement_gate_ticks("ledge_fd_yoshi", 420);
}

#[test]
fn ledge_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("ledge_fd_yoshi", 420, "ledger");
}

#[test]
fn ledgeclimb_fd_yoshi_420() {
    movement_gate_ticks("ledgeclimb_fd_yoshi", 420);
}

#[test]
fn ledgeclimb_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeclimb_fd_yoshi", 420, "ledger");
}

#[test]
fn ledgeescape_fd_yoshi_420() {
    movement_gate_ticks("ledgeescape_fd_yoshi", 420);
}

#[test]
fn ledgeescape_yoshi_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeescape_fd_yoshi", 420, "ledger");
}

#[test]
fn idle_dl_fox_600() {
    movement_gate_ticks("idle_dl_fox", 600);
}
#[test]
fn start_dl_fox_600() {
    movement_gate_ticks("start_dl_fox", 600);
}
#[test]
fn start_dl_fox_cold_600() {
    movement_gate_ticks("start_dl_fox_cold", 600);
}
#[test]
fn dream_land_idle_particle_rng_order() {
    particle_rng_sites_with_ledger("idle_dl_fox", 600, "ledger600");
}
#[test]
fn dream_land_start_particle_rng_order() {
    particle_rng_sites_with_ledger("start_dl_fox", 600, "ledger600");
}
#[test]
fn idle_fod_fox_600() {
    movement_gate_ticks("idle_fod_fox", 600);
}
#[test]
fn start_fod_fox_600() {
    movement_gate_ticks("start_fod_fox", 600);
}
#[test]
fn fountain_of_dreams_particle_rng_order() {
    particle_rng_sites_with_ledger("idle_fod_fox", 600, "ledger600");
    particle_rng_sites_with_ledger("start_fod_fox", 600, "ledger600");
    particle_rng_sites_with_ledger("start_fod_fox_marth4", 600, "ledger600");
}

#[test]
fn idle_fd_puff_600() {
    movement_gate_ticks("idle_fd_puff", 600);
}

#[test]
fn idle_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_puff", 600, "ledger600");
}

#[test]
fn start_fd_puff_600() {
    movement_gate_ticks("start_fd_puff", 600);
}

#[test]
fn start_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_puff", 600, "ledger600");
}

#[test]
fn squat_fd_puff_300() {
    movement_gate_ticks("squat_fd_puff", 300);
}

#[test]
fn squat_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("squat_fd_puff", 300, "ledger");
}

#[test]
fn turn_fd_puff_300() {
    movement_gate_ticks("turn_fd_puff", 300);
}

#[test]
fn turn_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("turn_fd_puff", 300, "ledger");
}

#[test]
fn walk_fd_puff_300() {
    movement_gate_ticks("walk_fd_puff", 300);
}

#[test]
fn walk_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("walk_fd_puff", 300, "ledger");
}

#[test]
fn walkfast_fd_puff_300() {
    movement_gate_ticks("walkfast_fd_puff", 300);
}

#[test]
fn walkfast_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("walkfast_fd_puff", 300, "ledger");
}

#[test]
fn dash_fd_puff_300() {
    movement_gate_ticks("dash_fd_puff", 300);
}

#[test]
fn dash_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("dash_fd_puff", 300, "ledger");
}

#[test]
fn turnrun_fd_puff_300() {
    movement_gate_ticks("turnrun_fd_puff", 300);
}

#[test]
fn turnrun_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("turnrun_fd_puff", 300, "ledger");
}

#[test]
fn jump_fd_puff_300() {
    movement_gate_ticks("jump_fd_puff", 300);
}

#[test]
fn jump_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("jump_fd_puff", 300, "ledger");
}

#[test]
fn airjumpb_fd_puff_300() {
    movement_gate_ticks("airjumpb_fd_puff", 300);
}

#[test]
fn airjumpb_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("airjumpb_fd_puff", 300, "ledger");
}

#[test]
fn shield_fd_puff_300() {
    movement_gate_ticks("shield_fd_puff", 300);
}

#[test]
fn shield_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("shield_fd_puff", 300, "ledger");
}

#[test]
fn spotdodge_fd_puff_300() {
    movement_gate_ticks("spotdodge_fd_puff", 300);
}

#[test]
fn spotdodge_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("spotdodge_fd_puff", 300, "ledger");
}

#[test]
fn roll_fd_puff_300() {
    movement_gate_ticks("roll_fd_puff", 300);
}

#[test]
fn roll_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("roll_fd_puff", 300, "ledger");
}

#[test]
fn airdodge_fd_puff_300() {
    movement_gate_ticks("airdodge_fd_puff", 300);
}

#[test]
fn airdodge_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("airdodge_fd_puff", 300, "ledger");
}

#[test]
fn wavedash_fd_puff_300() {
    movement_gate_ticks("wavedash_fd_puff", 300);
}

#[test]
fn wavedash_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("wavedash_fd_puff", 300, "ledger");
}

#[test]
fn ledge_fd_puff_420() {
    movement_gate_ticks("ledge_fd_puff", 420);
}

#[test]
fn ledge_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("ledge_fd_puff", 420, "ledger");
}

#[test]
fn ledgeclimb_fd_puff_420() {
    movement_gate_ticks("ledgeclimb_fd_puff", 420);
}

#[test]
fn ledgeclimb_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeclimb_fd_puff", 420, "ledger");
}

#[test]
fn ledgeescape_fd_puff_420() {
    movement_gate_ticks("ledgeescape_fd_puff", 420);
}

#[test]
fn ledgeescape_puff_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeescape_fd_puff", 420, "ledger");
}
