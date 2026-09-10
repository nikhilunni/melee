//! Numerical solver regression from the first endpoint correction in the S2
//! fairlc Marth replay. Input and native-C expected bits have explicit provenance.
use hsd_types::Vec3;
use melee_lb::ik::TwoJointIk;
use std::{path::Path, process::Command};

#[test]
fn fairlc_marth_tick_142_solver_matches_native_retail_sequence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/data/fairlc_marth_142_ik.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(fixture["source"]["scenario"], "fairlc_fd_marth");
    assert_eq!(fixture["source"]["tick"], 142);
    let words: Vec<u32> = fixture["input_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    assert_eq!(words.len(), 20);
    let inputs: Vec<f32> = words.iter().copied().map(f32::from_bits).collect();
    let point = |index| Vec3::new(inputs[index], inputs[index + 1], inputs[index + 2]);
    let mut solver = TwoJointIk {
        hip: point(0),
        knee: point(3),
        foot: point(6),
        extended_foot: point(9),
        target: point(12),
        upper_length: inputs[15],
        lower_length: inputs[16],
    };
    let expected: Vec<u32> = fixture["expected_angle_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("s2-leg-ik");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("native-reference");
    let build = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .args([
            "-std=c99",
            "-O0",
            "-ffp-contract=off",
            "-fno-builtin",
            "-fno-strict-aliasing",
            "-Wno-incompatible-library-redeclaration",
            "-I",
        ])
        .arg(root.join("tests/ref/lbtrigf/shim"))
        .arg("-I")
        .arg(root.join("tests/ref/lbtrigf"))
        .arg(root.join("tests/ref/ik.c"))
        .arg("-lm")
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("native C compiler required for the IK oracle");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let native = Command::new(executable)
        .args(words.iter().map(|v| format!("{v:08x}")))
        .output()
        .unwrap();
    assert!(native.status.success());
    let native_bits: Vec<u32> = String::from_utf8(native.stdout)
        .unwrap()
        .split_whitespace()
        .map(|v| u32::from_str_radix(v, 16).unwrap())
        .collect();
    assert_eq!(
        native_bits, expected,
        "fixture agrees with the independent C transcription"
    );
    assert_eq!(
        solver.angles(point(17)).map(f32::to_bits).as_slice(),
        expected
    );
}
