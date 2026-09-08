//! Native C oracle for the four pure mplib geometry functions.
//!
//! `ref/mplib/geom.c` contains exact decomp function extracts; `retail/geom.c`
//! changes only the expressions contracted by MWCC. Implicit contraction is
//! disabled in both builds. Finite inputs cover game coordinates, differing
//! exponents, cancellation, tolerances, degenerate lines and signed zero.
//! Every output bit (including signed zero) and hit/miss decision is checked;
//! no NaN equivalence or epsilon comparison is used.

use std::path::{Path, PathBuf};
use std::process::Command;

use melee_mp::{line_intersection, line_intersection_h, line_intersection_v, remap_2d};

const FUNCTIONS: [&str; 4] = [
    "mpRemap2d",
    "mpLineIntersection",
    "mpLineIntersectionH",
    "mpLineIntersectionV",
];

fn ref_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ref/mplib")
}

fn extract_function<'a>(source: &'a str, name: &str) -> &'a str {
    let signature = if name == "mpRemap2d" {
        format!("static void {name}(")
    } else {
        format!("bool {name}(")
    };
    // Only definitions start at column zero; the forward declaration in
    // mplib.c follows an address comment and must not be extracted.
    let start = source
        .match_indices(&signature)
        .find(|(i, _)| *i == 0 || source.as_bytes()[i - 1] == b'\n')
        .expect("C function definition")
        .0;
    let body = start + source[start..].find('{').unwrap();
    let mut depth = 0;
    for (i, b) in source.bytes().enumerate().skip(body) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..=i];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated C function {name}")
}

#[test]
fn geometry_reference_extracts_match_submodule() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/melee-decomp/src/melee/mp/mplib.c");
    if !source.exists() {
        eprintln!("melee-decomp absent; skipping source-extract check");
        return;
    }
    let source = std::fs::read_to_string(source).unwrap();
    let reference = std::fs::read_to_string(ref_dir().join("geom.c")).unwrap();
    for name in FUNCTIONS {
        assert_eq!(
            extract_function(&reference, name),
            extract_function(&source, name),
            "{name}: verbatim reference changed"
        );
    }
}

