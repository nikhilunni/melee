//! End-to-end fighter replay: tick zero is the imported savestate boundary.
mod fighter_support;
use fighter_support::*;
use gekko_math::rng::HsdRng;
use hsd_types::Vec3;
use melee_diff::{first_divergence, Record, RecordSink};
use melee_ft::{
    fighter::{interleaved_order, FighterProc},
    input::PadSample,
};
use melee_types::snapshot::{PrefixSink, Snapshot};

#[test]
fn idle_fox_600() {
    let trace_path = harness().join("traces/idle_fd_fox.tick.expected.jsonl");
    let ledger_path = harness().join("traces/idle_fd_fox.ledger600.raw.jsonl");
    let saved_path = harness().join("roms/idle_fd_fox.sav");
    if !trace_path.exists() || !ledger_path.exists() || !saved_path.exists() {
        eprintln!("skipping: local FD tick trace/ledger/savestate absent");
        return;
    }
    let Some(mut fixture) = Fixture::load() else {
        return;
    };
    let trace = json_lines(&trace_path);
    let ledger = json_lines(&ledger_path);
    assert_eq!(trace.len(), 600);
    assert_eq!(ledger.len(), 600);
    let mut fighters = [
        fixture.import(&raw(&ledger[0], 0)),
        fixture.import(&raw(&ledger[0], 1)),
    ];
    let saved = saved_pose::SavedPose::load(
        &saved_path,
        &raw(&ledger[0], 0),
        u32::from_str_radix(
            ledger[0]["fighters"][0]["base"]
                .as_str()
                .unwrap()
                .trim_start_matches("0x"),
            16,
        )
        .unwrap(),
    );
    for (player, fighter) in fighters.iter_mut().enumerate() {
        saved.restore(fighter, &raw(&ledger[0], player));
    }
    let mut matched = [0; 2];
    let mut total_draws = 0;
    let mut rng = HsdRng::new(ledger[0]["seed"].as_u64().unwrap() as u32);
    for tick in 0..600 {
        assert_eq!(trace[tick]["tick"], ledger[tick]["tick"]);
        let draws = ledger[tick]["rng_draws"].as_array().unwrap();
        let sites = draws
            .iter()
            .enumerate()
            .filter(|(_, d)| d["lr"].as_u64().unwrap() - 4 == 0x8008_A8BC)
            .collect::<Vec<_>>();
        let mut used = 0;
        if tick != 0 {
            for (proc, player) in interleaved_order(fighters.len()) {
                let f = &mut fighters[player];
                match proc {
                    FighterProc::Status => f.proc_status(),
                    FighterProc::Animation => {
                        // Other RNG consumers belong to T13. As in T7, seed the
                        // stream before each fighter draw from the call ledger.
                        if let Some((index, _)) = sites.get(used) {
                            rng.seed = if *index == 0 {
                                ledger[tick - 1]["seed"].as_u64().unwrap()
                            } else {
                                draws[index - 1]["seed"].as_u64().unwrap()
                            } as u32;
                        }
                        if let Some(choice) = f.proc_anim(&fixture.assets, &mut rng).unwrap() {
                            used += choice.draws;
                            total_draws += choice.draws;
                            assert!(used <= sites.len(), "extra Wait draw tick {tick} p{player}");
                            assert_eq!(
                                rng.seed,
                                sites[used - 1].1["seed"].as_u64().unwrap() as u32,
                                "Wait post-seed tick {tick} p{player}"
                            );
                        }
                    }
                    FighterProc::CpuGate => f.proc_cpu_gate(),
                    FighterProc::Input => f.proc_input(&fixture.assets, &PadSample::default()),
                    FighterProc::Update => f.proc_update(&fixture.assets, &fixture.map, Vec3::ZERO),
                    FighterProc::Map => f.proc_map(&mut fixture.map),
                    FighterProc::Pose => f.proc_pose(&fixture.map),
                    FighterProc::Accessories => f.proc_accessories(),
                    FighterProc::HitboxPositions => f.proc_hitbox_positions(),
                    FighterProc::Grab => f.proc_grab(),
                    FighterProc::HitDetection => f.proc_hit_detection(),
                    FighterProc::ProcessHit => f.proc_process_hit(&fixture.assets),
                    FighterProc::Dynamics => f.proc_dynamics(),
                    FighterProc::Camera => f.proc_camera(&fixture.assets, 1.0),
                    FighterProc::PlayerMirror => f.proc_player_mirror(),
                }
            }
        }
        assert_eq!(used, sites.len(), "unconsumed fighter draws tick {tick}");
        let mut expected: Record = serde_json::from_value(trace[tick].clone()).unwrap();
        // This gate owns 24 fields per fighter. Global RNG is T13's field.
        expected.state.remove("rng.seed");
        assert_eq!(expected.state.len(), 48);
        let mut sink = RecordSink::new(expected.frame, expected.phase.clone());
        for (player, f) in fighters.iter().enumerate() {
            f.snapshot(&mut PrefixSink::new(&mut sink, &format!("p{player}")));
        }
        let actual = sink.finish();
        assert_eq!(actual.state.len(), 48);
        if let Some(mismatch) = first_divergence([&expected], [&actual]) {
            panic!("first mismatch: {mismatch}");
        }
        for (player, f) in fighters.iter().enumerate() {
            let bytes = raw(&ledger[tick], player);
            // Establish the empty branches of s_links 8/9/12/13 against all
            // ledger rows, not just the recorded snapshot subset.
            for offset in [
                0x60C, // efAsync queue
                0x914, 0xA4C, 0xB84, 0xCBC, // ordinary attack states
                0x1974, 0x197C, 0x1980, // held/attached items
                0x1A58, 0x1A60, // grab victim/target item
                0x2094, 0x20A0, // combo opponent/accessory model
                0x21B0, 0x21B4, 0x21B8, 0x21BC, // accessory callbacks
            ] {
                assert_eq!(
                    word(&bytes, offset),
                    0,
                    "idle branch +{offset:X} tick {tick} p{player}"
                );
            }
            assert_eq!(bytes[0x221E] & 2, 0, "catch hitbox inactive");
            assert_eq!(
                f.status.name_tag_timer,
                u16::from_be_bytes([bytes[0x209A], bytes[0x209B]])
            );
            assert_eq!(f.thrown_hitbox.state, word(&bytes, 0x1064));
            for (position, offset) in [
                (f.thrown_hitbox.position, 0x10B0),
                (f.thrown_hitbox.previous_position, 0x10BC),
            ] {
                let expected = vector(&bytes, offset);
                assert_eq!(
                    [
                        position.x.to_bits(),
                        position.y.to_bits(),
                        position.z.to_bits()
                    ],
                    [
                        expected.x.to_bits(),
                        expected.y.to_bits(),
                        expected.z.to_bits()
                    ],
                    "thrown capsule +{offset:X} tick {tick} p{player}"
                );
            }
            assert_eq!(
                f.animation.motion_id as u32,
                word(&bytes, 0x14),
                "submotion tick {tick} p{player}"
            );
            assert_eq!(
                f.commands.timer.to_bits(),
                word(&bytes, 0x3E4),
                "command timer tick {tick} p{player}"
            );
            assert_eq!(
                f.commands.frame.to_bits(),
                word(&bytes, 0x3E8),
                "command frame tick {tick} p{player}"
            );
            assert_eq!(
                f.dynamics_first_bone[0],
                word(&bytes, 0x2F0),
                "disabled dynamics tick {tick} p{player}"
            );
            matched[player] += 1;
        }
    }
    assert_eq!(total_draws, 9);
    assert_eq!(matched, [600, 600]);
    eprintln!("P0: 600/600; P1: 600/600; 24 fields each, first mismatch: none; all 9 Wait draws matched (frame 0 + 599 transitions)");
}
