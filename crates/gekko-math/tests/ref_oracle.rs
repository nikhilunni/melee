//! Bit-for-bit comparison of the `gekko_math::msl` transcriptions against the
//! decomp's own C, compiled natively with fusion disabled.
//!
//! What this proves: the Rust performs the same IEEE operations in the same
//! order as the C source. What it does not prove: which of those operations
//! MWCC contracted into fused multiply-adds on the retail disc. Those sites
//! are marked `FUSION AUDIT PENDING` in `src/msl.rs`.
//!
//! The C is compiled with `cc -O0 -ffp-contract=off -fno-builtin
//! -fno-strict-aliasing -fwrapv`. If no `cc` is on `PATH` the test prints a
//! notice and passes; the std-tolerance tests in `src/msl.rs` still run.
//!
//! The estimate instructions (`frsqrte`, `fres`) have a C twin in
//! `tests/ref/gekko_estimate.h`, shared with the melee-lb and hsd-anim oracle
//! builds. `estimates_and_sqrtf_match_native_c` checks the twin against the
//! Rust module directly, and MSL's `sqrtf` on top of it.
//!
//! Known host-vs-Gekko differences that are deliberately *not* modelled in
//! the C build (the Rust models Gekko): float -> int conversion of NaN
//! (`fctiwz` gives `i32::MIN`, hosts give 0 or `i32::MIN`), and
//! float -> long long of NaN (`__cvt_dbl_usll` saturates by sign). In every
//! reachable case the final result is NaN either way, and NaN outputs are
//! compared as "both NaN" because payload propagation is hardware-specific.

// The sweeps spell MSL's constants exactly as the C does.
#![allow(clippy::excessive_precision)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gekko_math::{estimate, msl};

const REF_FILES: &[&str] = &["trigf.c", "math_data.c", "math_1.c", "math.c"];

fn ref_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ref/msl")
}

fn decomp_msl_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/melee-decomp/src/MSL")
}

/// The copies under `tests/ref/msl/` must stay byte-identical to the
/// submodule so a reviewer can trust "verbatim". Skips if the submodule is
/// not checked out.
#[test]
fn ref_sources_match_submodule() {
    let decomp = decomp_msl_dir();
    if !decomp.join("trigf.c").exists() {
        eprintln!("melee-decomp submodule not present; skipping copy check");
        return;
    }
    for f in REF_FILES {
        let ours = std::fs::read(ref_dir().join(f)).unwrap();
        let theirs = std::fs::read(decomp.join(f)).unwrap();
        assert!(
            ours == theirs,
            "tests/ref/msl/{f} differs from the decomp submodule; re-copy it and re-audit the port"
        );
    }
}

fn build_oracle() -> Option<PathBuf> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    match Command::new(&cc).arg("--version").output() {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("no working C compiler (`{cc}`) on PATH; skipping native oracle comparison");
            return None;
        }
    }
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("msl_ref_oracle");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("driver");
    let mut cmd = Command::new(&cc);
    cmd.args([
        "-std=c99",
        "-O0",
        "-ffp-contract=off",
        "-fno-builtin",
        "-fno-strict-aliasing",
        "-fwrapv",
        "-Wno-incompatible-library-redeclaration",
        "-o",
    ])
    .arg(&exe)
    .arg(ref_dir().join("driver.c"));
    for f in REF_FILES {
        cmd.arg(ref_dir().join(f));
    }
    let out = cmd.output().expect("spawn cc");
    assert!(
        out.status.success(),
        "compiling reference oracle failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Some(exe)
}

fn run_oracle(exe: &Path, op: &str, input: &[u8]) -> Vec<u8> {
    let dir = exe.parent().unwrap();
    let in_path = dir.join(format!("{op}.in"));
    let out_path = dir.join(format!("{op}.out"));
    std::fs::write(&in_path, input).unwrap();
    let status = Command::new(exe)
        .arg(op)
        .arg(&in_path)
        .arg(&out_path)
        .status()
        .expect("run oracle");
    assert!(status.success(), "oracle {op} failed");
    std::fs::read(&out_path).unwrap()
}