fn build_oracles() -> Option<(PathBuf, PathBuf)> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    if !Command::new(&cc)
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
    {
        eprintln!("no working C compiler ({cc}); skipping native geometry oracle");
        return None;
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("mp_geom_oracle_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let retail = dir.join("retail");
    let unfused = dir.join("unfused");
    for (exe, separate) in [(&retail, false), (&unfused, true)] {
        let mut cmd = Command::new(&cc);
        cmd.args([
            "-std=c99",
            "-O0",
            "-ffp-contract=off",
            "-fno-builtin",
            "-fno-strict-aliasing",
            "-fwrapv",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-I",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../gekko-math/tests/ref"))
        .arg(ref_dir().join("driver.c"))
        .arg("-lm")
        .arg("-o")
        .arg(exe);
        if separate {
            cmd.arg("-DMP_REF_UNFUSED");
        }
        let out = cmd.output().expect("spawn C compiler");
        assert!(
            out.status.success(),
            "C oracle compilation failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Some((retail, unfused))
}

fn run_oracle(exe: &Path, op: usize, inputs: &[[f32; 10]]) -> Vec<[u32; 3]> {
    let dir = exe.parent().unwrap();
    let input = dir.join("input.bin");
    let output = dir.join("output.bin");
    let bytes: Vec<_> = inputs
        .iter()
        .flatten()
        .flat_map(|x| x.to_ne_bytes())
        .collect();
    std::fs::write(&input, bytes).unwrap();
    let status = Command::new(exe)
        .arg(op.to_string())
        .arg(input)
        .arg(&output)
        .status()
        .expect("run C oracle");
    assert!(status.success());
    let bytes = std::fs::read(output).unwrap();
    assert_eq!(bytes.len(), inputs.len() * 12);
    bytes
        .chunks_exact(12)
        .map(|r| {
            std::array::from_fn(|i| u32::from_ne_bytes(r[i * 4..i * 4 + 4].try_into().unwrap()))
        })
        .collect()
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn coordinate(&mut self) -> f32 {
        (f64::from(self.next()) / f64::from(u32::MAX) * 2000.0 - 1000.0) as f32
    }
}

fn sweep(op: usize) -> Vec<[f32; 10]> {
    let mut rng = Lcg(0x4d50_4745_4f4d + op as u64);
    (0..50_000)
        .map(|i| {
            let mut a = std::array::from_fn(|_| rng.coordinate());
            match i % 10 {
                0 => {
                    // Independently varying exponents, finite and far from overflow.
                    for x in &mut a {
                        let bits = rng.next();
                        *x = f32::from_bits((bits & 0x807f_ffff) | ((87 + rng.next() % 81) << 23));
                    }
                }
                1 => {
                    // Zero-length old/map line, including the remap fallback.
                    a[2] = a[0];
                    a[3] = a[1];
                }
                2 => {
                    // Crossing the origin with non-dyadic barycentric weights:
                    // double FMA retains the cancellation residual before frsp.
                    a[0] = -1.0;
                    a[1] = -1.0;
                    a[2] = 2.0 + (i % 13) as f32;
                    a[3] = a[2];
                    a[4] = 0.0;
                    a[5] = 3.0;
                    a[6] = 0.0;
                    a[7] = -3.0;
                }
                3 => {
                    // Near the 1e-4 horizontal/vertical sweep threshold.
                    a[0] = -10.0;
                    a[1] = 0.0;
                    a[2] = 10.0;
                    a[3] = 10.0;
                    let epsilon = f32::from_bits(0.0001f32.to_bits() - 8 + i % 17);
                    a[4] = -epsilon * 0.5;
                    a[5] = epsilon * 0.5;
                    a[6] = epsilon * 0.5;
                    a[7] = -epsilon * 0.5;
                }
                4 => {
                    // Collinear, parallel and endpoint sweeps with signed zeros.
                    a = [-1.0, 0.0, 1.0, -0.0, -1.0, -0.0, 1.0, 0.0, 0.0, -0.0];
                    a[(i / 10) as usize % 10] = rng.coordinate();
                }
                5 => {
                    // Remap t=0/1 clamps; disjoint intersection segments.
                    a[8] = a[0];
                    a[9] = a[1];
                    if i % 20 == 5 {
                        a[8] = a[2];
                        a[9] = a[3];
                    }
                }
                _ => {}
            }
            a
        })
        .collect()
}

fn rust_record(op: usize, a: &[f32; 10]) -> [u32; 3] {
    let result = match op {
        0 => Some(remap_2d(
            a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9],
        )),
        1 => line_intersection(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]),
        2 => line_intersection_h(a[0], a[1], a[2], a[4], a[5], a[6], a[7]),
        3 => line_intersection_v(a[0], a[1], a[3], a[4], a[5], a[6], a[7]),
        _ => unreachable!(),
    };
    result.map_or([0; 3], |(x, y)| [1, x.to_bits(), y.to_bits()])
}

#[test]
fn geometry_matches_retail_c_bit_for_bit_and_measures_fusion() {
    let Some((retail, unfused)) = build_oracles() else {
        return;
    };
    let mut changed_total = 0;
    for (op, name) in FUNCTIONS.iter().enumerate() {
        let inputs = sweep(op);
        let expected = run_oracle(&retail, op, &inputs);
        let separate = run_oracle(&unfused, op, &inputs);
        let mut mismatches = 0;
        let mut samples = Vec::new();
        for (i, (a, c)) in inputs.iter().zip(&expected).enumerate() {
            let rust = rust_record(op, a);
            if rust != *c {
                mismatches += 1;
                if samples.len() < 8 {
                    samples.push(format!("input {i} {a:?}: Rust {rust:08x?}, C {c:08x?}"));
                }
            }
        }
        let changed = expected
            .iter()
            .zip(&separate)
            .filter(|(a, b)| a != b)
            .count();
        let hits = expected.iter().filter(|r| r[0] != 0).count();
        changed_total += changed;
        eprintln!(
            "{name}: {} inputs, {hits} hits, {mismatches} bit mismatches; \
             fusion changes {changed} input records",
            inputs.len()
        );
        assert!(
            hits > 1000,
            "{name}: sweep must exercise successful geometry"
        );
        assert_eq!(mismatches, 0, "{name}:\n{}", samples.join("\n"));
    }
    assert!(changed_total > 0, "sweep must distinguish fusion");
    std::fs::remove_dir_all(retail.parent().unwrap()).unwrap();
}
