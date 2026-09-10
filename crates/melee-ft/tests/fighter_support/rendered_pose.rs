//! Post-render matrix oracle. Alignment uses only animation-frame/position
//! bits, never a matrix value or an assumed VI-to-tick index offset.
use super::{harness, json_lines, replay};
use std::collections::BTreeMap;

type PoseKey = [u32; 4];
struct TickPose {
    tick: usize,
    matrices: Vec<[u32; 12]>,
}

pub fn compare_start() {
    let paths = [0, 1]
        .map(|player| harness().join(format!("traces/start_fd_fox.bones_vi_p{player}.jsonl")));
    let meta_paths = paths
        .each_ref()
        .map(|path| path.with_extension("jsonl.meta.jsonl"));
    if !melee_test_support::require_files(paths.iter().chain(&meta_paths)) {
        return;
    }
    let mut poses: [BTreeMap<PoseKey, Vec<TickPose>>; 2] = Default::default();
    replay::replay_with_observer(
        "start",
        130,
        replay::BoneOracle::Rendered,
        |tick, fighters| {
            for (player, fighter) in fighters.iter().enumerate() {
                let position = fighter.physics.position;
                let key =
                    [fighter.animation.frame, position.x, position.y, position.z].map(f32::to_bits);
                // HSD_JObjGetMtxPtr (jobj.h:697-701) demands SetupMatrix.
                // Use a clone: display demand must not change scheduler state.
                let mut tree = fighter.skeleton.clone();
                let matrices = fighter
                    .animation
                    .parts
                    .iter()
                    .map(|part| {
                        tree.setup_matrix(part.joint);
                        let matrix = &tree.get(part.joint).mtx.0;
                        std::array::from_fn(|index| matrix[index / 4][index % 4].to_bits())
                    })
                    .collect();
                poses[player]
                    .entry(key)
                    .or_default()
                    .push(TickPose { tick, matrices });
            }
        },
    );
    if poses.iter().all(BTreeMap::is_empty) {
        return; // Replay already reported missing disc/trace/savestate.
    }
    let mut first = None;
    for player in 0..2 {
        let metadata = json_lines(&meta_paths[player]);
        assert_eq!(metadata.len(), 130);
        // M2 dumps contain one word per JSON line; both files use p0 keys.
        let mut frames: BTreeMap<usize, BTreeMap<String, u32>> = BTreeMap::new();
        for row in json_lines(&paths[player]) {
            let frame = row["frame"].as_u64().unwrap() as usize;
            for (key, value) in row["state"].as_object().unwrap() {
                assert!(frames
                    .entry(frame)
                    .or_default()
                    .insert(key.clone(), value["v"]["bits"].as_u64().unwrap() as u32)
                    .is_none());
            }
        }
        assert_eq!(frames.len(), 130);
        let (mut aligned, mut matched, mut skipped, mut empty, mut words) = (0, 0, 0, 0, 0);
        let mut last_tick = 0;
        for (frame, meta) in metadata.iter().enumerate() {
            assert_eq!(meta["frame"].as_u64().unwrap() as usize, frame);
            let expected = &frames[&frame];
            assert_eq!(expected.len(), 73 * 22);
            let key = [
                meta["cur_anim_frame"].as_u64().unwrap() as u32,
                meta["cur_pos"][0].as_u64().unwrap() as u32,
                meta["cur_pos"][1].as_u64().unwrap() as u32,
                meta["cur_pos"][2].as_u64().unwrap() as u32,
            ];
            let Some(pose) = poses[player]
                .get(&key)
                .and_then(|candidates| candidates.iter().find(|pose| pose.tick >= last_tick))
            else {
                skipped += 1;
                eprintln!("VI p{player} frame {frame}: no completed tick for key {key:08X?}");
                continue;
            };
            last_tick = pose.tick;
            aligned += 1;
            let mut dirty = [false; 73];
            for bone in meta["dirty_bones"].as_array().unwrap() {
                dirty[bone.as_u64().unwrap() as usize] = true;
            }
            let count = dirty.iter().filter(|&&value| !value).count() * 12;
            words += count;
            empty += usize::from(count == 0);
            let mut differs = false;
            // Earliest chronological match, allowing repeated VI samples of
            // one tick. Never match a later motion's identical frame/position
            // when an earlier completed tick is available.
            for (bone, matrix) in pose.matrices.iter().enumerate() {
                if dirty[bone] {
                    continue;
                }
                for (index, &actual) in matrix.iter().enumerate() {
                    let bits = expected[&format!("p0.bone[{bone}].mtx[{index}]")];
                    if actual != bits {
                        differs = true;
                        first.get_or_insert_with(|| format!("first VI matrix mismatch: VI {frame}, tick {}, p{player}, bone {bone}, mtx[{index}]: expected {bits:08X}, actual {actual:08X}, dynamics={}", pose.tick, (17..=20).contains(&bone)));
                    }
                }
            }
            matched += usize::from(!differs);
        }
        eprintln!("VI p{player}: {aligned}/130 aligned, {matched} matched, {skipped} skipped, {empty} all-dirty frames, {words} matrix words compared");
        assert!(aligned > empty, "no non-dirty aligned matrix coverage");
    }
    assert!(first.is_none(), "{}", first.unwrap_or_default());
}