/// Deterministic 64-bit LCG (Knuth MMIX constants) for reproducible sweeps.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
}

fn f32_edge_cases() -> Vec<f32> {
    let mut v = vec![
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0xFFFF_FFFF),
        f32::from_bits(1), // smallest denormal
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF), // largest denormal
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        core::f32::consts::E,
        3.45266983e-4, // trig __epsilon
        -3.45266983e-4,
        1e-3,
        16777216.0, // 2^24
        16777217.0,
        2147483648.0, // 2^31
        4294967296.0,
        1e10,
        -1e10,
        3.3e38,
        1e-30,
    ];
    // Around the trig epsilon boundary.
    let eps = 3.45266983e-4f32;
    for k in -8i32..=8 {
        v.push(f32::from_bits((eps.to_bits() as i32 + k) as u32));
        v.push(-f32::from_bits((eps.to_bits() as i32 + k) as u32));
    }
    // Integer multiples of pi/4, pi/2, pi and their neighbours.
    for k in -64i32..=64 {
        for q in [
            core::f64::consts::FRAC_PI_4,
            core::f64::consts::FRAC_PI_2,
            core::f64::consts::PI,
        ] {
            let x = (k as f64 * q) as f32;
            for d in -2i32..=2 {
                v.push(f32::from_bits((x.to_bits() as i32).wrapping_add(d) as u32));
            }
        }
    }
    v
}

fn f32_sweep() -> Vec<f32> {
    let mut v = f32_edge_cases();
    // Dense uniform sweep over [-2pi, 2pi].
    let two_pi = 2.0 * core::f64::consts::PI;
    for i in 0..=40_000 {
        v.push((-two_pi + 2.0 * two_pi * i as f64 / 40_000.0) as f32);
    }
    // Every binade, both signs, several mantissas.
    let mut rng = Lcg(0x5EED_0001);
    for exp in 0u32..=254 {
        for k in 0..8 {
            let mant = if k == 0 { 0 } else { rng.u32() & 0x7F_FFFF };
            v.push(f32::from_bits((exp << 23) | mant));
            v.push(f32::from_bits(0x8000_0000 | (exp << 23) | mant));
        }
    }
    // Random bit patterns (includes NaNs, infs, denormals).
    for _ in 0..44_000 {
        v.push(f32::from_bits(rng.u32()));
    }
    // Random "game-sized" arguments: |x| < 1000.
    for _ in 0..12_000 {
        let u = rng.u32() as f64 / u32::MAX as f64;
        v.push(((u - 0.5) * 2000.0) as f32);
    }
    v
}

fn f64_sweep() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        f64::from_bits(0x0000_0000_FFFF_FFFF), // subnormal with zero high word
        f64::from_bits(0x0010_0000_0000_0000),
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        1.0,
        -1.0,
        0.5,
        0.75,
        1e300,
        1e-300,
    ];
    let mut rng = Lcg(0xF8E7_0002);
    for exp in 0u64..=2046 {
        for k in 0..4 {
            let mant = if k == 0 {
                0
            } else {
                rng.next() & 0x000F_FFFF_FFFF_FFFF
            };
            v.push(f64::from_bits((exp << 52) | mant));
            v.push(f64::from_bits(0x8000_0000_0000_0000 | (exp << 52) | mant));
        }
    }
    for _ in 0..30_000 {
        v.push(f64::from_bits(rng.next()));
    }
    v
}

