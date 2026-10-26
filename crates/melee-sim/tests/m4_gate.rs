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
/// The N64 stages' start boundaries (docs/PORT_NOTES/N64_STAGES.md):
/// Yoshi's Island N64's guest draws every frame; Kongo Jungle N64's barrel
/// and bird draw at creation.
#[test]
fn n64_stage_starts_600() {
    movement_gate_ticks("start_yi64_fox_marth4", 600);
    movement_gate_ticks("start_kj64_fox_marth4", 600);
}
#[test]
fn n64_stage_particle_rng_order() {
    particle_rng_sites_with_ledger("start_yi64_fox_marth4", 600, "ledger600");
    particle_rng_sites_with_ledger("start_kj64_fox_marth4", 600, "ledger600");
}

#[test]
fn jungle_japes_and_kongo_jungle_start_particle_rng_order() {
    particle_rng_sites_with_ledger("start_jj_fox_marth4", 600, "ledger600");
    particle_rng_sites_with_ledger("start_kj_fox_marth4", 600, "ledger600");
}
/// Hyrule Temple's start boundary: three static maps and the permanent
/// stage wind (grShrine_80201C64's lb_80011A50), which Marth's cape follows.
#[test]
fn start_ht_fox_marth4_600() {
    movement_gate_ticks("start_ht_fox_marth4", 600);
    movement_gate_ticks("start_ht_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_ht_fox_marth4", 600, "ledger600");
}
/// Fourside's start boundary: the crane and the helicopter draw their
/// first waits at creation (grFourside_801F3154, 801F3C40).
#[test]
fn start_fs_fox_marth4_600() {
    movement_gate_ticks("start_fs_fox_marth4", 600);
    movement_gate_ticks("start_fs_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_fs_fox_marth4", 600, "ledger600");
}
/// Brinstar Depths' start boundary: Kraid draws his first wait at
/// creation (grKraid_801FE818).
#[test]
fn start_bd_fox_marth4_600() {
    movement_gate_ticks("start_bd_fox_marth4", 600);
    movement_gate_ticks("start_bd_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_bd_fox_marth4", 600, "ledger600");
}
/// Yoshi's Island's start boundary: map 1's model loop and bones 23 and
/// 26's, the blocks deferred to the countdown's end (grYorster_80202160).
#[test]
fn start_yi_fox_marth4_600() {
    movement_gate_ticks("start_yi_fox_marth4", 600);
    movement_gate_ticks("start_yi_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_yi_fox_marth4", 600, "ledger600");
}
/// Mushroom Kingdom's start boundary: nineteen brick material items made
/// at creation, the lifts level, items off (no `?` blocks).
#[test]
fn start_mk_fox_marth4_600() {
    movement_gate_ticks("start_mk_fox_marth4", 600);
    movement_gate_ticks("start_mk_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_mk_fox_marth4", 600, "ledger600");
}
/// Brinstar's start boundaries: map 4's wait and the acid's first delay and
/// height at creation, the bubbles' hundred settling ticks, the stale pulse
/// clock (grZebes_801D8644 leaves map 6's xF8 to the heap) on the first tick.
#[test]
fn start_brin_fox_marth4_600() {
    movement_gate_ticks("start_brin_fox_marth4", 600);
    movement_gate_ticks("start_brin_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_brin_fox_marth4", 600, "ledger600");
    movement_gate_ticks("start_brin_falco_peach4", 600);
    movement_gate_ticks("start_brin_falco_peach4_cold", 600);
    particle_rng_sites_with_ledger("start_brin_falco_peach4", 600, "ledger600");
}
/// Venom's start boundary: no draw at creation (the first Arwing's wait is
/// 600 to 600), the Great Fox's animations requested and its joints bound.
#[test]
fn start_venom_fox_marth4_600() {
    movement_gate_ticks("start_venom_fox_marth4", 600);
    movement_gate_ticks("start_venom_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_venom_fox_marth4", 600, "ledger600");
}
/// Onett's start boundary: the smoke's first pause and the cars' discarded
/// draw at creation, the cars released when the countdown ends.
#[test]
fn start_onett_fox_marth4_600() {
    movement_gate_ticks("start_onett_fox_marth4", 600);
    movement_gate_ticks("start_onett_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_onett_fox_marth4", 600, "ledger600");
}
/// Mute City's start boundary: the route's first 600 ticks (the platform's
/// first stop on the track), the thirty cars driving with their draws.
#[test]
fn start_mc_fox_marth4_600() {
    movement_gate_ticks("start_mc_fox_marth4", 600);
    movement_gate_ticks("start_mc_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_mc_fox_marth4", 600, "ledger600");
}
/// Poké Floats' start boundary: the fixed camera and the sequence's first
/// 600 ticks (the first Pokémon floating in), the sky's draws.
#[test]
fn start_pf_fox_marth4_600() {
    movement_gate_ticks("start_pf_fox_marth4", 600);
    movement_gate_ticks("start_pf_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_pf_fox_marth4", 600, "ledger600");
}
/// Mushroom Kingdom II's start boundary: the four timers' draws at
/// creation, the first lift made and rising.
#[test]
fn start_mk2_fox_marth4_600() {
    movement_gate_ticks("start_mk2_fox_marth4", 600);
    movement_gate_ticks("start_mk2_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_mk2_fox_marth4", 600, "ledger600");
}
/// Great Bay's start boundary: Tingle's creation draws, the turtle's and
/// the floating platform's timers, the giants and the moon's countdown.
/// Icicle Mountain's start boundary: the controller's two segment draws
/// (a count found from the seed, stageGObj9_OnInit), the fixed camera, and
/// the scroll's first speed once the countdown releases it.
#[test]
fn start_im_fox_marth4_600() {
    movement_gate_ticks("start_im_fox_marth4", 600);
    movement_gate_ticks("start_im_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_im_fox_marth4", 600, "ledger600");
}
#[test]
fn start_gb_fox_marth4_600() {
    movement_gate_ticks("start_gb_fox_marth4", 600);
    movement_gate_ticks("start_gb_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_gb_fox_marth4", 600, "ledger600");
}
/// Princess Peach's Castle's start boundary: no setup draws; the switch
/// and Bullet Bill countdowns drawn when the countdown ends, the wind's
/// draws every tick.
#[test]
fn start_pc_fox_marth4_600() {
    movement_gate_ticks("start_pc_fox_marth4", 600);
    movement_gate_ticks("start_pc_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_pc_fox_marth4", 600, "ledger600");
}
/// Green Greens' start boundary: the block layout's eighteen draws, the
/// first block wait, Whispy's wait and side at creation; the blocks and
/// Whispy start when the countdown ends.
#[test]
fn start_gg_fox_marth4_600() {
    movement_gate_ticks("start_gg_fox_marth4", 600);
    movement_gate_ticks("start_gg_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_gg_fox_marth4", 600, "ledger600");
}
/// Rainbow Cruise's start boundary: the ship's creation (no draws), the
/// camera path's first projection and the scroll; Fox rides the ship's
/// mast in his entry.
#[test]
fn start_rc_fox_marth4_600() {
    movement_gate_ticks("start_rc_fox_marth4", 600);
    movement_gate_ticks("start_rc_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_rc_fox_marth4", 600, "ledger600");
}
/// Rainbow Cruise with the Ice Climbers: Nana lands on the ship first and
/// Popo's entry accessory raises him to her (fn_800C6F34); standing
/// together after the entry she steps behind him (ftCommon_8007DFD0).
#[test]
fn start_rc_iceclimbers_fox4_600() {
    movement_gate_ticks("start_rc_iceclimbers_fox4", 600);
    movement_gate_ticks("start_rc_iceclimbers_fox4_cold", 600);
    particle_rng_sites_with_ledger("start_rc_iceclimbers_fox4", 600, "ledger600");
}
/// Big Blue's start boundary: the road scrolling and drawing its first
/// segments (only 0, 1 and 4 before GO), the cars' creation draws (a
/// variable count the cold start inverts) and the Falcon Flyer waiting.
#[test]
fn start_bb_fox_marth4_600() {
    movement_gate_ticks("start_bb_fox_marth4", 600);
    movement_gate_ticks("start_bb_fox_marth4_cold", 600);
    particle_rng_sites_with_ledger("start_bb_fox_marth4", 600, "ledger600");
    movement_gate_ticks("start_bb_fox_marth_time8", 600);
    movement_gate_ticks("start_bb_fox_marth_time8_cold", 600);
}
#[test]
fn start_jj_and_kj_fox_marth4_600() {
    movement_gate_ticks("start_jj_fox_marth4", 600);
    movement_gate_ticks("start_jj_fox_marth4_cold", 600);
    movement_gate_ticks("start_kj_fox_marth4", 600);
    movement_gate_ticks("start_kj_fox_marth4_cold", 600);
}

