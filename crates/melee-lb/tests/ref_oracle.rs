//! Bit-for-bit comparison of `melee_lb::trigf` against the decomp's own C,
//! compiled natively.
//!
//! Two builds of the C exist ([`Variant`]). The retail-faithful copy in
//! `tests/ref/lbtrigf/retail/` spells out, with `fmaf`, the multiply-adds
//! MWCC fused on the disc (each site cites its instruction address;
//! `src/trigf.rs` carries the same citations). The port must match that
//! build bit for bit. The verbatim decomp sources are built too, and
//! `fusion_changes_results_within_the_sweep` reports on how many inputs the
//! two disagree: the evidence that the audit changed observable results.
//! `__frsqrte` is the table-exact model on both sides (`gekko_math::estimate`
//! and its C twin `crates/gekko-math/tests/ref/gekko_estimate.h`).
//!
//! The C is compiled with `cc -O0 -ffp-contract=off -fno-builtin
//! -fno-strict-aliasing -fwrapv`, so the only fused operations are the ones
//! written as such. If no `cc` is on `PATH` the tests print a notice and
//! pass; the std-tolerance tests in `src/trigf.rs` still run.
//!
//! NaN outputs are compared as "both NaN": payload propagation through
//! `x * NaN` and friends is hardware-specific, and the Rust models Gekko
//! only where the C source is explicit about the bit pattern.
//!
//! `expf` and `powf` never return for some inputs (see the module docs in
//! `src/trigf.rs`); the sweeps for those two steer clear of them, since a
//! hang would stall the C and the Rust alike.
#![allow(clippy::disallowed_methods)] // std math builds inputs and tolerances

// The sweeps spell the C's constants exactly as the C does.
#![allow(clippy::excessive_precision)]

use std::path::{Path, PathBuf};
use std::process::Command;

use melee_lb::trigf;

/// Files copied verbatim from the decomp; see `tests/ref/lbtrigf/NOTICE`.
const REF_FILES: &[&str] = &["lbtrigf.c", "lbtrigf.h", "lb_00CE.c", "lb_00CE.h"];

/// Which C the oracle is built from (`driver.c` picks by `-D`).
#[derive(Clone, Copy)]
enum Variant {
    /// `retail/lbtrigf.c` plus the fused `sqrtf` shim: the retail
    /// instruction sequence.
    Retail,
    /// The decomp sources verbatim (`-DLB_REF_UNFUSED`): every multiply and
    /// add separate.
    Verbatim,
}

fn ref_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ref/lbtrigf")
}

fn decomp_lb_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/melee-decomp/src/melee/lb")
}

/// The copies under `tests/ref/lbtrigf/` must stay byte-identical to the
/// submodule so a reviewer can trust "verbatim". Skips if the submodule is
/// not checked out.
#[test]
fn ref_sources_match_submodule() {
    let decomp = decomp_lb_dir();
    if !decomp.join("lbtrigf.c").exists() {
        eprintln!("[NON-DATA OMITTED] melee-decomp submodule not present; omitting copy check");
        return;
    }
    for f in REF_FILES {
        let ours = std::fs::read(ref_dir().join(f)).unwrap();
        let theirs = std::fs::read(decomp.join(f)).unwrap();
        assert!(
            ours == theirs,
            "tests/ref/lbtrigf/{f} differs from the decomp submodule; re-copy it, re-derive \
             tests/ref/lbtrigf/retail/{f} from it if one exists, and re-audit the port"
        );
    }
}

