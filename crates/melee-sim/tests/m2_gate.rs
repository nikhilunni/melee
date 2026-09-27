//! Yoshi's Story VI matrix oracle. A capture ordinal is not animation time:
//! align to complete simulated fighter poses using frame/position bits.
use std::{collections::BTreeMap, path::Path};

use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario};
use melee_test_support::{rendered_pose::PoseTimeline, M2_CAPTURE_COMMAND};
use serde_json::Value;

const CAPTURE_FRAMES: usize = 130;
// At least 100 distinct scheduler ticks must contribute clean matrix words.
// Repeated VI observations and all-dirty poses do not count toward this bound.
const MIN_COVERED_TICKS: usize = 100;
const BONES: usize = 73;

fn json_lines(path: &Path) -> Vec<Value> {
    melee_test_support::trace::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn fox_wait1_bones_match_the_real_game_bit_for_bit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join("harness/scenarios/idle_ys_fox.toml")).unwrap();
    let oracle_path = root.join("harness/traces/fox_ys.bones.expected.jsonl");
    let meta_path = oracle_path.with_extension("jsonl.meta.jsonl");
    if !melee_test_support::require_files(
        [&oracle_path, &meta_path]
            .into_iter()
            .chain(scenario.required_files().iter()),
    ) {
        return;
    }
    let metadata = json_lines(&meta_path);
    assert_eq!(metadata.len(), CAPTURE_FRAMES,
        "M2 requires {CAPTURE_FRAMES} VI frames; Claude must record from main with: {M2_CAPTURE_COMMAND}");
    let mut frames: BTreeMap<usize, BTreeMap<String, u32>> = BTreeMap::new();
    for row in json_lines(&oracle_path) {
        let frame = row["frame"].as_u64().unwrap() as usize;
        assert!(frame < CAPTURE_FRAMES);
        for (key, value) in row["state"].as_object().unwrap() {
            assert!(
                frames
                    .entry(frame)
                    .or_default()
                    .insert(
                        key.clone(),
                        value["v"]["bits"].as_u64().unwrap().try_into().unwrap(),
                    )
                    .is_none(),
                "duplicate oracle word {frame} {key}"
            );
        }
    }
    assert_eq!(frames.len(), CAPTURE_FRAMES);

    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    let mut poses = PoseTimeline::default();
    for tick in 0..scenario.frames as usize {
        simulation.tick_without_snapshot().unwrap();
        let pose = simulation.rendered_fighter_pose(0);
        assert_eq!(pose.matrices.len(), BONES);
        poses.insert(tick, pose.key, pose.matrices);
    }
    let (mut aligned, mut unmatched, mut words, mut mismatches) = (0, 0, 0, 0);
    let mut first_mismatch = None;
    for (frame, meta) in metadata.iter().enumerate() {
        assert_eq!(meta["frame"].as_u64(), Some(frame as u64));
        assert_eq!(meta["joints"].as_array().unwrap().len(), BONES);
        let expected = &frames[&frame];
        assert_eq!(
            expected.len(),
            BONES * 22,
            "complete input capture at VI {frame}"
        );
        let key = [
            meta["cur_anim_frame"].as_u64().unwrap() as u32,
            meta["cur_pos"][0].as_u64().unwrap() as u32,
            meta["cur_pos"][1].as_u64().unwrap() as u32,
            meta["cur_pos"][2].as_u64().unwrap() as u32,
        ];
        let Some(pose) = poses.align(&key) else {
            unmatched += 1;
            eprintln!("M2 VI {frame}: no completed tick for key {key:08x?}");
            continue;
        };
        aligned += 1;
        let tick = pose.tick;
        let mut dirty = [false; BONES];
        for bone in meta["dirty_bones"].as_array().unwrap() {
            let bone = bone.as_u64().unwrap() as usize;
            assert!(bone < BONES && !dirty[bone], "invalid/duplicate dirty bone");
            dirty[bone] = true;
        }
        let mut frame_words = 0;
        for (bone, matrix) in pose.matrices.iter().enumerate() {
            if dirty[bone] {
                continue;
            }
            for (component, actual) in matrix.iter().enumerate() {
                let expected = expected[&format!("p0.bone[{bone}].mtx[{component}]")];
                frame_words += 1;
                if *actual != expected {
                    mismatches += 1;
                    first_mismatch.get_or_insert_with(|| format!(
                        "VI {frame}, tick {tick}, bone {bone}, mtx[{component}]: expected {expected:08x}, actual {actual:08x}"
                    ));
                }
            }
        }
        words += frame_words;
        poses.record_coverage(tick, frame_words);
    }
    eprintln!("M2: {aligned}/{CAPTURE_FRAMES} aligned VI frames, {unmatched} unmatched, {} distinct covered ticks, {words} clean matrix words, {mismatches} mismatches", poses.covered_ticks());
    assert!(poses.covered_ticks() >= MIN_COVERED_TICKS,
        "M2 requires {MIN_COVERED_TICKS} distinct aligned ticks with non-dirty matrix coverage; record with: {M2_CAPTURE_COMMAND}");
    assert_eq!(mismatches, 0, "{}", first_mismatch.unwrap_or_default());
}