fn fmodf_sweep() -> Vec<(f32, f32)> {
    let mut v = vec![
        (5.5, 2.0),
        (-5.5, 2.0),
        (1.0, 3.0),
        (6.0, 3.0),
        (0.0, 0.0),
        (-0.0, 0.0),
        (1.0, 0.0),
        (f32::INFINITY, 1.0),
        (1.0, f32::INFINITY),
        (f32::INFINITY, f32::INFINITY),
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (f32::MAX, 1.0),
        (f32::MAX, f32::MIN_POSITIVE),
        (1e20, 3.0),
        (3.0, 1e-20),
    ];
    let mut rng = Lcg(0xD00D_0003);
    for _ in 0..30_000 {
        v.push((f32::from_bits(rng.u32()), f32::from_bits(rng.u32())));
    }
    for _ in 0..30_000 {
        let a = ((rng.u32() as f64 / u32::MAX as f64 - 0.5) * 2000.0) as f32;
        let b = ((rng.u32() as f64 / u32::MAX as f64 - 0.5) * 20.0) as f32;
        v.push((a, b));
    }
    v
}

struct Mismatches {
    op: &'static str,
    count: usize,
    nan_payload_only: usize,
    samples: Vec<String>,
}

impl Mismatches {
    fn new(op: &'static str) -> Self {
        Self {
            op,
            count: 0,
            nan_payload_only: 0,
            samples: Vec::new(),
        }
    }
    fn check_f32(&mut self, input: String, rust: f32, c: f32) {
        if rust.to_bits() == c.to_bits() {
            return;
        }
        if rust.is_nan() && c.is_nan() {
            self.nan_payload_only += 1;
            return;
        }
        self.count += 1;
        if self.samples.len() < 10 {
            self.samples.push(format!(
                "{}({input}): rust {:08x} ({rust:e}) vs c {:08x} ({c:e})",
                self.op,
                rust.to_bits(),
                c.to_bits()
            ));
        }
    }
    fn finish(self, total: usize) {
        eprintln!(
            "{}: {total} inputs, {} mismatches, {} NaN-payload-only differences",
            self.op, self.count, self.nan_payload_only
        );
        assert_eq!(
            self.count,
            0,
            "{} bit mismatches vs native C:\n{}",
            self.op,
            self.samples.join("\n")
        );
    }
}

fn compare_unary(exe: &Path, op: &'static str, f: fn(f32) -> f32, inputs: &[f32]) {
    let bytes: Vec<u8> = inputs
        .iter()
        .flat_map(|x| x.to_bits().to_ne_bytes())
        .collect();
    let out = run_oracle(exe, op, &bytes);
    assert_eq!(out.len(), inputs.len() * 4);
    let mut mm = Mismatches::new(op);
    for (i, x) in inputs.iter().enumerate() {
        let c = f32::from_bits(u32::from_ne_bytes(
            out[i * 4..i * 4 + 4].try_into().unwrap(),
        ));
        mm.check_f32(format!("{:08x} = {x:e}", x.to_bits()), f(*x), c);
    }
    mm.finish(inputs.len());
}

fn compare_unary_u64(exe: &Path, op: &'static str, f: fn(u64) -> u64, inputs: &[u64]) {
    let bytes: Vec<u8> = inputs.iter().flat_map(|x| x.to_ne_bytes()).collect();
    let out = run_oracle(exe, op, &bytes);
    assert_eq!(out.len(), inputs.len() * 8);
    let mut count = 0;
    let mut samples = Vec::new();
    for (i, x) in inputs.iter().enumerate() {
        let c = u64::from_ne_bytes(out[i * 8..i * 8 + 8].try_into().unwrap());
        let r = f(*x);
        if r != c {
            count += 1;
            if samples.len() < 10 {
                samples.push(format!("{op}({x:016x}): rust {r:016x} vs c {c:016x}"));
            }
        }
    }
    eprintln!("{op}: {} inputs, {count} mismatches", inputs.len());
    assert_eq!(
        count,
        0,
        "{op} mismatches vs native C:\n{}",
        samples.join("\n")
    );
}

