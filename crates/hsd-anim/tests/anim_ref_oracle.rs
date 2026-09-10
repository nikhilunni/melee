//! Bit-for-bit comparison of FObj interpretation and Hermite interpolation
//! against retail-faithful fobj.c/spline.c copies with explicit gekko_fma.h
//! calls. All remaining expressions compile with fusion disabled. Verbatim
//! copies are checked against the submodule; the second build measures the
//! number of sweep inputs changed by the four audited fused operations.
//!
//! `aobj.c` is not part of the oracle: it pulls in every HSD object header
//! through `HSD_ForeachAnim` and is plain control flow; `tests/anim_aobj.rs`
//! covers it by hand.
//!
//! The C is compiled with `cc -O0 -ffp-contract=off -fno-builtin
//! -fno-strict-aliasing -fwrapv`. If no `cc` is on `PATH` the test prints a
//! notice and passes.

mod anim_common;

use std::path::{Path, PathBuf};
use std::process::Command;

use anim_common::{raw_range, Lcg, Stream};
use hsd_anim::fobj::*;

/// Files copied verbatim from the submodule (checked by
/// `ref_sources_match_submodule`).
const REF_FILES: &[&str] = &["fobj.c", "spline.c"];

fn ref_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/anim_ref")
}

fn decomp_baselib_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/melee-decomp/src/sysdolphin/baselib")
}

#[test]
fn ref_sources_match_submodule() {
    let decomp = decomp_baselib_dir();
    if !decomp.join("fobj.c").exists() {
        eprintln!("[NON-DATA OMITTED] melee-decomp submodule not present; omitting copy check");
        return;
    }
    for f in REF_FILES {
        let ours = std::fs::read(ref_dir().join(f)).unwrap();
        let theirs = std::fs::read(decomp.join(f)).unwrap();
        assert!(
            ours == theirs,
            "tests/anim_ref/{f} differs from the decomp submodule; re-copy it and re-audit the port"
        );
    }
}

fn build_oracle(retail: bool) -> Option<PathBuf> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    match Command::new(&cc).arg("--version").output() {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("[NON-DATA OMITTED] no working C compiler (`{cc}`) on PATH; omitting native oracle comparison");
            return None;
        }
    }
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("anim_ref_oracle")
        .join(std::thread::current().name().unwrap_or("unnamed"))
        .join(if retail { "retail" } else { "verbatim" });
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
        "-Wno-unused-function",
        "-I",
    ])
    .arg(ref_dir())
    .arg("-I")
    .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../gekko-math/tests/ref"))
    .arg("-o")
    .arg(&exe)
    .arg(ref_dir().join("driver.c"));
    for f in REF_FILES {
        cmd.arg(if retail {
            ref_dir().join("retail").join(f)
        } else {
            ref_dir().join(f)
        });
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

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
        .collect()
}

// ---------------------------------------------------------------------------
// splGetHelmite
// ---------------------------------------------------------------------------

#[test]
fn helmite_matches_native_c_bit_for_bit() {
    compare_helmite(false);
}

#[test]
fn fusion_changes_results_within_the_sweep() {
    compare_helmite(true);
    compare_fobj(true);
}

