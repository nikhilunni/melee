//! HSD_PObjDesc / HSD_VtxDescList (pobj.h), GX display lists, and envelopes.
//! This decodes data only: matrix evaluation and skinning belong to presentation.
use super::{invalid, unsupported, Result};
use crate::{add_offset, Archive, Reader};
use std::collections::BTreeSet;

/// A decoded vertex. `matrix` selects a polygon-local matrix palette entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4],
    pub matrix: u8,
}
impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0; 2],
            color: [255; 4],
            matrix: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Influence {
    /// Archive joint identifier, to resolve against the model's joint table.
    pub joint: u32,
    pub weight: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum MatrixBinding {
    /// The joint that owns the containing display object.
    Owner,
    Joint(u32),
    Envelope(Vec<Influence>),
}
#[derive(Clone, Debug)]
pub struct Polygon {
    pub flags: u16,
    pub vertices: Vec<Vertex>,
    /// Triangle list, with original winding preserved.
    pub indices: Vec<u32>,
    pub matrices: Vec<MatrixBinding>,
}
#[derive(Clone, Debug)]
struct Attribute {
    pub semantic: u32,
    pub encoding: u32,
    pub components: u32,
    pub component_type: u32,
    pub fractional_bits: u8,
    pub stride: u16,
    pub array: Option<u32>,
}

/// Read a polygon chain without following its joint references recursively.
/// Reject cycles, malformed arrays, and unsupported commands explicitly.
pub fn read_polygons(archive: &Archive, head: Option<u32>) -> Result<Vec<Polygon>> {
    let r = Reader::new(archive.data());
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = head;
    while let Some(offset) = next {
        if !seen.insert(offset) {
            return Err(invalid(offset, "polygon chain cycle"));
        }
        r.slice(offset, 24)?;
        let attributes = attributes(archive, archive.link(add_offset(offset, 8)?)?)?;
        let flags = r.u16(add_offset(offset, 12)?)?;
        let length = u32::from(r.u16(add_offset(offset, 14)?)?) * 32;
        let display = archive
            .link(add_offset(offset, 16)?)?
            .ok_or_else(|| invalid(offset, "missing display list"))?;
        let binding = archive.link(add_offset(offset, 20)?)?;
        let matrices = match flags & 0x3000 {
            0 => {
                let mut list = vec![MatrixBinding::Owner];
                if let Some(joint) = binding {
                    list.push(MatrixBinding::Joint(joint));
                }
                list
            }
            0x2000 => envelopes(archive, binding)?,
            value => {
                return Err(unsupported(
                    offset,
                    "polygon skinning mode",
                    u32::from(value),
                ))
            }
        };
        let (vertices, indices) = display_list(archive, r.slice(display, length)?, &attributes)?;
        if vertices
            .iter()
            .any(|v| usize::from(v.matrix) >= matrices.len())
        {
            return Err(invalid(
                offset,
                "vertex matrix is outside the polygon palette",
            ));
        }
        result.push(Polygon {
            flags,
            vertices,
            indices,
            matrices,
        });
        next = archive.link(add_offset(offset, 4)?)?;
    }
    Ok(result)
}
fn attributes(archive: &Archive, mut next: Option<u32>) -> Result<Vec<Attribute>> {
    let r = Reader::new(archive.data());
    let mut result = Vec::new();
    while let Some(offset) = next {
        let semantic = r.u32(offset)?;
        if semantic == 255 {
            if !result
                .iter()
                .any(|a: &Attribute| a.semantic == 9 && a.encoding != 0)
            {
                return Err(invalid(offset, "missing position attribute"));
            }
            return Ok(result);
        }
        if result.len() == 32 {
            return Err(invalid(offset, "unterminated vertex attributes"));
        }
        r.slice(offset, 24)?;
        result.push(Attribute {
            semantic,
            encoding: r.u32(offset + 4)?,
            components: r.u32(offset + 8)?,
            component_type: r.u32(offset + 12)?,
            fractional_bits: r.u8(offset + 16)?,
            stride: r.u16(offset + 18)?,
            array: archive.link(offset + 20)?,
        });
        next = Some(add_offset(offset, 24)?);
    }
    Err(invalid(0, "missing vertex attributes"))
}
fn envelopes(archive: &Archive, table: Option<u32>) -> Result<Vec<MatrixBinding>> {
    let r = Reader::new(archive.data());
    let mut cursor = table.ok_or_else(|| invalid(0, "missing envelope palette"))?;
    let mut result = Vec::new();
    while let Some(mut entry) = archive.link(cursor)? {
        if result.len() == 10 {
            return Err(invalid(cursor, "more than ten position matrices"));
        }
        let mut influences = Vec::new();
        while let Some(joint) = archive.link(entry)? {
            let weight = r.f32(add_offset(entry, 4)?)?;
            if !weight.is_finite() || weight < 0.0 {
                return Err(invalid(entry, "invalid envelope weight"));
            }
            influences.push(Influence { joint, weight });
            entry = add_offset(entry, 8)?;
        }
        if influences.is_empty() {
            return Err(invalid(entry, "empty envelope"));
        }
        result.push(MatrixBinding::Envelope(influences));
        cursor = add_offset(cursor, 4)?;
    }
    Ok(result)
}
fn scalar(r: Reader<'_>, offset: u32, ty: u32, frac: u8) -> Result<(f32, u32)> {
    if frac > 31 {
        return Err(invalid(offset, "invalid fractional width"));
    }
    let (value, width) = match ty {
        0 => (f32::from(r.u8(offset)?), 1),
        1 => (f32::from(r.s8(offset)?), 1),
        2 => (f32::from(r.u16(offset)?), 2),
        3 => (f32::from(r.s16(offset)?), 2),
        4 => return Ok((r.f32(offset)?, 4)),
        _ => return Err(unsupported(offset, "vertex component type", ty)),
    };
    // GX fixed-point storage conversion by an exact power of two, not gameplay math.
    Ok((value / (1_u64 << frac) as f32, width))
}
fn color(r: Reader<'_>, offset: u32, ty: u32) -> Result<([u8; 4], u32)> {
    Ok(match ty {
        0 => (super::color::rgb565(r.u16(offset)?), 2),
        1 | 2 => (
            [r.u8(offset)?, r.u8(offset + 1)?, r.u8(offset + 2)?, 255],
            if ty == 1 { 3 } else { 4 },
        ),
        3 => {
            let v = r.u16(offset)?;
            (
                std::array::from_fn(|i| (((v >> (12 - 4 * i)) & 15) * 17) as u8),
                2,
            )
        }
        4 => {
            let bytes = r.slice(offset, 3)?;
            let v = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
            (
                std::array::from_fn(|i| {
                    let c = (v >> (18 - 6 * i)) & 63;
                    ((c << 2) | (c >> 4)) as u8
                }),
                3,
            )
        }
        5 => (r.array(offset)?, 4),
        _ => return Err(unsupported(offset, "vertex color type", ty)),
    })
}
fn read_attribute(r: Reader<'_>, offset: u32, a: &Attribute, v: &mut Vertex) -> Result<u32> {
    match a.semantic {
        0..=8 => {
            let index = r.u8(offset)?;
            if a.semantic == 0 {
                if index % 3 != 0 {
                    return Err(invalid(offset, "unaligned matrix index"));
                }
                v.matrix = index / 3;
            }
            Ok(1)
        }
        11 | 12 => {
            let (rgba, size) = color(r, offset, a.component_type)?;
            if a.semantic == 11 {
                v.color = rgba;
            }
            Ok(size)
        }
        // GX_VA_NBT (25) stores the normal, binormal and tangent together.
        9 | 10 | 13..=20 | 25 => {
            let n = match a.semantic {
                9 => match a.components {
                    0 => 2,
                    1 => 3,
                    _ => return Err(invalid(offset, "invalid position component count")),
                },
                10 => match a.components {
                    0 => 3,
                    1 => 9,
                    _ => return Err(unsupported(offset, "normal component count", a.components)),
                },
                25 => match a.components {
                    0 | 1 => 9,
                    _ => return Err(unsupported(offset, "NBT component count", a.components)),
                },
                _ => match a.components {
                    0 => 1,
                    1 => 2,
                    _ => return Err(invalid(offset, "invalid texture coordinate count")),
                },
            };
            let mut size = 0;
            for i in 0..n {
                let frac = if matches!(a.semantic, 10 | 25) {
                    match a.component_type {
                        1 => 6,
                        3 => 14,
                        _ => 0,
                    }
                } else {
                    a.fractional_bits
                };
                let (value, width) = scalar(r, add_offset(offset, size)?, a.component_type, frac)?;
                if !value.is_finite() {
                    return Err(invalid(offset, "nonfinite vertex component"));
                }
                if a.semantic == 9 {
                    v.position[i] = value;
                }
                if matches!(a.semantic, 10 | 25) && i < 3 {
                    v.normal[i] = value;
                }
                if a.semantic == 13 {
                    v.uv[i] = value;
                }
                size += width;
            }
            Ok(size)
        }
        _ => Err(unsupported(offset, "vertex attribute", a.semantic)),
    }
}
fn display_list(
    archive: &Archive,
    data: &[u8],
    attrs: &[Attribute],
) -> Result<(Vec<Vertex>, Vec<u32>)> {
    let r = Reader::new(data);
    let mut cursor = 0;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    while (cursor as usize) < data.len() {
        let command = r.u8(cursor)?;
        cursor += 1;
        if command == 0 {
            continue;
        }
        let primitive = command & 0xf8;
        // GX_POINTS (0xb8) keeps its vertices without triangles.
        if !matches!(primitive, 0x80 | 0x90 | 0x98 | 0xa0 | 0xb8) {
            return Err(unsupported(cursor - 1, "GX primitive", u32::from(command)));
        }
        let count = u32::from(r.u16(cursor)?);
        cursor += 2;
        let start = vertices.len() as u32;
        for _ in 0..count {
            let mut vertex = Vertex::default();
            for attr in attrs {
                match attr.encoding {
                    0 => {}
                    1 => {
                        cursor = add_offset(cursor, read_attribute(r, cursor, attr, &mut vertex)?)?
                    }
                    2 | 3 => {
                        let index = if attr.encoding == 2 {
                            let v = u32::from(r.u8(cursor)?);
                            cursor += 1;
                            v
                        } else {
                            let v = u32::from(r.u16(cursor)?);
                            cursor += 2;
                            v
                        };
                        let base = attr
                            .array
                            .ok_or_else(|| invalid(cursor, "indexed attribute has no array"))?;
                        let offset = add_offset(
                            base,
                            index
                                .checked_mul(u32::from(attr.stride))
                                .ok_or_else(|| invalid(base, "attribute index overflow"))?,
                        )?;
                        read_attribute(Reader::new(archive.data()), offset, attr, &mut vertex)?;
                    }
                    value => return Err(unsupported(cursor, "vertex attribute encoding", value)),
                }
            }
            vertices.push(vertex);
        }
        triangulate(primitive, start, count, &mut indices)?;
    }
    Ok((vertices, indices))
}
fn triangulate(primitive: u8, start: u32, count: u32, out: &mut Vec<u32>) -> Result<()> {
    match primitive {
        0x80 if count.is_multiple_of(4) => {
            for i in (0..count).step_by(4) {
                out.extend([
                    start + i,
                    start + i + 1,
                    start + i + 2,
                    start + i,
                    start + i + 2,
                    start + i + 3,
                ]);
            }
        }
        0x90 if count.is_multiple_of(3) => out.extend(start..start + count),
        0x98 if count >= 3 => {
            for i in 2..count {
                out.extend(if i % 2 == 0 {
                    [start + i - 2, start + i - 1, start + i]
                } else {
                    [start + i - 1, start + i - 2, start + i]
                });
            }
        }
        0xa0 if count >= 3 => {
            for i in 2..count {
                out.extend([start, start + i - 1, start + i]);
            }
        }
        0xb8 => {}
        _ => return Err(invalid(start, "invalid primitive vertex count")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strip_and_fan_keep_winding_and_primitive_boundaries() {
        let mut out = Vec::new();
        triangulate(0x98, 7, 5, &mut out).unwrap();
        assert_eq!(out, [7, 8, 9, 9, 8, 10, 9, 10, 11]);
        out.clear();
        triangulate(0xa0, 2, 5, &mut out).unwrap();
        assert_eq!(out, [2, 3, 4, 2, 4, 5, 2, 5, 6]);
        out.clear();
        triangulate(0x80, 0, 4, &mut out).unwrap();
        assert_eq!(out, [0, 1, 2, 0, 2, 3]);
        assert!(triangulate(0x90, 0, 4, &mut out).is_err());
    }
    #[test]
    fn fixed_point_position_and_signed_normal_use_distinct_scales() {
        let r = Reader::new(&[0xff, 0x00, 0, 0x80, 0, 0]);
        let mut v = Vertex::default();
        let mut a = Attribute {
            semantic: 9,
            encoding: 1,
            components: 1,
            component_type: 3,
            fractional_bits: 8,
            stride: 6,
            array: None,
        };
        assert_eq!(read_attribute(r, 0, &a, &mut v).unwrap(), 6);
        assert_eq!(v.position, [-1.0, 0.5, 0.0]);
        a.semantic = 10;
        a.component_type = 1;
        a.components = 0;
        assert_eq!(
            read_attribute(Reader::new(&[0xc0, 0, 0x40]), 0, &a, &mut v).unwrap(),
            3
        );
        assert_eq!(v.normal, [-1.0, 0.0, 1.0]);
        assert!(read_attribute(Reader::new(&[0]), 0, &a, &mut v).is_err());
    }
}
