//! Melee collision descriptors from `third_party/melee-decomp/src/melee/mp/types.h`.
//! Pointers are data-section offsets, never host addresses. Relocation slots
//! distinguish a real pointer to offset zero from a null pointer.

use core::fmt;
use hsd_archive::{Archive, Reader};
use hsd_types::Vec2;
use melee_types::mp::{LineSection, MapCollData, MapJoint, MapLine};

/// Sizes of `MapCollData`, `Vec2`, `MapLine`, and `MapJoint` in mp/types.h.
const COLL_DATA_SIZE: u32 = 0x30;
const VERTEX_SIZE: u32 = 8;
const LINE_SIZE: u32 = 0x10;
const JOINT_SIZE: u32 = 0x28;

/// A malformed collision descriptor, with the field and data offset for diagnosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollDescError {
    MissingPublic,
    OutOfRangePointer {
        field: &'static str,
        offset: u32,
        bytes: u32,
        available: usize,
    },
    /// A non-null pointer must have a relocation entry.
    UnrelocatedPointer {
        field: &'static str,
        slot: u32,
        target: u32,
    },
    BadCount {
        field: &'static str,
        count: i32,
        reason: &'static str,
    },
    Read(hsd_archive::Error),
}

impl fmt::Display for CollDescError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPublic => f.write_str("archive has no coll_data public"),
            Self::OutOfRangePointer { field, offset, bytes, available } => write!(
                f, "{field}: {bytes:#x} bytes at data offset {offset:#x} exceed data section of {available:#x} bytes"
            ),
            Self::UnrelocatedPointer { field, slot, target } => write!(
                f, "{field}: pointer slot {slot:#x} holds {target:#x} without a relocation entry"
            ),
            Self::BadCount { field, count, reason } => write!(f, "{field}: bad count {count}: {reason}"),
            Self::Read(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for CollDescError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            _ => None,
        }
    }
}

impl From<hsd_archive::Error> for CollDescError {
    fn from(error: hsd_archive::Error) -> Self {
        Self::Read(error)
    }
}

/// Read the stage's `coll_data` public (`MapCollData` in mp/types.h).
pub fn read_public_coll_data(archive: &Archive) -> Result<MapCollData, CollDescError> {
    let offset = archive
        .public("coll_data")
        .ok_or(CollDescError::MissingPublic)?;
    read_coll_data(archive, offset)
}

/// Read `struct MapCollData` (mp/types.h:119), 0x30 bytes: two pointer/s32
/// count pairs at +0/+8, five s16 start/count pairs at +0x10, a pointer/s32
/// joint count at +0x24, and the inferred s32 `x2C` at +0x2C.
///
/// Array extents and section ranges are checked before returning owned data.
/// Empty arrays may have null pointers; nonempty arrays require relocation
/// entries, including when the target is zero. Line topology is preserved
/// verbatim, not validated or repaired here.
pub fn read_coll_data(archive: &Archive, offset: u32) -> Result<MapCollData, CollDescError> {
    let r = bounded_reader(archive, "coll_data", offset, COLL_DATA_SIZE)?;
    // The bounded 0x30-byte header guarantees these slot additions cannot overflow.
    let line_slot = offset + 0x08;
    let joint_slot = offset + 0x24;
    let vert_count = r.s32(0x04)?;
    let line_count = r.s32(0x0C)?;
    let joint_count = r.s32(0x28)?;
    let verts = read_array(
        archive,
        "verts",
        offset,
        vert_count,
        VERTEX_SIZE,
        read_vertex,
    )?;
    let lines = read_array(
        archive, "lines", line_slot, line_count, LINE_SIZE, read_line,
    )?;
    let joints = read_array(
        archive,
        "joints",
        joint_slot,
        joint_count,
        JOINT_SIZE,
        read_joint,
    )?;
    let data = MapCollData {
        verts,
        lines,
        floor_start: r.s16(0x10)?,
        floor_count: r.s16(0x12)?,
        ceiling_start: r.s16(0x14)?,
        ceiling_count: r.s16(0x16)?,
        right_wall_start: r.s16(0x18)?,
        right_wall_count: r.s16(0x1A)?,
        left_wall_start: r.s16(0x1C)?,
        left_wall_count: r.s16(0x1E)?,
        dynamic_start: r.s16(0x20)?,
        dynamic_count: r.s16(0x22)?,
        joints,
        x2c: r.s32(0x2C)?,
    };
    check_ranges(&data)?;
    Ok(data)
}

