use melee_diff::{Record, Value};
use melee_sim::replay::{self, Setup, Stop};
use slp::{Replay, SLIPPI_FIRST_FRAME};
use std::{collections::BTreeMap, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture(name: &str) -> Replay {
    Replay::parse(&std::fs::read(root().join("crates/slp/tests/data").join(name)).unwrap()).unwrap()
}

#[test]
fn fixture_matched_frame_counts_never_decrease() {
    // Zero remains the corpus floor: v3.16 reaches the Online setup boundary.
    // Pin the composition and the reason as well, so an unconditional skip or
    // a setup/parser regression cannot masquerade as a successful zero floor.
    let cases = [
        ("ics.slp", "FinalDestination", "Cpu control", 0),
        ("joystick_udlr.slp", "FinalDestination", "Ganondorf", 0),
        ("netplay.slp", "FountainOfDreams", "cold stage", 0),
        ("v0.1.slp", "DreamLand", "cold stage", 0),
        ("v3.12.slp", "PokemonStadium", "cold stage", 0),
        ("v3.13.slp", "FinalDestination", "Pichu", 0),
        ("v3.16.slp", "YoshisStory", "Online", 0),
        ("v3.18.slp", "FountainOfDreams", "cold stage", 0),
    ];
    for (name, stage, reason, floor) in cases {
        let report = replay::run(&fixture(name), &root(), Setup::default()).unwrap();
        assert_eq!(report.stage, stage);
        assert!(report.matched >= floor, "{name}: {report}");
        match &report.stop {
            Stop::Unsupported(reasons) => {
                assert_eq!(report.matched, 0);
                assert!(
                    reasons.iter().any(|s| s.contains(reason)),
                    "{name}: {report}"
                );
            }
            Stop::Complete | Stop::Unported { .. } => {
                assert!(report.matched > 0, "{name}: {report}")
            }
            other => panic!("{name}: unexpected {other:?}"),
        }
    }
}

fn float(record: &Record, key: &str) -> f32 {
    let Value::F32 { bits, .. } = record.state[key] else {
        panic!("{key}")
    };
    f32::from_bits(bits)
}
fn int(record: &Record, key: &str) -> i64 {
    match record.state[key] {
        Value::Int(i) => i,
        Value::UInt(i) => i as i64,
        _ => panic!("{key}"),
    }
}

/// Project an existing independent Dolphin oracle into fields Slippi records.
/// This is adapter proof, NOT an additional real .slp fixture or corpus count.
fn oracle_replay(scene: &str) -> Option<(Replay, Vec<Record>)> {
    let path = root().join(format!("harness/traces/{scene}.tick.expected.jsonl"));
    if !melee_test_support::require_files([&path, &root().join("harness/roms/files/PlCo.dat")]) {
        return None;
    }
    let expected = melee_diff::read_trace(melee_test_support::trace::open(&path).unwrap()).unwrap();
    let mut replay = fixture("v3.13.slp");
    // The recorded cold oracle uses items off; v3.13's template has items on.
    replay.start.item_spawn_behavior = -1;
    let template = replay.frames[&SLIPPI_FIRST_FRAME].clone();
    replay.start.players[1] = replay.start.players[0].clone();
    replay.start.players[1].port = 1;
    replay.start.players[1].costume = 1;
    replay.start.players[2].player_type = slp::PlayerType::Empty;
    replay.start.players[0].costume = 0;
    for p in &mut replay.start.players[..2] {
        p.dashback_fix = Some(0);
        p.shield_drop_fix = Some(0);
    }
    replay.frames = BTreeMap::new();
    for tick in 1..expected.len() {
        let number = SLIPPI_FIRST_FRAME + tick as i32 - 1;
        let mut frame = template.clone();
        frame.number = number;
        frame.ports = Default::default();
        frame.items.clear();
        frame.bookend = None;
        frame.start = Some(slp::FrameStart {
            frame: number,
            random_seed: int(&expected[tick - 1], "rng.seed") as u32,
            scene_frame_counter: Some(tick as u32 - 1),
        });
        for port in 0..2 {
            let mut pre = template.ports[0].leader.pre.clone().unwrap();
            pre.frame = number;
            pre.player_index = port as u8;
            pre.random_seed = frame.start.unwrap().random_seed;
            pre.buttons_physical = 0;
            pre.buttons_processed = 0;
            pre.joystick_x = 0.0;
            pre.joystick_y = 0.0;
            pre.cstick_x = 0.0;
            pre.cstick_y = 0.0;
            pre.trigger = 0.0;
            pre.physical_l_trigger = 0.0;
            pre.physical_r_trigger = 0.0;
            pre.raw_joystick_x = None;
            pre.raw_joystick_y = None;
            pre.raw_cstick_x = None;
            pre.raw_cstick_y = None;
            let mut post = template.ports[0].leader.post.clone().unwrap();
            let key = |field| format!("p{port}.{field}");
            let r = &expected[tick];
            post.frame = number;
            post.player_index = port as u8;
            post.internal_character = int(r, &key("kind")) as u8;
            post.action_state = int(r, &key("motion_id")) as u16;
            post.position_x = float(r, &key("cur_pos.x"));
            post.position_y = float(r, &key("cur_pos.y"));
            post.facing_direction = float(r, &key("facing_dir"));
            post.percent = float(r, &key("percent"));
            post.action_state_frame = Some(float(r, &key("cur_anim_frame")));
            post.airborne = Some(int(r, &key("ground_or_air")) != 0);
            post.self_air_speed_x = Some(float(r, &key("self_vel.x")));
            post.self_speed_y = Some(float(r, &key("self_vel.y")));
            post.attack_speed_x = Some(float(r, &key("kb_vel.x")));
            post.attack_speed_y = Some(float(r, &key("kb_vel.y")));
            frame.ports[port].leader = slp::PlayerFrame {
                pre: Some(pre),
                post: Some(post),
            };
        }
        replay.frames.insert(number, frame);
    }
    Some((replay, expected))
}

#[test]
fn cold_replay_bridge_matches_599_independent_oracle_frames() {
    let Some((replay, _)) = oracle_replay("start_fd_fox") else {
        return;
    };
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(false),
            boundary_seed: None,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 599, "{report}");
    assert!(matches!(report.stop, Stop::Complete), "{report}");
}

