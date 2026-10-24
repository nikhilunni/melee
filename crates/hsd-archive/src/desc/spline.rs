//! HSD_Spline (spline.h): path data used by JObj PATH tracks and by stage
//! code that walks a path (Ground_801C247C).
use super::{DescError, Result, Vec3};
use crate::{reader::add_offset, Archive};

/// `HSD_Spline.type`: how the control points make the curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplineKind {
    /// 0: straight segments between the points.
    Linear,
    /// 1: cubic Bezier segments, three points apart.
    Bezier,
    /// 2: uniform cubic B-spline; the curve has two more points than
    /// segments plus one.
    BSpline,
    /// 3: cardinal spline with the spline's tension.
    Cardinal,
}

/// `HSD_Spline` (0x18 bytes): type, numcv, tension, cv, totalLength,
/// segLength, segPoly.
#[derive(Clone, Debug, PartialEq)]
pub struct Spline {
    pub kind: SplineKind,
    /// +0x2 `numcv`: the knots the arc length is measured over.
    pub knots: usize,
    /// +0x4.
    pub tension: f32,
    /// +0x8 `cv`: every control point the evaluation reads.
    pub points: Vec<Vec3>,
    /// +0xC `totalLength`.
    pub total_length: f32,
    /// +0x10 `segLength`: each knot's share of the length, from 0 to 1.
    pub lengths: Vec<f32>,
    /// +0x14 `segPoly`: per segment, the speed-squared polynomial's five
    /// coefficients (t^4 first) the arc length integrates.
    pub polynomials: Vec<[f32; 5]>,
}
/// The linear paths stage lights and older code read.
pub type LinearSpline = Spline;

impl Spline {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = archive.reader();
        r.slice(offset, 0x18)?;
        let kind = match r.u8(offset)? {
            0 => SplineKind::Linear,
            1 => SplineKind::Bezier,
            2 => SplineKind::BSpline,
            3 => SplineKind::Cardinal,
            _ => {
                return Err(DescError::InvalidSpline {
                    offset,
                    reason: "unknown spline type",
                })
            }
        };
        let count = r.s16(add_offset(offset, 2)?)?;
        if count < 2 {
            return Err(DescError::InvalidSpline {
                offset,
                reason: "requires a spline with at least two points",
            });
        }
        let knots = count as usize;
        // splGetSplinePoint's reads: Bezier segments share their ends, the
        // B-spline and cardinal windows reach two points past the knots.
        let point_count = match kind {
            SplineKind::Linear => knots,
            SplineKind::Bezier => 3 * (knots - 1) + 1,
            SplineKind::BSpline | SplineKind::Cardinal => knots + 2,
        };
        let link = |at: u32, field: &'static str| -> Result<u32> {
            archive
                .link(add_offset(offset, at)?)?
                .ok_or(DescError::NullPointer {
                    field,
                    at: offset + at,
                })
        };
        let points = link(8, "Spline.cv")?;
        let lengths = link(16, "Spline.segLength")?;
        let polynomials = match kind {
            SplineKind::Linear => None,
            _ => Some(link(20, "Spline.segPoly")?),
        };
        let spline = Self {
            kind,
            knots,
            tension: r.f32(add_offset(offset, 4)?)?,
            points: (0..point_count as u32)
                .map(|i| {
                    let p = add_offset(points, i * 12)?;
                    Ok(Vec3 {
                        x: r.f32(p)?,
                        y: r.f32(add_offset(p, 4)?)?,
                        z: r.f32(add_offset(p, 8)?)?,
                    })
                })
                .collect::<Result<_>>()?,
            total_length: r.f32(add_offset(offset, 12)?)?,
            lengths: (0..knots as u32)
                .map(|i| Ok(r.f32(add_offset(lengths, i * 4)?)?))
                .collect::<Result<_>>()?,
            polynomials: match polynomials {
                None => Vec::new(),
                Some(base) => (0..knots as u32 - 1)
                    .map(|segment| {
                        let mut c = [0.0; 5];
                        for (i, c) in c.iter_mut().enumerate() {
                            *c = r.f32(add_offset(base, segment * 20 + i as u32 * 4)?)?;
                        }
                        Ok(c)
                    })
                    .collect::<Result<_>>()?,
            },
        };
        if spline.lengths.first() != Some(&0.0)
            || spline.lengths.last() != Some(&1.0)
            || spline.lengths.iter().any(|v| !v.is_finite())
            || spline.lengths.windows(2).any(|p| p[0] > p[1])
        {
            return Err(DescError::InvalidSpline {
                offset,
                reason: "segment fractions must increase from zero to one",
            });
        }
        Ok(spline)
    }
}