fn compare_helmite(report_fusion: bool) {
    let Some(exe) = build_oracle(true) else {
        return;
    };
    let mut rng = Lcg(0x4E1A_0001);
    let mut inputs: Vec<[f32; 6]> = Vec::new();
    // Realistic: reciprocal of an integer segment length, integer-ish time.
    for _ in 0..40_000 {
        let fterm = rng.range(1, 400) as u16;
        let inv = (1.0f64 / fterm as f64) as f32;
        let t = rng.range(0, fterm as i32) as f32 + [0.0, 0.5, 0.25, 0.1][rng.below(4) as usize];
        let v = |r: &mut Lcg, s: f32| (r.unit() - 0.5) * s;
        inputs.push([
            inv,
            t,
            v(&mut rng, 200.0),
            v(&mut rng, 200.0),
            v(&mut rng, 20.0),
            v(&mut rng, 20.0),
        ]);
    }
    // Arbitrary bit patterns, including NaN/inf/denormals.
    for _ in 0..20_000 {
        inputs.push([
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
        ]);
    }
    let bytes: Vec<u8> = inputs
        .iter()
        .flat_map(|r| {
            r.iter()
                .flat_map(|x| x.to_bits().to_ne_bytes())
                .collect::<Vec<_>>()
        })
        .collect();
    let out = words(&run_oracle(&exe, "helmite", &bytes));
    assert_eq!(out.len(), inputs.len());
    if report_fusion {
        let verbatim = build_oracle(false).expect("build verbatim oracle");
        let old = words(&run_oracle(&verbatim, "helmite", &bytes));
        assert_eq!(old.len(), out.len());
        let changed = out.iter().zip(&old).filter(|(a, b)| a != b).count();
        eprintln!(
            "helmite: retail and verbatim C differ on {changed} of {} inputs",
            inputs.len()
        );
    }
    let mut mismatches = 0;
    let mut nan_only = 0;
    let mut samples = Vec::new();
    for (i, r) in inputs.iter().enumerate() {
        let rust = spl_get_helmite(r[0], r[1], r[2], r[3], r[4], r[5]);
        let c = f32::from_bits(out[i]);
        if rust.to_bits() == c.to_bits() {
            continue;
        }
        if rust.is_nan() && c.is_nan() {
            nan_only += 1;
            continue;
        }
        mismatches += 1;
        if samples.len() < 10 {
            samples.push(format!(
                "helmite({r:?}): rust {:08x} vs c {:08x}",
                rust.to_bits(),
                c.to_bits()
            ));
        }
    }
    eprintln!(
        "helmite: {} inputs, {mismatches} mismatches, {nan_only} NaN-payload-only differences",
        inputs.len()
    );
    assert_eq!(
        mismatches,
        0,
        "splGetHelmite mismatches:\n{}",
        samples.join("\n")
    );
}

// ---------------------------------------------------------------------------
// HSD_FObjInterpretAnim
// ---------------------------------------------------------------------------

/// One oracle test case: a track description plus a schedule of rates.
struct Case {
    obj_type: u8,
    frac_value: u8,
    frac_slope: u8,
    startframe: f32,
    req_start: f32,
    ad: Vec<u8>,
    rates: Vec<f32>,
}

impl Case {
    fn encode(&self, out: &mut Vec<u8>) {
        let w = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_ne_bytes());
        w(out, self.obj_type as u32);
        w(out, self.frac_value as u32);
        w(out, self.frac_slope as u32);
        w(out, self.startframe.to_bits());
        w(out, self.req_start.to_bits());
        w(out, self.rates.len() as u32);
        w(out, self.ad.len() as u32);
        out.extend_from_slice(&self.ad);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        for r in &self.rates {
            w(out, r.to_bits());
        }
    }
}

const STEP_WORDS: usize = 16;

/// Per-step record produced by both sides.
#[derive(Debug, PartialEq, Eq)]
struct StepOut {
    count: u32,
    first: u32,
    last: u32,
    sum: u32,
    uninit: u32,
    flags: u32,
    op: u32,
    op_intrp: u32,
    nb_pack: u32,
    fterm: u32,
    pos: u32,
    time: u32,
    p0: u32,
    p1: u32,
    d0: u32,
    d1: u32,
}

impl StepOut {
    fn from_words(w: &[u32]) -> StepOut {
        StepOut {
            count: w[0],
            first: w[1],
            last: w[2],
            sum: w[3],
            uninit: w[4],
            flags: w[5],
            op: w[6],
            op_intrp: w[7],
            nb_pack: w[8],
            fterm: w[9],
            pos: w[10],
            time: w[11],
            p0: w[12],
            p1: w[13],
            d0: w[14],
            d1: w[15],
        }
    }

    /// NaN payloads are hardware-specific; compare them as "both NaN".
    fn same(&self, o: &StepOut) -> bool {
        fn f(a: u32, b: u32) -> bool {
            a == b || (f32::from_bits(a).is_nan() && f32::from_bits(b).is_nan())
        }
        // The sum covers values between first and last; skip it when NaNs
        // are around since their payload bits are not comparable.
        let nan_seen = f32::from_bits(self.first).is_nan() || f32::from_bits(self.last).is_nan();
        self.count == o.count
            && f(self.first, o.first)
            && f(self.last, o.last)
            && (self.sum == o.sum || nan_seen)
            && self.uninit == o.uninit
            && self.flags == o.flags
            && self.op == o.op
            && self.op_intrp == o.op_intrp
            && self.nb_pack == o.nb_pack
            && self.fterm == o.fterm
            && self.pos == o.pos
            && f(self.time, o.time)
            && f(self.p0, o.p0)
            && f(self.p1, o.p1)
            && f(self.d0, o.d0)
            && f(self.d1, o.d1)
    }
}