/// Compiles the driver into a per-test directory (tests run in parallel, so
/// they must not share an executable path).
fn build_oracle(variant: Variant, name: &str) -> Option<PathBuf> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    match Command::new(&cc).arg("--version").output() {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("[NON-DATA OMITTED] no working C compiler (`{cc}`) on PATH; omitting native oracle comparison");
            return None;
        }
    }
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("lbtrigf_ref_oracle_{name}"));
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
        "-I",
    ])
    .arg(ref_dir().join("shim"))
    .arg("-I")
    .arg(ref_dir())
    .arg("-o")
    .arg(&exe)
    .arg(ref_dir().join("driver.c"))
    .arg("-lm");
    if let Variant::Verbatim = variant {
        cmd.arg("-DLB_REF_UNFUSED");
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
    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Uniform in `[lo, hi)`.
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// `x` moved `k` representable values away (in bit-pattern order).
fn step(x: f32, k: i32) -> f32 {
    f32::from_bits((x.to_bits() as i32).wrapping_add(k) as u32)
}

/// `x` and its neighbours within `n` ulps, both signs.
fn around(v: &mut Vec<f32>, x: f32, n: i32) {
    for k in -n..=n {
        v.push(step(x, k));
        v.push(-step(x, k));
    }
}

fn f32_specials() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0x7FFF_FFFF), // MSL's NaN datum
        f32::from_bits(0xFFFF_FFFF),
        f32::from_bits(1), // smallest denormal
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF), // largest denormal
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        -f32::EPSILON,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        4.0,
        -4.0,
        1e-30,
        -1e-30,
        1e10,
        -1e10,
        3.3e38,
        16777216.0, // 2^24
    ]
}

/// Inputs for the unary lbtrigf functions: `atanf`'s interval boundaries,
/// `asinf`/`acosf`'s `|x| -> 1` region, denormals, and a broad random sweep.
fn unary_trig_sweep() -> Vec<f32> {
    let mut v = f32_specials();

    // atanf: the silver ratio and its conjugate, the four bit-pattern
    // thresholds, and the binade edges the switch keys on.
    for x in [
        2.4142136573791504f32,
        0.4142135679721832,
        f32::from_bits(0x3F08_D5B9),
        f32::from_bits(0x3F52_1801),
        f32::from_bits(0x3F9B_F7EC),
        f32::from_bits(0x3FEF_789E),
        0.5,
        1.0,
        2.0,
        4.0,
        0.25,
    ] {
        around(&mut v, x, 24);
    }
    // asinf/acosf: |x| near 1 (result goes 0/inf/NaN across the boundary)
    // and near 0.
    around(&mut v, 1.0, 512);
    around(&mut v, 0.0, 64);
    around(&mut v, f32::MIN_POSITIVE, 64);
    // Odd multiples of pi/8 and their tangents: atanf's offsets.
    for k in 1..=7 {
        let t = (k as f64 * core::f64::consts::FRAC_PI_8).tan() as f32;
        around(&mut v, t, 8);
    }

    // Dense sweeps: [-3, 3] covers all of atanf's ranges and the whole
    // asinf/acosf domain; [-1.001, 1.001] leans on the near-1 precision.
    for i in 0..=30_000 {
        v.push((-3.0 + 6.0 * i as f64 / 30_000.0) as f32);
    }
    for i in 0..=20_000 {
        v.push((-1.001 + 2.002 * i as f64 / 20_000.0) as f32);
    }

    // Every binade, both signs, several mantissas.
    let mut rng = Lcg(0x1B7A_0001);
    for exp in 0u32..=254 {
        for k in 0..8 {
            let mant = if k == 0 { 0 } else { rng.u32() & 0x7F_FFFF };
            v.push(f32::from_bits((exp << 23) | mant));
            v.push(f32::from_bits(0x8000_0000 | (exp << 23) | mant));
        }
    }
    // Random bit patterns (includes NaNs, infs, denormals).
    for _ in 0..36_000 {
        v.push(f32::from_bits(rng.u32()));
    }
    // Random game-sized arguments: |x| < 1000.
    for _ in 0..10_000 {
        v.push(rng.range(-1000.0, 1000.0) as f32);
    }
    v
}

