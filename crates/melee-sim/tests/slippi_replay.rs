use melee_diff::{Record, Value};
use melee_sim::replay::{self, Setup, Stop};
use melee_sim::replay_lag_frames::{LagFrames, LagKind};
use slp::{Replay, SLIPPI_FIRST_FRAME};
use std::{collections::BTreeMap, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The default setup with a display pass before every tick: no stop is
/// explained by a lag frame. Every test here names its mode; only
/// `a_console_lag_frame_is_inferred_only_where_the_recording_confirms_it`
/// and the mismatch test run with inference.
fn every_tick() -> Setup {
    Setup {
        lag_frames: LagFrames::Off,
        ..Setup::default()
    }
}
fn fixture(name: &str) -> Replay {
    Replay::parse(&std::fs::read(root().join("crates/slp/tests/data").join(name)).unwrap()).unwrap()
}

#[test]
fn fixture_matched_frame_counts_never_decrease() {
    // Pin the composition and the reason as well, so an unconditional skip or
    // a setup/parser regression cannot masquerade as a successful zero floor.
    // The three online games (`replay_online`): the 2020 one runs to its
    // last frame; the 2022 and 2024 ones list netplay codes the port does
    // not run and are refused by name.
    let cases = [
        ("ics.slp", "FinalDestination", "Cpu control", 0),
        ("joystick_udlr.slp", "FinalDestination", "Cpu control", 0),
        ("netplay.slp", "FountainOfDreams", "complete", 128),
        // Pre-2.2: no Frame Start shows the unlock flag, which Setup::default omits.
        ("v0.1.slp", "DreamLand", "unlock flag", 0),
        (
            "v3.12.slp",
            "PokemonStadium",
            "netplay code PSCameraIndependentMonitor at 801D24FC is not ported",
            0,
        ),
        (
            "v3.13.slp",
            "FinalDestination",
            "requires stock mode, items off, normal damage and countdown/no timer",
            0,
        ),
        (
            "v3.16.slp",
            "YoshisStory",
            "netplay code FreezeDeadUpFallPhysics (InitHitVelocity) at 800D4C1C is not ported",
            0,
        ),
        ("v3.18.slp", "FountainOfDreams", "Cpu control", 0),
    ];
    for (name, stage, reason, floor) in cases {
        // Only a game that runs reads the disc's files.
        let runs = reason == "complete";
        if runs && !melee_test_support::require_files([&root().join("harness/roms/files/PlCo.dat")]) {
            continue;
        }
        let report = replay::run(&fixture(name), &root(), every_tick()).unwrap();
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
            Stop::NeedsSetup(why) => {
                assert_eq!(report.matched, 0);
                assert!(why.contains(reason), "{name}: {report}");
            }
            Stop::Complete => {
                assert_eq!(reason, "complete", "{name}: {report}");
                assert_eq!(report.matched, report.frames, "{name}: {report}");
            }
            Stop::Unported { .. } => {
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
/// Game Start's seed precedes the setup's fixed draws and, for an unlocked
/// rule-6 stage, the music draw; invert them from the first Frame Start.
fn oracle_replay(
    scene: &str,
    setup_draws: usize,
    music_draw: bool,
) -> Option<(Replay, Vec<Record>)> {
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
    let mut seed = int(&expected[0], "rng.seed") as u32;
    for _ in 0..setup_draws + usize::from(music_draw) {
        seed = seed
            .wrapping_sub(gekko_math::HsdRng::INCREMENT)
            .wrapping_mul(0xB9B3_3155);
    }
    replay.start.random_seed = seed;
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
    let Some((replay, _)) = oracle_replay("start_fd_fox", 8, false) else {
        return;
    };
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(false),
            boundary_seed: None,
            ignore_controller_fixes: false,
            controller_fix: None,
            frozen_stages: None,
            screen: None,
            skipped_display_pass: None,
            lag_frames: LagFrames::Off,
            strict_zeros: false,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 599, "{report}");
    assert!(matches!(report.stop, Stop::Complete), "{report}");
}

#[test]
fn a_ported_state_one_bit_mismatch_is_a_failure() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox", 8, false) else {
        return;
    };
    let post = replay.frames.get_mut(&SLIPPI_FIRST_FRAME).unwrap().ports[0]
        .leader
        .post
        .as_mut()
        .unwrap();
    post.position_x = f32::from_bits(post.position_x.to_bits() ^ 1);
    // With lag-frame inference too: a position outside the screen KO is no
    // lag frame's, and the stop is reported as it is without inference.
    for lag_frames in [LagFrames::Off, LagFrames::Infer] {
        let report = replay::run(
            &replay,
            &root(),
            Setup {
                all_characters_unlocked: Some(false),
                boundary_seed: None,
                ignore_controller_fixes: false,
                controller_fix: None,
                frozen_stages: None,
                screen: None,
                skipped_display_pass: None,
                lag_frames,
                strict_zeros: false,
            },
        )
        .unwrap();
        assert_eq!(report.matched, 0);
        assert_eq!(
            report.lag_frames,
            (lag_frames == LagFrames::Infer).then(Vec::new)
        );
        let Stop::Diverged(diff) = report.stop else {
            panic!("{report}")
        };
        assert_eq!(diff.frame, 0);
        assert_eq!(diff.path, "p0.cur_pos.x");
        assert_ne!(diff.expected, diff.actual);
    }
}

/// The corpus replays whose stops are a console's lag frame, each proven on
/// retail with a recording pair (`docs/SLIPPI.md`, "Lag frames read from a
/// replay"): with a display pass before every tick the port stops where
/// retail in Dolphin leaves the console, and the inference names the tick
/// retail's lag-frame recording skipped.
#[test]
fn a_console_lag_frame_is_inferred_only_where_the_recording_confirms_it() {
    use melee_lib::slippi::Screen;
    use melee_lib::ControllerFix::Ucf074;
    // Read the main checkout's data from a worktree, as `melee-sim replay`.
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(root, PathBuf::from);
    let corpus = PathBuf::from(std::env::var_os("HOME").expect("HOME"))
        .join("melee-data/replays/public-v3.7-b");
    let game = corpus.join("FOX/Fox vs Marth [YS] Game_20200115T192755.slp");
    if !melee_test_support::require_files([&game, &root.join("harness/roms/files/PlCo.dat")]) {
        return;
    }
    let mut replay = Replay::parse(&std::fs::read(&game).unwrap()).unwrap();
    // The KO (Marth hits the screen at tick 5873) and what follows it; the
    // rest of the game adds nothing but time.
    let kept = SLIPPI_FIRST_FRAME + 6100;
    replay.frames.retain(|&number, _| number < kept);
    // The codes this console ran, as the runner reads them from the whole
    // game (the dry passes are not what is tested here).
    let setup = |lag_frames| Setup {
        all_characters_unlocked: Some(true),
        boundary_seed: None,
        ignore_controller_fixes: false,
        controller_fix: Some(Ucf074),
        frozen_stages: Some(false),
        screen: Some(Screen::Standard),
        skipped_display_pass: None,
        lag_frames,
        strict_zeros: false,
    };

    let every_tick = replay::run(&replay, &root, setup(LagFrames::Off)).unwrap();
    assert_eq!(every_tick.matched, 5873, "{every_tick}");
    assert_eq!(every_tick.lag_frames, None);
    let Stop::Diverged(diff) = &every_tick.stop else {
        panic!("{every_tick}")
    };
    assert_eq!(diff.path, "p1.cur_pos.x", "{diff}");

    let inferred = replay::run(&replay, &root, setup(LagFrames::Infer)).unwrap();
    assert!(matches!(inferred.stop, Stop::Complete), "{inferred}");
    assert_eq!(inferred.matched, replay.frames.len(), "{inferred}");
    let found: Vec<_> = inferred
        .lag_frames
        .iter()
        .flatten()
        .map(|lag| (lag.tick, lag.kind, lag.stop, lag.field.as_str()))
        .collect();
    assert_eq!(found, [(5873, LagKind::ScreenKo, 5873, "p1.cur_pos.x")]);

    // The same pass named by hand is not inferred again, and one skipped
    // pass elsewhere in the KO does not make its frame the recording's.
    for (skipped, matched, lag_frames) in [(5873, replay.frames.len(), 0), (5872, 5872, 0)] {
        let named = Setup {
            skipped_display_pass: Some(skipped),
            ..setup(LagFrames::Infer)
        };
        let report = replay::run(&replay, &root, named).unwrap();
        assert_eq!(report.matched, matched, "{skipped}: {report}");
        assert_eq!(
            report.lag_frames.as_ref().map(Vec::len),
            Some(lag_frames),
            "{skipped}: {report}"
        );
    }
}

/// A lag frame whose stop comes 302 ticks after it (`docs/SLIPPI.md`, "A lag
/// frame that shows 302 ticks later"): with no display pass before tick 9381
/// Pokemon Stadium's jumbotron keeps a close-up one tick more, the standings
/// that follow (no draw, a fixed length) end one tick later, and the seeds
/// are the console's. Proven on retail by
/// `slp_ps_fox_drmario_wide9263_lag9382b_t9800`. The inference names that
/// tick from the simulation's own jumbotron and no other.
#[test]
fn a_jumbotron_lag_frame_is_the_pass_where_the_close_up_was_lost() {
    use melee_lib::slippi::Screen;
    use melee_lib::ControllerFix::Ucf074;
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(root, PathBuf::from);
    let corpus = PathBuf::from(std::env::var_os("HOME").expect("HOME"))
        .join("melee-data/replays/public-v3.7-b");
    let game = corpus.join(
        "DOC/20200219 - HNC 1 - PM 0655 - Fox (Default) vs ME Dr. Mario (Red) - Pokemon Stadium.slp",
    );
    if !melee_test_support::require_files([&game, &root.join("harness/roms/files/PlCo.dat")]) {
        return;
    }
    let mut replay = Replay::parse(&std::fs::read(&game).unwrap()).unwrap();
    // The stop (9684) and the 600 frames that must match after it.
    let kept = SLIPPI_FIRST_FRAME + 10400;
    replay.frames.retain(|&number, _| number < kept);
    let setup = |lag_frames, skipped_display_pass| Setup {
        all_characters_unlocked: Some(true),
        boundary_seed: None,
        ignore_controller_fixes: false,
        controller_fix: Some(Ucf074),
        frozen_stages: Some(true),
        screen: Some(Screen::Widescreen92By63),
        skipped_display_pass,
        lag_frames,
        strict_zeros: false,
    };

    let every_tick = replay::run(&replay, &root, setup(LagFrames::Off, None)).unwrap();
    assert_eq!(every_tick.matched, 9684, "{every_tick}");
    assert_eq!(every_tick.lag_frames, None);
    let Stop::Diverged(diff) = &every_tick.stop else {
        panic!("{every_tick}")
    };
    assert_eq!(diff.path, "p0.input_seed", "{diff}");

    let inferred = replay::run(&replay, &root, setup(LagFrames::Infer, None)).unwrap();
    assert!(matches!(inferred.stop, Stop::Complete), "{inferred}");
    assert_eq!(inferred.matched, replay.frames.len(), "{inferred}");
    let found: Vec<_> = inferred
        .lag_frames
        .iter()
        .flatten()
        .map(|lag| (lag.tick, lag.kind, lag.stop, lag.field.as_str(), lag.confirmed))
        .collect();
    assert_eq!(
        found,
        [(9381, LagKind::Jumbotron, 9684, "p0.input_seed", 600)]
    );

    // The same pass named by hand is not inferred again. A pass skipped on
    // either side of it is not the console's: by itself it leaves the stop
    // where it was.
    let named = replay::run(&replay, &root, setup(LagFrames::Infer, Some(9381))).unwrap();
    assert_eq!(named.matched, replay.frames.len(), "{named}");
    assert_eq!(named.lag_frames.as_ref().map(Vec::len), Some(0), "{named}");
    for skipped in [9380, 9382] {
        let report = replay::run(&replay, &root, setup(LagFrames::Off, Some(skipped))).unwrap();
        assert_eq!(report.matched, 9684, "{skipped}: {report}");
    }
}

#[test]
fn battlefield_music_seed_and_first_full_tick_match_the_oracle() {
    let Some((mut replay, _)) = oracle_replay("start_bf_fox", 5, true) else {
        return;
    };
    replay.start.stage = 31;
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(true),
            boundary_seed: None,
            ignore_controller_fixes: false,
            controller_fix: None,
            frozen_stages: None,
            screen: None,
            skipped_display_pass: None,
            lag_frames: LagFrames::Off,
            strict_zeros: false,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 599, "{report}");
    assert!(matches!(report.stop, Stop::Complete), "{report}");
}

