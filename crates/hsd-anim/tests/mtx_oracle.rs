//! Bit-for-bit comparison against retail-faithful copies of baselib mtx.c
//! and quatlib.c. Explicit gekko_fma.h calls preserve the DOL's fused sites;
//! `-ffp-contract=off` keeps all other expressions unfused. Verbatim copies
//! are checked against the submodule, and a second build measures how many
//! sweep records changed. Supporting SDK and MSL math is identical in both.
//!
//! The SDK `PSMTX*`/`PSVEC*` routines are asm-only in the decomp, so the
//! driver carries an independent C re-transcription of the same asm (see
//! `tests/ref/mtx/driver.c`). Agreement there catches transcription slips
//! between two readings of the asm; it is not an independent oracle.
//!
//! `sinf`/`cosf` come from MSL `trigf.c` (gekko-math's retail copy).
//! `sqrtf` and the `fres`/`frsqrte` steps use the table-exact estimate model
//! on both sides (`gekko_math::estimate` and its C twin
//! `crates/gekko-math/tests/ref/gekko_estimate.h`); the `fmuls` on a
//! double-width `frsqrte` result quantizes its multiplier to 25 significant
//! bits, independently expressed with frexp/ldexp in C. `atan2f`/`asinf`/`acosf` are deterministic
//! stand-ins on both sides (`StubTrig` here, the same expressions in the
//! driver), since the real ones live in `melee-lb`.
//!
//! If no `cc` is on `PATH`, or the decomp submodule is not checked out, the
//! test prints a notice and passes; the in-crate unit tests still run.
#![allow(clippy::disallowed_methods)] // std math builds inputs and tolerances

use std::path::{Path, PathBuf};
use std::process::Command;

use hsd_anim::mtx::{self, InverseTrig};
use hsd_anim::quat::{self, Quaternion};
use hsd_types::{Mtx, Vec3};

/// Mirrors the stand-ins in `driver.c` exactly (no fusion on either side).
struct StubTrig;
impl InverseTrig for StubTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        let a = y * 0.5;
        let b = x * 0.25;
        a + b
    }
    fn asinf(x: f32) -> f32 {
        x * 0.5
    }
    fn acosf(x: f32) -> f32 {
        let a = x * 0.5;
        1.0 - a
    }
}

