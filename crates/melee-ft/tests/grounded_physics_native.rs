//! Native-C oracle with explicit retail fusion and a contraction-disabled
//! control. Finite values include both zeros, cancellation and both signs.
use hsd_types::{Vec2, Vec3};
use melee_ft::physics::{
    friction::decay_knockback,
    grounded::{wait_physics, GroundedParameters},
    integrate::{integrate_environment, integrate_velocity, VelocityBlend},
    FighterPhysics,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[test]
fn ground_reference_sources_match_decomp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["ftcommon.c", "ft_084E.c"] {
        let decomp = root
            .join("../../third_party/melee-decomp/src/melee/ft")
            .join(name);
        if !decomp.exists() {
            eprintln!("submodule absent: skipping source provenance check");
            return;
        }
        let source = fs::read_to_string(decomp).unwrap();
        let excerpts =
            fs::read_to_string(root.join("tests/ref/physics").join(format!("{name}.inc"))).unwrap();
        for function in excerpts.trim_end().split("\n\nvoid ") {
            assert!(source.contains(function), "oracle excerpt changed: {name}");
        }
    }
}

fn build(name: &str, unfused: bool) -> PathBuf {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("grounded_physics_native");
    fs::create_dir_all(&out).unwrap();
    let exe = out.join(name);
    let mut cc = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
    cc.args([
        "-std=c99",
        "-O0",
        "-ffp-contract=off",
        "-fno-builtin",
        "-fno-strict-aliasing",
        "-fwrapv",
    ]);
    if unfused {
        cc.arg("-DUNFUSED");
    }
    let output = cc
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ref/physics/driver.c"))
        .args(["-lm", "-o"])
        .arg(&exe)
        .output()
        .expect("native C compiler is required");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    exe
}
fn vector(f: &[f32]) -> Vec3 {
    Vec3::new(f[0], f[1], f[2])
}
fn push_vec(out: &mut Vec<u32>, v: Vec3) {
    out.extend([v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]);
}
fn evaluate(input: &[f32; 36]) -> Vec<u32> {
    let mut state = FighterPhysics::standing(vector(&input[8..]), 1.0);
    state.ground_velocity = input[0];
    state.knockback_velocity = vector(&input[11..]);
    state.shield_knockback_velocity = vector(&input[14..]);
    state.player_nudge = Vec2::new(input[17], input[19]);
    state.velocity_blend = VelocityBlend {
        duration: input[29] as i32,
        remaining: input[30] as i32,
        origin: vector(&input[26..]),
    };
    state.secondary_ground_acceleration = input[31];
    let params = GroundedParameters {
        friction: input[1],
        walk_max_velocity: input[2],
        above_walk_multiplier: input[3],
        knockback_multiplier: 1.0,
        shield_knockback_multiplier: 1.0,
    };
    wait_physics(&mut state, &params, vector(&input[5..]), input[4]);
    let mut out = vec![state.ground_acceleration.to_bits()];
    push_vec(&mut out, state.animation_velocity);
    push_vec(&mut out, state.self_velocity);
    integrate_velocity(&mut state);
    integrate_environment(
        &mut state,
        if input[32] != 0.0 {
            Some(vector(&input[20..]))
        } else {
            None
        },
        vector(&input[23..]),
    );
    out.push(state.ground_velocity.to_bits());
    push_vec(&mut out, state.self_velocity);
    push_vec(&mut out, state.position);
    out.push((state.velocity_blend.duration as f32).to_bits());
    out.push((state.velocity_blend.remaining as f32).to_bits());
    out.push(decay_knockback(input[33], input[34]).to_bits());
    out
}

#[test]
fn grounded_physics_native_100k() {
    let retail = build("retail", false);
    let control = build("unfused", true);
    let input_path = retail.with_file_name("inputs.bin");
    let mut file = fs::File::create(&input_path).unwrap();
    let mut seed = 0x1234_5678u32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    let mut cases = Vec::with_capacity(100_000);
    for i in 0..100_000 {
        let mut f = [0.0; 36];
        for value in &mut f {
            let bits = random();
            // Normal finite exponents across cancellation-prone scales.
            *value = f32::from_bits((bits & 0x807f_ffff) | (((bits >> 24) % 24 + 115) << 23));
        }
        f[1] = f[1].abs();
        f[2] = 1.6;
        f[3] = 2.0;
        f[4] = [1.0, 0.2, 1.5, 0.1][i % 4];
        f[29] = if i % 3 == 0 {
            0.0
        } else {
            (i % 251 + 1) as f32
        };
        f[30] = if f[29] == 0.0 {
            0.0
        } else {
            (i % 17 + 1) as f32
        };
        f[32] = (i % 2) as f32;
        f[34] = f[34].abs();
        if i % 10 == 0 {
            f[0] = if i % 20 == 0 { -0.0 } else { 0.0 };
            f[5] = -0.0;
            f[6] = f32::from_bits(0x3f7f_ffff);
        }
        if i % 10 == 1 {
            f[1] = f[0].abs();
        } // exact stop boundary
        if i % 10 == 2 {
            f[1] = 0.0;
            f[0] = -0.0;
        } // negative zero survives the selector
        if i % 10 == 3 {
            for v in &mut f[8..29] {
                *v = if i % 2 == 0 { 0.0 } else { -0.0 };
            }
        }
        for value in f {
            file.write_all(&value.to_ne_bytes()).unwrap();
        }
        cases.push(f);
    }
    drop(file);
    let run = |exe| {
        let output = Command::new(exe)
            .stdin(Stdio::from(fs::File::open(&input_path).unwrap()))
            .output()
            .unwrap();
        assert!(output.status.success());
        output
            .stdout
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>()
    };
    let expected = run(retail);
    let unfused = run(control);
    const WORDS: usize = 17;
    assert_eq!(expected.len(), cases.len() * WORDS);
    let mut fusion_differences = 0;
    for (i, input) in cases.iter().enumerate() {
        let actual = evaluate(input);
        let reference = &expected[i * WORDS..(i + 1) * WORDS];
        for (field, (&a, &e)) in actual.iter().zip(reference).enumerate() {
            assert_eq!(
                a, e,
                "first mismatch case {i} word {field}: actual {a:08x}, C {e:08x}; input {input:?}"
            );
        }
        if reference != &unfused[i * WORDS..(i + 1) * WORDS] {
            fusion_differences += 1;
        }
    }
    assert!(
        fusion_differences > 100,
        "the sweep must exercise observable retail fusion"
    );
    eprintln!(
        "100000 inputs x 17 words matched; {fusion_differences} cases differ without retail fusion"
    );
}