/// Inputs for `atan2f(y, x)`: every pairing of the specials (both-zero in
/// all four sign combinations, infinities, NaNs, denormals), the axes and
/// diagonals with their neighbours, and random pairs.
fn atan2f_sweep() -> Vec<(f32, f32)> {
    let mut v = Vec::new();
    let specials = f32_specials();
    for &y in &specials {
        for &x in &specials {
            v.push((y, x));
        }
    }
    // Quadrant boundaries: y = +/-x and its neighbours, y or x zero with the
    // other running over signs and magnitudes.
    let mags = [1e-38f32, 1e-10, 0.5, 1.0, 2.0, 1e3, 1e10, 1e38];
    for &m in &mags {
        for sy in [1.0f32, -1.0] {
            for sx in [1.0f32, -1.0] {
                for k in -4..=4 {
                    v.push((sy * m, sx * step(m, k)));
                    v.push((sy * step(m, k), sx * m));
                }
            }
            for z in [0.0f32, -0.0] {
                v.push((sy * m, z));
                v.push((z, sy * m));
            }
        }
    }
    let mut rng = Lcg(0xA7A2_0002);
    // Random bit patterns.
    for _ in 0..30_000 {
        v.push((f32::from_bits(rng.u32()), f32::from_bits(rng.u32())));
    }
    // Random game-sized pairs.
    for _ in 0..40_000 {
        v.push((
            rng.range(-1000.0, 1000.0) as f32,
            rng.range(-1000.0, 1000.0) as f32,
        ));
    }
    // Random pairs with very different magnitudes (y/x over- or underflows).
    for _ in 0..20_000 {
        let y =
            rng.range(-1.0, 1.0) as f32 * f32::from_bits(((rng.u32() % 254 + 1) << 23) | 0x40_0000);
        let x =
            rng.range(-1.0, 1.0) as f32 * f32::from_bits(((rng.u32() % 254 + 1) << 23) | 0x40_0000);
        v.push((y, x));
    }
    // Random unit-circle angles.
    for _ in 0..20_000 {
        let a = rng.range(-core::f64::consts::PI, core::f64::consts::PI);
        let r = rng.range(1e-3, 100.0);
        v.push(((r * a.sin()) as f32, (r * a.cos()) as f32));
    }
    v
}

/// `expf` never returns for NaN or for `|x|` in about `[12.6157, 14.7106]`
/// (measured on the C with an iteration cap); keep a margin around that.
fn expf_terminates(x: f32) -> bool {
    let a = x.abs();
    !x.is_nan() && !(12.5..=14.9).contains(&a)
}

fn expf_sweep() -> Vec<f32> {
    let mut v: Vec<f32> = f32_specials();
    for x in [1.0f32, 12.0, 15.0, 88.0, 89.0, 100.0] {
        around(&mut v, x, 8);
    }
    around(&mut v, 0.0, 64);
    around(&mut v, f32::MIN_POSITIVE, 16);
    // Dense over the convergent range.
    for i in 0..=50_000 {
        v.push((-12.0 + 24.0 * i as f64 / 50_000.0) as f32);
    }
    let mut rng = Lcg(0xE8F0_0003);
    for _ in 0..20_000 {
        v.push(rng.range(-12.0, 12.0) as f32);
    }
    // Beyond the band: overflow to inf / underflow to 0 through the series.
    for _ in 0..10_000 {
        let m = rng.range(15.0, 200.0) as f32;
        v.push(if rng.u32() & 1 == 0 { m } else { -m });
    }
    // Random bit patterns.
    for _ in 0..20_000 {
        v.push(f32::from_bits(rng.u32()));
    }
    // Drop every non-terminating input, including the NaNs that `around`
    // produces by stepping below +0.0 into the 0xFFFFFFxx patterns.
    v.retain(|x| expf_terminates(*x));
    v
}