const SENTINEL: f32 = 12345.678;

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn ref_dir() -> PathBuf {
    manifest().join("tests/ref/mtx")
}
fn decomp_dir() -> PathBuf {
    manifest().join("../../third_party/melee-decomp")
}
fn msl_ref_dir() -> PathBuf {
    manifest().join("../gekko-math/tests/ref/msl")
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
    let source_dir = if retail {
        ref_dir().join("retail")
    } else {
        ref_dir()
    };
    let mtx_c = source_dir.join("mtx.c");
    let quatlib_c = source_dir.join("quatlib.c");
    // The retail-faithful trigf.c (sinf/cosf with the fused multiply-adds the
    // disc's asm shows; see gekko-math's ref_oracle) plus the tables and
    // fabsf__Ff it links against.
    let msl_files = ["retail/trigf.c", "math_data.c", "math_1.c"].map(|f| msl_ref_dir().join(f));
    if !decomp_dir().join("src/sysdolphin/baselib/mtx.h").exists() {
        eprintln!("[NON-DATA OMITTED] melee-decomp submodule not present; omitting native oracle comparison");
        return None;
    }
    for f in &msl_files {
        assert!(
            f.exists(),
            "gekko-math MSL reference copy missing at {}",
            f.display()
        );
    }

    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("mtx_ref_oracle")
        .join(std::thread::current().name().unwrap_or("unnamed"))
        .join(if retail { "retail" } else { "verbatim" });
    std::fs::create_dir_all(&out_dir).unwrap();
    const CFLAGS: [&str; 7] = [
        "-std=c99",
        "-O0",
        "-ffp-contract=off",
        "-fno-builtin",
        "-fno-strict-aliasing",
        "-fwrapv",
        "-Wno-incompatible-library-redeclaration",
    ];

    // The MSL sources `#include "math.h"` expecting gekko-math's shim (which
    // supplies the fixed-width types and the gekko_fmadds helpers), while
    // mtx.c/quatlib.c expect this crate's shim. Compile the MSL files to
    // objects against their own include dir first, then link everything.
    let mut msl_objects = Vec::new();
    for src in &msl_files {
        let obj = out_dir.join(format!("{}.o", src.file_stem().unwrap().to_string_lossy()));
        let out = Command::new(&cc)
            .args(CFLAGS)
            .arg("-I")
            .arg(msl_ref_dir())
            .arg("-c")
            .arg(src)
            .arg("-o")
            .arg(&obj)
            .output()
            .expect("spawn cc");
        assert!(
            out.status.success(),
            "compiling {} failed:\n{}",
            src.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        msl_objects.push(obj);
    }

    let exe = out_dir.join("driver");
    let mut cmd = Command::new(&cc);
    cmd.args(CFLAGS)
        // Keep the decomp's debug.h (and its OS dependencies) out; the shim
        // objalloc.h supplies HSD_ASSERT instead.
        .arg("-DSYSDOLPHIN_BASELIB_DEBUG_H")
        .arg("-I")
        .arg(ref_dir().join("include"))
        .arg("-I")
        .arg(decomp_dir().join("src/sysdolphin/baselib"))
        .arg("-I")
        .arg(msl_ref_dir().join(".."))
        .arg("-o")
        .arg(&exe)
        .arg(ref_dir().join("driver.c"))
        .arg(&mtx_c)
        .arg(&quatlib_c)
        .args(&msl_objects);
    let out = cmd.output().expect("spawn cc");
    assert!(
        out.status.success(),
        "compiling reference oracle failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Some(exe)
}

fn run_oracle(exe: &Path, op: &str, input: &[f32]) -> Vec<f32> {
    let dir = exe.parent().unwrap();
    let in_path = dir.join(format!("{op}.in"));
    let out_path = dir.join(format!("{op}.out"));
    let bytes: Vec<u8> = input
        .iter()
        .flat_map(|x| x.to_bits().to_ne_bytes())
        .collect();
    std::fs::write(&in_path, bytes).unwrap();
    let status = Command::new(exe)
        .arg(op)
        .arg(&in_path)
        .arg(&out_path)
        .status()
        .expect("run oracle");
    assert!(status.success(), "oracle {op} failed");
    let out = std::fs::read(&out_path).unwrap();
    out.chunks_exact(4)
        .map(|c| f32::from_bits(u32::from_ne_bytes(c.try_into().unwrap())))
        .collect()
}

/// Deterministic 64-bit LCG (Knuth MMIX constants).
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
    /// Uniform in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = self.u32() as f64 / (u32::MAX as f64 + 1.0);
        (lo as f64 + (hi as f64 - lo as f64) * u) as f32
    }
    fn vec(&mut self, lo: f32, hi: f32) -> Vec3 {
        Vec3::new(self.range(lo, hi), self.range(lo, hi), self.range(lo, hi))
    }
    fn mtx(&mut self, lo: f32, hi: f32) -> Mtx {
        let mut m = Mtx::ZERO;
        for r in m.0.iter_mut() {
            for e in r.iter_mut() {
                *e = self.range(lo, hi);
            }
        }
        m
    }
    fn quat(&mut self) -> Quaternion {
        Quaternion::new(
            self.range(-1.0, 1.0),
            self.range(-1.0, 1.0),
            self.range(-1.0, 1.0),
            self.range(-1.0, 1.0),
        )
    }
    /// Unit quaternion (normalised with std math; input generation only).
    fn unit_quat(&mut self) -> Quaternion {
        let q = self.quat();
        let n = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
        if n < 1e-3 {
            return Quaternion::IDENTITY;
        }
        Quaternion::new(q.x / n, q.y / n, q.z / n, q.w / n)
    }
    fn angles(&mut self) -> Vec3 {
        let pi = core::f32::consts::PI;
        self.vec(-pi, pi)
    }
}

// --- record helpers ---------------------------------------------------------

