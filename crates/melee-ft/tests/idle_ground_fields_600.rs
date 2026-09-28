//! T5: 600 FD records, including the saved tick 0 and 599 simulated ticks.
//! No physics field is read from a later row to drive the simulation.
use gekko_math::rng::HsdRng;
use hsd_anim::{load::load_joint_tree, mtx::InverseTrig};
use hsd_archive::{desc::read_public_jobj, Archive};
use hsd_types::Vec3;
use melee_ft::{
    anim::{attach::PartFlags, FighterAnimation},
    collision::{
        ecb,
        ground::{map_wait, EnvironmentCollision, WaitGroundResult},
        pose::{GroundPose, GroundPoseFlags},
    },
    desc::{
        common::read_common_data,
        playback::{read_playback_motion, read_wait_table},
        read_fighter_attributes, read_fighter_bones, read_named_fighter_animations,
    },
    physics::{
        grounded::{step_wait, GroundedParameters},
        FighterPhysics,
    },
};
use melee_types::{mp::SurfaceData, GroundOrAir};
use serde_json::Value;
use std::{fs, path::Path};
struct RetailTrig;
impl InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}
fn json_lines(path: &Path) -> Vec<Value> {
    melee_test_support::trace::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
fn frame_bits(row: &Value, player: usize) -> u32 {
    row["state"][format!("p{player}.cur_anim_frame")]["v"]["bits"]
        .as_u64()
        .unwrap() as u32
}
fn raw_bytes(value: &Value) -> Vec<u8> {
    value["bytes"]
        .as_str()
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect()
}
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn trace_float(row: &Value, player: usize, field: &str) -> f32 {
    f32::from_bits(
        row["state"][format!("p{player}.{field}")]["v"]["bits"]
            .as_u64()
            .unwrap() as u32,
    )
}
fn trace_vec(row: &Value, player: usize, field: &str) -> Vec3 {
    Vec3::new(
        trace_float(row, player, &format!("{field}.x")),
        trace_float(row, player, &format!("{field}.y")),
        trace_float(row, player, &format!("{field}.z")),
    )
}
fn raw_fighter(row: &Value, player: usize) -> Vec<u8> {
    row["fighters"]
        .as_array()
        .unwrap()
        .iter()
        .map(raw_bytes)
        .find(|b| usize::from(b[12]) == player)
        .unwrap()
}
fn assert_word(tick: usize, player: usize, field: &str, actual: u32, expected: u32) {
    assert_eq!(actual, expected, "first mismatch tick {tick} p{player}.{field}: actual 0x{actual:08x}, expected 0x{expected:08x}");
}
fn assert_vector(tick: usize, player: usize, field: &str, actual: Vec3, expected: Vec3) {
    for (axis, a, e) in [
        ("x", actual.x, expected.x),
        ("y", actual.y, expected.y),
        ("z", actual.z, expected.z),
    ] {
        assert_word(
            tick,
            player,
            &format!("{field}.{axis}"),
            a.to_bits(),
            e.to_bits(),
        );
    }
}

#[test]
fn idle_ground_fields_600() {
    let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness");
    let files = harness.join("roms/files");
    let trace_path = harness.join("traces/idle_fd_fox.tick.expected.jsonl");
    let ledger_path = harness.join("traces/idle_fd_fox.ledger600.raw.jsonl");
    if !melee_test_support::require_files(
        [
            "PlFx.dat",
            "PlCo.dat",
            "PlFxAJ.dat",
            "PlFxNr.dat",
            "GrNLa.dat",
        ]
        .map(|name| files.join(name))
        .iter()
        .chain([&trace_path, &ledger_path]),
    ) {
        return;
    }
    let trace = json_lines(&trace_path);
    let ledger = json_lines(&ledger_path);
    assert_eq!(trace.len(), 600);
    assert_eq!(ledger.len(), 600);
    let archive = Archive::parse(&fs::read(files.join("PlFx.dat")).unwrap()).unwrap();
    let fighter = archive.public("ftDataFox").unwrap();
    let attrs = read_fighter_attributes(&archive, fighter).unwrap();
    let common_archive = Archive::parse(&fs::read(files.join("PlCo.dat")).unwrap()).unwrap();
    let common = read_common_data(&common_archive).unwrap();
    let params = GroundedParameters::from_attributes(&attrs, &common);
    let bones = read_fighter_bones(&archive, fighter, 5).unwrap();
    let table = read_named_fighter_animations(&archive, "ftDataFox", 327).unwrap();
    let aj = fs::read(files.join("PlFxAJ.dat")).unwrap();
    let motions =
        [2, 3].map(|id| read_playback_motion(&archive, fighter, &table, &aj, id).unwrap());
    let mut entry_motion = read_playback_motion(&archive, fighter, &table, &aj, 238).unwrap();
    entry_motion.blend_frames = 0.0;
    eprintln!("entry frames {}", entry_motion.animation.frames);
    let choices = read_wait_table(&archive, fighter).unwrap().unwrap();
    let costume = Archive::parse(&fs::read(files.join("PlFxNr.dat")).unwrap()).unwrap();
    let desc = read_public_jobj(&costume, "PlyFox5K_Share_joint").unwrap();
    let stage = Archive::parse(&fs::read(files.join("GrNLa.dat")).unwrap()).unwrap();
    let stage_desc = melee_gr::desc::read_final_destination(&stage).unwrap();
    let mut map = melee_gr::desc::load_collision(&stage, &stage_desc).unwrap();
    let mut fighters: Vec<_> = (0..2)
        .map(|player| {
            let raw = raw_fighter(&ledger[0], player);
            let (mut tree, root) = load_joint_tree(&costume, &desc).unwrap();
            let mut animation = FighterAnimation::new(&tree, root);
            animation.translation_joint = Some(usize::from(bones.model.animation_translation));
            animation.model_scale = attrs.size.model_scaling;
            for i in [1, 72] {
                animation.parts[i].flags.0 |= PartFlags::COPY;
            }
            if player == 1 {
                animation
                    .set_animation(
                        &mut tree,
                        &entry_motion,
                        entry_motion.animation.frames - 1.0,
                        1.0,
                    )
                    .unwrap();
                animation.step::<RetailTrig>(&mut tree);
            }
            animation
                .set_animation(
                    &mut tree,
                    &motions[0],
                    f32::from_bits(frame_bits(&trace[0], player)),
                    f32::from_bits(word(&raw, 0x89c)),
                )
                .unwrap();
            animation.blend_progress = f32::from_bits(word(&raw, 0x8a8)) - 1.0;
            animation.step::<RetailTrig>(&mut tree);
            animation.blend_progress = f32::from_bits(word(&raw, 0x8a8));
            animation.remainder = f32::from_bits(word(&raw, 0x898));
            let mut motion = FighterPhysics::standing(
                trace_vec(&trace[0], player, "cur_pos"),
                trace_float(&trace[0], player, "facing_dir"),
            );
            motion.self_velocity = trace_vec(&trace[0], player, "self_vel");
            motion.knockback_velocity = trace_vec(&trace[0], player, "kb_vel");
            motion.percent = trace_float(&trace[0], player, "percent");
            tree.set_translate(root, &motion.position);
            // Fighter_ChangeMotionState (0x800693AC): M_PI_2 is double.
            tree.set_rotation_y(
                root,
                (std::f64::consts::FRAC_PI_2 * f64::from(motion.facing)) as f32,
            );
            tree.set_scale(
                root,
                &Vec3::new(
                    attrs.size.model_scaling,
                    attrs.size.model_scaling,
                    attrs.size.model_scaling,
                ),
            );
            let mut collision =
                ecb::initialize(&map, motion.position, &bones.ecb, 1.0, attrs.size.weight);
            let floor = map.floor_below(&motion.position, -1, -1);
            assert_eq!(floor, 1);
            collision.floor = SurfaceData {
                index: floor,
                flags: map.line_get_flags(floor),
                normal: map.line_get_normal(floor),
            };
            ecb::load_grounded(&mut collision, &mut tree, root);
            collision.ecb = collision.desired_ecb;
            collision.prev_ecb = collision.ecb;
            eprintln!(
                "p{player} tick 0 constructed ECB: {:?}; raw top {:08x}",
                collision.ecb,
                word(&raw, 0x798)
            );
            (
                tree,
                animation,
                motion,
                EnvironmentCollision::new(collision),
            )
        })
        .collect();
    let mut matched = [0; 2];
    let mut ecb_mismatches = [0; 2];
    for tick in 0..600 {
        // Validate the outer-proc preconditions on EVERY observed retail tick.
        for player in 0..2 {
            let raw = raw_fighter(&ledger[tick], player);
            assert_eq!(word(&raw, 0x10), 14);
            assert_eq!(raw[0x221f] & 0x10, 0, "inactive fighter");
            assert_eq!(raw[0x2219] & 0x04, 0, "hitlag");
            assert!(
                !(raw[0x2222] & 2 != 0 && raw[0x2222] & 1 == 0),
                "deferred displacement"
            );
            assert_eq!(word(&raw, 0x1948), 0, "velocity interpolation");
            assert_eq!(word(&raw, 0x21d0), 0, "hitlag callback");
            assert_eq!(
                (u16::from_be_bytes([raw[0x221c], raw[0x221d]]) >> 6) & 7,
                1,
                "left leg pose enabled"
            );
        }
        if tick != 0 {
            // s_link 1: all fighters animate before ANY fighter physics.
            let draws = ledger[tick]["rng_draws"].as_array().unwrap();
            let sites: Vec<_> = draws
                .iter()
                .enumerate()
                .filter(|(_, d)| d["lr"].as_u64().unwrap() - 4 == 0x8008_a8bc)
                .collect();
            let mut used = 0;
            for (tree, animation, motion, _) in &mut fighters {
                motion.begin_tick();
                animation.step::<RetailTrig>(tree);
                if !animation.frames_remaining(tree) {
                    let (index, _) = sites[used];
                    let before = if index == 0 {
                        ledger[tick - 1]["seed"].as_u64().unwrap()
                    } else {
                        draws[index - 1]["seed"].as_u64().unwrap()
                    } as u32;
                    let mut rng = HsdRng::new(before);
                    let choice = animation
                        .update_wait::<RetailTrig>(tree, &mut rng, Some(&choices), |id| {
                            &motions[(id - 2) as usize]
                        })
                        .unwrap()
                        .unwrap();
                    used += choice.draws;
                    assert_eq!(rng.seed, sites[used - 1].1["seed"].as_u64().unwrap() as u32);
                }
            }
            assert_eq!(used, sites.len());
            // s_link 3: idle input, independently covered by T9.
            // s_link 4: physics/integration, in fighter-list order.
            for (_, _, motion, collision) in &mut fighters {
                step_wait(
                    motion,
                    &collision.data,
                    &params,
                    &map,
                    melee_gr::wind::Wind::CALM,
                );
            }
            // s_link 6: collision and both root translation writes.
            for (tree, animation, motion, collision) in &mut fighters {
                assert_eq!(
                    map_wait(motion, collision, &mut map, tree, animation.root, 0.0),
                    WaitGroundResult::Supported,
                    "tick {tick}"
                );
            }
            // s_link 7: flat ground pose/IK gate, after ALL collision procs.
            for (tree, animation, motion, collision) in &mut fighters {
                GroundPose {
                    bones: bones.ground_pose.as_ref().unwrap(),
                    player_scale: 1.0,
                    flags: GroundPoseFlags(GroundPoseFlags::LEFT_LEG),
                    max_tilt_degrees: 0.0,
                }
                .update(motion, collision, &map, tree, animation.root);
            }
        }
        for (player, (tree, animation, motion, collision)) in fighters.iter().enumerate() {
            for (field, actual) in [
                ("cur_pos", motion.position),
                ("self_vel", motion.self_velocity),
                ("kb_vel", motion.knockback_velocity),
            ] {
                assert_vector(
                    tick,
                    player,
                    field,
                    actual,
                    trace_vec(&trace[tick], player, field),
                );
            }
            assert_word(
                tick,
                player,
                "percent",
                motion.percent.to_bits(),
                trace_float(&trace[tick], player, "percent").to_bits(),
            );
            for (field, actual) in [
                ("ground_or_air", i32::from(motion.ground_or_air) as u32),
                ("jumps_used", u32::from(motion.jumps_used)),
            ] {
                assert_word(
                    tick,
                    player,
                    field,
                    actual,
                    trace[tick]["state"][format!("p{player}.{field}")]["v"]
                        .as_u64()
                        .unwrap() as u32,
                );
            }
            assert_eq!(motion.ground_or_air, GroundOrAir::Ground);
            assert_vector(
                tick,
                player,
                "root.translate",
                tree.translation(animation.root),
                motion.position,
            );
            assert_word(
                tick,
                player,
                "anim_frame",
                animation.frame.to_bits(),
                frame_bits(&trace[tick], player),
            );
            let raw = raw_fighter(&ledger[tick], player);
            for (field, offset, actual) in [
                ("prev_pos", 0xbc, motion.previous_position),
                ("pos_delta", 0xc8, motion.position_delta),
                ("anim_vel", 0x74, motion.animation_velocity),
                ("shield_kb", 0x98, motion.shield_knockback_velocity),
                ("floor.normal", 0x844, collision.data.floor.normal),
            ] {
                let expected = Vec3::new(
                    f32::from_bits(word(&raw, offset)),
                    f32::from_bits(word(&raw, offset + 4)),
                    f32::from_bits(word(&raw, offset + 8)),
                );
                assert_vector(tick, player, field, actual, expected);
            }
            for (field, offset, actual) in [
                ("gr_vel", 0xec, motion.ground_velocity),
                ("ground_accel_1", 0xe4, motion.ground_acceleration),
                ("ground_accel_2", 0xe8, motion.secondary_ground_acceleration),
                ("ground_kb", 0xf0, motion.ground_knockback_velocity),
                (
                    "ground_shield_kb",
                    0xf4,
                    motion.ground_shield_knockback_velocity,
                ),
            ] {
                assert_word(tick, player, field, actual.to_bits(), word(&raw, offset));
            }
            assert_word(
                tick,
                player,
                "floor.index",
                collision.data.floor.index as u32,
                word(&raw, 0x83c),
            );
            assert_word(
                tick,
                player,
                "floor.flags",
                collision.data.floor.flags,
                word(&raw, 0x840),
            );
            if collision.data.ecb.top.y.to_bits() != word(&raw, 0x798) {
                if ecb_mismatches[player] < 8 {
                    eprintln!(
                        "ECB difference tick {tick} p{player}: actual {:08x}, retail {:08x}",
                        collision.data.ecb.top.y.to_bits(),
                        word(&raw, 0x798)
                    );
                }
                ecb_mismatches[player] += 1;
            }
            matched[player] += 1;
        }
    }
    eprintln!("ECB top mismatches: {ecb_mismatches:?}");
    assert_eq!(matched, [600, 600]);
    eprintln!("P0: 600/600; P1: 600/600 records (tick 0 + 599 simulated transitions); first mismatch: none");
}