/// Flat Zone (grflatzone.c, docs/PORT_NOTES/FLAT_ZONE.md): the start
/// boundary, warm and cold (the controller's and the platforms' setup
/// draws, the first variant changes), and the per-tick-rendered capture.
#[test]
fn flat_zone_starts_600() {
    movement_gate_ticks("w25_start_fz_fox_marth4_v01", 600);
    movement_gate_ticks("w25_start_fz_fox_marth4_v01_cold", 600);
    movement_gate_ticks("w25_fz_start600_v01", 600);
}
#[test]
fn flat_zone_start_particle_rng_order() {
    particle_rng_sites_with_ledger("w25_start_fz_fox_marth4_v01", 600, "ledger600");
    particle_rng_sites_with_ledger("w25_fz_start600_v01", 600, "ledger");
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

#[test]
fn idle_fd_mario_600() {
    movement_gate_ticks("idle_fd_mario", 600);
}

#[test]
fn idle_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("idle_fd_mario", 600, "ledger600");
}

#[test]
fn start_fd_mario_600() {
    movement_gate_ticks("start_fd_mario", 600);
}

#[test]
fn start_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_mario", 600, "ledger600");
}

#[test]
fn squat_fd_mario_300() {
    movement_gate_ticks("squat_fd_mario", 300);
}