fn check_ranges(data: &MapCollData) -> Result<(), CollDescError> {
    for section in LineSection::ALL {
        check_range("coll_data section", data.section(section), data.lines.len())?;
        for joint in &data.joints {
            check_range("joint section", joint.section(section), data.lines.len())?;
        }
    }
    for joint in &data.joints {
        check_range(
            "joint vertices",
            (joint.vtx_start, joint.vtx_count),
            data.verts.len(),
        )?;
    }
    Ok(())
}

fn bounded_reader<'a>(
    archive: &'a Archive,
    field: &'static str,
    offset: u32,
    bytes: u32,
) -> Result<Reader<'a>, CollDescError> {
    let slice =
        archive
            .reader()
            .slice(offset, bytes)
            .map_err(|_| CollDescError::OutOfRangePointer {
                field,
                offset,
                bytes,
                available: archive.data().len(),
            })?;
    Ok(Reader::new(slice))
}

fn read_array<T>(
    archive: &Archive,
    field: &'static str,
    slot: u32,
    count: i32,
    stride: u32,
    read: fn(Reader<'_>) -> hsd_archive::Result<T>,
) -> Result<Vec<T>, CollDescError> {
    let bytes = u32::try_from(count)
        .ok()
        .and_then(|n| n.checked_mul(stride))
        .ok_or(CollDescError::BadCount {
            field,
            count,
            reason: "negative or byte size overflows u32",
        })?;
    let target = archive.reader().u32(slot)?;
    if !archive.is_relocated_offset(slot) {
        if target != 0 {
            return Err(CollDescError::UnrelocatedPointer {
                field,
                slot,
                target,
            });
        }
        if count != 0 {
            return Err(CollDescError::BadCount {
                field,
                count,
                reason: "nonempty array has a null pointer",
            });
        }
        return Ok(Vec::new());
    }
    let r = bounded_reader(archive, field, target, bytes)?;
    r.bytes()
        .chunks_exact(stride as usize)
        .map(|bytes| read(Reader::new(bytes)).map_err(CollDescError::from))
        .collect()
}

fn check_range(
    field: &'static str,
    (start, count): (i16, i16),
    len: usize,
) -> Result<(), CollDescError> {
    // Empty sections do not dereference their start (which may be -1).
    if count < 0 || (count > 0 && (start < 0 || start as usize + count as usize > len)) {
        return Err(CollDescError::BadCount {
            field,
            count: i32::from(count),
            reason: "start/count range is outside its table",
        });
    }
    Ok(())
}

/// `MapCollData.verts` in mp/types.h points to Vec2 (two f32s), not CollVtx.
fn read_vertex(r: Reader<'_>) -> hsd_archive::Result<Vec2> {
    Ok(Vec2::new(r.f32(0x00)?, r.f32(0x04)?))
}

/// `struct MapLine` (mp/types.h:47): two u16 indices, four s16 links,
/// then two u16 flag words, in precisely that order (16 bytes).
fn read_line(r: Reader<'_>) -> hsd_archive::Result<MapLine> {
    Ok(MapLine {
        v0_idx: r.u16(0x00)?,
        v1_idx: r.u16(0x02)?,
        prev_id0: r.s16(0x04)?,
        next_id0: r.s16(0x06)?,
        prev_id1: r.s16(0x08)?,
        next_id1: r.s16(0x0A)?,
        hi_flags: r.u16(0x0C)?,
        lo_flags: r.u16(0x0E)?,
    })
}

/// `struct MapJoint` (mp/types.h:83): five s16 section pairs, four f32
/// bounds, then an s16 vertex start/count pair (40 bytes).
fn read_joint(r: Reader<'_>) -> hsd_archive::Result<MapJoint> {
    Ok(MapJoint {
        floor_start: r.s16(0x00)?,
        floor_count: r.s16(0x02)?,
        ceiling_start: r.s16(0x04)?,
        ceiling_count: r.s16(0x06)?,
        right_wall_start: r.s16(0x08)?,
        right_wall_count: r.s16(0x0A)?,
        left_wall_start: r.s16(0x0C)?,
        left_wall_count: r.s16(0x0E)?,
        dynamic_start: r.s16(0x10)?,
        dynamic_count: r.s16(0x12)?,
        left_bound: r.f32(0x14)?,
        bottom_bound: r.f32(0x18)?,
        right_bound: r.f32(0x1C)?,
        top_bound: r.f32(0x20)?,
        vtx_start: r.s16(0x24)?,
        vtx_count: r.s16(0x26)?,
    })
}