fn run_rust(case: &Case) -> Vec<StepOut> {
    let mut f = FObj::new(
        &case.ad,
        case.startframe,
        case.obj_type,
        case.frac_value,
        case.frac_slope,
    );
    f.req_anim(case.req_start);
    let mut out = Vec::new();
    for &rate in &case.rates {
        let mut count = 0u32;
        let mut first = 0u32;
        let mut last = 0u32;
        let mut sum = 0u32;
        {
            let mut cb = |_t: u8, v: f32| {
                let b = v.to_bits();
                if count == 0 {
                    first = b;
                }
                last = b;
                sum = sum.wrapping_add(b);
                count += 1;
            };
            f.interpret_anim(Some(&mut cb), rate);
        }
        // The Rust callback cannot see op_intrp mid-call; the C side's
        // `uninit` counter must be 0 for the comparison to be meaningful,
        // and the generator guarantees that (asserted by the caller).
        out.push(StepOut {
            count,
            first,
            last,
            sum,
            uninit: 0,
            flags: f.flags as u32,
            op: f.op as u32,
            op_intrp: f.op_intrp as u32,
            nb_pack: f.nb_pack as u32,
            fterm: f.fterm as u32,
            pos: f.pos as u32,
            time: f.time.to_bits(),
            p0: f.p0.to_bits(),
            p1: f.p1.to_bits(),
            d0: f.d0.to_bits(),
            d1: f.d1.to_bits(),
        });
    }
    out
}

fn pick_frac(rng: &mut Lcg) -> u8 {
    match rng.below(5) {
        0 => HSD_A_FRAC_FLOAT,
        1 => frac(FracType::S8, rng.range(0, 7) as u8),
        2 => frac(FracType::U8, rng.range(0, 7) as u8),
        3 => frac(FracType::S16, rng.range(0, 14) as u8),
        _ => frac(FracType::U16, rng.range(0, 14) as u8),
    }
}

fn frac_type(frac: u8) -> FracType {
    match frac & 0xE0 {
        HSD_A_FRAC_S8 => FracType::S8,
        HSD_A_FRAC_U8 => FracType::U8,
        HSD_A_FRAC_S16 => FracType::S16,
        HSD_A_FRAC_U16 => FracType::U16,
        _ => FracType::Float,
    }
}

fn push_value(s: &mut Stream, rng: &mut Lcg, frac: u8, scale: f32) {
    if frac == HSD_A_FRAC_FLOAT {
        let v = match rng.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => rng.range(-50, 50) as f32,
            _ => (rng.unit() - 0.5) * scale,
        };
        s.f32(v);
    } else {
        let (lo, hi) = raw_range(frac_type(frac));
        s.raw(frac, rng.range(lo, hi));
    }
}