#[test]
fn squat_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("squat_fd_mario", 300, "ledger");
}

#[test]
fn turn_fd_mario_300() {
    movement_gate_ticks("turn_fd_mario", 300);
}

#[test]
fn turn_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("turn_fd_mario", 300, "ledger");
}

#[test]
fn walk_fd_mario_300() {
    movement_gate_ticks("walk_fd_mario", 300);
}

#[test]
fn walk_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("walk_fd_mario", 300, "ledger");
}

#[test]
fn walkfast_fd_mario_300() {
    movement_gate_ticks("walkfast_fd_mario", 300);
}

#[test]
fn walkfast_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("walkfast_fd_mario", 300, "ledger");
}

#[test]
fn dash_fd_mario_300() {
    movement_gate_ticks("dash_fd_mario", 300);
}

#[test]
fn dash_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("dash_fd_mario", 300, "ledger");
}

#[test]
fn turnrun_fd_mario_300() {
    movement_gate_ticks("turnrun_fd_mario", 300);
}

#[test]
fn turnrun_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("turnrun_fd_mario", 300, "ledger");
}

#[test]
fn jump_fd_mario_300() {
    movement_gate_ticks("jump_fd_mario", 300);
}

#[test]
fn jump_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("jump_fd_mario", 300, "ledger");
}

#[test]
fn airjumpb_fd_mario_300() {
    movement_gate_ticks("airjumpb_fd_mario", 300);
}

#[test]
fn airjumpb_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("airjumpb_fd_mario", 300, "ledger");
}

#[test]
fn shield_fd_mario_300() {
    movement_gate_ticks("shield_fd_mario", 300);
}

#[test]
fn shield_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("shield_fd_mario", 300, "ledger");
}

