//! Independent recorded joint/AppSRT checks across FD's layered and tilt phases.
use super::*;
use std::{io::BufRead, path::Path};

#[test]
fn fd_unobserved_animation_clocks_and_late_matrix_reads_match_eager() {
    let scenario = crate::scenario::Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/start_fd_fox.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let initial = InitialState::from_savestate_traces(&scenario).unwrap();
    let mut eager = crate::scene_stage::last::load_animations(&initial.assets).unwrap();
    let mut lazy = crate::scene_stage::last::load_animations(&initial.assets).unwrap();
    let bits = |matrix: hsd_types::Mtx| matrix.0.map(|row| row.map(f32::to_bits));
    for frame in 0..600 {
        for (&map, eager) in &mut eager {
            let lazy = lazy.get_mut(&map).unwrap();
            let eager_requests = eager.tick::<RetailTrig>();
            let lazy_requests = lazy.tick::<RetailTrig>();
            assert_eq!(eager_requests.len(), lazy_requests.len());
            for (a, b) in eager_requests.iter().zip(lazy_requests) {
                assert_eq!((a.bank, a.kind, a.joint), (b.bank, b.kind, b.joint));
                assert_eq!(
                    bits(a.matrix),
                    bits(b.matrix),
                    "spawn map {map} tick {frame}"
                );
            }
            eager.for_each_matrix(|_, _| {});
            for joint in 0..eager.joint_count() {
                assert_eq!(
                    eager.joint_frame(joint).map(f32::to_bits),
                    lazy.joint_frame(joint).map(f32::to_bits),
                    "clock map {map} joint {joint} tick {frame}"
                );
                // A first consumer can arrive after hundreds of unobserved ticks,
                // disappear, and return without changing any matrix bits.
                if frame == 239 || frame == 599 {
                    assert_eq!(
                        bits(eager.joint_matrix(joint)),
                        bits(lazy.joint_matrix(joint)),
                        "late read map {map} joint {joint} tick {frame}"
                    );
                }
            }
        }
    }
}

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
    let captured: BTreeMap<usize, serde_json::Value> = melee_test_support::trace::open(&path)
        .unwrap()
        .lines()
        .enumerate()
        .filter(|(frame, _)| frames.contains(frame))
        .map(|(frame, line)| (frame, serde_json::from_str(&line.unwrap()).unwrap()))
        .collect();
    let mut simulation = super::TestSimulation::with_inputs(
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
