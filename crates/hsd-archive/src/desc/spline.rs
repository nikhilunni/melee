//! HSD_Spline (spline.h), linear path data used by JObj PATH tracks.
use super::{DescError, Result, Vec3};
use crate::{reader::add_offset, Archive};

#[derive(Clone, Debug, PartialEq)]
pub struct LinearSpline {
    pub points: Vec<Vec3>,
    pub lengths: Vec<f32>,
}
impl LinearSpline {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = archive.reader();
        r.slice(offset, 0x18)?;
        let count = r.s16(add_offset(offset, 2)?)?;
        if r.u8(offset)? != 0 || count < 2 {
            return Err(DescError::InvalidSpline {
                offset,
                reason: "requires a linear spline with at least two points",
            });
        }
        let points =
            archive
                .link(add_offset(offset, 8)?)?
                .ok_or_else(|| DescError::NullPointer {
                    field: "Spline.cv",
                    at: offset + 8,
                })?;
        let lengths =
            archive
                .link(add_offset(offset, 16)?)?
                .ok_or_else(|| DescError::NullPointer {
                    field: "Spline.segLength",
                    at: offset + 16,
                })?;
        let spline = Self {
            points: (0..count as u32)
                .map(|i| {
                    let p = add_offset(points, i * 12)?;
                    Ok(Vec3 {
                        x: r.f32(p)?,
                        y: r.f32(add_offset(p, 4)?)?,
                        z: r.f32(add_offset(p, 8)?)?,
                    })
                })
                .collect::<Result<_>>()?,
            lengths: (0..count as u32)
                .map(|i| Ok(r.f32(add_offset(lengths, i * 4)?)?))
                .collect::<Result<_>>()?,
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