fn m_of(i: &[f32]) -> Mtx {
    let mut m = Mtx::ZERO;
    for (r, row) in m.0.iter_mut().enumerate() {
        row.copy_from_slice(&i[r * 4..r * 4 + 4]);
    }
    m
}
fn m_put(o: &mut [f32], m: &Mtx) {
    for (r, row) in m.0.iter().enumerate() {
        o[r * 4..r * 4 + 4].copy_from_slice(row);
    }
}
fn m_words(m: &Mtx) -> Vec<f32> {
    m.0.iter().flatten().copied().collect()
}
fn v_of(i: &[f32]) -> Vec3 {
    Vec3::new(i[0], i[1], i[2])
}
fn v_put(o: &mut [f32], v: &Vec3) {
    o[0] = v.x;
    o[1] = v.y;
    o[2] = v.z;
}
fn v_words(v: &Vec3) -> Vec<f32> {
    vec![v.x, v.y, v.z]
}
fn q_of(i: &[f32]) -> Quaternion {
    Quaternion::new(i[0], i[1], i[2], i[3])
}
fn q_put(o: &mut [f32], q: &Quaternion) {
    o[0] = q.x;
    o[1] = q.y;
    o[2] = q.z;
    o[3] = q.w;
}
fn q_words(q: &Quaternion) -> Vec<f32> {
    vec![q.x, q.y, q.z, q.w]
}
fn sentinel_mtx() -> Mtx {
    Mtx([[SENTINEL; 4]; 3])
}

// --- input corpora -----------------------------------------------------------

fn matrices(rng: &mut Lcg) -> Vec<Mtx> {
    let mut v = vec![Mtx::IDENTITY, Mtx::ZERO];
    // Diagonal / axis-aligned cases with exact arithmetic.
    for s in [1.0f32, 2.0, 0.5, -1.0, -3.0] {
        v.push(Mtx([
            [s, 0.0, 0.0, 1.0],
            [0.0, s, 0.0, 2.0],
            [0.0, 0.0, s, 3.0],
        ]));
    }
    v.push(Mtx([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, 0.0],
    ]));
    v.push(Mtx([
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]));
    v.push(Mtx([
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, 0.0],
    ]));
    v.push(Mtx([
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0],
    ]));
    // Uniform random entries at several magnitudes.
    for _ in 0..1500 {
        v.push(rng.mtx(-4.0, 4.0));
    }
    for _ in 0..300 {
        v.push(rng.mtx(-100.0, 100.0));
    }
    for _ in 0..300 {
        v.push(rng.mtx(-0.01, 0.01));
    }
    // Proper SRT matrices (as jobj produces), with and without parent scale.
    for k in 0..800 {
        let scale = rng.vec(0.1, 3.0);
        let rot = rng.angles();
        let trans = rng.vec(-50.0, 50.0);
        let mut m = Mtx::ZERO;
        if k % 2 == 0 {
            mtx::hsd_mtx_srt(&mut m, &scale, &rot, &trans, None);
        } else {
            let q = rng.unit_quat();
            mtx::hsd_mtx_srt_quat(&mut m, &scale, &q, &trans, None);
        }
        v.push(m);
    }
    // Pure rotations.
    for _ in 0..400 {
        let mut m = Mtx::ZERO;
        mtx::hsd_mk_rotation_mtx(&mut m, &rng.angles());
        v.push(m);
    }
    // Singular: duplicate rows, zero row, zero column, rank 1.
    for _ in 0..60 {
        let mut m = rng.mtx(-4.0, 4.0);
        m.0[2] = m.0[1];
        v.push(m);
        let mut m = rng.mtx(-4.0, 4.0);
        m.0[0] = [0.0, 0.0, 0.0, m.0[0][3]];
        v.push(m);
        let mut m = rng.mtx(-4.0, 4.0);
        for r in m.0.iter_mut() {
            r[1] = 0.0;
        }
        v.push(m);
    }
    // Tiny-determinant (below HSD's 1e-10 but not exactly zero).
    for _ in 0..60 {
        let mut m = rng.mtx(-4.0, 4.0);
        m.0[2] = m.0[1];
        m.0[2][0] += 1e-6;
        v.push(m);
    }
    // Raw bit patterns (NaN, inf, denormals included).
    for _ in 0..200 {
        let mut m = Mtx::ZERO;
        for r in m.0.iter_mut() {
            for e in r.iter_mut() {
                *e = f32::from_bits(rng.u32());
            }
        }
        v.push(m);
    }
    v
}

fn vectors(rng: &mut Lcg) -> Vec<Vec3> {
    let mut v = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(3.0, 4.0, 0.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-0.0, -0.0, -0.0),
        Vec3::new(1e-20, 1e-20, 1e-20),
        Vec3::new(1e-39, 0.0, 0.0), // denormal: below FLT_MIN
        Vec3::new(1e19, 1e19, 1e19),
        Vec3::new(f32::INFINITY, 1.0, 1.0),
        Vec3::new(f32::NAN, 1.0, 1.0),
    ];
    for _ in 0..3000 {
        v.push(rng.vec(-4.0, 4.0));
    }
    for _ in 0..500 {
        v.push(rng.vec(-1000.0, 1000.0));
    }
    for _ in 0..500 {
        v.push(rng.vec(-1e-3, 1e-3));
    }
    for _ in 0..200 {
        v.push(Vec3::new(
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
        ));
    }
    v
}

