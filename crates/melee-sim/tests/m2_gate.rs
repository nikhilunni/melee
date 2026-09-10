//! Milestone 2 gate: Fox's bone matrices from the real game vs `hsd-anim`.
//!
//! The oracle trace `harness/traces/fox_ys.bones.expected.jsonl` is captured
//! from Dolphin at the `idle_ys_fox` savestate (docs/M2_GATE.md). Both it and
//! the disc are machine-local and required unless explicitly opted out.
//! First passed 2026-09-08: 0 mismatches on every bone the game had recomputed
//! that frame.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use hsd_types::Vec3;
use melee_diff::{read_trace, Record, Value};
use melee_sim::bones::{default_assets, write_fox_wait1_bones, FighterPose};

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
    if !melee_test_support::require_files(
        [
            oracle_path.clone(),
            oracle_path.with_extension("jsonl.meta.jsonl"),
        ]
        .into_iter()
        .chain(["PlFxNr.dat", "PlFx.dat", "PlFxAJ.dat"].map(|name| default_assets().join(name))),
    ) {
        return;
    }
    let oracle = by_key(read_trace(&std::fs::read(&oracle_path).unwrap()[..]).unwrap());

    // Use the metadata captured alongside this exact bone trace. Savestate
    // identity alone is insufficient after a recording is replaced.
    let metadata: Vec<serde_json::Value> =
        std::fs::read_to_string(oracle_path.with_extension("jsonl.meta.jsonl"))
            .expect("bone trace requires its capture metadata")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert!(!metadata.is_empty());
    let float = |value: &serde_json::Value| f32::from_bits(value.as_u64().unwrap() as u32);
    let first = &metadata[0];
    let archive =
        hsd_archive::Archive::parse(&std::fs::read(default_assets().join("PlFx.dat")).unwrap())
            .unwrap();
    let data = archive.public("ftDataFox").unwrap();
    let attributes = melee_ft::desc::read_fighter_attributes(&archive, data).unwrap();
    let bones = melee_ft::desc::read_fighter_bones(&archive, data, 5).unwrap();
    let model_scale = attributes.size.model_scaling;
    let pose = FighterPose {
        position: Some(Vec3::new(
            float(&first["cur_pos"][0]),
            float(&first["cur_pos"][1]),
            float(&first["cur_pos"][2]),
        )),
        facing_dir: Some(float(&first["facing_dir"])),
        model_scale: Some(model_scale),
        bone_scales: bones
            .scaled_joint
            .map(|joint| (joint as usize, 1.0 / model_scale))
            .into_iter()
            .collect(),
    };
    let mut ours = Vec::new();
    write_fox_wait1_bones(
        &default_assets(),
        float(&first["cur_anim_frame"]),
        metadata.len() as u64,
        &pose,
        &mut ours,
    )
    .unwrap();
    let ours = by_key(read_trace(&ours[..]).unwrap());

    let dirty: Vec<HashSet<usize>> = metadata
        .iter()
        .map(|row| {
            row["dirty_bones"]
                .as_array()
                .unwrap()
                .iter()
                .map(|bone| bone.as_u64().unwrap() as usize)
                .collect()
        })
        .collect();
    let expected_words: usize = metadata
        .iter()
        .zip(&dirty)
        .map(|(row, dirty)| (row["joints"].as_array().unwrap().len() - dirty.len()) * 22)
        .sum();
    assert_eq!(ours.len(), oracle.len(), "complete bone-word coverage");
    let mut mismatches = Vec::new();
    let mut compared = 0;
    for (key, expected) in &oracle {
        if dirty[key.0 as usize].contains(&bone_index(&key.1)) {
            continue;
        }
        let actual = ours.get(key).unwrap_or_else(|| panic!("missing {key:?}"));
        compared += 1;
        if actual != expected {
            mismatches.push(format!("{key:?}: oracle {expected:?} vs ours {actual:?}"));
        }
    }
    assert_eq!(
        compared, expected_words,
        "all non-dirty bones in each capture x 22 words"
    );
    assert!(
        mismatches.is_empty(),
        "{} bone words differ from the real game:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
