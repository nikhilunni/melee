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
    replay_config(scene, Some(ticks), "ledger600", oracle, false, &mut observe);
}

/// Replay only Status/Anim/Input/Phys/Coll; the simulation gate owns the full scene.
/// Tick count comes from the trace; the caller identifies the ledger capture.
pub fn replay_state_callbacks(scene: &str, ledger_suffix: &str) {
    replay_config(
        scene,
        None,
        ledger_suffix,
        BoneOracle::None,
        true,
        &mut |_, _| {},
    );
}

fn replay_config(
    scene: &str,
    tick_limit: Option<usize>,
    ledger_suffix: &str,
    oracle: BoneOracle,
    callbacks_only: bool,
    observe: &mut impl FnMut(usize, &[FoxFighter; 2]),
) {
    let compare_bones = oracle != BoneOracle::None;
    let shield_scene = matches!(scene, "shield" | "spotdodge" | "roll");
    let trace_path = harness().join(format!("traces/{scene}_fd_fox.tick.expected.jsonl"));
    let ledger_path = harness().join(format!("traces/{scene}_fd_fox.{ledger_suffix}.raw.jsonl"));
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
    assert!(!trace.is_empty());
    assert_eq!(raw_trace.len(), trace.len());
    assert_eq!(ledger.len(), trace.len());
    let ticks = tick_limit.unwrap_or(trace.len());
    assert!(ticks <= trace.len());
    let mut fighters = [
        fixture.import(&raw(&raw_trace[0], 0)),
        fixture.import(&raw(&raw_trace[0], 1)),
    ];
    let bones_path = harness().join(format!("traces/{scene}_fd_fox.bones.jsonl"));
    let save_path = harness().join(format!("roms/{scene}_fd_fox.sav"));
    if !callbacks_only && (!save_path.exists() || (compare_bones && !bones_path.exists())) {
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
    if !callbacks_only {
        let saved = saved_pose::SavedPose::load(&save_path, &first_raw, address);
        for (player, fighter) in fighters.iter_mut().enumerate() {
            saved.restore(fighter, &raw(&raw_trace[0], player));
        }
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
                if callbacks_only
                    && !(shield_scene && proc == FighterProc::ProcessHit)
                    && !matches!(
                        proc,
                        FighterProc::Status
                            | FighterProc::Animation
                            | FighterProc::CpuGate
                            | FighterProc::Input
                            | FighterProc::Update
                            | FighterProc::Map
                    )
                {
                    continue;
                }
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
                        FighterProc::Input => f.proc_input(
                            &fixture.assets,
                            &recorded_pad(&trace[tick], usize::from(f.player.id)),
                        ),
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
                        FighterProc::Camera => {
                            f.proc_camera_with_map(&fixture.assets, 1.0, &mut fixture.map)
                        }
                        FighterProc::PlayerMirror => f.proc_player_mirror(),
                    }));
                let count = f.resolve_graphics_commands(&fixture.assets, &mut rng);
                used += count;
                total_draws += count;
                if count != 0 {
                    assert!(used <= sites.len(), "extra GFX draw tick {tick} p{player}");
                    assert_eq!(
                        rng.seed,
                        sites[used - 1].1["seed"].as_u64().unwrap() as u32,
                        "GFX post-seed tick {tick}"
                    );
                }
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
            if callbacks_only {
                compare_movement_internals(f, &bytes, tick, player);
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

/// Decode only the recorded pad input, never subsequent fighter state.
pub fn recorded_pad(record: &serde_json::Value, port: usize) -> PadSample {
    use melee_ft::input::{Buttons, Stick};
    let Some(inputs) = record.get("inputs") else {
        return PadSample::default();
    };
    let pad = &inputs[format!("p{port}")];
    let float = |name: &str| {
        f32::from_bits(pad[name]["v"]["bits"].as_u64().expect("pad float bits") as u32)
    };
    PadSample {
        buttons: Buttons(pad["button"]["v"].as_u64().expect("pad buttons") as u32),
        stick: Stick {
            x: float("nml_stickX"),
            y: float("nml_stickY"),
        },
        cstick: Stick {
            x: float("nml_subStickX"),
            y: float("nml_subStickY"),
        },
        left_trigger: float("nml_analogL"),
        right_trigger: float("nml_analogR"),
    }
}

fn compare_movement_internals(fighter: &FoxFighter, bytes: &[u8], tick: usize, player: usize) {
    use melee_ft::fighter::MotionData;
    use melee_types::CommonMotionState as S;
    let check_float = |name: &str, actual: f32, offset| {
        assert_eq!(
            actual.to_bits(),
            word(bytes, offset),
            "{name} tick {tick} p{player}"
        )
    };
    check_float("animation speed", fighter.animation.speed, 0x89C);
    check_float("animation remainder", fighter.animation.remainder, 0x898);
    check_float("ground velocity", fighter.physics.ground_velocity, 0xEC);
    check_float("command timer", fighter.commands.timer, 0x3E4);
    check_float("command frame", fighter.commands.frame, 0x3E8);
    assert_eq!(
        fighter.status.name_tag_timer,
        u16::from_be_bytes([bytes[0x209A], bytes[0x209B]]),
        "nametag tick {tick} p{player}"
    );
    assert_eq!(
        fighter.effect_state.destroy_on_state_change,
        bytes[0x2219] & 0x80 != 0,
        "effect destruction flag tick {tick}"
    );
    assert_eq!(
        fighter.effect_state.rotating_bone_index,
        bytes[0x2220] >> 5,
        "effect bone cursor tick {tick}"
    );
    for (i, value) in fighter.commands.variables.iter().enumerate() {
        assert_eq!(
            *value,
            word(bytes, 0x2200 + i * 4),
            "command variable {i} tick {tick}"
        );
    }
    assert_eq!(
        fighter.physics.fast_fall,
        bytes[0x221A] & 8 != 0,
        "fast fall tick {tick} p{player}"
    );
    assert_eq!(
        fighter.input.vertical.tilt, bytes[0x671],
        "vertical input age tick {tick} p{player}"
    );
    assert_eq!(
        fighter.status.ledge_cooldown as u32,
        word(bytes, 0x2064),
        "ledge cooldown tick {tick}"
    );
    assert_eq!(
        fighter.status.ledge_intangibility as u32,
        word(bytes, 0x1990),
        "ledge intangibility tick {tick}"
    );
    assert_eq!(
        fighter.status.on_ledge,
        bytes[0x221D] & 1 != 0,
        "ledge flag tick {tick}"
    );
    assert_eq!(
        fighter.status.grab_exclusions.0,
        u16::from_be_bytes([bytes[0x1A6A], bytes[0x1A6B]]),
        "ledge option grab exclusion tick {tick}"
    );
    assert_eq!(
        fighter.status.ignore_fighter_nudge,
        bytes[0x221D] & 4 != 0,
        "fighter nudge exclusion tick {tick}"
    );
    let hurt = match fighter.commands.hurt_status {
        melee_ft::fighter::escape::HurtStatus::Normal => 0,
        melee_ft::fighter::escape::HurtStatus::Invincible => 1,
        melee_ft::fighter::escape::HurtStatus::Intangible => 2,
    };
    assert_eq!(
        hurt,
        word(bytes, 0x1988),
        "subaction hurt status tick {tick}"
    );
    assert_eq!(
        if fighter.status.ledge_intangibility > 0 {
            2
        } else {
            0
        },
        word(bytes, 0x198C),
        "timed hurt status tick {tick}"
    );
    if matches!(
        fighter.motion_state.id,
        S::CliffClimbQuick | S::CliffEscapeQuick
    ) {
        let translation = &fighter
            .animation
            .root_motion
            .as_ref()
            .unwrap()
            .primary_history;
        for (offset, actual) in [(0x68C, translation.position), (0x6A4, translation.offset)] {
            for (axis, value) in [actual.x, actual.y, actual.z].into_iter().enumerate() {
                check_float("ledge TransN", value, offset + axis * 4);
            }
        }
        assert_eq!(
            fighter.collision.data.floor.index as u32,
            word(bytes, 0x83C),
            "ledge floor tick {tick}"
        );
    }
    match (&fighter.state_data, fighter.motion_state.id) {
        (
            MotionData::Guard(guard),
            S::GuardOn | S::Guard | S::GuardOff | S::GuardReflect | S::GuardSetOff,
        ) => {
            check_float("shield health", fighter.status.shield_health, 0x1998);
            check_float("lightshield", fighter.shield.lightshield, 0x199C);
            check_float("guard elapsed", guard.elapsed, 0x2340);
            check_float("guard tilt magnitude", guard.tilt_magnitude, 0x2344);
            check_float("guard tilt frame", guard.tilt_frame, 0x2348);
            check_float("guard minimum hold", guard.minimum_hold, 0x2350);
            check_float("guard reflect frames", guard.reflect_frames, 0x2354);
            check_float("guard powershield frames", guard.powershield_frames, 0x2358);
            check_float(
                "guard previous lightshield",
                guard.previous_lightshield,
                0x236C,
            );
            assert_eq!(
                u32::from(guard.released),
                word(bytes, 0x234C),
                "release tick {tick}"
            );
            assert_eq!(guard.interrupt_frames as u32, word(bytes, 0x235C));
            assert_eq!(guard.jump_delay as u32, word(bytes, 0x2360));
            assert_eq!(guard.grab_delay as u32, word(bytes, 0x2364));
            for (actual, offset, mask) in [
                (fighter.shield.enabled, 0x221A, 1),
                (fighter.shield.active, 0x221B, 0x80),
                (fighter.shield.reflecting, 0x2218, 0x10),
                (fighter.shield.fresh_powershield, 0x221C, 0x10),
                (fighter.shield.reflect_window, 0x221C, 0x40),
                (fighter.shield.powershield_window, 0x221C, 0x20),
            ] {
                assert_eq!(
                    actual,
                    bytes[offset] & mask != 0,
                    "shield flag {offset:x}/{mask:x}, tick {tick}"
                );
            }
        }
        (MotionData::Escape(escape), S::EscapeF | S::EscapeB | S::EscapeN) => {
            if fighter.motion_state.id != S::EscapeN {
                assert_eq!(
                    escape.interrupt_frames as u32,
                    word(bytes, 0x2340),
                    "escape interrupt tick {tick}"
                );
            }
            let hurt = match fighter.commands.hurt_status {
                melee_ft::fighter::escape::HurtStatus::Normal => 0,
                melee_ft::fighter::escape::HurtStatus::Invincible => 1,
                melee_ft::fighter::escape::HurtStatus::Intangible => 2,
            };
            assert_eq!(hurt, word(bytes, 0x1988), "hurt status tick {tick}");
        }
        (MotionData::EscapeAir(dodge), S::EscapeAir) => {
            assert_eq!(
                dodge.item_throw_frames as u32,
                word(bytes, 0x2340),
                "air dodge timer tick {tick}"
            );
            for (axis, velocity) in [
                dodge.saved_velocity.x,
                dodge.saved_velocity.y,
                dodge.saved_velocity.z,
            ]
            .into_iter()
            .enumerate()
            {
                check_float("saved air momentum", velocity, 0x2344 + 4 * axis);
            }
        }
        (
            MotionData::Cliff(cliff),
            S::CliffCatch
            | S::CliffWait
            | S::CliffJumpQuick1
            | S::CliffJumpSlow1
            | S::CliffClimbQuick
            | S::CliffEscapeQuick,
        ) => {
            assert_eq!(
                cliff.ledge_id as u32,
                word(bytes, 0x2340),
                "ledge id tick {tick}"
            );
            if fighter.motion_state.id != S::CliffCatch {
                check_float("ledge wait timer", cliff.wait_frames, 0x2344);
                assert_eq!(
                    u32::from(cliff.neutral_seen),
                    word(bytes, 0x2348),
                    "ledge neutral latch tick {tick}"
                );
            }
        }
        (MotionData::CliffJump(jump), S::CliffJumpQuick2 | S::CliffJumpSlow2) => {
            assert_eq!(
                u32::from(jump.physics_started),
                word(bytes, 0x2340),
                "ledge launch latch tick {tick}"
            );
            check_float("retained ledge timer", jump.retained_wait_frames, 0x2344);
        }
        (
            MotionData::Landing {
                allow_interrupt,
                retained_drop_timer,
            },
            S::Landing | S::LandingFallSpecial,
        ) => {
            assert_eq!(
                u32::from(*allow_interrupt),
                word(bytes, 0x2340),
                "landing interruption tick {tick}"
            );
            check_float("retained landing word", *retained_drop_timer, 0x2344);
        }
        (MotionData::KneeBend(squat), S::KneeBend) => {
            assert_eq!(
                u32::from(squat.short_hop),
                word(bytes, 0x2340),
                "short hop tick {tick}"
            );
            let input = match squat.input {
                melee_ft::fighter::jump::JumpInput::Stick => 1,
                melee_ft::fighter::jump::JumpInput::Buttons => 3,
            };
            assert_eq!(input, word(bytes, 0x2344), "jump input tick {tick}");
        }
        (MotionData::Jump(jump), S::JumpF | S::JumpB) => {
            assert_eq!(u32::from(jump.short_hop), word(bytes, 0x2340));
            assert_eq!(u32::from(jump.physics_started), word(bytes, 0x2344));
            check_float("jump multiplier", jump.multiplier, 0x2348);
        }
        (MotionData::Dash(dash), S::Dash) => {
            check_float(
                "initial dash acceleration",
                dash.initial_acceleration,
                0x2340,
            );
            assert_eq!(
                u32::from(dash.early_interrupts),
                word(bytes, 0x2344),
                "dash interrupts tick {tick}"
            );
        }
        (MotionData::Run(run), S::Run) => {
            check_float("run interrupt delay", run.interrupt_delay, 0x2340);
            check_float(
                "run animation velocity",
                run.slippery_animation_velocity,
                0x2344,
            );
        }
        (MotionData::RunBrake(brake), S::RunBrake) => {
            assert_eq!(
                u32::from(brake.animation_paused),
                word(bytes, 0x2340),
                "brake paused tick {tick}"
            );
            check_float("brake frames", brake.remaining_frames, 0x2344);
        }
        (MotionData::TurnRun(turn), S::TurnRun) => {
            check_float("turn-run entry facing", turn.entry_facing, 0x234C);
            assert_eq!(
                u32::from(turn.animation_paused),
                word(bytes, 0x2354),
                "turn-run pause latch tick {tick}"
            );
        }
        (MotionData::Turn(turn), S::Turn) => {
            assert_eq!(
                u32::from(turn.has_turned),
                word(bytes, 0x2340),
                "turned tick {tick} p{player}"
            );
            assert_eq!(
                u32::from(turn.just_turned),
                word(bytes, 0x2358),
                "just turned tick {tick} p{player}"
            );
            assert_eq!(
                turn.buffered_buttons.0,
                word(bytes, 0x235C),
                "turn buffers tick {tick} p{player}"
            );
            check_float("destination facing", turn.facing_after, 0x2344);
            check_float("dash direction", turn.dash_direction, 0x2348);
            check_float("turn timer", turn.frames_to_turn, 0x2350);
        }
        (MotionData::Walk(walk), S::WalkSlow | S::WalkMiddle | S::WalkFast) => {
            check_float(
                "slippery animation velocity",
                walk.slippery_animation_velocity,
                0x2340,
            );
            check_float(
                "walk acceleration multiplier",
                walk.acceleration_multiplier,
                0x2360,
            );
        }
        (MotionData::Squat(squat), S::Squat | S::SquatWait) => {
            assert_eq!(
                u32::from(squat.platform_drop_pending),
                word(bytes, 0x2340),
                "drop pending tick {tick} p{player}"
            );
            if squat.platform_drop_pending {
                check_float("drop timer", squat.platform_drop_timer, 0x2344);
            }
        }
        _ => {}
    }
}