#[test]
fn spotdodge_fd_mario_300() {
    movement_gate_ticks("spotdodge_fd_mario", 300);
}

#[test]
fn spotdodge_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("spotdodge_fd_mario", 300, "ledger");
}

#[test]
fn roll_fd_mario_300() {
    movement_gate_ticks("roll_fd_mario", 300);
}

#[test]
fn roll_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("roll_fd_mario", 300, "ledger");
}

#[test]
fn airdodge_fd_mario_300() {
    movement_gate_ticks("airdodge_fd_mario", 300);
}

#[test]
fn airdodge_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("airdodge_fd_mario", 300, "ledger");
}

#[test]
fn wavedash_fd_mario_300() {
    movement_gate_ticks("wavedash_fd_mario", 300);
}

#[test]
fn wavedash_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("wavedash_fd_mario", 300, "ledger");
}

#[test]
fn ledge_fd_mario_420() {
    movement_gate_ticks("ledge_fd_mario", 420);
}

#[test]
fn ledge_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("ledge_fd_mario", 420, "ledger");
}

#[test]
fn ledgeclimb_fd_mario_420() {
    movement_gate_ticks("ledgeclimb_fd_mario", 420);
}

#[test]
fn ledgeclimb_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeclimb_fd_mario", 420, "ledger");
}

#[test]
fn ledgeescape_fd_mario_420() {
    movement_gate_ticks("ledgeescape_fd_mario", 420);
}

#[test]
fn ledgeescape_mario_particle_draw_order() {
    particle_rng_sites_with_ledger("ledgeescape_fd_mario", 420, "ledger");
}

/// Ice Climbers vs Fox from the FD start boundary: Nana is a second fighter
/// of port 1, entering beside Popo, whose inputs her CPU (mode 6) follows.
#[test]
fn start_fd_iceclimbers_fox4_600() {
    movement_gate_ticks("start_fd_iceclimbers_fox4", 600);
}

#[test]
fn start_fd_iceclimbers_fox4_cold_600() {
    movement_gate_ticks("start_fd_iceclimbers_fox4_cold", 600);
}

/// Ness vs Fox from the FD start boundary: PlNs.dat's entry and wait, from
/// the savestate and from a cold construction.
#[test]
fn start_fd_ness_fox4_600() {
    movement_gate_ticks("start_fd_ness_fox4", 600);
}

#[test]
fn start_fd_ness_fox4_cold_600() {
    movement_gate_ticks("start_fd_ness_fox4_cold", 600);
}

/// Kirby vs Fox from the FD start boundary: PlKb.dat's entry and wait, the
/// thirteen stand-in joints of his empty hat parts, the Stone form drawn at
/// creation (ftKb_Init_OnDeath) and the bump star of his first landing.
#[test]
fn start_fd_kirby_fox4_600() {
    movement_gate_ticks("start_fd_kirby_fox4", 600);
}

#[test]
fn start_fd_kirby_fox4_cold_600() {
    movement_gate_ticks("start_fd_kirby_fox4_cold", 600);
}

#[test]
fn start_kirby_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_kirby_fox4", 600, "ledger600");
}

/// Kirby vs Fox on Yoshi's Story, for Stone on its slopes.
#[test]
fn start_ys_kirby_fox4_600() {
    movement_gate_ticks("start_ys_kirby_fox4", 600);
}

#[test]
fn start_ys_kirby_fox4_cold_600() {
    movement_gate_ticks("start_ys_kirby_fox4_cold", 600);
}

#[test]
fn start_ys_kirby_particle_draw_order() {
    particle_rng_sites_with_ledger("start_ys_kirby_fox4", 600, "ledger600");
}

/// Kirby vs Captain Falcon on Final Destination, for a helmet's midair jumps.
#[test]
fn start_fd_kirby_captainfalcon4_600() {
    movement_gate_ticks("start_fd_kirby_captainfalcon4", 600);
}

#[test]
fn start_fd_kirby_captainfalcon4_cold_600() {
    movement_gate_ticks("start_fd_kirby_captainfalcon4_cold", 600);
}

