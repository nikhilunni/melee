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
    /// +34 (`jumpaerial_unk`, a halfword: 40 on the disc): the Inhale's
    /// loop lasts at least this many frames (u.kb.xE4).
    pub inhale_minimum_frames: i16,
    /// +D4..+E4: Final Cutter.
    pub final_cutter: FinalCutterAttributes,
}

/// ftKb_DatAttrs +D4..+E8 (`specialhi_*`). +E8 (`specialhi_unk`, 0 on the
/// disc) has no reader in ftkirbyspecialhi.c.
#[derive(Clone, Debug, PartialEq)]
pub struct FinalCutterAttributes {
    /// +D4: the scale on the rise's upward TransN speed.
    pub vertical_momentum: f32,
    /// +D8: the scale on PlCo's aerial drift acceleration.
    pub horizontal_momentum: f32,
    /// +DC, +E0: the beam's spawn point from TopN, ahead and up.
    pub projectile_spawn_x: f32,
    pub projectile_spawn_y: f32,
    /// +E4: the stick deflection that turns Kirby round at the start.
    pub reverse_stick_range: f32,
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
            inhale_minimum_frames: r.s16(0x34)?,
            final_cutter: FinalCutterAttributes {
                vertical_momentum: r.f32(0xD4)?,
                horizontal_momentum: r.f32(0xD8)?,
                projectile_spawn_x: r.f32(0xDC)?,
                projectile_spawn_y: r.f32(0xE0)?,
                reverse_stick_range: r.f32(0xE4)?,
            },
        })
    }
}