#[test]
fn missing_late_input_preserves_the_matched_prefix() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox", 8, false) else {
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
            ignore_controller_fixes: false,
            controller_fix: None,
            frozen_stages: None,
            screen: None,
            skipped_display_pass: None,
            lag_frames: LagFrames::Off,
            strict_zeros: false,
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
fn gapped_ports_keep_spawn_markers_and_delay_entry_by_slot() {
    let Some((mut replay, _)) = oracle_replay("start_fd_fox", 8, false) else {
        return;
    };
    // Relabel the slot-1 recording as slot 2 (port 3) with slot 1's marker.
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
        ignore_controller_fixes: false,
        controller_fix: None,
        frozen_stages: None,
        screen: None,
        skipped_display_pass: None,
        lag_frames: LagFrames::Off,
        strict_zeros: false,
    };
    // The marker, facing and pads route by port, so the prefix matches until
    // entry. fn_8016D8AC's delay grows by five per slot before this one
    // (tournament replays: ports 2 and 4 enter at ticks 10 and 20), so slot 2
    // enters at 15, not at the recording's 10.
    let report = replay::run(&replay, &root(), setup).unwrap();
    assert_eq!(report.matched, 10, "{report}");
    let Stop::Diverged(diff) = report.stop else {
        panic!("{report}")
    };
    assert_eq!(diff.path, "p1.cur_anim_frame", "{diff}");
    assert_eq!(diff.actual, Some(Value::f32(-1.0)), "{diff}");
}

