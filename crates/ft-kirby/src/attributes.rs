//! ftKirby/types.h ftKb_DatAttrs: ftData.ext_attr (+4), 0x424 bytes.
//! The first 0x34 bytes are the shared Fighter_x2D0_t multijump data
//! (ft/types.h), read with its consumers' types: `lwz` for the turn count
//! and action ids, `lfs` for the impulses (ftCo_800D74A4).
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_ft::fighter::multi_jump::MultiJumpAttributes;

type Result<T> = std::result::Result<T, FighterDescError>;

pub const KIRBY_ATTRIBUTES_SIZE: u32 = 0x424;

/// ftKb_DatAttrs. Groups are added with the states that read them.
#[derive(Clone, Debug, PartialEq)]
pub struct KirbyAttributes {
    /// +00..+30: the midair jumps; `first_actions` are the bare (341) and
    /// helmeted (346) families.
    pub multi_jump: MultiJumpAttributes,
}

pub fn read_kirby_attributes(archive: &Archive) -> Result<KirbyAttributes> {
    let root = archive.public("ftDataKirby").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataKirby".into(),
        })
    })?;
    KirbyAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl KirbyAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, KIRBY_ATTRIBUTES_SIZE)?);
        Ok(Self {
            multi_jump: MultiJumpAttributes {
                turn_frames: r.s32(0)?,
                reverse_threshold: r.f32(4)?,
                horizontal_impulse: r.f32(8)?,
                acceleration_multiplier: r.f32(0xC)?,
                speed_multiplier: r.f32(0x10)?,
                vertical_impulses: [
                    r.f32(0x14)?,
                    r.f32(0x18)?,
                    r.f32(0x1C)?,
                    r.f32(0x20)?,
                    r.f32(0x24)?,
                ],
                state_count: r.s32(0x28)?,
                first_actions: [r.s32(0x2C)?, r.s32(0x30)?],
            },
        })
    }
}
