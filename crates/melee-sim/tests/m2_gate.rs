//! Milestone 2 gate: Fox's bone matrices from the real game vs `hsd-anim`.
//!
//! The oracle trace `harness/traces/fox_ys.bones.expected.jsonl` is captured
//! from Dolphin at the `idle_ys_fox` savestate (docs/M2_GATE.md). Both it and
//! the disc are machine-local, so this test skips with a note when either is
//! missing. First passed 2026-09-08: 0 mismatches on every bone the game had
//! recomputed that frame.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use hsd_types::Vec3;
use melee_diff::{read_trace, Record, Value};
use melee_sim::bones::{default_assets, write_fox_wait1_bones, FighterPose};

/// Where the savestate put Fox: Yoshi's Story side platform, facing right.
/// Values are what `harness/traces/idle_ys_fox.expected.jsonl` records at
/// frame 0 (`p0.cur_pos`, `p0.facing_dir`, `p0.cur_anim_frame`).
const FOX_POSITION: Vec3 = Vec3 {
    x: -42.0,
    y: 23.450098,
    z: 0.0,
};
const FOX_FACING: f32 = 1.0;
const FOX_MODEL_SCALE: f32 = 0.96;
const FOX_ANIM_FRAME: f32 = 6.0;
/// Retail scales this bone by 1/model_scale. TODO(meaning): find the site.
const INVERSE_SCALED_BONE: (usize, f32) = (67, 1.0416667);
/// Bones whose cached matrix the game had not recomputed at the savestate
/// (`dirty_bones` in the oracle's `.meta.jsonl`); their oracle values are stale.
const DIRTY_BONES: [usize; 3] = [67, 71, 72];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bone_index(key: &str) -> usize {
    let start = key.find('[').unwrap() + 1;
    let end = key.find(']').unwrap();
    key[start..end].parse().unwrap()
}

fn by_key(records: Vec<Record>) -> HashMap<(u64, String), Value> {
    records
        .into_iter()
        .flat_map(|r| {
            let frame = r.frame;
            r.state.into_iter().map(move |(k, v)| ((frame, k), v))
        })
        .collect()
}

#[test]
fn fox_wait1_bones_match_the_real_game_bit_for_bit() {
    let oracle_path = repo().join("harness/traces/fox_ys.bones.expected.jsonl");
    if !oracle_path.exists() || !default_assets().is_dir() {
        eprintln!("skipping: oracle trace or disc missing (see docs/M2_GATE.md)");
        return;
    }
    let oracle = by_key(read_trace(&std::fs::read(&oracle_path).unwrap()[..]).unwrap());

    let pose = FighterPose {
        position: Some(FOX_POSITION),
        facing_dir: Some(FOX_FACING),
        model_scale: Some(FOX_MODEL_SCALE),
        bone_scales: vec![INVERSE_SCALED_BONE],
    };
    let mut ours = Vec::new();
    write_fox_wait1_bones(&default_assets(), FOX_ANIM_FRAME, 2, &pose, &mut ours).unwrap();
    let ours = by_key(read_trace(&ours[..]).unwrap());

    let dirty: HashSet<usize> = DIRTY_BONES.into_iter().collect();
    let mut mismatches = Vec::new();
    let mut compared = 0;
    for (key, expected) in &oracle {
        if dirty.contains(&bone_index(&key.1)) {
            continue;
        }
        let actual = ours.get(key).unwrap_or_else(|| panic!("missing {key:?}"));
        compared += 1;
        if actual != expected {
            mismatches.push(format!("{key:?}: oracle {expected:?} vs ours {actual:?}"));
        }
    }
    assert_eq!(
        compared,
        2 * 70 * 22,
        "two frames of 70 live bones x 22 words"
    );
    assert!(
        mismatches.is_empty(),
        "{} bone words differ from the real game:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
