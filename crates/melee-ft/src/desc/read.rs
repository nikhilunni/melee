use std::fmt;

use hsd_archive::reader::add_offset;
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FighterDescError {
    Archive(hsd_archive::desc::DescError),
    InvalidEnum(melee_types::InvalidDiscriminant),
    InvalidData {
        field: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for FighterDescError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(e) => e.fmt(f),
            Self::InvalidEnum(e) => e.fmt(f),
            Self::InvalidData { field, reason } => write!(f, "{field}: {reason}"),
        }
    }
}

impl std::error::Error for FighterDescError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Archive(e) => Some(e),
            Self::InvalidEnum(e) => Some(e),
            Self::InvalidData { .. } => None,
        }
    }
}

impl From<hsd_archive::Error> for FighterDescError {
    fn from(e: hsd_archive::Error) -> Self {
        Self::Archive(e.into())
    }
}

impl From<melee_types::InvalidDiscriminant> for FighterDescError {
    fn from(e: melee_types::InvalidDiscriminant) -> Self {
        Self::InvalidEnum(e)
    }
}

pub(super) type Result<T> = std::result::Result<T, FighterDescError>;

pub(super) fn invalid(field: &'static str, reason: &'static str) -> FighterDescError {
    FighterDescError::InvalidData { field, reason }
}

pub(super) fn public(archive: &Archive, name: &str) -> Result<u32> {
    archive.public(name).ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol { name: name.into() })
    })
}

pub(super) fn block(archive: &Archive, offset: u32, size: u32) -> Result<Reader<'_>> {
    Ok(Reader::new(archive.reader().slice(offset, size)?))
}

pub(super) fn pointer(
    archive: &Archive,
    base: u32,
    offset: u32,
    field: &'static str,
) -> Result<Option<u32>> {
    let at = add_offset(base, offset)?;
    let target = archive.link(at)?;
    let value = archive.reader().u32(at)?;
    if target.is_none() && value != 0 {
        return Err(FighterDescError::Archive(
            hsd_archive::desc::DescError::UnrelocatedPointer { field, at, value },
        ));
    }
    Ok(target)
}

pub(super) fn required(
    archive: &Archive,
    base: u32,
    offset: u32,
    field: &'static str,
) -> Result<u32> {
    pointer(archive, base, offset, field)?.ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::NullPointer {
            field,
            at: base + offset,
        })
    })
}

pub(super) fn vec3(r: Reader<'_>, offset: u32) -> Result<Vec3> {
    Ok(Vec3 {
        x: r.f32(offset)?,
        y: r.f32(add_offset(offset, 4)?)?,
        z: r.f32(add_offset(offset, 8)?)?,
    })
}

/// Resolve `ftData.ext_attr` (+4, `ft/types.h:614`) for a character reader.
/// Uses relocation metadata, including a genuine pointer to data offset zero.
pub fn special_attributes_offset(archive: &Archive, ft_data: u32) -> Result<u32> {
    required(archive, ft_data, 4, "ftData.ext_attr")
}