/// The C twin of `gekko_math::estimate` (`tests/ref/gekko_estimate.h`) must
/// agree with the Rust bit for bit, NaN payloads included, and MSL `sqrtf`
/// built on either side must agree too.
#[test]
fn estimates_and_sqrtf_match_native_c() {
    let Some(exe) = build_oracle() else { return };

    let ds: Vec<u64> = f64_sweep().iter().map(|x| x.to_bits()).collect();
    compare_unary_u64(&exe, "frsqrte", estimate::frsqrte_bits, &ds);
    compare_unary_u64(&exe, "fres64", estimate::fres_bits, &ds);

    let xs = f32_sweep();
    // NaN payloads are compared exactly here: both sides pass NaN through.
    let bytes: Vec<u8> = xs.iter().flat_map(|x| x.to_bits().to_ne_bytes()).collect();
    let out = run_oracle(&exe, "fres", &bytes);
    let mut count = 0;
    let mut samples = Vec::new();
    for (i, x) in xs.iter().enumerate() {
        let c = u32::from_ne_bytes(out[i * 4..i * 4 + 4].try_into().unwrap());
        let r = estimate::fres(*x).to_bits();
        if r != c {
            count += 1;
            if samples.len() < 10 {
                samples.push(format!(
                    "fres({:08x}): rust {r:08x} vs c {c:08x}",
                    x.to_bits()
                ));
            }
        }
    }
    eprintln!("fres: {} inputs, {count} mismatches", xs.len());
    assert_eq!(
        count,
        0,
        "fres mismatches vs native C:\n{}",
        samples.join("\n")
    );

    compare_unary(&exe, "sqrtf", msl::sqrtf, &xs);
}

#[test]
fn msl_matches_native_c_bit_for_bit() {
    let Some(exe) = build_oracle() else { return };

    let xs = f32_sweep();
    assert!(xs.len() >= 100_000, "sweep has {} inputs", xs.len());
    compare_unary(&exe, "sinf", msl::sinf, &xs);
    compare_unary(&exe, "cosf", msl::cosf, &xs);
    compare_unary(&exe, "tanf", msl::tanf, &xs);
    compare_unary(&exe, "logf", msl::logf, &xs);

    // fmodf
    let pairs = fmodf_sweep();
    let bytes: Vec<u8> = pairs
        .iter()
        .flat_map(|(a, b)| [a.to_bits().to_ne_bytes(), b.to_bits().to_ne_bytes()].concat())
        .collect();
    let out = run_oracle(&exe, "fmodf", &bytes);
    assert_eq!(out.len(), pairs.len() * 4);
    let mut mm = Mismatches::new("fmodf");
    for (i, (a, b)) in pairs.iter().enumerate() {
        let c = f32::from_bits(u32::from_ne_bytes(
            out[i * 4..i * 4 + 4].try_into().unwrap(),
        ));
        mm.check_f32(format!("{a:e}, {b:e}"), msl::fmodf(*a, *b), c);
    }
    mm.finish(pairs.len());

    // frexp
    let ds = f64_sweep();
    let bytes: Vec<u8> = ds.iter().flat_map(|x| x.to_bits().to_ne_bytes()).collect();
    let out = run_oracle(&exe, "frexp", &bytes);
    assert_eq!(out.len(), ds.len() * 16);
    let mut count = 0;
    let mut samples = Vec::new();
    for (i, x) in ds.iter().enumerate() {
        let rec = &out[i * 16..i * 16 + 16];
        let c_m = f64::from_bits(u64::from_ne_bytes(rec[..8].try_into().unwrap()));
        let c_e = i64::from_ne_bytes(rec[8..].try_into().unwrap()) as i32;
        let (r_m, r_e) = msl::frexp(*x);
        let same_m = r_m.to_bits() == c_m.to_bits() || (r_m.is_nan() && c_m.is_nan());
        if !same_m || r_e != c_e {
            count += 1;
            if samples.len() < 10 {
                samples.push(format!(
                    "frexp({:016x}): rust ({:016x}, {r_e}) vs c ({:016x}, {c_e})",
                    x.to_bits(),
                    r_m.to_bits(),
                    c_m.to_bits()
                ));
            }
        }
    }
    eprintln!("frexp: {} inputs, {count} mismatches", ds.len());
    assert_eq!(
        count,
        0,
        "frexp mismatches vs native C:\n{}",
        samples.join("\n")
    );
}
