//! Fighter motion-table descriptors. `ft/types.h` defines `struct ftData`
//! (+0x0C animation-table pointer) and `Fighter_WaitAnimData` (0x18-byte rows).
//! `ftdata.h` declares the separate `ftData_Table_Unk0` runtime count table;
//! there is no row count in the archive. Melee layouts belong in this crate,
//! per the owning-game-crate decision in TRACKER.md.

use std::fmt;

use hsd_archive::reader::add_offset;
use hsd_archive::{Archive, Reader};

/// `ftData_Table_Unk0[FTKIND_FOX]` (`ftdata.c:255`, retail `0x803C0FD0`).
pub const FOX_ANIMATION_COUNT: u32 = 327;
/// `ftCo_SM_Wait1_0` (`ft/kinds/ftCommon/forward.h`, enum starts at -1).
/// This is a submotion/table index, not the `ftCo_MS_Wait` action state.
pub const WAIT1_ANIMATION_INDEX: usize = 2;
/// `sizeof(Fighter_WaitAnimData)` (`ft/types.h:885`), six 32-bit fields.
const ANIMATION_ROW_SIZE: u32 = 0x18;
const ANIMATION_TABLE_POINTER: u32 = 0x0C;
/// `Fighter_x59C_t` buffer ceiling asserted by `ftData_80085A14`.
const MAX_ANIMATION_SIZE: u32 = 0x8000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationDescError {
    Read(hsd_archive::Error),
    MissingPublic(String),
    NullTable,
    UnrelocatedPointer { slot: u32, target: u32 },
    InvalidRow { offset: u32, reason: &'static str },
}

impl fmt::Display for AnimationDescError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => error.fmt(f),
            Self::MissingPublic(name) => write!(f, "archive has no {name} public"),
            Self::NullTable => f.write_str("nonempty fighter animation table has a null pointer"),
            Self::UnrelocatedPointer { slot, target } => {
                write!(
                    f,
                    "pointer slot {slot:#x} holds {target:#x} without relocation"
                )
            }
            Self::InvalidRow { offset, reason } => {
                write!(f, "animation row at {offset:#x}: {reason}")
            }
        }
    }
}

impl std::error::Error for AnimationDescError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            _ => None,
        }
    }
}

impl From<hsd_archive::Error> for AnimationDescError {
    fn from(error: hsd_archive::Error) -> Self {
        Self::Read(error)
    }
}

/// File-backed subset of `Fighter_WaitAnimData` (`ft/types.h:885`). Script
/// (+0x0C), animation flags (+0x10) and the runtime RAM pointer (+0x14) are
/// deliberately not interpreted by this motion locator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationEntry {
    /// `x0`: public name inside the AJ sub-archive; null for an absent motion.
    pub symbol_name: Option<String>,
    /// `x4`: byte offset in the flat AJ file, not a relocated pointer.
    pub aj_offset: u32,
    /// `x8`: exact archive size, excluding alignment padding; zero means absent.
    pub aj_size: u32,
}

impl AnimationEntry {
    /// File slice used by `ftData_80085A14` / `ftData_80085E50` (`ftdata.c`,
    /// retail `0x80085A14` / `0x80085E50`). Caller parses these exact bytes with
    /// `Archive::parse`, then resolves `symbol_name`. No runtime buffer cache,
    /// ARAM transfer or cross-fighter relocation is needed for owned archives.
    pub fn sub_archive<'a>(
        &self,
        aj_file: &'a [u8],
    ) -> Result<Option<&'a [u8]>, AnimationDescError> {
        if self.aj_size == 0 {
            return Ok(None);
        }
        Ok(Some(
            Reader::new(aj_file).slice(self.aj_offset, self.aj_size)?,
        ))
    }
}

/// `ftData.xC` and its externally supplied count, consumed by
/// `ftData_80085A14` (`ftdata.c`, retail `0x80085A14`). Row indices are retained,
/// including absent motions and multiple rows sharing the same sub-archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FighterAnimations {
    pub table_offset: Option<u32>,
    /// The count is `entries.len()`; it is not inferred from adjacent data.
    pub entries: Vec<AnimationEntry>,
}

