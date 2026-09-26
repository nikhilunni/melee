//! State-callback integration only. The separate start_fox_600 test requires
//! all scheduler procs, including the currently unported tail dynamics solver.
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
fn start_fox_state_callbacks_600() {
    let trace_path = harness().join("traces/start_fd_fox.tick.expected.jsonl");
    let ledger_path = harness().join("traces/start_fd_fox.ledger600.raw.jsonl");
    let raw_path = harness().join("traces/start_fd_fox.tick.raw.jsonl");
    if !melee_test_support::require_files([&trace_path, &ledger_path, &raw_path]) {
        return;
    }
    let Some(mut fixture) = Fixture::load() else {
        return;
    };
    let trace = json_lines(&trace_path);
    let ledger = json_lines(&ledger_path);
    let raw_trace = json_lines(&raw_path);
    assert_eq!(raw_trace.len(), 600);
    assert_eq!(trace.len(), 600);
    assert_eq!(ledger.len(), 600);
    let mut fighters = [
        fixture.import(&raw(&raw_trace[0], 0)),
        fixture.import(&raw(&raw_trace[0], 1)),
    ];
    let mut matched = [0; 2];
    let mut total_draws = 0;
    let mut rng = HsdRng::new(ledger[0]["seed"].as_u64().unwrap() as u32);
    for tick in 0..600 {
        assert_eq!(trace[tick]["tick"], ledger[tick]["tick"]);
        assert_eq!(trace[tick]["tick"], raw_trace[tick]["tick"]);
        let draws = ledger[tick]["rng_draws"].as_array().unwrap();
        let sites = draws
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                matches!(
                    d["lr"].as_u64().unwrap() - 4,
                    0x8008_A8BC | 0x8009_FCDC | 0x8009_FD00 | 0x8009_FD24
                )
            })
            .collect::<Vec<_>>();
        let mut used = 0;
        if tick != 0 {
            for (proc, player) in interleaved_order(fighters.len()) {
                let f = &mut fighters[player];
                // Particle/stage draws are externally supplied before each
                // fighter callback. All draws inside one callback are contiguous.
                if let Some((index, _)) = sites.get(used) {
                    rng.seed = if *index == 0 {
                        ledger[tick - 1]["seed"].as_u64().unwrap()
                    } else {
                        draws[index - 1]["seed"].as_u64().unwrap()
                    } as u32;
                }
                match proc {
                    FighterProc::Status => f.proc_status(),
                    FighterProc::Animation => {
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
                    FighterProc::Map => {
                        f.proc_map_with_assets(&fixture.assets, &mut fixture.map)
                            .unwrap();
                        // Landing dust draws with the proc's graphics.
                        let count = f.resolve_graphics_commands(&fixture.assets, &mut rng);
                        used += count;
                        total_draws += count;
                        if count != 0 {
                            assert!(
                                used <= sites.len(),
                                "extra fighter draw tick {tick} p{player}"
                            );
                            assert_eq!(
                                rng.seed,
                                sites[used - 1].1["seed"].as_u64().unwrap() as u32
                            );
                        }
                    }
                    // This focused test owns Anim/IASA/Phys/Coll and their
                    // prerequisite status/input procs. It is not the full gate.
                    _ => {}
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
            panic!(
                "first mismatch tick {tick}: {mismatch}; states {:?}",
                fighters.each_ref().map(|f| f.motion_state.id)
            );
        }
        for (player, f) in fighters.iter().enumerate() {
            let bytes = raw(&raw_trace[tick], player);
            assert_eq!(
                f.animation.motion_id as u32,
                word(&bytes, 0x14),
                "submotion tick {tick} p{player}"
            );
            if let melee_ft::fighter::MotionData::Entry(entry) = &f.state_data {
                assert_eq!(
                    entry.timer,
                    word(&bytes, 0x2340) as i32,
                    "entry timer tick {tick} p{player}"
                );
            }
            matched[player] += 1;
        }
    }
    let expected_draws: usize = ledger
        .iter()
        .skip(1)
        .map(|row| {
            row["rng_draws"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|draw| {
                    matches!(
                        draw["lr"].as_u64().unwrap() - 4,
                        0x8008_A8BC | 0x8009_FCDC | 0x8009_FD00 | 0x8009_FD24
                    )
                })
                .count()
        })
        .sum();
    assert_eq!(total_draws, expected_draws);
    assert_eq!(matched, [600, 600]);
    eprintln!("P0: 600/600; P1: 600/600; 24 fields each, first mismatch: none; all {total_draws} fighter draws matched (frame 0 + 599 transitions)");
}
