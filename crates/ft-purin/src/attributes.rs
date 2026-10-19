//! ftPurin/types.h: ftData.ext_attr (+4), 0x100 bytes.
//! The first 0x34 bytes are shared Fighter_x2D0_t multijump data (ft/types.h).
//! Its consumer types correct stale float/integer declarations in ftPurin/types.h.
pub use ft_purin_family::RolloutAttributes;
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_ft::fighter::multi_jump::MultiJumpAttributes;
type Result<T> = std::result::Result<T, FighterDescError>;
pub const PURIN_ATTRIBUTES_SIZE: u32 = 0x100;
#[derive(Clone, Debug, PartialEq)]
pub struct PurinAttributes {
    pub multi_jump: MultiJumpAttributes,
    pub rollout: RolloutAttributes,
    pub pound: PoundAttributes,
    /// +E8/+EC: UNK_T in types.h, no consumers or relocations in PlPr.dat.
    pub unused_pound_words: [u32; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct PoundAttributes {
    /// +0xDC, ftPurin/types.h; ftpurinspecials.c.
    pub angle_stick_minimum: f32,
    /// +0xE0, ftPurin/types.h; ftpurinspecials.c.
    pub angle_stick_maximum: f32,
    /// +0xE4, ftPurin/types.h; ftpurinspecials.c.
    pub maximum_angle_degrees: f32,
    /// +0xF0, ftPurin/types.h; ftpurinspecials.c.
    pub air_speed: f32,
    /// +0xF4, ftPurin/types.h; ftpurinspecials.c.
    pub air_speed_decay: f32,
}
pub fn read_purin_attributes(archive: &Archive) -> Result<PurinAttributes> {
    let root = archive.public("ftDataPurin").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataPurin".into(),
        })
    })?;
    PurinAttributes::read(archive, special_attributes_offset(archive, root)?)
}
impl PurinAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, PURIN_ATTRIBUTES_SIZE)?);
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
            rollout: RolloutAttributes::read(&r, 0x34)?,
            pound: PoundAttributes {
                angle_stick_minimum: r.f32(0xDC)?,
                angle_stick_maximum: r.f32(0xE0)?,
                maximum_angle_degrees: r.f32(0xE4)?,
                air_speed: r.f32(0xF0)?,
                air_speed_decay: r.f32(0xF4)?,
            },
            unused_pound_words: [r.u32(0xE8)?, r.u32(0xEC)?],
        })
    }
}