/// `powf(b, e)` runs `expf(e * ln(b))`, so `e * ln(b)` must stay in the
/// convergent `expf` range; the series for `ln(b)` never returns for NaN or
/// infinite `b`, and takes millions of steps as `b -> 0` or `b -> inf`
/// (`(b - 1) / (b + 1)` rounds to `+/-1`), so those are sampled sparingly.
fn powf_sweep() -> Vec<(f32, f32)> {
    let mut v: Vec<(f32, f32)> = vec![
        (0.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (-0.0, 2.5),
        (0.0, f32::NAN),
        (0.0, f32::INFINITY),
        (1.0, 0.0),
        (1.0, 1e10),
        (1.0, -1e10),
        (1.0, f32::MAX),
        (-1.0, 1.0),
        (-1.0, -1.0),
        (-1.0, 0.5),
        (-0.5, 2.0),
        (-2.0, 0.5),
        (-100.0, 3.0),
        (2.0, 200.0),
        (2.0, -200.0),
        (0.5, 200.0),
        (10.0, 5.0),
        (10.0, -5.0),
        // Slow but finite: the series for ln runs to the f32 harmonic limit.
        (f32::MIN_POSITIVE, 0.0),
        (f32::MIN_POSITIVE, 0.5),
        (f32::from_bits(1), 1.0),
        (f32::MAX, 0.5),
        (1e30, -0.25),
        (1e-30, 0.25),
    ];
    for k in -16..=16 {
        v.push((step(1.0, k), 3.0));
        v.push((step(1.0, k), -3.0));
        v.push((step(2.0, k), 0.5));
    }
    let mut rng = Lcg(0x90F0_0004);
    // Random bases, fast-converging range, exponents scaled so the product
    // stays well inside expf's convergent range.
    let mut n = 0;
    while n < 80_000 {
        let base = rng.range(0.05, 20.0) as f32;
        let exp = rng.range(-3.0, 3.0) as f32;
        if (exp as f64 * (base as f64).ln()).abs() < 11.5 {
            v.push((base, exp));
            n += 1;
        }
    }
    // Wider base range, small exponents.
    let mut n = 0;
    while n < 20_000 {
        let base = f32::from_bits(((rng.u32() % 20 + 117) << 23) | (rng.u32() & 0x7F_FFFF));
        let exp = rng.range(-1.0, 1.0) as f32;
        if (exp as f64 * (base as f64).ln()).abs() < 11.5 {
            v.push((base, exp));
            n += 1;
        }
    }
    // Negative bases: the series diverges to -inf and expf sees +/-inf.
    for _ in 0..5_000 {
        let base = -(rng.range(0.05, 20.0) as f32);
        let mut exp = rng.range(-3.0, 3.0) as f32;
        if exp == 0.0 {
            exp = 1.0;
        }
        v.push((base, exp));
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

fn compare_binary(exe: &Path, op: &'static str, f: fn(f32, f32) -> f32, inputs: &[(f32, f32)]) {
    let bytes: Vec<u8> = inputs
        .iter()
        .flat_map(|(a, b)| [a.to_bits().to_ne_bytes(), b.to_bits().to_ne_bytes()].concat())
        .collect();
    let out = run_oracle(exe, op, &bytes);
    assert_eq!(out.len(), inputs.len() * 4);
    let mut mm = Mismatches::new(op);
    for (i, (a, b)) in inputs.iter().enumerate() {
        let c = f32::from_bits(u32::from_ne_bytes(
            out[i * 4..i * 4 + 4].try_into().unwrap(),
        ));
        mm.check_f32(
            format!("{:08x} = {a:e}, {:08x} = {b:e}", a.to_bits(), b.to_bits()),
            f(*a, *b),
            c,
        );
    }
    mm.finish(inputs.len());
}

#[test]
fn atanf_lookup_matches_native_c_bit_for_bit() {
    let Some(exe) = build_oracle(Variant::Retail, "table") else {
        return;
    };
    let out = run_oracle(&exe, "atanf_lookup", &[]);
    assert_eq!(
        out.len(),
        trigf::ATANF_LOOKUP.len() * 4,
        "table length differs from the C"
    );
    for (i, want) in trigf::ATANF_LOOKUP.iter().enumerate() {
        let c = u32::from_ne_bytes(out[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(
            want.to_bits(),
            c,
            "ATANF_LOOKUP[{i}]: rust {:08x} vs c {c:08x}",
            want.to_bits()
        );
    }
}

#[test]
fn lbtrigf_matches_native_c_bit_for_bit() {
    let Some(exe) = build_oracle(Variant::Retail, "trig") else {
        return;
    };

    let xs = unary_trig_sweep();
    assert!(xs.len() >= 100_000, "sweep has {} inputs", xs.len());
    compare_unary(&exe, "atanf", trigf::atanf, &xs);
    compare_unary(&exe, "asinf", trigf::asinf, &xs);
    compare_unary(&exe, "acosf", trigf::acosf, &xs);
    compare_unary(&exe, "lb_sqrtf", trigf::lb_sqrtf, &xs);

    let pairs = atan2f_sweep();
    assert!(pairs.len() >= 100_000, "sweep has {} inputs", pairs.len());
    compare_binary(&exe, "atan2f", trigf::atan2f, &pairs);
}

#[test]
fn lb_00ce_matches_native_c_bit_for_bit() {
    let Some(exe) = build_oracle(Variant::Retail, "exp") else {
        return;
    };

    let xs = expf_sweep();
    assert!(xs.len() >= 100_000, "sweep has {} inputs", xs.len());
    compare_unary(&exe, "expf", trigf::expf, &xs);

    let pairs = powf_sweep();
    assert!(pairs.len() >= 100_000, "sweep has {} inputs", pairs.len());
    compare_binary(&exe, "powf", trigf::powf, &pairs);
}

/// Number of `f32` records that differ between two oracle outputs, counting
/// two NaNs as equal.
fn count_f32_differences(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len());
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(x, y)| {
            let (x, y) = (
                f32::from_bits(u32::from_ne_bytes((*x).try_into().unwrap())),
                f32::from_bits(u32::from_ne_bytes((*y).try_into().unwrap())),
            );
            x.to_bits() != y.to_bits() && !(x.is_nan() && y.is_nan())
        })
        .count()
}

fn unary_bytes(xs: &[f32]) -> Vec<u8> {
    xs.iter().flat_map(|x| x.to_bits().to_ne_bytes()).collect()
}

fn binary_bytes(pairs: &[(f32, f32)]) -> Vec<u8> {
    pairs
        .iter()
        .flat_map(|(a, b)| [a.to_bits().to_ne_bytes(), b.to_bits().to_ne_bytes()].concat())
        .collect()
}

/// The retail-faithful C and the verbatim C disagree on some inputs of the
/// sweep for every function with a fused site, and on none for `expf` and
/// `powf`, which have no fused instruction in retail. That is the evidence
/// that the fusion audit changed observable results, and that the sweep is
/// sensitive enough to catch a fused site written unfused (or the reverse).
/// Per-function counts are printed; run with `--nocapture` to see them.
#[test]
fn fusion_changes_results_within_the_sweep() {
    let (Some(retail), Some(verbatim)) = (
        build_oracle(Variant::Retail, "fusion_retail"),
        build_oracle(Variant::Verbatim, "fusion_verbatim"),
    ) else {
        return;
    };

    let trig = unary_trig_sweep();
    let trig_bytes = unary_bytes(&trig);
    let atan2 = atan2f_sweep();
    let atan2_bytes = binary_bytes(&atan2);
    let exp = expf_sweep();
    let exp_bytes = unary_bytes(&exp);
    let pow = powf_sweep();
    let pow_bytes = binary_bytes(&pow);

    let mut fused_total = 0;
    for (op, input, n, has_fused_site) in [
        ("atanf", &trig_bytes, trig.len(), true),
        ("asinf", &trig_bytes, trig.len(), true),
        ("acosf", &trig_bytes, trig.len(), true),
        ("lb_sqrtf", &trig_bytes, trig.len(), true),
        ("atan2f", &atan2_bytes, atan2.len(), true),
        ("expf", &exp_bytes, exp.len(), false),
        ("powf", &pow_bytes, pow.len(), false),
    ] {
        let differing = count_f32_differences(
            &run_oracle(&retail, op, input),
            &run_oracle(&verbatim, op, input),
        );
        eprintln!("{op}: retail-fused and verbatim C differ on {differing} of {n} inputs");
        if has_fused_site {
            fused_total += differing;
        } else {
            assert_eq!(
                differing, 0,
                "{op} has no fused site; the two builds must agree"
            );
        }
    }
    assert!(
        fused_total > 0,
        "the sweep cannot tell fused from unfused arithmetic"
    );
}