#[test]
fn start_kirby_falcon_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_kirby_captainfalcon4", 600, "ledger600");
}

/// Kirby vs Ganondorf on Final Destination, for the copied Warlock Punch.
#[test]
fn start_fd_kirby_ganondorf4_600() {
    movement_gate_ticks("start_fd_kirby_ganondorf4", 600);
}

#[test]
fn start_fd_kirby_ganondorf4_cold_600() {
    movement_gate_ticks("start_fd_kirby_ganondorf4_cold", 600);
}

#[test]
fn start_kirby_ganondorf_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_kirby_ganondorf4", 600, "ledger600");
}

/// Kirby vs Mario, Dr. Mario and Luigi on Final Destination, for the
/// copied Fireball, Megavitamin and green Fireball: warm, cold and the
/// particle draw order.
#[test]
fn start_fd_kirby_fireball_boundaries_600() {
    for name in [
        "start_fd_kirby_mario4",
        "start_fd_kirby_drmario4",
        "start_fd_kirby_luigi4",
    ] {
        movement_gate_ticks(name, 600);
        movement_gate_ticks(&format!("{name}_cold"), 600);
        particle_rng_sites_with_ledger(name, 600, "ledger600");
    }
}

/// Kirby vs Samus on Final Destination, for the copied Charge Shot.
#[test]
fn start_fd_kirby_samus4_600() {
    movement_gate_ticks("start_fd_kirby_samus4", 600);
}

#[test]
fn start_fd_kirby_samus4_cold_600() {
    movement_gate_ticks("start_fd_kirby_samus4_cold", 600);
}

#[test]
fn start_kirby_samus_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_kirby_samus4", 600, "ledger600");
}

/// Kirby vs Ness on Final Destination, for the copied PK Flash: warm,
/// cold and the particle draw order.
#[test]
fn start_fd_kirby_ness_boundary_600() {
    let name = "start_fd_kirby_ness4";
    movement_gate_ticks(name, 600);
    movement_gate_ticks(&format!("{name}_cold"), 600);
    particle_rng_sites_with_ledger(name, 600, "ledger600");
}

/// Kirby vs Yoshi on Final Destination, for the copied Egg Lay: warm, cold
/// and the particle draw order.
#[test]
fn start_fd_kirby_yoshi_boundary_600() {
    let name = "start_fd_kirby_yoshi4";
    movement_gate_ticks(name, 600);
    movement_gate_ticks(&format!("{name}_cold"), 600);
    particle_rng_sites_with_ledger(name, 600, "ledger600");
}

/// Kirby vs Link and Young Link on Final Destination, for the copied bow:
/// warm, cold and the particle draw order.
#[test]
fn start_fd_kirby_bow_boundaries_600() {
    for name in ["start_fd_kirby_link4", "start_fd_kirby_younglink4"] {
        movement_gate_ticks(name, 600);
        movement_gate_ticks(&format!("{name}_cold"), 600);
        particle_rng_sites_with_ledger(name, 600, "ledger600");
    }
}

/// Kirby vs Zelda, Bowser, the Ice Climbers and Sheik on Final
/// Destination, for their copied neutral specials: warm, cold and the
/// particle draw order.
#[test]
fn start_fd_kirby_copy_wave13_boundaries_600() {
    const BOUNDARIES: &[&str] = &[
        "start_fd_kirby_zelda4",
        "start_fd_kirby_bowser4",
        "start_fd_kirby_iceclimbers4",
        "start_fd_kirby_sheik4",
    ];
    for name in BOUNDARIES {
        movement_gate_ticks(name, 600);
        movement_gate_ticks(&format!("{name}_cold"), 600);
        particle_rng_sites_with_ledger(name, 600, "ledger600");
    }
}

