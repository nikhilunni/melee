//! 50k one-bone force/constraint steps against independent decomp C excerpts.
use hsd_anim::{jobj::JObjTree, quat::Quaternion};
use hsd_types::Vec3;
use melee_lb::dynamics::{arithmetic::normalize, spring_direction, BoneSpring, SpringParameters};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn dynamics_one_bone_50000() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("dynamics_ref");
    fs::create_dir_all(&output).unwrap();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    if Command::new(&cc).arg("--version").output().is_err() {
        eprintln!("[NON-DATA OMITTED] omitting: no C compiler");
        return;
    }
    let reference = root.join("tests/ref");
    let msl = root.join("../gekko-math/tests/ref/msl");
    let executable = output.join("driver");
    let trig_object = output.join("msl_trig.o");
    let trig_build = Command::new(&cc)
        .args([
            "-std=c99",
            "-O0",
            "-ffp-contract=off",
            "-fno-builtin",
            "-fno-strict-aliasing",
            "-fwrapv",
        ])
        .arg("-I")
        .arg(&msl)
        .arg("-c")
        .arg(msl.join("retail/trigf.c"))
        .arg("-o")
        .arg(&trig_object)
        .output()
        .unwrap();
    assert!(
        trig_build.status.success(),
        "{}",
        String::from_utf8_lossy(&trig_build.stderr)
    );
    let build = Command::new(cc)
        .args([
            "-std=c99",
            "-O0",
            "-ffp-contract=off",
            "-fno-builtin",
            "-fno-strict-aliasing",
            "-fwrapv",
            "-Wno-incompatible-library-redeclaration",
        ])
        .arg("-I")
        .arg(reference.join("lbtrigf/shim"))
        .arg("-I")
        .arg(reference.join("lbtrigf"))
        .arg(reference.join("dynamics/driver.c"))
        .arg(trig_object)
        .arg(msl.join("math_data.c"))
        .arg(msl.join("math_1.c"))
        .arg("-lm")
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut seed = 0xA126_2D09_7427_771Bu64;
    let mut random = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 40) as f32) / 16777216.0
    };
    let mut tree = JObjTree::new();
    let joint = tree.alloc();
    let mut input = Vec::new();
    let mut actual = Vec::new();
    for i in 0..50_000 {
        let mut values = [0.0f32; 20];
        values[0] = random() * 2.0;
        values[1] = random() * 2.0;
        values[2] = if i % 5 == 0 { 0.0 } else { random() * 0.1 };
        values[3] = random() * std::f32::consts::PI;
        values[4] = random() * 0.15;
        values[5] = if i % 7 == 0 { 0.0 } else { random() * 0.1 };
        values[6] = if i % 11 == 0 {
            0.0
        } else {
            (random() - 0.5) * 0.3
        };
        values[7] = 0.1 + random() * 10.0;
        for start in [8, 11, 14, 17] {
            let v = normalize(Vec3::new(random() - 0.5, random() - 0.5, random() - 0.5));
            values[start..start + 3].copy_from_slice(&[v.x, v.y, v.z]);
        }
        let vector = |start| Vec3::new(values[start], values[start + 1], values[start + 2]);
        let bone = BoneSpring {
            joint,
            rest_rotation: Quaternion::default(),
            rest_translate: Vec3::ZERO,
            rest_scale: Vec3::ZERO,
            position: Vec3::ZERO,
            velocity_axis: vector(17),
            angular_velocity: values[6],
            length: values[7],
            dominant_axis: 0,
            gravity: values[5],
            parameters: SpringParameters {
                stiffness: values[1],
                convergence: values[2],
                natural_rotation: Quaternion::default(),
                max_deviation: values[3],
                rotation_max: Vec3::ZERO,
                rotation_min: Vec3::ZERO,
                damping: 0.0,
                max_step: values[4],
            },
        };
        let result = spring_direction(
            &bone,
            values[0],
            vector(8),
            vector(11),
            vector(14),
            &[],
            false,
            Vec3::ZERO,
        );
        actual.extend([result.x, result.y, result.z]);
        for value in values {
            input.extend(value.to_ne_bytes());
        }
    }
    let input_path = output.join("input.bin");
    let expected_path = output.join("output.bin");
    fs::write(&input_path, input).unwrap();
    assert!(Command::new(executable)
        .arg(input_path)
        .arg(&expected_path)
        .status()
        .unwrap()
        .success());
    let expected = fs::read(expected_path).unwrap();
    assert_eq!(expected.len(), actual.len() * 4);
    for (i, (bytes, actual)) in expected.chunks_exact(4).zip(actual).enumerate() {
        let expected = u32::from_ne_bytes(bytes.try_into().unwrap());
        assert_eq!(
            actual.to_bits(),
            expected,
            "step {} axis {}: expected {expected:08X}, actual {:08X}",
            i / 3,
            i % 3,
            actual.to_bits()
        );
    }
    eprintln!("50,000 native-C bone steps, 150,000 components: bit-exact");
}

#[test]
fn dynamics_c_excerpts_match_decomp() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let reference = root.join("tests/ref/dynamics");
    let lb = root.join("../../third_party/melee-decomp/src/melee/lb");
    let source = fs::read_to_string(lb.join("lb_00F9.c")).unwrap();
    assert!(source.contains(&fs::read_to_string(reference.join("spring_step.c.inc")).unwrap()));
    let source = fs::read_to_string(lb.join("lbvector.c")).unwrap();
    let saved = fs::read_to_string(reference.join("lbvector.c.inc")).unwrap();
    for name in [
        "lbVector_Normalize",
        "lbVector_Angle",
        "lbvector_sin",
        "lbvector_cos",
        "lbVector_RotateAboutUnitAxis",
    ] {
        let start = source.find(&format!("{name}(")).unwrap();
        let start = source[..start].rfind('\n').unwrap() + 1;
        let open = start + source[start..].find('{').unwrap();
        let mut depth = 1;
        let mut end = open + 1;
        while depth != 0 {
            match source.as_bytes()[end] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            end += 1;
        }
        assert!(
            saved.contains(&source[start..end]),
            "changed C excerpt {name}"
        );
    }
}
