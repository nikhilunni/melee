mod input_support;
use input_support::{common, root};
use melee_ft::input::{crosses_stick_circle, human::apply_deadzone, pad::normalize_stick, Stick};
use std::{path::Path, process::Command};

fn excerpt(source: &str, signature: &str) -> String {
    let start = source.find(signature).unwrap();
    let end = start + source[start..].find("\n}").unwrap() + 2;
    format!("{}\n", &source[start..end])
}
#[test]
fn oracle_excerpts_match_decomp() {
    let root = root();
    let controller = root.join("third_party/melee-decomp/src/sysdolphin/baselib/controller.c");
    if !controller.exists() {
        eprintln!("skipping excerpt check: decomp absent");
        return;
    }
    let controller = std::fs::read_to_string(controller).unwrap();
    let circle =
        std::fs::read_to_string(root.join("third_party/melee-decomp/src/melee/lb/lb_00CE.c"))
            .unwrap();
    for (file, source, signature) in [
        ("clamp.c", &controller, "static void HSD_PadClampCheck3"),
        ("scale.c", &controller, "static void HSD_PadScale"),
        ("circle.c", &circle, "s32 lb_8000D148"),
    ] {
        assert_eq!(
            std::fs::read_to_string(root.join("crates/melee-ft/tests/ref/input").join(file))
                .unwrap(),
            excerpt(source, signature),
            "{file}"
        );
    }
}

#[test]
fn analog_scaling_65536_matches_native_c() {
    let root = root();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fighter_input_oracle");
    std::fs::create_dir_all(&dir).unwrap();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let exe = dir.join("input-oracle");
    let build = Command::new(cc)
        .args([
            "-std=c99",
            "-O0",
            "-ffp-contract=off",
            "-fno-builtin",
            "-fno-strict-aliasing",
            "-I",
        ])
        .arg(root.join("crates/gekko-math/tests/ref"))
        .arg(root.join("crates/melee-ft/tests/ref/input/driver.c"))
        .arg("-lm")
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("native C compiler required for input oracle");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let threshold = common().thresholds.horizontal_stick_deadzone;
    let mut bytes = threshold.to_ne_bytes().to_vec();
    let mut paths = Vec::new();
    let mut seed = 123456789_u32;
    let mut next = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    // 65k independently varied old/new stick paths exercise all fused circle sites.
    for _ in 0..65_536 {
        let point = [next(), next(), next(), next()].map(|n| (n as i32 as f32) / i32::MAX as f32);
        let path = [point[0], point[1], point[2], point[3], 0.25];
        for v in path {
            bytes.extend(v.to_ne_bytes());
        }
        paths.push(path);
    }
    for path in [
        [0.0_f32, 0.0, 0.0, 0.0, 0.25],
        [0.25, 0.0, 0.5, 0.0, 0.25],
        [-0.25, 0.0, 0.25, 0.0, 0.25],
        [0.0, 0.0, 0.25, 0.0, 0.25],
        [0.5, 0.25, -0.5, 0.25, 0.25],
        [0.5, 0.25000003, -0.5, 0.25000003, 0.25],
    ] {
        for v in path {
            bytes.extend(v.to_ne_bytes());
        }
        paths.push(path);
    }
    let input = dir.join("in.bin");
    let output = dir.join("out.bin");
    std::fs::write(&input, bytes).unwrap();
    assert!(Command::new(exe)
        .arg(input)
        .arg(&output)
        .status()
        .unwrap()
        .success());
    let output = std::fs::read(output).unwrap();
    assert_eq!(output.len(), 65_536 * 16 + paths.len() * 4);
    for (index, pair) in (-128..=127)
        .flat_map(|x| (-128..=127).map(move |y| (x as i8, y as i8)))
        .enumerate()
    {
        let stick = normalize_stick(pair.0, pair.1);
        let values = [
            stick.x,
            stick.y,
            apply_deadzone(stick.x, threshold),
            apply_deadzone(stick.y, threshold),
        ];
        for (component, value) in values.into_iter().enumerate() {
            let at = index * 16 + component * 4;
            let expected = u32::from_ne_bytes(output[at..at + 4].try_into().unwrap());
            assert_eq!(
                value.to_bits(),
                expected,
                "raw {pair:?}, component {component}"
            );
        }
    }
    for (index, path) in paths.iter().enumerate() {
        let at = 65_536 * 16 + index * 4;
        let expected = i32::from_ne_bytes(output[at..at + 4].try_into().unwrap()) != 0;
        assert_eq!(
            crosses_stick_circle(
                Stick {
                    x: path[0],
                    y: path[1]
                },
                Stick {
                    x: path[2],
                    y: path[3]
                },
                path[4]
            ),
            expected,
            "circle path {path:?}"
        );
    }
}

#[test]
fn native_wait_predicate_order_and_short_circuit() {
    use melee_ft::input::{WaitPredicate as P, WAIT_PREDICATES};
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("wait_iasa_oracle");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("wait-oracle");
    let build = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .args(["-std=c99", "-O0", "-ffp-contract=off"])
        .arg(root().join("crates/melee-ft/tests/ref/input/wait_driver.c"))
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("native C compiler");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = Command::new(exe).output().unwrap();
    assert!(output.status.success());
    let identity = [
        P::SpecialSide,
        P::SpecialUp,
        P::SpecialNeutral,
        P::SpecialDown,
        P::Grab,
        P::SmashSide,
        P::SmashUp,
        P::SmashDown,
        P::TiltSide,
        P::TiltUp,
        P::TiltDown,
        P::Jab,
        P::Escape,
        P::Shield,
        P::FoxTaunt,
        P::Taunt,
        P::Jump,
        P::Dash,
        P::Squat,
        P::Turn,
        P::Walk,
    ];
    let rows: Vec<_> = output
        .stdout
        .split(|b| *b == 255)
        .filter(|row| !row.is_empty())
        .collect();
    assert_eq!(rows.len(), 1 + 21 * 21);
    let decode = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|b| identity[usize::from(*b)])
            .collect::<Vec<_>>()
    };
    assert_eq!(decode(rows[0]), WAIT_PREDICATES);
    for a in 0..21 {
        for b in 0..21 {
            let stop = WAIT_PREDICATES
                .iter()
                .position(|p| *p == identity[a] || *p == identity[b])
                .unwrap();
            assert_eq!(decode(rows[1 + a * 21 + b]), WAIT_PREDICATES[..=stop]);
        }
    }
    let source_path =
        root().join("third_party/melee-decomp/src/melee/ft/kinds/ftCommon/ftCo_Wait.c");
    if source_path.exists() {
        let source = std::fs::read_to_string(source_path).unwrap();
        assert_eq!(
            std::fs::read_to_string(root().join("crates/melee-ft/tests/ref/input/wait.c")).unwrap(),
            excerpt(&source, "void ftCo_Wait_IASA")
        );
    }
}