/// InitOnlinePlay.asm's FN_SyncRNG writes the seed before player animation
/// on every scheduler pass: the runner supplies it (`replay_online`). This
/// 2024 game is still refused, for the netplay codes its Gecko list names
/// and the port does not run, each by name; Online itself is no reason.
#[test]
fn online_story_fixture_is_refused_for_its_unported_netplay_codes_only() {
    use melee_sim::replay_online::{sync_seed, Listed, Online, OnlineReason};
    let replay = fixture("v3.16.slp");
    assert_eq!(replay.frames.len(), 308);
    for frame in replay.frames.values() {
        let start = frame.start.expect("v3.16 Frame Start");
        let counter = start.scene_frame_counter.expect("v3.16 scene counter");
        assert_eq!(
            start.random_seed,
            sync_seed(counter, replay.start.random_seed)
        );
    }
    let online = Online::of(&replay).expect("an online game");
    assert_eq!(online.reason, OnlineReason::Scene);
    assert_eq!(online.offset, 0x3AAE);
    assert_eq!(online.seeds_off_the_tick_count(), 0);
    // Six frames were sent again in five rollbacks; each rollback's loop
    // also ran the frame after the newest, so at least as many ticks had
    // no display pass before them.
    assert_eq!((online.frames_resent, online.longest_rollback), (6, 2));
    assert!(online.ticks_resimulated() >= online.frames_resent);
    let reasons = replay::unsupported_setup(&replay, every_tick());
    assert_eq!(Listed::of(&replay).unwrap_err(), reasons);
    assert_eq!(
        reasons,
        [
            "netplay code FreezeDeadUpFallPhysics (InitHitVelocity) at 800D4C1C is not ported",
            "netplay code FreezeDeadUpFallPhysics (UpdateFallVelocity) at 800D4D68 is not ported",
            "netplay code FreezeDeadUpFallPhysics (UpdateModelPos) at 80080E80 is not ported",
            "netplay code WhispyBlowDirFix at 8008653C is not ported",
            "netplay code NanaDeterminism at 800AC5B8 is not ported",
            "netplay code PSCameraIndependentMonitor at 801D24FC is not ported",
            "netplay code PreventWobbling at 800DB880 is not ported",
            "netplay code PreventWobbling (Wobble Check) at 8008F090 is not ported",
        ]
    );
}

