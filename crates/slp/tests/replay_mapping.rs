use melee_diff::Value;
use slp::{Replay, SLIPPI_FIRST_FRAME};

fn fixture(name: &str) -> Replay {
    Replay::parse(
        &std::fs::read(format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap(),
    )
    .unwrap()
}

#[test]
fn ports_with_gaps_use_fighter_list_indices() {
    let replay = fixture("v3.13.slp");
    assert_eq!(replay.leader_ports().collect::<Vec<_>>(), [0, 2]);
    let records = slp::to_trace(&replay);
    assert_eq!(records[0].state["p1.player_id"], Value::UInt(2));
    assert_eq!(records[0].state["p1.kind"], Value::Int(23));
    assert!(!records[0].state.keys().any(|key| key.starts_with("p2.")));
    let post = replay.frames[&SLIPPI_FIRST_FRAME].ports[2]
        .leader
        .post
        .as_ref()
        .unwrap();
    assert_eq!(
        records[0].state["p1.cur_pos.x"],
        Value::f32(post.position_x)
    );
}

#[test]
fn only_adjacent_scheduler_start_seeds_are_end_of_frame_evidence() {
    let mut replay = fixture("v3.13.slp");
    let next = SLIPPI_FIRST_FRAME + 1;
    let seed = replay.frames[&next].start.unwrap().random_seed;
    assert_eq!(
        slp::to_trace(&replay)[0].state["rng.seed"],
        Value::UInt(u64::from(seed))
    );
    replay.frames.get_mut(&next).unwrap().start = None;
    assert!(!slp::to_trace(&replay)[0].state.contains_key("rng.seed"));
    replay.frames.remove(&next);
    assert!(!slp::to_trace(&replay)[0].state.contains_key("rng.seed"));
    assert!(slp::to_trace(&fixture("v0.1.slp"))
        .iter()
        .all(|r| !r.state.contains_key("rng.seed")));
}

#[test]
fn absent_velocity_fields_are_omitted_and_float_bits_survive() {
    let mut replay = fixture("v3.13.slp");
    let post = replay.frames.get_mut(&SLIPPI_FIRST_FRAME).unwrap().ports[0]
        .leader
        .post
        .as_mut()
        .unwrap();
    post.position_x = -0.0;
    post.self_air_speed_x = None;
    post.self_speed_y = None;
    post.attack_speed_x = None;
    post.attack_speed_y = None;
    let records = slp::to_trace(&replay);
    assert_eq!(records[0].state["p0.cur_pos.x"], Value::f32(-0.0));
    assert!(!records[0].state.contains_key("p0.self_vel.x"));
    assert!(!records[0].state.contains_key("p0.kb_vel.y"));
}

#[test]
fn cold_export_preserves_rules_ports_types_and_seed_phase() {
    let replay = fixture("joystick_udlr.slp");
    let scenario = slp::cold::ColdScenario::from_replay(&replay, "test", 1234, false).unwrap();
    assert_eq!(scenario.seed, 1234);
    assert_eq!(
        scenario.replay_rules.game_start_seed,
        replay.start.random_seed
    );
    assert_eq!(scenario.replay_rules.time_seconds, 480);
    assert_eq!(scenario.fighters[0].stocks, 4);
    assert_eq!(scenario.fighters[1].controller, "cpu");
    assert_eq!(scenario.replay_inputs.len(), replay.frames.len() * 2);
    let text = scenario.to_toml().unwrap();
    let value: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(value["seed"].as_integer(), Some(1234));
    assert!(value.get("savestate").is_none());
}