/// Kirby vs the five fighters whose hats are grafted into his own parts,
/// on Final Destination, warm and cold, with the particle draw order.
#[test]
fn start_fd_kirby_grafted_hat_boundaries_600() {
    for kind in ["falco", "jigglypuff", "donkeykong", "mewtwo", "gameandwatch"] {
        movement_gate_ticks(&format!("start_fd_kirby_{kind}4"), 600);
        movement_gate_ticks(&format!("start_fd_kirby_{kind}4_cold"), 600);
        particle_rng_sites_with_ledger(&format!("start_fd_kirby_{kind}4"), 600, "ledger600");
    }
}

#[test]
fn start_iceclimbers_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_iceclimbers_fox4", 600, "ledger600");
}

/// Donkey Kong vs Fox on Final Destination: the twenty-first character's
/// cold construction.
#[test]
fn start_fd_donkeykong_fox4_cold_600() {
    movement_gate_ticks("start_fd_donkeykong_fox4_cold", 600);
}

#[test]
fn start_donkeykong_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_donkeykong_fox4", 600, "ledger600");
}

/// Bowser vs Fox on Final Destination: his cold construction (the
/// landing model, efAsync 0x3FB).
#[test]
fn start_fd_bowser_fox4_cold_600() {
    movement_gate_ticks("start_fd_bowser_fox4_cold", 600);
}

#[test]
fn start_bowser_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_bowser_fox4", 600, "ledger600");
}

/// Mr. Game & Watch vs Fox on Final Destination: the flat fighter's cold
/// construction (x34_scale.z on the model root, no effect file).
#[test]
fn start_fd_gameandwatch_fox4_cold_600() {
    movement_gate_ticks("start_fd_gameandwatch_fox4_cold", 600);
}

#[test]
fn start_gameandwatch_particle_draw_order() {
    particle_rng_sites_with_ledger("start_fd_gameandwatch_fox4", 600, "ledger600");
}

/// Mr. Game & Watch vs Samus on Final Destination, the boundary of the
/// Oil Panic witness that absorbs Charge Shots.
#[test]
fn start_fd_gameandwatch_samus4_cold_600() {
    movement_gate_ticks("start_fd_gameandwatch_samus4_cold", 600);
}

/// Peach vs Fox on Pokemon Stadium: the boundary of the held-item StopWall
/// witness (the rock form's wall).
#[test]
fn start_ps_peach_fox4_cold_600() {
    movement_gate_ticks("start_ps_peach_fox4_cold", 600);
}

#[test]
fn start_ps_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("start_ps_peach_fox4", 600, "ledger600");
}

/// Samus vs Fox and Link vs Fox on Pokemon Stadium: the boundaries of the
/// sliding-bomb witnesses (the rock form's peak; the dipped form's banks).
#[test]
fn start_ps_samus_fox4_cold_600() {
    movement_gate_ticks("start_ps_samus_fox4_cold", 600);
}

#[test]
fn start_ps_link_fox4_cold_600() {
    movement_gate_ticks("start_ps_link_fox4_cold", 600);
}

/// Ice Climbers vs Peach on Pokemon Stadium: the benchmark matchup
/// (`sim_bench throughput`) and the boundary of the witness where Nana runs
/// the grounded Blizzard row airborne.
#[test]
fn start_ps_iceclimbers_peach4_cold_600() {
    movement_gate_ticks("start_ps_iceclimbers_peach4_cold", 600);
}

#[test]
fn start_ps_iceclimbers_peach_particle_draw_order() {
    particle_rng_sites_with_ledger("start_ps_iceclimbers_peach4", 600, "ledger600");
}

/// Ice Climbers vs Fox on Yoshi's Story: every think of Nana's asks whether
/// her island's joint moved (ftCo_800A2718 -> mpIsland_8005AC8C, 0x8005AC8C).
#[test]
fn start_ys_iceclimbers_fox4_cold_600() {
    movement_gate_ticks("start_ys_iceclimbers_fox4_cold", 600);
}

#[test]
fn start_ys_iceclimbers_particle_draw_order() {
    particle_rng_sites_with_ledger("start_ys_iceclimbers_fox4", 600, "ledger600");
}
