use std::process::Command;

use melee_diff::{first_divergence, read_trace, Record, Value};
use melee_sim::bones::{default_assets, write_fox_wait1_bones, FighterPose};

const RECORDS_PER_FRAME: usize = 73 * (12 + 10);

fn disc_present() -> bool {
    let assets = default_assets();
    melee_test_support::require_files(
        ["PlFxNr.dat", "PlFx.dat", "PlFxAJ.dat"].map(|name| assets.join(name)),
    )
}

fn check_frame(records: &[Record], frame: u64) {
    assert_eq!(records.len(), RECORDS_PER_FRAME);
    let mut index = 0;
    for bone in 0..73 {
        for (field, count) in [("mtx", 12), ("rotate", 4), ("scale", 3), ("translate", 3)] {
            for component in 0..count {
                let record = &records[index];
                assert_eq!(record.frame, frame);
                assert_eq!(record.phase, "bones");
                assert_eq!(record.state.len(), 1);
                let value = &record.state[&format!("p0.bone[{bone}].{field}[{component}]")];
                assert!(
                    matches!(value, Value::F32 { bits, .. } if f32::from_bits(*bits).is_finite())
                );
                index += 1;
            }
        }
    }
}

#[test]
fn real_fox_frames_zero_and_one_round_trip_and_advance() {
    if !disc_present() {
        return;
    }
    let mut jsonl = Vec::new();
    write_fox_wait1_bones(
        &default_assets(),
        0.0,
        2,
        &FighterPose::default(),
        &mut jsonl,
    )
    .unwrap();
    let records = read_trace(jsonl.as_slice()).unwrap();
    assert_eq!(records.len(), 2 * RECORDS_PER_FRAME);
    for (frame, chunk) in records.chunks_exact(RECORDS_PER_FRAME).enumerate() {
        check_frame(chunk, frame as u64);
    }
    assert!(
        records[..RECORDS_PER_FRAME]
            .iter()
            .zip(&records[RECORDS_PER_FRAME..])
            .any(|(zero, one)| {
                zero.state.keys().any(|key| key.contains(".mtx[")) && zero.state != one.state
            }),
        "animation must update world matrices, not just SRT"
    );
    // The identity root must have been set up; an uninitialized zero matrix
    // would still satisfy the count/finite checks above.
    assert_eq!(records[0].state["p0.bone[0].mtx[0]"], Value::f32(1.0));
    // A fresh request of frame 1 must equal normal playback's second sample.
    let mut requested = Vec::new();
    write_fox_wait1_bones(
        &default_assets(),
        1.0,
        1,
        &FighterPose::default(),
        &mut requested,
    )
    .unwrap();
    let mut requested = read_trace(requested.as_slice()).unwrap();
    check_frame(&requested, 0);
    for record in &mut requested {
        record.frame = 1;
    }
    assert_eq!(
        first_divergence(&records[RECORDS_PER_FRAME..], &requested),
        None
    );
    let roundtrip = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap() + "\n")
        .collect::<String>();
    assert_eq!(read_trace(roundtrip.as_bytes()).unwrap(), records);
}

#[test]
fn real_fox_cli_matches_library_from_another_working_directory() {
    if !disc_present() {
        return;
    }
    let result = Command::new(env!("CARGO_BIN_EXE_melee-sim"))
        .current_dir(std::env::temp_dir())
        .args([
            "bones",
            "--fighter",
            "fox",
            "--anim",
            "Wait1",
            "--frame",
            "0",
            "--frames",
            "2",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut expected = Vec::new();
    write_fox_wait1_bones(
        &default_assets(),
        0.0,
        2,
        &FighterPose::default(),
        &mut expected,
    )
    .unwrap();
    assert_eq!(
        read_trace(result.stdout.as_slice()).unwrap(),
        read_trace(expected.as_slice()).unwrap()
    );
}

#[test]
fn invalid_frame_and_count_fail_before_loading_assets() {
    for (frame, count) in [(f32::NAN, 1), (f32::INFINITY, 1), (-1.0, 1), (0.0, 0)] {
        assert!(write_fox_wait1_bones(
            std::path::Path::new("absent"),
            frame,
            count,
            &FighterPose::default(),
            Vec::new()
        )
        .is_err());
    }
}