fn quats(rng: &mut Lcg) -> Vec<Quaternion> {
    let mut v = vec![
        Quaternion::IDENTITY,
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Quaternion::new(0.0, 1.0, 0.0, 0.0),
        Quaternion::new(0.0, 0.0, 1.0, 0.0),
        Quaternion::new(0.5, 0.5, 0.5, 0.5),
        Quaternion::new(0.0, 0.0, 0.0, 2.0),
        Quaternion::new(0.0, 0.0, 0.0, 0.0),
    ];
    for _ in 0..2000 {
        v.push(rng.unit_quat());
    }
    for _ in 0..500 {
        v.push(rng.quat());
    }
    for _ in 0..200 {
        let q = rng.quat();
        v.push(Quaternion::new(
            q.x * 50.0,
            q.y * 50.0,
            q.z * 50.0,
            q.w * 50.0,
        ));
    }
    for _ in 0..100 {
        v.push(Quaternion::new(
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
            f32::from_bits(rng.u32()),
        ));
    }
    v
}

// --- op table ----------------------------------------------------------------

struct Op {
    name: &'static str,
    nin: usize,
    nout: usize,
    /// Rust side: `(input record) -> output record` (pre-filled with SENTINEL).
    rust: fn(&[f32], &mut [f32]),
    /// Input corpus for this op, flattened.
    inputs: fn(&mut Lcg) -> Vec<f32>,
}

fn flatten<I: IntoIterator<Item = Vec<f32>>>(it: I) -> Vec<f32> {
    it.into_iter().flatten().collect()
}