/// A random but well-formed stream: at least two data records so the
/// uninitialised-value path is never reached, every key but possibly the
/// last followed by a wait.
fn random_stream(rng: &mut Lcg, frac_value: u8, frac_slope: u8) -> Vec<u8> {
    let mut s = Stream::new();
    let npacks = rng.range(1, 5);
    let mut data_records = 0;
    let mut keys: Vec<u8> = Vec::new();
    for _ in 0..npacks {
        let op = match rng.below(20) {
            0..=3 => HSD_A_OP_CON,
            4..=7 => HSD_A_OP_LIN,
            8..=9 => HSD_A_OP_SPL0,
            10..=13 => HSD_A_OP_SPL,
            14..=15 => HSD_A_OP_SLP,
            _ => HSD_A_OP_KEY,
        };
        let count = if rng.below(10) == 0 {
            rng.range(9, 20)
        } else {
            rng.range(1, 4)
        } as u32;
        for _ in 0..count {
            keys.push(op);
        }
    }
    // Guarantee two non-SLP records.
    while keys.iter().filter(|op| **op != HSD_A_OP_SLP).count() < 2 {
        keys.push(HSD_A_OP_CON);
    }
    // Emit as packs of consecutive equal opcodes.
    let mut i = 0;
    while i < keys.len() {
        let op = keys[i];
        let mut j = i;
        while j < keys.len() && keys[j] == op {
            j += 1;
        }
        s.pack(op, (j - i) as u32);
        for k in i..j {
            let last = k == keys.len() - 1;
            match op {
                HSD_A_OP_SPL => {
                    push_value(&mut s, rng, frac_value, 100.0);
                    push_value(&mut s, rng, frac_slope, 10.0);
                    data_records += 1;
                }
                HSD_A_OP_SLP => {
                    push_value(&mut s, rng, frac_slope, 10.0);
                }
                _ => {
                    push_value(&mut s, rng, frac_value, 100.0);
                    data_records += 1;
                }
            }
            if op != HSD_A_OP_SLP && !(last && rng.below(3) == 0) {
                let w = match rng.below(12) {
                    0 => 0,
                    1 => rng.range(128, 2000) as u32,
                    _ => rng.range(1, 12) as u32,
                };
                s.wait(w);
            }
        }
        i = j;
    }
    assert!(data_records >= 2);
    s.finish()
}

fn random_case(rng: &mut Lcg) -> Case {
    let frac_value = pick_frac(rng);
    let frac_slope = pick_frac(rng);
    let ad = random_stream(rng, frac_value, frac_slope);
    let nsteps = rng.range(1, 60) as usize;
    let mut rates = Vec::with_capacity(nsteps);
    rates.push(if rng.below(4) == 0 {
        rng.range(0, 3) as f32
    } else {
        0.0
    });
    let base = [1.0f32, 1.0, 1.0, 0.5, 1.5, 2.0, 0.25, 3.0, 0.7, 10.0][rng.below(10) as usize];
    for _ in 1..nsteps {
        rates.push(match rng.below(20) {
            0 => 0.0,
            1 => -0.5,
            2 => rng.unit() * 4.0,
            _ => base,
        });
    }
    Case {
        obj_type: rng.range(1, 39) as u8,
        frac_value,
        frac_slope,
        startframe: match rng.below(5) {
            0 => rng.range(-5, 5) as f32,
            1 => rng.unit() * 6.0 - 3.0,
            _ => 0.0,
        },
        req_start: match rng.below(4) {
            0 => rng.range(0, 6) as f32,
            1 => rng.unit() * 5.0,
            _ => 0.0,
        },
        ad,
        rates,
    }
}

/// Hand-built streams mirroring `anim_fobj.rs`, so the oracle also pins
/// the cases whose expected values were derived by hand.
fn hand_cases() -> Vec<Case> {
    let mk = |ad: Vec<u8>, fv: u8, fs: u8, rates: Vec<f32>| Case {
        obj_type: 5,
        frac_value: fv,
        frac_slope: fs,
        startframe: 0.0,
        req_start: 0.0,
        ad,
        rates,
    };
    let ones = |n: usize| {
        let mut v = vec![0.0f32];
        v.extend(vec![1.0f32; n]);
        v
    };
    let f = HSD_A_FRAC_FLOAT;
    let mut v = Vec::new();
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 3)
        .f32(0.25)
        .wait(3)
        .f32(-7.5)
        .wait(2)
        .f32(3.0)
        .wait(0);
    v.push(mk(s.finish(), f, f, ones(8)));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 2).f32(0.3).wait(4).f32(1.7);
    v.push(mk(s.finish(), f, f, ones(7)));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2)
        .f32(-1.0)
        .f32(0.75)
        .wait(5)
        .f32(2.5)
        .f32(-0.125);
    v.push(mk(s.finish(), f, f, ones(7)));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2)
        .f32(10.0)
        .f32(-3.0)
        .wait(3)
        .f32(-4.0)
        .f32(1.0);
    v.push(mk(
        s.finish(),
        f,
        f,
        vec![0.0, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7],
    ));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL0, 2).f32(2.0).wait(4).f32(6.0);
    v.push(mk(s.finish(), f, f, ones(5)));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 1).f32(1.0).wait(5);
    s.pack(HSD_A_OP_SLP, 1).f32(4.0);
    s.pack(HSD_A_OP_LIN, 1).f32(3.0);
    v.push(mk(s.finish(), f, f, ones(6)));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 3)
        .f32(5.0)
        .wait(2)
        .f32(6.0)
        .wait(3)
        .f32(7.0);
    v.push(mk(s.finish(), f, f, ones(8)));
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 4)
        .f32(1.0)
        .wait(1)
        .f32(2.0)
        .wait(1)
        .f32(3.0)
        .wait(5)
        .f32(4.0);
    v.push(mk(s.finish(), f, f, vec![0.0, 2.5, 2.5, 2.5]));
    let fv = frac(FracType::S16, 8);
    let fs = frac(FracType::S8, 4);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2)
        .raw(fv, 256)
        .raw(fs, -16)
        .wait(4)
        .raw(fv, -512)
        .raw(fs, 8);
    v.push(mk(s.finish(), fv, fs, ones(6)));
    // Negative start offset.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 2).f32(1.0).wait(2).f32(2.0);
    let mut c = mk(s.finish(), f, f, ones(5));
    c.startframe = -2.0;
    v.push(c);
    // Unknown opcode.
    v.push(mk(vec![0x07, 0, 0, 0, 0, 0, 0, 0, 0], f, f, ones(3)));
    v
}

