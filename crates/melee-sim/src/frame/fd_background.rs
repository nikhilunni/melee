//! Independent recorded joint/AppSRT checks across FD's layered and tilt phases.
use super::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

#[test]
fn human_match_fd_transition_attachments() {
    let scenario = crate::scenario::Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/scenarios/match2_fd_foxmarth.toml"),
    )
    .unwrap();
    let path = scenario.trace_path("particles.jsonl.meta.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([path.clone()]),
    ) {
        return;
    }
    let frames = [
        1886, 2000, 4000, 6000, 7095, 7096, 7200, 8500, 8900, 8901, 9000,
    ];
    let captured: BTreeMap<usize, serde_json::Value> = BufReader::new(File::open(path).unwrap())
        .lines()
        .enumerate()
        .filter(|(frame, _)| frames.contains(frame))
        .map(|(frame, line)| (frame, serde_json::from_str(&line.unwrap()).unwrap()))
        .collect();
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        crate::trace::pad_script(&scenario).unwrap(),
    );
    let mut matrices = 0;
    let mut transforms = 0;
    for frame in 0..=9000 {
        simulation.tick_without_snapshot().unwrap();
        let Some(meta) = captured.get(&frame).map(|m| &m["particles"]) else {
            continue;
        };
        let actual = &simulation.runtime.state.particles;
        let generators: Vec<_> = actual.generators.iter().filter(|g| g.bank == 30).collect();
        let expected: Vec<_> = meta["generators"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|g| g["fields"]["bank"] == 30)
            .collect();
        assert_eq!(
            generators.len(),
            expected.len(),
            "stage generators tick {frame}"
        );
        for (generator, expected) in generators.into_iter().zip(expected) {
            let fields = &expected["fields"];
            if fields["jobj"] != 0 {
                let joint = meta["joints"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|j| j["pointer"] == fields["jobj"])
                    .unwrap();
                let expected: Vec<_> = joint["fields"]["matrix"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect();
                let actual: Vec<_> = generator
                    .joint_matrix
                    .unwrap()
                    .0
                    .iter()
                    .flatten()
                    .map(|v| v.to_bits())
                    .collect();
                assert_eq!(actual, expected, "stage joint tick {frame}");
                matrices += 1;
            }
            if fields["appsrt"] != 0 {
                let srt = &meta["appsrts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["pointer"] == fields["appsrt"])
                    .unwrap()["fields"];
                let actual = generator.application_transform.as_ref().unwrap();
                for (key, value) in [
                    ("translation", actual.translation),
                    ("rotation", actual.rotation),
                    ("scale", actual.scale),
                ] {
                    let expected: Vec<_> = srt[key]
                        .as_array()
                        .unwrap()
                        .iter()
                        .take(3)
                        .map(|v| v.as_u64().unwrap() as u32)
                        .collect();
                    assert_eq!(
                        [value.x.to_bits(), value.y.to_bits(), value.z.to_bits()].as_slice(),
                        expected,
                        "stage AppSRT {key} tick {frame}"
                    );
                }
                assert_eq!(
                    u64::from(actual.camera_facing),
                    srt["unknown_byte"].as_u64().unwrap(),
                    "stage AppSRT camera flag tick {frame}"
                );
                transforms += 1;
            }
        }
    }
    assert!(matrices > 0 && transforms > 0);
    eprintln!(
        "FD transitions: {matrices} attached matrices and {transforms} AppSRT transforms exact"
    );
}