#[test]
fn a_ported_state_one_bit_mismatch_is_a_failure() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox") else {
        return;
    };
    let post = replay.frames.get_mut(&SLIPPI_FIRST_FRAME).unwrap().ports[0]
        .leader
        .post
        .as_mut()
        .unwrap();
    post.position_x = f32::from_bits(post.position_x.to_bits() ^ 1);
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(false),
            boundary_seed: None,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 0);
    let Stop::Diverged(diff) = report.stop else {
        panic!("{report}")
    };
    assert_eq!(diff.frame, 0);
    assert_eq!(diff.path, "p0.cur_pos.x");
    assert_ne!(diff.expected, diff.actual);
}

#[test]
fn battlefield_music_seed_and_first_full_tick_match_the_oracle() {
    let Some((mut replay, _)) = oracle_replay("start_bf_fox") else {
        return;
    };
    replay.start.stage = 31;
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(true),
            boundary_seed: None,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 599, "{report}");
    assert!(matches!(report.stop, Stop::Complete), "{report}");
}

#[test]
fn unported_action_names_the_boundary_without_counting_that_frame() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox") else {
        return;
    };
    let number = SLIPPI_FIRST_FRAME + 150;
    // Synthetic protocol probe; no captured expected values are changed.
    replay.frames.get_mut(&number).unwrap().ports[0]
        .leader
        .post
        .as_mut()
        .unwrap()
        .action_state = 65;
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(false),
            boundary_seed: None,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 150, "{report}");
    let Stop::Unported { tick, action, .. } = report.stop else {
        panic!("{report}")
    };
    assert_eq!(tick, 150);
    assert!(action.contains("AttackAirN (65)"));
}

#[test]
fn missing_late_input_preserves_the_matched_prefix() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox") else {
        return;
    };
    let number = SLIPPI_FIRST_FRAME + 150;
    replay.frames.get_mut(&number).unwrap().ports[0]
        .leader
        .pre
        .as_mut()
        .unwrap()
        .physical_l_trigger = f32::NAN;
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(false),
            boundary_seed: None,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 150, "{report}");
    assert!(
        matches!(report.stop, Stop::UnavailableInput { tick: 150, .. }),
        "{report}"
    );
}

#[test]
fn gapped_ports_preserve_spawn_markers_and_route_pads_by_port() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox") else {
        return;
    };
    replay.start.players[2] = replay.start.players[1].clone();
    replay.start.players[2].port = 2;
    replay.start.players[2].spawn_point = 1;
    replay.start.players[1].player_type = slp::PlayerType::Empty;
    for frame in replay.frames.values_mut() {
        frame.ports[2] = std::mem::take(&mut frame.ports[1]);
        frame.ports[2].leader.pre.as_mut().unwrap().player_index = 2;
        frame.ports[2].leader.post.as_mut().unwrap().player_index = 2;
    }
    let setup = Setup {
        all_characters_unlocked: Some(false),
        boundary_seed: None,
    };
    let report = replay::run(&replay, &root(), setup).unwrap();
    assert_eq!(report.matched, 599, "{report}");
    let pre = replay
        .frames
        .get_mut(&(SLIPPI_FIRST_FRAME + 150))
        .unwrap()
        .ports[2]
        .leader
        .pre
        .as_mut()
        .unwrap();
    pre.buttons_physical = 0x400;
    pre.buttons_processed = 0x400;
    let report = replay::run(&replay, &root(), setup).unwrap();
    assert_eq!(report.matched, 150, "{report}");
    let Stop::Diverged(diff) = report.stop else {
        panic!("{report}")
    };
    assert!(diff.path.starts_with("p1."), "{diff}");
}

/// InitOnlinePlay.asm's FN_SyncRNG resets before player animation on every
/// scheduler pass. A match-rules change cannot reproduce this fixture.
#[test]
fn online_story_fixture_requires_per_frame_netplay_rng_reconstruction() {
    let replay = fixture("v3.16.slp");
    assert_eq!(replay.frames.len(), 308);
    for frame in replay.frames.values() {
        let start = frame.start.expect("v3.16 Frame Start");
        let counter = start.scene_frame_counter.expect("v3.16 scene counter");
        assert_eq!(
            start.random_seed,
            counter
                .rotate_left(16)
                .wrapping_add(replay.start.random_seed)
        );
    }
    let reasons = replay::unsupported_setup(&replay);
    assert_eq!(reasons, ["Slippi Online initialization/seed resets"]);
}

#[test]
fn story_primary_music_seed_and_first_full_tick_match_the_oracle() {
    let Some((mut replay, _)) = oracle_replay("start_ys_fox") else {
        return;
    };
    replay.start.stage = 8;
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(true),
            boundary_seed: None,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 599, "{report}");
    assert!(matches!(report.stop, Stop::Complete), "{report}");
}