/// The 2020 online fixture runs: every tick from its Frame Start seed, the
/// codes read from its Gecko list, every recorded frame matched.
#[test]
fn online_fixture_of_2020_matches_every_frame_from_its_frame_start_seeds() {
    use melee_sim::replay_online::{Listed, Online, OnlineReason};
    if !melee_test_support::require_files([&root().join("harness/roms/files/PlCo.dat")]) {
        return;
    }
    let replay = fixture("netplay.slp");
    let online = Online::of(&replay).expect("an online game");
    assert_eq!(online.reason, OnlineReason::Scene);
    assert_eq!(online.seeds_off_the_tick_count(), 0);
    let listed = Listed::of(&replay).expect("codes the port runs");
    assert!(listed.offscreen_damage_by_camera_limits);
    assert_eq!(listed.screen, melee_lib::slippi::Screen::Standard);
    // Fountain of Dreams has no stage code.
    assert!(!listed.frozen_stages);
    let report = replay::run(&replay, &root(), Setup::default()).unwrap();
    assert!(matches!(report.stop, Stop::Complete), "{report}");
    assert_eq!((report.matched, report.frames), (128, 128), "{report}");
    let seeds = report.online.expect("the online report");
    assert_eq!(
        (seeds.pre_frame_seeds, seeds.pre_frame_seeds_after_draws),
        (256, 2)
    );
    assert_eq!(seeds.zeros_of_the_other_sign, Some(0));
    // Lag frames are not inferred for an online game.
    assert_eq!(report.lag_frames, None);
}

#[test]
fn story_primary_music_seed_and_first_full_tick_match_the_oracle() {
    let Some((mut replay, _)) = oracle_replay("start_ys_fox", 5, false) else {
        return;
    };
    replay.start.stage = 8;
    let report = replay::run(
        &replay,
        &root(),
        Setup {
            all_characters_unlocked: Some(true),
            boundary_seed: None,
            ignore_controller_fixes: false,
            controller_fix: None,
            frozen_stages: None,
            screen: None,
            skipped_display_pass: None,
            lag_frames: LagFrames::Off,
            strict_zeros: false,
        },
    )
    .unwrap();
    assert_eq!(report.matched, 599, "{report}");
    assert!(matches!(report.stop, Stop::Complete), "{report}");
}