#[test]
fn fobj_interpreter_matches_native_c_bit_for_bit() {
    compare_fobj(false);
}

fn compare_fobj(report_fusion: bool) {
    let Some(exe) = build_oracle(true) else {
        return;
    };
    let mut rng = Lcg(0xF0B1_0002);
    let mut cases = hand_cases();
    for _ in 0..4000 {
        cases.push(random_case(&mut rng));
    }

    let mut input = Vec::new();
    for c in &cases {
        c.encode(&mut input);
    }
    let out = words(&run_oracle(&exe, "fobj", &input));
    let total_steps: usize = cases.iter().map(|c| c.rates.len()).sum();
    assert_eq!(out.len(), total_steps * STEP_WORDS, "oracle output length");

    if report_fusion {
        let verbatim = build_oracle(false).expect("build verbatim oracle");
        let old = words(&run_oracle(&verbatim, "fobj", &input));
        assert_eq!(old.len(), out.len());
        let mut cursor = 0;
        let mut changed_cases = 0;
        let mut changed_steps = 0;
        for case in &cases {
            let end = cursor + case.rates.len() * STEP_WORDS;
            let changed = out[cursor..end]
                .chunks_exact(STEP_WORDS)
                .zip(old[cursor..end].chunks_exact(STEP_WORDS))
                .filter(|(a, b)| a != b)
                .count();
            changed_cases += usize::from(changed != 0);
            changed_steps += changed;
            cursor = end;
        }
        eprintln!("fobj: retail and verbatim C differ on {changed_cases} of {} cases, {changed_steps} of {total_steps} steps", cases.len());
    }
    let mut cursor = 0;
    let mut mismatches = 0;
    let mut samples = Vec::new();
    for (ci, c) in cases.iter().enumerate() {
        let rust = run_rust(c);
        for (si, r) in rust.iter().enumerate() {
            let cw = &out[cursor..cursor + STEP_WORDS];
            cursor += STEP_WORDS;
            let cs = StepOut::from_words(cw);
            assert_eq!(
                cs.uninit, 0,
                "case {ci}: generator produced an uninitialised-value stream"
            );
            if !r.same(&cs) {
                mismatches += 1;
                if samples.len() < 8 {
                    samples.push(format!(
                        "case {ci} step {si} (rate {}, frac {:#x}/{:#x}, stream {:02x?}):\n  rust {r:?}\n  c    {cs:?}",
                        c.rates[si], c.frac_value, c.frac_slope, c.ad
                    ));
                }
            }
        }
    }
    eprintln!(
        "fobj: {} cases, {total_steps} steps, {mismatches} mismatches",
        cases.len()
    );
    assert_eq!(
        mismatches,
        0,
        "HSD_FObjInterpretAnim mismatches:\n{}",
        samples.join("\n")
    );
}
