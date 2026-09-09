//! Metadata used by `ftCo_8008A7A8` and `ftAnim_8006EBE8`.
use super::{AnimationDescError, FighterAnimations};
use crate::anim::{Motion, MotionFlags, WaitEntry};
use hsd_archive::desc::read_public_figatree;
use hsd_archive::reader::add_offset;
use hsd_archive::Archive;

/// `ftData.x24`: sentinel-terminated `WaitStruct` pairs (ftwaitanim.c).
/// The sentinel is retained so callers/tests can verify the actual table.
pub fn read_wait_table(
    archive: &Archive,
    fighter: u32,
) -> Result<Vec<WaitEntry>, AnimationDescError> {
    let base = archive
        .link(add_offset(fighter, 0x24)?)?
        .ok_or(AnimationDescError::NullTable)?;
    let mut result = Vec::new();
    let mut offset = base;
    loop {
        let motion = archive.reader().s32(offset)?;
        let weight = archive.reader().s32(add_offset(offset, 4)?)?;
        result.push(WaitEntry { motion, weight });
        if motion == -1 {
            return Ok(result);
        }
        offset = add_offset(offset, 8)?;
    }
}

/// ftData.x28, used by ftCo_SquatWait_Anim (0x800D6448).
/// A null table asks the common animation helper to restart without RNG.
pub fn read_squat_table(
    archive: &Archive,
    fighter: u32,
) -> Result<Option<Vec<WaitEntry>>, AnimationDescError> {
    let Some(mut offset) = archive.link(add_offset(fighter, 0x28)?)? else {
        return Ok(None);
    };
    let mut result = Vec::new();
    loop {
        let motion = archive.reader().s32(offset)?;
        let weight = archive.reader().s32(add_offset(offset, 4)?)?;
        result.push(WaitEntry { motion, weight });
        if motion == -1 {
            return Ok(Some(result));
        }
        offset = add_offset(offset, 8)?;
    }
}

/// Motion-table flags (+0x10) and blend byte (`ftData.x10[id][0]`), used
/// by ftwaitanim.c:93-113. Buffer loading uses the existing AJ locator.
pub fn read_playback_motion(
    archive: &Archive,
    fighter: u32,
    table: &FighterAnimations,
    aj: &[u8],
    id: usize,
) -> Result<Motion, Box<dyn std::error::Error>> {
    let entry = table.entries.get(id).ok_or("motion index outside table")?;
    let row = add_offset(
        table.table_offset.ok_or("missing animation table")?,
        u32::try_from(id)?
            .checked_mul(0x18)
            .ok_or("motion index overflow")?,
    )?;
    let flags = archive.reader().u32(add_offset(row, 0x10)?)?;
    let blends = archive
        .link(add_offset(fighter, 0x10)?)?
        .ok_or("missing blend table")?;
    let blend_frames = f32::from(
        archive.reader().u8(add_offset(
            blends,
            u32::try_from(id)?
                .checked_mul(2)
                .ok_or("blend index overflow")?,
        )?)?,
    );
    let animation_archive = Archive::parse(entry.sub_archive(aj)?.ok_or("absent motion")?)?;
    let animation = read_public_figatree(
        &animation_archive,
        entry
            .symbol_name
            .as_deref()
            .ok_or("missing animation public")?,
    )?;
    Ok(Motion {
        id: i32::try_from(id)?,
        animation,
        flags: MotionFlags(flags),
        blend_frames,
        remap: None,
    })
}
