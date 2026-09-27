//! The retail gameplay-camera dump (`record.py --camera`,
//! harness/dolphin_camera_tick_snippet.py): decoding and the isolated oracle
//! that replays the dumped subjects through the ported camera.
use anyhow::{ensure, Context, Result};
use melee_cm::{GameCamera, StageCamera, Subject, Transform};
use std::path::Path;

fn bytes(hex: &str) -> Result<Vec<u8>> {
    ensure!(hex.len().is_multiple_of(2), "odd hex length");
    hex.as_bytes()
        .chunks_exact(2)
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}

/// One dumped tick: `game_camera` and the subject list in list order.
#[derive(Clone, Debug)]
pub struct CameraSample {
    pub frame: u64,
    pub camera: GameCamera,
    pub subjects: Vec<Subject>,
    /// ifMagnify's per-player `is_offscreen`.
    pub magnified: [bool; 6],
}

/// One dump row's `state`: struct Camera, the CmSubject list and ifMagnify.
pub fn decode_state(state: &serde_json::Value) -> Result<(GameCamera, Vec<Subject>, [bool; 6])> {
    use melee_lib::diagnostics::{decode_camera, decode_subject, magnified};
    let camera = decode_camera(&bytes(state["camera"].as_str().context("camera bytes")?)?)?;
    let subjects = state["subjects"]
        .as_array()
        .context("subjects")?
        .iter()
        .map(|hex| decode_subject(&bytes(hex.as_str().context("subject bytes")?)?))
        .collect::<Result<_>>()?;
    let magnify = bytes(state["magnify"].as_str().context("magnify bytes")?)?;
    Ok((
        camera,
        subjects,
        std::array::from_fn(|slot| magnified(&magnify, slot)),
    ))
}

pub fn read_dump(path: &Path) -> Result<Vec<CameraSample>> {
    melee_trace_io::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?
        .lines()
        .map(|line| {
            let row: serde_json::Value = serde_json::from_str(line)?;
            let (camera, subjects, magnified) = decode_state(&row["state"])?;
            Ok(CameraSample {
                frame: row["frame"].as_u64().context("frame")?,
                camera,
                subjects,
                magnified,
            })
        })
        .collect()
}

/// Every differing word between two camera states, as `field: actual vs
/// expected` lines.
pub fn camera_mismatches(
    actual: &GameCamera,
    actual_subjects: &[Subject],
    expected: &GameCamera,
    expected_subjects: &[Subject],
) -> Vec<String> {
    let mut found = Vec::new();
    let mut compare = |name: String, a: f32, e: f32| {
        if a.to_bits() != e.to_bits() {
            found.push(format!(
                "{name}: actual {a} ({:08X}), expected {e} ({:08X})",
                a.to_bits(),
                e.to_bits()
            ));
        }
    };
    let mut transform = |prefix: &str, a: &Transform, e: &Transform| {
        for (name, av, ev) in [
            ("interest", a.interest, e.interest),
            ("target_interest", a.target_interest, e.target_interest),
            ("position", a.position, e.position),
            ("target_position", a.target_position, e.target_position),
        ] {
            compare(format!("{prefix}.{name}.x"), av.x, ev.x);
            compare(format!("{prefix}.{name}.y"), av.y, ev.y);
            compare(format!("{prefix}.{name}.z"), av.z, ev.z);
        }
        compare(format!("{prefix}.fov"), a.fov, e.fov);
        compare(format!("{prefix}.target_fov"), a.target_fov, e.target_fov);
    };
    transform("transform", &actual.transform, &expected.transform);
    transform(
        "transform_copy",
        &actual.transform_copy,
        &expected.transform_copy,
    );
    compare(
        "translation.x".into(),
        actual.translation.x,
        expected.translation.x,
    );
    compare(
        "translation.y".into(),
        actual.translation.y,
        expected.translation.y,
    );
    compare(
        "zoom_distance".into(),
        actual.zoom_distance,
        expected.zoom_distance,
    );
    compare(
        "bounds_width_average".into(),
        actual.bounds_width_average,
        expected.bounds_width_average,
    );
    compare(
        "bounds_width_sum".into(),
        actual.bounds_width_sum,
        expected.bounds_width_sum,
    );
    for (i, (a, e)) in actual_subjects.iter().zip(expected_subjects).enumerate() {
        let (a, e) = (a.extents, e.extents);
        compare(format!("subject[{i}].ext.left"), a.left, e.left);
        compare(format!("subject[{i}].ext.right"), a.right, e.right);
        compare(format!("subject[{i}].ext.top"), a.top, e.top);
        compare(format!("subject[{i}].ext.bottom"), a.bottom, e.bottom);
        compare(format!("subject[{i}].ext.radius"), a.radius, e.radius);
    }
    if actual.bounds_width_samples != expected.bounds_width_samples {
        found.push(format!(
            "bounds_width_samples: actual {}, expected {}",
            actual.bounds_width_samples, expected.bounds_width_samples
        ));
    }
    found
}

/// Replay each dumped tick in isolation: start from sample N, take every
/// subject's owner-written fields from sample `N + 1` (fighters update their
/// subjects before the camera proc) and compare the camera with sample N + 1.
pub fn isolated_diff(samples: &[CameraSample], stage: &StageCamera, limit: usize) -> Vec<String> {
    let mut report = Vec::new();
    for pair in samples.windows(2) {
        let (before, after) = (&pair[0], &pair[1]);
        if before.subjects.len() != after.subjects.len() {
            report.push(format!("frame {}: subject count changed", after.frame));
            continue;
        }
        let mut camera = before.camera.clone();
        let mut subjects: Vec<Subject> = before
            .subjects
            .iter()
            .zip(&after.subjects)
            .map(|(old, new)| Subject {
                extents: old.extents,
                was_framed: old.was_framed,
                state_timer: old.state_timer,
                ..new.clone()
            })
            .collect();
        let mut refs: Vec<&mut Subject> = subjects.iter_mut().collect();
        camera.update_standard(&mut refs, stage);
        for line in camera_mismatches(&camera, &subjects, &after.camera, &after.subjects) {
            report.push(format!("frame {}: {line}", after.frame));
            if report.len() >= limit {
                return report;
            }
        }
    }
    report
}

/// `melee-sim camera-diff`: the isolated oracle over `<scenario>.camera.jsonl`.
pub fn camera_diff(
    scenario: &crate::scenario::Scenario,
    limit: usize,
) -> Result<(usize, Vec<String>)> {
    let samples = read_dump(&scenario.trace_path("camera.jsonl"))?;
    let stage = crate::trace::simulation(scenario)?.stage_camera();
    Ok((samples.len(), isolated_diff(&samples, &stage, limit)))
}