fn ops() -> Vec<Op> {
    vec![
        Op {
            name: "vec_sqmag",
            nin: 3,
            nout: 1,
            rust: |i, o| o[0] = mtx::vec_square_mag(&v_of(i)),
            inputs: |r| flatten(vectors(r).iter().map(v_words)),
        },
        Op {
            name: "vec_mag",
            nin: 3,
            nout: 1,
            rust: |i, o| o[0] = mtx::vec_mag(&v_of(i)),
            inputs: |r| flatten(vectors(r).iter().map(v_words)),
        },
        Op {
            name: "vec_normalize",
            nin: 3,
            nout: 3,
            rust: |i, o| {
                let mut d = Vec3::ZERO;
                mtx::vec_normalize(&v_of(i), &mut d);
                v_put(o, &d);
            },
            inputs: |r| flatten(vectors(r).iter().map(v_words)),
        },
        Op {
            name: "vec_dot",
            nin: 6,
            nout: 1,
            rust: |i, o| o[0] = mtx::vec_dot_product(&v_of(i), &v_of(&i[3..])),
            inputs: |r| {
                let vs = vectors(r);
                flatten(
                    vs.iter()
                        .zip(vs.iter().rev())
                        .map(|(a, b)| [v_words(a), v_words(b)].concat()),
                )
            },
        },
        Op {
            name: "vec_cross",
            nin: 6,
            nout: 3,
            rust: |i, o| {
                let mut d = Vec3::ZERO;
                mtx::vec_cross_product(&v_of(i), &v_of(&i[3..]), &mut d);
                v_put(o, &d);
            },
            inputs: |r| {
                let vs = vectors(r);
                flatten(
                    vs.iter()
                        .zip(vs.iter().rev())
                        .map(|(a, b)| [v_words(a), v_words(b)].concat()),
                )
            },
        },
        Op {
            name: "vec_addsubscale",
            nin: 7,
            nout: 9,
            rust: |i, o| {
                let (a, b) = (v_of(i), v_of(&i[3..]));
                let mut d = Vec3::ZERO;
                mtx::vec_add(&a, &b, &mut d);
                v_put(o, &d);
                mtx::vec_subtract(&a, &b, &mut d);
                v_put(&mut o[3..], &d);
                mtx::vec_scale(&a, &mut d, i[6]);
                v_put(&mut o[6..], &d);
            },
            inputs: |r| {
                let vs = vectors(r);
                let n = vs.len();
                flatten((0..n).map(|k| {
                    [
                        v_words(&vs[k]),
                        v_words(&vs[n - 1 - k]),
                        vec![r.range(-3.0, 3.0)],
                    ]
                    .concat()
                }))
            },
        },
        Op {
            name: "mtx_concat",
            nin: 24,
            nout: 12,
            rust: |i, o| {
                let mut ab = Mtx::ZERO;
                mtx::mtx_concat(&m_of(i), &m_of(&i[12..]), &mut ab);
                m_put(o, &ab);
            },
            inputs: |r| {
                let ms = matrices(r);
                flatten(
                    ms.iter()
                        .zip(ms.iter().rev())
                        .map(|(a, b)| [m_words(a), m_words(b)].concat()),
                )
            },
        },
        Op {
            name: "mtx_transpose",
            nin: 12,
            nout: 12,
            rust: |i, o| {
                let mut t = Mtx::ZERO;
                mtx::mtx_transpose(&m_of(i), &mut t);
                m_put(o, &t);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "mtx_inverse",
            nin: 12,
            nout: 13,
            rust: |i, o| {
                let mut inv = sentinel_mtx();
                o[0] = if mtx::mtx_inverse(&m_of(i), &mut inv) {
                    1.0
                } else {
                    0.0
                };
                m_put(&mut o[1..], &inv);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "mtx_invxpose",
            nin: 12,
            nout: 13,
            rust: |i, o| {
                let mut inv = sentinel_mtx();
                o[0] = if mtx::mtx_inv_xpose(&m_of(i), &mut inv) {
                    1.0
                } else {
                    0.0
                };
                m_put(&mut o[1..], &inv);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "mtx_quat",
            nin: 4,
            nout: 12,
            rust: |i, o| {
                let mut m = Mtx::ZERO;
                mtx::mtx_quat(&mut m, &q_of(i));
                m_put(o, &m);
            },
            inputs: |r| flatten(quats(r).iter().map(q_words)),
        },
        Op {
            name: "mtx_multvec",
            nin: 15,
            nout: 3,
            rust: |i, o| {
                let mut d = Vec3::ZERO;
                mtx::mtx_mult_vec(&m_of(i), &v_of(&i[12..]), &mut d);
                v_put(o, &d);
            },
            inputs: |r| {
                let ms = matrices(r);
                let vs = vectors(r);
                flatten(
                    ms.iter()
                        .zip(vs.iter().cycle())
                        .map(|(m, v)| [m_words(m), v_words(v)].concat()),
                )
            },
        },
        Op {
            name: "mtx_multvecsr",
            nin: 15,
            nout: 3,
            rust: |i, o| {
                let mut d = Vec3::ZERO;
                mtx::mtx_mult_vec_sr(&m_of(i), &v_of(&i[12..]), &mut d);
                v_put(o, &d);
            },
            inputs: |r| {
                let ms = matrices(r);
                let vs = vectors(r);
                flatten(
                    ms.iter()
                        .zip(vs.iter().cycle())
                        .map(|(m, v)| [m_words(m), v_words(v)].concat()),
                )
            },
        },
        Op {
            name: "mtx_rotrad",
            nin: 2,
            nout: 12,
            rust: |i, o| {
                let mut m = sentinel_mtx();
                mtx::mtx_rot_rad(&mut m, i[0] as u8, i[1]);
                m_put(o, &m);
            },
            inputs: |r| {
                let mut v = Vec::new();
                for axis in [b'x', b'y', b'z', b'X', b'Y', b'Z', b'w', 0u8] {
                    for _ in 0..300 {
                        v.push(axis as f32);
                        v.push(r.range(-7.0, 7.0));
                    }
                }
                v
            },
        },
        Op {
            name: "mtx_scale_trans",
            nin: 3,
            nout: 24,
            rust: |i, o| {
                let mut m = Mtx::ZERO;
                mtx::mtx_scale(&mut m, i[0], i[1], i[2]);
                m_put(o, &m);
                mtx::mtx_trans(&mut m, i[0], i[1], i[2]);
                m_put(&mut o[12..], &m);
            },
            inputs: |r| flatten(vectors(r).iter().map(v_words)),
        },
        Op {
            name: "hsd_inverse",
            nin: 12,
            nout: 12,
            rust: |i, o| {
                let mut d = Mtx::ZERO;
                mtx::hsd_mtx_inverse(&m_of(i), &mut d);
                m_put(o, &d);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "hsd_inverse_concat",
            nin: 24,
            nout: 12,
            rust: |i, o| {
                let mut d = Mtx::ZERO;
                mtx::hsd_mtx_inverse_concat(&m_of(i), &m_of(&i[12..]), &mut d);
                m_put(o, &d);
            },
            inputs: |r| {
                let ms = matrices(r);
                flatten(
                    ms.iter()
                        .zip(ms.iter().rev())
                        .map(|(a, b)| [m_words(a), m_words(b)].concat()),
                )
            },
        },
        Op {
            name: "hsd_inverse_transpose",
            nin: 12,
            nout: 12,
            rust: |i, o| {
                let mut d = Mtx::ZERO;
                mtx::hsd_mtx_inverse_transpose(&m_of(i), &mut d);
                m_put(o, &d);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "hsd_get_rotation",
            nin: 12,
            nout: 3,
            rust: |i, o| {
                let mut v = Vec3::new(SENTINEL, SENTINEL, SENTINEL);
                mtx::hsd_mtx_get_rotation::<StubTrig>(&m_of(i), &mut v);
                v_put(o, &v);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "hsd_get_translate",
            nin: 12,
            nout: 3,
            rust: |i, o| {
                let mut v = Vec3::ZERO;
                mtx::hsd_mtx_get_translate(&m_of(i), &mut v);
                v_put(o, &v);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "hsd_get_scale",
            nin: 12,
            nout: 3,
            rust: |i, o| {
                let mut v = Vec3::ZERO;
                mtx::hsd_mtx_get_scale(&m_of(i), &mut v);
                v_put(o, &v);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "hsd_mk_rotation",
            nin: 3,
            nout: 12,
            rust: |i, o| {
                let mut m = Mtx::ZERO;
                mtx::hsd_mk_rotation_mtx(&mut m, &v_of(i));
                m_put(o, &m);
            },
            inputs: |r| {
                let mut v = flatten((0..3000).map(|_| v_words(&r.angles())));
                v.extend(flatten(vectors(r).iter().map(v_words)));
                v
            },
        },
        Op {
            name: "hsd_mtx_quat",
            nin: 4,
            nout: 12,
            rust: |i, o| {
                let mut m = Mtx::ZERO;
                mtx::hsd_mtx_quat(&mut m, &q_of(i));
                m_put(o, &m);
            },
            inputs: |r| flatten(quats(r).iter().map(q_words)),
        },
        Op {
            name: "hsd_srt",
            nin: 13,
            nout: 12,
            rust: |i, o| {
                let mut m = Mtx::ZERO;
                let v4 = v_of(&i[10..]);
                mtx::hsd_mtx_srt(
                    &mut m,
                    &v_of(i),
                    &v_of(&i[3..]),
                    &v_of(&i[6..]),
                    if i[9] != 0.0 { Some(&v4) } else { None },
                );
                m_put(o, &m);
            },
            inputs: |r| {
                flatten((0..4000).map(|k| {
                    [
                        v_words(&r.vec(0.05, 4.0)),
                        v_words(&r.angles()),
                        v_words(&r.vec(-50.0, 50.0)),
                        vec![(k % 2) as f32],
                        v_words(&r.vec(0.2, 3.0)),
                    ]
                    .concat()
                }))
            },
        },
        Op {
            name: "hsd_srt_quat",
            nin: 14,
            nout: 12,
            rust: |i, o| {
                let mut m = Mtx::ZERO;
                let v4 = v_of(&i[11..]);
                mtx::hsd_mtx_srt_quat(
                    &mut m,
                    &v_of(i),
                    &q_of(&i[3..]),
                    &v_of(&i[7..]),
                    if i[10] != 0.0 { Some(&v4) } else { None },
                );
                m_put(o, &m);
            },
            inputs: |r| {
                flatten((0..4000).map(|k| {
                    let q = if k % 3 == 0 { r.quat() } else { r.unit_quat() };
                    [
                        v_words(&r.vec(0.05, 4.0)),
                        q_words(&q),
                        v_words(&r.vec(-50.0, 50.0)),
                        vec![(k % 2) as f32],
                        v_words(&r.vec(0.2, 3.0)),
                    ]
                    .concat()
                }))
            },
        },
        Op {
            name: "hsd_scaled_add",
            nin: 25,
            nout: 12,
            rust: |i, o| {
                let mut d = Mtx::ZERO;
                mtx::hsd_mtx_scaled_add(&m_of(i), &m_of(&i[12..]), &mut d, i[24]);
                m_put(o, &d);
            },
            inputs: |r| {
                let ms = matrices(r);
                let n = ms.len();
                flatten((0..n).map(|k| {
                    [
                        m_words(&ms[k]),
                        m_words(&ms[n - 1 - k]),
                        vec![r.range(-1.5, 1.5)],
                    ]
                    .concat()
                }))
            },
        },
        Op {
            name: "mat_to_quat",
            nin: 12,
            nout: 4,
            rust: |i, o| {
                let mut q = Quaternion::default();
                quat::mat_to_quat(&m_of(i), &mut q);
                q_put(o, &q);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "mtx_to_euler",
            nin: 12,
            nout: 3,
            rust: |i, o| {
                let mut e = Vec3::ZERO;
                quat::mtx_to_euler::<StubTrig>(&m_of(i), &mut e);
                v_put(o, &e);
            },
            inputs: |r| flatten(matrices(r).iter().map(m_words)),
        },
        Op {
            name: "quat_mul",
            nin: 8,
            nout: 4,
            rust: |i, o| {
                let mut out = Quaternion::default();
                quat::quat_mul(&q_of(i), &q_of(&i[4..]), &mut out);
                q_put(o, &out);
            },
            inputs: |r| {
                let qs = quats(r);
                flatten(
                    qs.iter()
                        .zip(qs.iter().rev())
                        .map(|(a, b)| [q_words(a), q_words(b)].concat()),
                )
            },
        },
        Op {
            name: "quat_axis_angle",
            nin: 4,
            nout: 5,
            rust: |i, o| {
                let mut q = Quaternion::new(SENTINEL, SENTINEL, SENTINEL, SENTINEL);
                o[0] = if quat::quat_from_axis_angle(&v_of(i), &mut q, i[3]) {
                    1.0
                } else {
                    0.0
                };
                q_put(&mut o[1..], &q);
            },
            inputs: |r| {
                let vs = vectors(r);
                flatten(
                    vs.iter()
                        .map(|v| [v_words(v), vec![r.range(-7.0, 7.0)]].concat()),
                )
            },
        },
        Op {
            name: "euler_to_quat",
            nin: 3,
            nout: 4,
            rust: |i, o| {
                let mut q = Quaternion::default();
                quat::euler_to_quat(&v_of(i), &mut q);
                q_put(o, &q);
            },
            inputs: |r| {
                let mut v = flatten((0..3000).map(|_| v_words(&r.angles())));
                v.extend(flatten(vectors(r).iter().map(v_words)));
                v
            },
        },
        Op {
            name: "quat_slerp",
            nin: 9,
            nout: 4,
            rust: |i, o| {
                let mut out = Quaternion::default();
                quat::quat_slerp::<StubTrig>(&q_of(i), &q_of(&i[4..]), &mut out, i[8]);
                q_put(o, &out);
            },
            inputs: |r| {
                let mut v = Vec::new();
                // General pairs, t spanning a little beyond [0, 1].
                for _ in 0..2500 {
                    v.extend(q_words(&r.unit_quat()));
                    v.extend(q_words(&r.unit_quat()));
                    v.push(r.range(-0.25, 1.25));
                }
                // Exact t = 0, 0.5, 1 and the branch split at 0.5.
                for t in [0.0f32, 0.5, 1.0, 0.25, 0.75, 0.49999997, 0.50000006] {
                    for _ in 0..100 {
                        let p = r.unit_quat();
                        let q = r.unit_quat();
                        v.extend(q_words(&p));
                        v.extend(q_words(&q));
                        v.push(t);
                        // Nearly opposite: perpendicular branch.
                        let e = r.range(-1e-6, 1e-6);
                        v.extend(q_words(&p));
                        v.extend(q_words(&Quaternion::new(-p.x + e, -p.y, -p.z, -p.w)));
                        v.push(t);
                        // Nearly equal: linear branch.
                        v.extend(q_words(&p));
                        v.extend(q_words(&Quaternion::new(p.x + e, p.y, p.z, p.w)));
                        v.push(t);
                        // Exactly opposite / exactly equal.
                        v.extend(q_words(&p));
                        v.extend(q_words(&Quaternion::new(-p.x, -p.y, -p.z, -p.w)));
                        v.push(t);
                        v.extend(q_words(&p));
                        v.extend(q_words(&p));
                        v.push(t);
                    }
                }
                // Unnormalised inputs.
                for _ in 0..500 {
                    v.extend(q_words(&r.quat()));
                    v.extend(q_words(&r.quat()));
                    v.push(r.range(0.0, 1.0));
                }
                v
            },
        },
    ]
}

// --- comparison --------------------------------------------------------------

struct Mismatches {
    op: &'static str,
    count: usize,
    nan_payload_only: usize,
    samples: Vec<String>,
}

impl Mismatches {
    fn check(&mut self, record: usize, word: usize, input: &[f32], rust: f32, c: f32) {
        if rust.to_bits() == c.to_bits() {
            return;
        }
        if rust.is_nan() && c.is_nan() {
            self.nan_payload_only += 1;
            return;
        }
        self.count += 1;
        if self.samples.len() < 8 {
            let inp: Vec<String> = input.iter().map(|x| format!("{x:e}")).collect();
            self.samples.push(format!(
                "{}[{record}] word {word}: rust {:08x} ({rust:e}) vs c {:08x} ({c:e}); input [{}]",
                self.op,
                rust.to_bits(),
                c.to_bits(),
                inp.join(", ")
            ));
        }
    }
}

#[test]
fn mtx_and_quat_match_native_c_bit_for_bit() {
    compare_sweep(false);
}

#[test]
fn fusion_changes_results_within_the_sweep() {
    compare_sweep(true);
}

#[test]
fn ref_sources_match_submodule() {
    let baselib = decomp_dir().join("src/sysdolphin/baselib");
    if !baselib.join("mtx.c").exists() {
        eprintln!("[NON-DATA OMITTED] melee-decomp submodule not present; omitting copy check");
        return;
    }
    for file in ["mtx.c", "quatlib.c"] {
        assert_eq!(
            std::fs::read(ref_dir().join(file)).unwrap(),
            std::fs::read(baselib.join(file)).unwrap(),
            "{file} differs from the submodule; re-copy and re-audit"
        );
    }
}

fn compare_sweep(report_fusion: bool) {
    let Some(exe) = build_oracle(true) else {
        return;
    };
    let verbatim = report_fusion.then(|| build_oracle(false).expect("build verbatim oracle"));
    let mut changed_total = 0;
    let mut input_total = 0;

    let mut failures = Vec::new();
    for (k, op) in ops().iter().enumerate() {
        let mut rng = Lcg(0x4D74_7800 + k as u64);
        let input = (op.inputs)(&mut rng);
        assert!(
            !input.is_empty() && input.len() % op.nin == 0,
            "{}: bad corpus",
            op.name
        );
        let n = input.len() / op.nin;
        assert!(n >= 1000, "{}: only {n} records", op.name);

        let c_out = run_oracle(&exe, op.name, &input);
        assert_eq!(c_out.len(), n * op.nout, "{}: output length", op.name);

        if let Some(verbatim) = &verbatim {
            let old = run_oracle(verbatim, op.name, &input);
            assert_eq!(old.len(), c_out.len());
            let changed = c_out
                .chunks_exact(op.nout)
                .zip(old.chunks_exact(op.nout))
                .filter(|(a, b)| {
                    a.iter()
                        .zip(b.iter())
                        .any(|(a, b)| a.to_bits() != b.to_bits())
                })
                .count();
            eprintln!(
                "{}: retail and verbatim C differ on {changed} of {n} inputs",
                op.name
            );
            changed_total += changed;
            input_total += n;
        }
        let mut mm = Mismatches {
            op: op.name,
            count: 0,
            nan_payload_only: 0,
            samples: Vec::new(),
        };
        let mut r_out = vec![SENTINEL; op.nout];
        for rec in 0..n {
            let inp = &input[rec * op.nin..(rec + 1) * op.nin];
            r_out.iter_mut().for_each(|x| *x = SENTINEL);
            (op.rust)(inp, &mut r_out);
            let c = &c_out[rec * op.nout..(rec + 1) * op.nout];
            for (w, (rv, cv)) in r_out.iter().zip(c.iter()).enumerate() {
                mm.check(rec, w, inp, *rv, *cv);
            }
        }
        eprintln!(
            "{}: {n} records, {} mismatches, {} NaN-payload-only differences",
            op.name, mm.count, mm.nan_payload_only
        );
        if mm.count != 0 {
            failures.push(format!(
                "{} ({} mismatches):\n  {}",
                op.name,
                mm.count,
                mm.samples.join("\n  ")
            ));
        }
    }
    if report_fusion {
        eprintln!("matrix/quaternion total: {changed_total} of {input_total} inputs changed");
    }
    assert!(
        failures.is_empty(),
        "bit mismatches vs native C:\n{}",
        failures.join("\n")
    );
}
