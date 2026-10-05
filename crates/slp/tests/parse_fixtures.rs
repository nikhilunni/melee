//! Parse the real replays under tests/data and check plausible invariants.

use slp::{harness_frame, PlayerType, Replay, Version, SLIPPI_FIRST_FRAME};
use std::path::PathBuf;

fn data(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

const FIXTURES: &[&str] = &[
    "v0.1.slp",
    "v3.12.slp",
    "v3.13.slp",
    "v3.16.slp",
    "v3.18.slp",
    "netplay.slp",
    "joystick_udlr.slp",
    "ics.slp",
];

fn check_invariants(name: &str, r: &Replay) {
    let gs = &r.start;
    assert!(
        slp::ids::stage_name(gs.stage).is_some(),
        "{name}: stage id {} not a known stage",
        gs.stage
    );
    let present: Vec<_> = gs.players.iter().filter(|p| p.is_present()).collect();
    assert!(!present.is_empty(), "{name}: no players");
    for p in &present {
        assert!(
            slp::ids::external_character_name(p.character).is_some(),
            "{name}: port {} has unknown character {}",
            p.port,
            p.character
        );
        assert!(
            p.stock_start_count >= 1 && p.stock_start_count <= 99,
            "{name}: stocks {}",
            p.stock_start_count
        );
        if p.player_type == PlayerType::Cpu {
            assert!(
                (1..=9).contains(&p.cpu_level),
                "{name}: cpu level {}",
                p.cpu_level
            );
        }
    }

    assert!(!r.frames.is_empty(), "{name}: no frames");
    let first = r.first_frame().unwrap();
    let last = r.last_frame().unwrap();
    assert_eq!(first, SLIPPI_FIRST_FRAME, "{name}: first frame");
    assert_eq!(harness_frame(first), Some(0));
    assert_eq!(
        r.frames.len() as i64,
        i64::from(last) - i64::from(first) + 1,
        "{name}: frame gaps"
    );
    if let Some(mf) = r.metadata_last_frame() {
        assert_eq!(
            mf, last,
            "{name}: metadata.lastFrame disagrees with parsed frames"
        );
    }
    assert!(!r.incomplete, "{name}: unexpectedly incomplete");

    let has_frame_start = gs.version.at_least(2, 2, 0);
    let mut prev_seed: Option<u32> = None;
    let mut seed_changes = 0usize;
    let mut posts = 0usize;
    for (n, f) in &r.frames {
        assert_eq!(*n, f.number);
        assert_eq!(
            f.start.is_some(),
            has_frame_start,
            "{name}: frame {n} Frame Start presence"
        );
        if gs.version.at_least(3, 0, 0) {
            let b = f
                .bookend
                .unwrap_or_else(|| panic!("{name}: frame {n} lacks bookend"));
            assert_eq!(b.frame, *n);
        }
        let seed = f
            .start_seed()
            .unwrap_or_else(|| panic!("{name}: frame {n} has no seed"));
        if prev_seed.is_some_and(|s| s != seed) {
            seed_changes += 1;
        }
        prev_seed = Some(seed);

        for (port, pf) in f.ports.iter().enumerate() {
            let expect_present = gs.players[port].is_present();
            assert_eq!(
                pf.leader.post.is_some(),
                expect_present,
                "{name}: frame {n} port {port} post-frame presence"
            );
            assert_eq!(
                pf.leader.pre.is_some(),
                expect_present,
                "{name}: frame {n} port {port} pre-frame presence"
            );
            for pl in [&pf.leader, &pf.follower] {
                if let Some(pre) = &pl.pre {
                    assert_eq!(pre.frame, *n);
                    assert_eq!(pre.player_index as usize, port);
                    assert!(
                        pre.action_state < 0x400,
                        "{name}: pre action state {:#X}",
                        pre.action_state
                    );
                    for v in [pre.joystick_x, pre.joystick_y, pre.cstick_x, pre.cstick_y] {
                        assert!((-1.0..=1.0).contains(&v), "{name}: stick value {v}");
                    }
                    assert!(
                        (0.0..=1.0).contains(&pre.trigger),
                        "{name}: trigger value {}",
                        pre.trigger
                    );
                    // The physical L/R trigger fields are not range-checked:
                    // several fixtures (0.1.0 and 3.x alike) carry the constant
                    // 0x65000C80 there, apparently uninitialised memory.
                    assert!(
                        pre.physical_l_trigger.is_finite() && pre.physical_r_trigger.is_finite()
                    );
                }
                if let Some(post) = &pl.post {
                    posts += 1;
                    assert_eq!(post.frame, *n);
                    assert_eq!(post.player_index as usize, port);
                    assert!(
                        post.action_state < 0x400,
                        "{name}: post action state {:#X}",
                        post.action_state
                    );
                    assert!(
                        post.position_x.is_finite() && post.position_y.is_finite(),
                        "{name}: position"
                    );
                    assert!(
                        post.position_x.abs() < 1000.0 && post.position_y.abs() < 1000.0,
                        "{name}: position magnitude"
                    );
                    assert!(
                        post.facing_direction == 1.0
                            || post.facing_direction == -1.0
                            || post.facing_direction == 0.0
                    );
                    assert!(
                        post.percent >= 0.0 && post.percent.is_finite(),
                        "{name}: percent {}",
                        post.percent
                    );
                    assert!(
                        (0.0..=60.0).contains(&post.shield_size),
                        "{name}: shield {}",
                        post.shield_size
                    );
                    assert!(post.stocks <= 99);
                    assert!(
                        slp::ids::internal_character_name(post.internal_character).is_some(),
                        "{name}: internal character {}",
                        post.internal_character
                    );
                    assert_eq!(
                        post.action_state_frame.is_some(),
                        gs.version.at_least(0, 2, 0)
                    );
                    assert_eq!(post.jumps_remaining.is_some(), gs.version.at_least(2, 0, 0));
                    assert_eq!(
                        post.self_air_speed_x.is_some(),
                        gs.version.at_least(3, 5, 0)
                    );
                    assert_eq!(post.instance_id.is_some(), gs.version.at_least(3, 16, 0));
                    if let Some(j) = post.jumps_remaining {
                        assert!(j <= 6, "{name}: jumps remaining {j}");
                    }
                }
            }
        }
    }
    assert!(posts > 0);
    assert!(
        seed_changes * 10 > r.frames.len(),
        "{name}: seed changed on only {seed_changes} of {} frames",
        r.frames.len()
    );
}

#[test]
fn all_fixtures_parse_with_plausible_state() {
    for name in FIXTURES {
        let r = Replay::parse(&data(name)).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        check_invariants(name, &r);
    }
}

#[test]
fn versions_and_version_gated_fields() {
    let r = Replay::parse(&data("v0.1.slp")).unwrap();
    assert_eq!(r.version(), Version::new(0, 1, 0));
    assert!(r.metadata.is_some());
    assert!(r.start.pal.is_none());
    let post = r.frames[&0].ports[0].leader.post.as_ref().unwrap();
    assert!(post.action_state_frame.is_none());
    assert!(post.state_flags.is_none());
    assert!(r.end.is_some());
    assert!(r.end.as_ref().unwrap().lras_initiator.is_none());

    let r = Replay::parse(&data("v3.18.slp")).unwrap();
    assert_eq!(r.version(), Version::new(3, 18, 0));
    assert_eq!(r.start.major_scene, Some(2));
    assert!(r.start.language.is_some());
    let post = r.frames[&0].ports[0].leader.post.as_ref().unwrap();
    assert!(post.animation_index.is_some());
    assert!(post.instance_id.is_some());
    let pre = r.frames[&0].ports[0].leader.pre.as_ref().unwrap();
    assert!(pre.raw_cstick_x.is_some());
    let fs = r.frames[&0].start.unwrap();
    assert!(fs.scene_frame_counter.is_some());
    assert!(r.frames[&0]
        .bookend
        .unwrap()
        .latest_finalized_frame
        .is_some());
    let end = r.end.as_ref().unwrap();
    assert!(end.placements.is_some());
    // Gecko codes travel as message-splitter events, skipped by size.
    assert!(r
        .skipped_commands
        .contains(&slp::event::CMD_MESSAGE_SPLITTER));
}

#[test]
fn seeds_are_consistent_between_frame_start_and_pre_frame() {
    // Pre Frame's seed is "the seed at this point" (before this port's input
    // is used); for port 0 it should be the frame's start seed unless the
    // game consumed randomness in between. Check that they agree on most frames.
    let r = Replay::parse(&data("v3.12.slp")).unwrap();
    let mut agree = 0;
    let mut total = 0;
    for f in r.frames.values() {
        let fs = f.start.unwrap().random_seed;
        if let Some(pre) = f.ports.iter().find_map(|p| p.leader.pre.as_ref()) {
            total += 1;
            if pre.random_seed == fs {
                agree += 1;
            }
        }
    }
    assert!(total > 0);
    assert!(
        agree * 2 > total,
        "frame-start and pre-frame seeds agree on {agree}/{total}"
    );
}

#[test]
fn ice_climbers_have_a_follower() {
    let r = Replay::parse(&data("ics.slp")).unwrap();
    let ics_port = r
        .start
        .players
        .iter()
        .find(|p| p.is_present() && p.character == 14)
        .map(|p| p.port as usize)
        .expect("an Ice Climbers player");
    let f = &r.frames[&0];
    let follower = &f.ports[ics_port].follower;
    assert!(
        follower.pre.is_some() && follower.post.is_some(),
        "Nana should have pre and post"
    );
    assert_eq!(
        follower.post.as_ref().unwrap().internal_character,
        11,
        "Nana internal id"
    );
    assert_eq!(
        f.ports[ics_port]
            .leader
            .post
            .as_ref()
            .unwrap()
            .internal_character,
        10,
        "Popo internal id"
    );
    for (port, p) in f.ports.iter().enumerate() {
        if port != ics_port {
            assert!(p.follower.is_empty());
        }
    }
}

#[test]
fn netplay_replay_is_online_and_finalised() {
    let r = Replay::parse(&data("netplay.slp")).unwrap();
    assert!(r.start.is_online());
    for f in r.frames.values() {
        let b = f.bookend.unwrap();
        assert!(b.latest_finalized_frame.unwrap() <= f.number);
    }
    assert!(r
        .frames
        .values()
        .all(|f| f.ports.iter().filter(|p| p.leader.post.is_some()).count() == 2));
}

#[test]
fn joystick_inputs_reach_the_scenario() {
    let r = Replay::parse(&data("joystick_udlr.slp")).unwrap();
    let toml = slp::to_scenario(&r);
    assert!(toml.starts_with("# Generated from a Slippi replay"));
    assert!(toml.contains("\nframes = "));
    assert!(toml.contains("\nseed = "));
    assert!(toml.contains("\n[[fighters]]\n"));
    assert!(toml.contains("controller = \"scripted\""));
    let inputs = toml.matches("\n[[inputs]]\n").count();
    let humans = r
        .start
        .players
        .iter()
        .filter(|p| p.player_type == PlayerType::Human)
        .count();
    assert_eq!(inputs, r.frames.len() * humans);
    // The fixture pushes the stick to the cardinals. Melee scales raw stick
    // values by 1/80, so a full deflection reads as 0.9875 or 1.0.
    let max_x = r
        .frames
        .values()
        .flat_map(|f| f.ports.iter().filter_map(|p| p.leader.pre.as_ref()))
        .flat_map(|pre| [pre.joystick_x.abs(), pre.joystick_y.abs()])
        .fold(0.0f32, |max, v| if v > max { v } else { max });
    assert!(max_x > 0.95, "max stick deflection {max_x}");
    assert!(
        toml.contains("stick = [0.9875, 0.0]")
            || toml.contains("stick = [1.0, 0.0]")
            || toml.contains("stick = [0.0, -1.0]")
    );
    // Harness frames start at 0 and every frame is present once per human.
    assert!(toml.contains("\nframe = 0\n"));
    let last = harness_frame(r.last_frame().unwrap()).unwrap();
    assert!(toml.contains(&format!("\nframe = {last}\n")));
}

#[test]
fn trace_has_schema_paths_and_shifted_frames() {
    let r = Replay::parse(&data("v3.16.slp")).unwrap();
    let recs = slp::to_trace(&r);
    assert_eq!(recs.len(), r.frames.len());
    assert_eq!(recs[0].frame, 0);
    assert_eq!(
        recs.last().unwrap().frame,
        harness_frame(r.last_frame().unwrap()).unwrap()
    );
    for (i, rec) in recs.iter().enumerate() {
        assert_eq!(rec.frame, i as u64);
        assert_eq!(rec.phase, "frame_end");
    }
    let first = &recs[0];
    let port = r
        .start
        .players
        .iter()
        .find(|p| p.is_present())
        .unwrap()
        .port;
    for path in [
        "cur_pos.x",
        "cur_pos.y",
        "motion_id",
        "facing_dir",
        "percent",
        "kind",
        "ground_or_air",
        "self_vel.x",
        "kb_vel.y",
    ] {
        assert!(
            first.state.contains_key(&format!("p{port}.{path}")),
            "missing p{port}.{path}"
        );
    }
    assert!(!first.state.keys().any(|k| k.ends_with("jumps_used")));
    // rng.seed on frame n is the start seed of frame n+1.
    let next_seed = r.frames[&(SLIPPI_FIRST_FRAME + 1)].start_seed().unwrap();
    assert_eq!(
        first.state["rng.seed"],
        melee_diff::Value::UInt(next_seed as u64)
    );
    assert!(!recs.last().unwrap().state.contains_key("rng.seed"));
    // Bit-exact float carriage.
    let post = r.frames[&SLIPPI_FIRST_FRAME].ports[port as usize]
        .leader
        .post
        .as_ref()
        .unwrap();
    assert_eq!(
        first.state[&format!("p{port}.cur_pos.x")],
        melee_diff::Value::f32(post.position_x)
    );
    // Round-trips through the JSONL reader.
    let mut buf = Vec::new();
    for rec in &recs {
        serde_json::to_writer(&mut buf, rec).unwrap();
        buf.push(b'\n');
    }
    let back = melee_diff::read_trace(std::io::Cursor::new(buf)).unwrap();
    // Compare by bit pattern: `approx` is informational and serde_json's
    // default float parser may perturb its last digit.
    assert_eq!(back.len(), recs.len());
    for (a, b) in recs.iter().zip(&back) {
        assert_eq!((a.frame, &a.phase), (b.frame, &b.phase));
        assert_eq!(a.state.len(), b.state.len());
        for (k, va) in &a.state {
            let vb = b
                .state
                .get(k)
                .unwrap_or_else(|| panic!("frame {} missing {k}", a.frame));
            match (va, vb) {
                (
                    melee_diff::Value::F32 { bits: x, .. },
                    melee_diff::Value::F32 { bits: y, .. },
                ) => assert_eq!(x, y, "{k}"),
                _ => assert_eq!(va, vb, "{k}"),
            }
        }
    }
}

#[test]
fn garbage_is_rejected() {
    assert!(Replay::parse(b"").is_err());
    assert!(Replay::parse(b"not a replay").is_err());
    let mut truncated = data("v3.12.slp");
    truncated.truncate(200);
    assert!(Replay::parse(&truncated).is_err());
}