/// Read `ftDataFox.xC` using `ftData_Table_Unk0[FTKIND_FOX].count`
/// (`ftdata.c`, retail `0x803C0FD0`); see `ftData_80085A14` at `0x80085A14`.
pub fn read_fox_animations(archive: &Archive) -> Result<FighterAnimations, AnimationDescError> {
    let offset = archive
        .public("ftDataFox")
        .ok_or_else(|| AnimationDescError::MissingPublic("ftDataFox".into()))?;
    read_fighter_animations(archive, offset, FOX_ANIMATION_COUNT)
}

/// Read `ftData.xC` (`ft/types.h:612`) as used by `ftData_80085A14`
/// (`ftdata.c`, retail `0x80085A14`). `count` comes from code, never the file.
/// Bounds-check the complete table before allocating; use relocation metadata
/// for pointers so a table or symbol at data offset zero remains valid.
pub fn read_fighter_animations(
    archive: &Archive,
    ft_data_offset: u32,
    count: u32,
) -> Result<FighterAnimations, AnimationDescError> {
    let slot = add_offset(ft_data_offset, ANIMATION_TABLE_POINTER)?;
    let table_offset = read_pointer(archive, slot)?;
    if count == 0 {
        return Ok(FighterAnimations {
            table_offset,
            entries: Vec::new(),
        });
    }
    let base = table_offset.ok_or(AnimationDescError::NullTable)?;
    let bytes =
        count
            .checked_mul(ANIMATION_ROW_SIZE)
            .ok_or(hsd_archive::Error::OffsetOverflow {
                offset: base,
                add: u32::MAX,
            })?;
    archive.reader().slice(base, bytes)?;
    let entries = (0..count)
        .map(|index| read_entry(archive, add_offset(base, index * ANIMATION_ROW_SIZE)?))
        .collect::<Result<_, AnimationDescError>>()?;
    Ok(FighterAnimations {
        table_offset,
        entries,
    })
}

/// `Fighter_WaitAnimData` loads in `ftData_80085A14` / `ftData_80085E50`
/// (`ftdata.c`, retail `0x80085A94-0x80085AE0` / `0x80085F90-0x80085F98`).
fn read_entry(archive: &Archive, offset: u32) -> Result<AnimationEntry, AnimationDescError> {
    let reader = archive.reader();
    let symbol_name = read_pointer(archive, offset)?
        .map(|target| reader.cstr(target).map(str::to_owned))
        .transpose()?;
    let aj_offset = reader.s32(add_offset(offset, 4)?)?;
    let aj_size = reader.s32(add_offset(offset, 8)?)?;
    let invalid = |reason| AnimationDescError::InvalidRow { offset, reason };
    let aj_offset = u32::try_from(aj_offset).map_err(|_| invalid("negative AJ offset"))?;
    let aj_size = u32::try_from(aj_size).map_err(|_| invalid("negative AJ size"))?;
    if aj_size > MAX_ANIMATION_SIZE {
        return Err(invalid(
            "archive exceeds the retail 0x8000-byte animation buffer",
        ));
    }
    if (aj_size != 0) != symbol_name.is_some() {
        return Err(invalid(
            "symbol presence disagrees with nonzero archive size",
        ));
    }
    Ok(AnimationEntry {
        symbol_name,
        aj_offset,
        aj_size,
    })
}

/// Resolve archive pointers for `ftData_80085A14` (`ftdata.c`, `0x80085A14`)
/// with the same relocation/null distinction as HSD's descriptor readers.
fn read_pointer(archive: &Archive, slot: u32) -> Result<Option<u32>, AnimationDescError> {
    let pointer = archive.link(slot)?;
    let target = archive.reader().u32(slot)?;
    if pointer.is_none() && target != 0 {
        return Err(AnimationDescError::UnrelocatedPointer { slot, target });
    }
    Ok(pointer)
}
