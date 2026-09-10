//! The warp's type-2 spline: spline.c splArcLengthPoint (0x803792C8).
//! Temporary ef dependency; move the evaluator to HSD when the general loader
//! gains spline/animation-reference ownership. Unsupported spline kinds fail.
use anyhow::{ensure, Context, Result};
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::sqrtf,
};
use hsd_archive::Archive;
use hsd_types::Vec3;

#[derive(Clone)]
pub(super) struct Spline {
    count: usize,
    points: Vec<[f32; 3]>,
    length: f32,
    segments: Vec<f32>,
    polynomials: Vec<[f32; 5]>,
}
impl Spline {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = archive.reader();
        ensure!(r.u8(offset)? == 2, "only the warp's B-spline is supported");
        let count = usize::try_from(r.s16(offset + 2)?)?;
        ensure!(count > 1, "invalid spline control-point count");
        let points = archive.link(offset + 8)?.context("spline control points")?;
        let segments = archive.link(offset + 16)?.context("spline segments")?;
        let polynomials = archive
            .link(offset + 20)?
            .context("spline arc polynomials")?;
        let floats = |base, n| {
            (0..n)
                .map(|i| r.f32(base + i as u32 * 4))
                .collect::<std::result::Result<Vec<_>, _>>()
        };
        Ok(Self {
            count,
            points: floats(points, (count + 2) * 3)?
                .chunks_exact(3)
                .map(|v| v.try_into().unwrap())
                .collect(),
            length: r.f32(offset + 12)?,
            segments: floats(segments, count)?,
            polynomials: floats(polynomials, (count - 1) * 5)?
                .chunks_exact(5)
                .map(|v| v.try_into().unwrap())
                .collect(),
        })
    }
    /// splArcLengthGetParameter, spline.c:178-233 (0x80378F40).
    fn parameter(&self, value: f32) -> f32 {
        if value <= 0.0 {
            return 0.0;
        }
        if value >= 1.0 {
            return 1.0;
        }
        let mut index = 0;
        while self.segments[index + 1] < value {
            index += 1;
        }
        let mut remaining = self.length * (value - self.segments[index]);
        let (mut start, mut end) = (0.0_f32, 1.0_f32);
        let mut result = 0.0;
        while (start - end).abs() >= 0.00001 {
            // 80379064..88: fadds, fmuls by 0.5 and 0.125, fadds.
            result = (start + end) * 0.5;
            let dx = (result - start) * 0.125;
            let mut t = start + dx;
            let mut middle = 0.0;
            for i in 2..=8 {
                // 803790E0/80379134: fmadds; not a separate sum/product.
                middle = fmadds(
                    if i & 1 == 0 { 4.0 } else { 2.0 },
                    polynomial(&self.polynomials[index], t),
                    middle,
                );
                t += dx;
            }
            // 803791E4..F4: two adds, multiply, divide, add (no fusion).
            let area = dx
                * ((middle + polynomial(&self.polynomials[index], start))
                    + polynomial(&self.polynomials[index], result))
                / 3.0;
            if remaining < 0.00001 + area {
                end = result;
            } else {
                start = result;
                remaining -= area;
            }
        }
        (result + index as f32) / (self.count as f32 - 1.0)
    }
    pub fn point(&self, value: f32) -> Vec3 {
        let u = self.parameter(value);
        let (index, t) = if u < 1.0 {
            let t = u * (self.count - 1) as f32;
            (t as usize, t - t as usize as f32)
        } else {
            (self.count - 2, 1.0)
        };
        // splGetSplinePoint B-spline arm, 80378C34..CF8 (endpoint E44..EF8).
        let t2 = t * t;
        let t3 = t2 * t;
        let inv = 1.0 - t;
        let sixth = 1.0_f32 / 6.0;
        let b0 = inv * (inv * (sixth * inv));
        let b1 = sixth * (4.0 + fmsubs(3.0, t3, 6.0 * t2)); // 80378C5C
        let b2 = sixth * fmadds(3.0, t + (-t3 + t2), 1.0); // 80378C9C
        let b3 = sixth * t3;
        let p = &self.points[index..index + 4];
        let component = |axis| {
            // 80378CA8/CAC/CB0, repeated for Y/Z: start with cp1*b1.
            fmadds(
                p[3][axis],
                b3,
                fmadds(p[2][axis], b2, fmadds(p[0][axis], b0, p[1][axis] * b1)),
            )
        };
        Vec3::new(component(0), component(1), component(2))
    }
}
fn polynomial(c: &[f32; 5], t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    // 803790B4..C4 (and 80379108..18/168..78/1B8..C8).
    let mut result = c[4] + fmadds(c[3], t, fmadds(c[2], t2, fmadds(c[0], t4, c[1] * t3)));
    if result < 0.0 && result > -0.001 {
        result = 0.0;
    }
    sqrtf(result)
}
