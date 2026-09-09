use super::*;
use gekko_math::rng::HsdRng;
use hsd_types::Vec3;
use melee_diff::{first_divergence, Record, RecordSink};
use melee_ft::{
    fighter::{interleaved_order, FighterProc},
    input::PadSample,
};
use melee_types::snapshot::{PrefixSink, Snapshot};

pub fn replay(scene: &str, ticks: usize, compare_bones: bool) {
    replay_with_observer(
        scene,
        ticks,
        if compare_bones {
            BoneOracle::Srt
        } else {
            BoneOracle::None
        },
        |_, _| {},
    );
}

#[derive(Clone, Copy, PartialEq)]
pub enum BoneOracle {
    None,
    Srt,
    Rendered,
}

type FoxFighter = melee_ft::fighter::Fighter<ft_fox::init::Fox>;

pub fn replay_with_observer(
    scene: &str,
    ticks: usize,
    oracle: BoneOracle,
    mut observe: impl FnMut(usize, &[FoxFighter; 2]),
) {
    let compare_bones = oracle != BoneOracle::None;
    let trace_path = harness().join(format!("traces/{scene}_fd_fox.tick.expected.jsonl"));
    let ledger_path = harness().join(format!("traces/{scene}_fd_fox.ledger600.raw.jsonl"));
    let raw_path = harness().join(format!("traces/{scene}_fd_fox.tick.raw.jsonl"));
    if !trace_path.exists() || !ledger_path.exists() || !raw_path.exists() {
        eprintln!("skipping: local FD expected trace/raw trace/RNG ledger absent");
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
    let bones_path = harness().join(format!("traces/{scene}_fd_fox.bones.jsonl"));
    let save_path = harness().join(format!("roms/{scene}_fd_fox.sav"));
    if !save_path.exists() || (compare_bones && !bones_path.exists()) {
        eprintln!("skipping: local savestate/bone oracle absent");
        return;
    }
    let first_raw = raw(&raw_trace[0], 0);
    let address = u32::from_str_radix(
        raw_trace[0]["fighters"][0]["base"]
            .as_str()
            .unwrap()
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let saved = saved_pose::SavedPose::load(&save_path, &first_raw, address);
    for (player, fighter) in fighters.iter_mut().enumerate() {
        saved.restore(fighter, &raw(&raw_trace[0], player));
    }
    let bones = compare_bones.then(|| json_lines(&bones_path));
    // Separate captures must agree on scheduler ticks and RNG boundaries.
    let bone_raw_path = harness().join(format!("traces/{scene}_fd_fox.bones.raw.jsonl"));
    if compare_bones && bone_raw_path.exists() {
        let bone_raw = json_lines(&bone_raw_path);
        assert_eq!(bone_raw.len(), ticks);
        for (a, b) in bone_raw.iter().zip(&raw_trace) {
            assert_eq!(a["tick"], b["tick"]);
            assert_eq!(a["seed"], b["seed"]);
        }
    }
    if let Some(rows) = &bones {
        assert_eq!(rows.len(), ticks);
        // This single row is the scheduler-boundary pose, not a later input.
        // Import the entire main tree: idle also has stale palm caches on
        // bones 26/56, outside the dynamics/part-animation bone lists.
        for (player, fighter) in fighters.iter_mut().enumerate() {
            saved_pose::restore_oracle_boundary(fighter, player, &rows[0]);
        }
    }
    let mut first_bone_mismatch = None;
    let mut matched = [0; 2];
    let mut total_draws = 0;
    let mut rng = HsdRng::new(ledger[0]["seed"].as_u64().unwrap() as u32);
    for tick in 0..ticks {
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
        assert!(
            draws.iter().all(|draw| {
                let site = draw["lr"].as_u64().unwrap() - 4;
                !(0x8001_044C..0x8001_15F4).contains(&site)
            }),
            "dynamics unexpectedly drew RNG at tick {tick}"
        );
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
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match proc {
                        FighterProc::Status => f.proc_status(),
                        FighterProc::Animation => {
                            if let Some(choice) = f.proc_anim(&fixture.assets, &mut rng).unwrap() {
                                used += choice.draws;
                                total_draws += choice.draws;
                                assert!(
                                    used <= sites.len(),
                                    "extra Wait draw tick {tick} p{player}"
                                );
                                assert_eq!(
                                    rng.seed,
                                    sites[used - 1].1["seed"].as_u64().unwrap() as u32,
                                    "Wait post-seed tick {tick} p{player}"
                                );
                            }
                        }
                        FighterProc::CpuGate => f.proc_cpu_gate(),
                        FighterProc::Input => f.proc_input(&fixture.assets, &PadSample::default()),
                        FighterProc::Update => {
                            f.proc_update(&fixture.assets, &fixture.map, Vec3::ZERO)
                        }
                        FighterProc::Map => {
                            let count = f
                                .proc_map_with_assets(&fixture.assets, &mut fixture.map, &mut rng)
                                .unwrap();
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
                        FighterProc::Pose => f.proc_pose(&fixture.map),
                        FighterProc::Accessories => f.proc_accessories(),
                        FighterProc::HitboxPositions => f.proc_hitbox_positions(),
                        FighterProc::Grab => f.proc_grab(),
                        FighterProc::HitDetection => f.proc_hit_detection(),
                        FighterProc::ProcessHit => f.proc_process_hit(&fixture.assets),
                        FighterProc::Dynamics => f.proc_dynamics_with_map(&mut fixture.map),
                        FighterProc::Camera => f.proc_camera(&fixture.assets, 1.0),
                        FighterProc::PlayerMirror => f.proc_player_mirror(),
                    }));
                if let Err(error) = result {
                    let message = error
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| error.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic");
                    panic!("replay stopped at tick {tick}, p{player}, s_link {}, state {:?}: {message}; completed records {matched:?}", proc.s_link(), f.motion_state.id);
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
        if let Some(bones) = &bones {
            if oracle == BoneOracle::Srt {
                if let Some(mismatch) = compare_pose(&fighters, &bones[tick], tick, scene) {
                    first_bone_mismatch.get_or_insert(mismatch);
                }
            }
        }
        observe(tick, &fighters);
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
    if scene == "start" && ticks == 600 {
        assert_eq!(total_draws, 16);
    }
    assert_eq!(matched, [ticks, ticks]);
    assert!(
        first_bone_mismatch.is_none(),
        "{}",
        first_bone_mismatch.unwrap_or_default()
    );
    eprintln!("{scene}: {ticks} records per fighter; 24 fields each; {total_draws} fighter draws; bones={compare_bones}");
}

fn compare_pose(
    fighters: &[melee_ft::fighter::Fighter<ft_fox::init::Fox>; 2],
    expected: &serde_json::Value,
    tick: usize,
    scene: &str,
) -> Option<String> {
    let mut first = None;
    let mut details = Vec::new();
    let mut mismatches = std::collections::BTreeMap::<(usize, usize, String), usize>::new();
    assert_eq!(expected["frame"].as_u64().unwrap(), tick as u64);
    assert_eq!(expected["state"].as_object().unwrap().len(), 3212);
    for (player, fighter) in fighters.iter().enumerate() {
        for (bone, part) in fighter.animation.parts.iter().enumerate() {
            let joint = fighter.skeleton.get(part.joint);
            let in_dynamics = fighter.bones.dynamics_roots.iter().any(|&root| {
                let mut id = fighter.skeleton.bone(fighter.animation.root, root as usize);
                while let Some(current) = id {
                    if current == part.joint {
                        return true;
                    }
                    id = fighter.skeleton.child(current);
                }
                false
            });
            let rotation = [
                joint.rotate.x,
                joint.rotate.y,
                joint.rotate.z,
                joint.rotate.w,
            ];
            let scale = [joint.scale.x, joint.scale.y, joint.scale.z];
            let translate = [joint.translate.x, joint.translate.y, joint.translate.z];
            for (field, values) in [
                ("rotate", &rotation[..]),
                ("scale", &scale[..]),
                ("translate", &translate[..]),
            ] {
                for (index, actual) in values.iter().enumerate() {
                    // Reviewer-approved exclusion: Euler mode never reads W.
                    // Keep W bit-exact whenever JOBJ_USE_QUATERNION is set.
                    if !compared_component(joint.flags, field, index) {
                        continue;
                    }
                    let key = format!("p{player}.bone[{bone}].{field}[{index}]");
                    let bits = expected["state"][&key]["v"]["bits"].as_u64().unwrap() as u32;
                    if actual.to_bits() != bits {
                        let category = if in_dynamics {
                            "dynamics"
                        } else if fighter
                            .bones
                            .animation_sets
                            .iter()
                            .flatten()
                            .any(|set| set.joints.contains(&(bone as u8)))
                        {
                            "part"
                        } else {
                            "other"
                        };
                        details.push(serde_json::json!({"player":player,"bone":bone,"field":field,"index":index,"expected":bits,"actual":actual.to_bits(),"category":category}));
                        first.get_or_insert_with(|| format!("first bone mismatch tick {tick}, p{player}, bone {bone}, {field}[{index}]: expected {bits:08X}, actual {:08X}, dynamics={in_dynamics}, port flags={:08X}", actual.to_bits(), joint.flags));
                        *mismatches
                            .entry((player, bone, format!("{field}[{index}]")))
                            .or_default() += 1;
                    }
                }
            }
        }
    }
    if std::env::var_os("MELEE_BONE_MISMATCH_DETAILS").is_some() {
        eprintln!(
            "BONE_DIFF {}",
            serde_json::json!({"scene":scene,"tick":tick,"differences":details})
        );
    }
    if let Some(first) = &first {
        eprintln!(
            "{first}; differing words: {}; first differing fields: {:?}",
            mismatches.len(),
            mismatches.keys().take(12).collect::<Vec<_>>()
        );
    }
    first
}

/// Euler W is undefined retail stack data. All quaternion components and
/// all other fields remain part of the bit-exact comparison.
pub fn compared_component(flags: u32, field: &str, index: usize) -> bool {
    field != "rotate" || index != 3 || flags & hsd_anim::jobj::JOBJ_USE_QUATERNION != 0
}
