//! Live FD frame-end trace replay. No copied disc/trace data is checked in.
use gekko_math::rng::HsdRng;
use hsd_anim::{load::load_joint_tree, mtx::InverseTrig};
use hsd_archive::{desc::read_public_jobj, Archive};
use melee_ft::anim::{attach::PartFlags, FighterAnimation};
use melee_ft::desc::{
    playback::{read_playback_motion, read_wait_table},
    read_fighter_bones, read_named_fighter_animations,
};
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
    fs::read_to_string(path)
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

#[test]
fn fox_wait_playback_600() {
    let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness");
    let files = harness.join("roms/files");
    if !files.is_dir() {
        eprintln!("skipping Fox playback: disc not extracted");
        return;
    }
    let trace_path = harness.join("traces/idle_fd_fox.tick.expected.jsonl");
    let ledger_path = harness.join("traces/idle_fd_fox.ledger600.raw.jsonl");
    if !trace_path.exists() || !ledger_path.exists() {
        eprintln!("skipping Fox playback: local FD trace/ledger absent");
        return;
    }
    let trace = json_lines(&trace_path);
    let ledger = json_lines(&ledger_path);
    assert_eq!(trace.len(), 600);
    assert_eq!(ledger.len(), 600);
    let archive = Archive::parse(&fs::read(files.join("PlFx.dat")).unwrap()).unwrap();
    let fighter = archive.public("ftDataFox").unwrap();
    let bones = read_fighter_bones(&archive, fighter, 5).unwrap();
    let table = read_named_fighter_animations(&archive, "ftDataFox", 327).unwrap();
    let aj = fs::read(files.join("PlFxAJ.dat")).unwrap();
    let motions =
        [2, 3].map(|id| read_playback_motion(&archive, fighter, &table, &aj, id).unwrap());
    let choices = read_wait_table(&archive, fighter).unwrap().unwrap();
    assert_eq!(
        choices
            .iter()
            .map(|e| (e.motion, e.weight))
            .collect::<Vec<_>>(),
        [(2, 70), (3, 30), (-1, -1)]
    );
    assert_eq!(motions[0].blend_frames, 6.0);
    assert_eq!(motions[1].blend_frames, 0.0);
    let costume = Archive::parse(&fs::read(files.join("PlFxNr.dat")).unwrap()).unwrap();
    let desc = read_public_jobj(&costume, "PlyFox5K_Share_joint").unwrap();
    let mut fighters: Vec<_> = (0..2)
        .map(|player| {
            let (mut tree, root) = load_joint_tree(&costume, &desc).unwrap();
            let mut animation = FighterAnimation::new(&tree, root);
            animation.translation_joint = Some(usize::from(bones.model.animation_translation));
            animation.model_scale = f32::from_bits(0x3f75_c28f);
            for i in [1, 72] {
                animation.parts[i].flags.0 |= PartFlags::COPY;
            }
            let initial = ledger[0]["fighters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| raw_bytes(f)[0xC] == player)
                .unwrap();
            let bytes = raw_bytes(initial);
            assert_eq!(word(&bytes, 0x14), 2); // hidden submotion, not action-state 14
            let start = f32::from_bits(frame_bits(&trace[0], usize::from(player)));
            animation
                .set_animation(
                    &mut tree,
                    &motions[0],
                    start,
                    f32::from_bits(word(&bytes, 0x89c)),
                )
                .unwrap();
            // Evaluate the saved pose once to consume FIRST_PLAY. The saved blend
            // progress is restored afterwards (6 for P0, 1 for P1).
            animation.step::<RetailTrig>(&mut tree);
            animation.blend_progress = f32::from_bits(word(&bytes, 0x8a8));
            animation.remainder = f32::from_bits(word(&bytes, 0x898));
            (tree, animation)
        })
        .collect();
    for (player, (_, animation)) in fighters.iter().enumerate() {
        assert_eq!(animation.frame.to_bits(), frame_bits(&trace[0], player));
    }
    let mut total_draws = 0;
    let mut matched = [0; 2];
    for tick in 0..600 {
        assert_eq!(trace[tick]["tick"], ledger[tick]["tick"]);
        let draws = ledger[tick]["rng_draws"].as_array().unwrap();
        let sites: Vec<_> = draws
            .iter()
            .enumerate()
            .filter(|(_, d)| d["lr"].as_u64().unwrap() - 4 == 0x8008_a8bc)
            .collect();
        let mut used = 0;
        for (player, (tree, animation)) in fighters.iter_mut().enumerate() {
            if tick != 0 {
                animation.step::<RetailTrig>(tree);
                if !animation.frames_remaining(tree) {
                    let (index, _) = sites[used];
                    let before = if index == 0 {
                        ledger[tick - 1]["seed"].as_u64().unwrap()
                    } else {
                        draws[index - 1]["seed"].as_u64().unwrap()
                    } as u32;
                    let mut rng = HsdRng::new(before);
                    let result = animation
                        .update_wait::<RetailTrig>(tree, &mut rng, Some(&choices), |id| {
                            &motions[(id - 2) as usize]
                        })
                        .unwrap()
                        .unwrap();
                    assert!(
                        used + result.draws <= sites.len(),
                        "too many choice draws at tick {tick}"
                    );
                    for pair in sites[used..used + result.draws].windows(2) {
                        assert_eq!(pair[1].0, pair[0].0 + 1, "retry draws must be contiguous");
                    }
                    let after = sites[used + result.draws - 1].1["seed"].as_u64().unwrap() as u32;
                    assert_eq!(
                        rng.seed, after,
                        "choice post-seed at tick {tick}, p{player}"
                    );
                    eprintln!("tick {tick} p{player}: motion {}, draws {}, seed {before:08x}->{after:08x}", result.motion, result.draws);
                    used += result.draws;
                    total_draws += result.draws;
                }
            }
            let expected = frame_bits(&trace[tick], player);
            assert_eq!(animation.frame.to_bits(), expected, "first mismatch tick {tick} p{player}: actual {} ({:08x}), expected {} ({expected:08x})", animation.frame, animation.frame.to_bits(), f32::from_bits(expected));
            // The ledger includes full Fighter bytes: also prove hidden id,
            // remainder, rate and blend fields at every frame-end boundary.
            let raw = ledger[tick]["fighters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| usize::from(raw_bytes(f)[0xc]) == player)
                .unwrap();
            let bytes = raw_bytes(raw);
            assert_eq!(
                animation.motion_id as u32,
                word(&bytes, 0x14),
                "motion tick {tick} p{player}"
            );
            for (offset, actual) in [
                (0x898, animation.remainder),
                (0x89c, animation.speed),
                (0x8a4, animation.blend_duration),
                (0x8a8, animation.blend_progress),
            ] {
                assert_eq!(
                    actual.to_bits(),
                    word(&bytes, offset),
                    "field {offset:x} tick {tick} p{player}"
                );
            }
            matched[player] += 1;
        }
        assert_eq!(used, sites.len(), "unconsumed Wait draws tick {tick}");
    }
    let expected_draws: usize = ledger
        .iter()
        .skip(1)
        .map(|row| {
            row["rng_draws"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|draw| draw["lr"].as_u64().unwrap() - 4 == 0x8008_a8bc)
                .count()
        })
        .sum();
    assert_eq!(total_draws, expected_draws);
    assert_eq!(matched, [600, 600]);
    eprintln!(
        "P0: 600/600; P1: 600/600 frame/id/remainder/rate/blend states; all {total_draws} Wait draws matched"
    );
}
