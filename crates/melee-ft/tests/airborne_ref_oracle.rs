use melee_ft::{
    desc::attributes::AirAttributes,
    physics::airborne::{drift, gravity},
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn extract(source: &str, name: &str) -> String {
    let start = source.find(&format!("void {name}(")).unwrap();
    let mut end = source[start..].find('{').unwrap() + start + 1;
    let mut depth = 1;
    while depth != 0 {
        match source.as_bytes()[end] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        end += 1;
    }
    format!("{}\n", &source[start..end])
}
#[test]
fn airborne_excerpts_match_decomp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let decomp = root.join("../../third_party/melee-decomp/src/melee/ft/ftcommon.c");
    if !decomp.exists() {
        eprintln!("skipping excerpt check: submodule absent");
        return;
    }
    let source = fs::read_to_string(decomp).unwrap();
    for name in [
        "ftCommon_ApplyFrictionAir",
        "ftCommon_8007D174",
        "ftCommon_8007D28C",
        "ftCommon_Fall",
    ] {
        assert_eq!(
            fs::read_to_string(root.join(format!("tests/ref/airborne/{name}.c.inc"))).unwrap(),
            extract(&source, name)
        );
    }
}
#[test]
fn airborne_integration_matches_native_c_100k() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("airborne-oracle");
    fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("driver");
    let build = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .args(["-std=c99", "-O0", "-ffp-contract=off", "-fno-builtin"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ref/airborne/driver.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut seed = 0xC61B0u32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    let mut inputs = Vec::new();
    for case in 0..100_000 {
        let mut values = [0.0; 10];
        for v in &mut values {
            *v = (random() % 100_000) as f32 / 8192.0;
        }
        values[0] -= 6.0;
        values[1] -= 6.0;
        values[2] = match case % 5 {
            0 => 0.0,
            1 => -0.0,
            _ => (random() % 161) as f32 / 80.0 - 1.0,
        };
        if case % 17 == 0 {
            values[0] = if case % 2 == 0 { 0.0 } else { -0.0 };
        }
        if case % 19 == 0 {
            values[0] = values[8];
        } // exact friction boundary
        if case % 23 == 0 {
            values[1] = -values[4];
        } // terminal speed boundary
        inputs.push(values);
    }
    let input_path = directory.join("inputs.bin");
    fs::write(
        &input_path,
        inputs
            .iter()
            .flatten()
            .flat_map(|x| x.to_ne_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let output = Command::new(executable)
        .stdin(Stdio::from(fs::File::open(input_path).unwrap()))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout.len(), inputs.len() * 12);
    for (case, v) in inputs.iter().enumerate() {
        let attrs = AirAttributes {
            gravity: v[3],
            terminal_velocity: v[4],
            air_drift_stick_mul: v[5],
            aerial_drift_base: v[6],
            air_drift_max: v[7],
            aerial_friction: v[8],
            air_max_horizontal_velocity: v[9],
            fast_fall_velocity: 0.0,
        };
        let acceleration = drift(v[0], v[2], &attrs);
        let actual = [
            gravity(v[1], v[3], v[4]).to_bits(),
            acceleration.to_bits(),
            (v[0] + acceleration).to_bits(),
        ];
        let expected = output.stdout[case * 12..case * 12 + 12]
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(
            actual.as_slice(),
            expected,
            "case {case}: inputs {v:?}; bits {actual:08x?}"
        );
    }
    eprintln!("100000 airborne gravity/terminal/drift inputs: zero bit mismatches");
}
