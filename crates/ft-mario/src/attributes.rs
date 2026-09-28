//! ftMario_DatAttrs, ft/kinds/ftMario/types.h; ftData.ext_attr (+4).
//! ftMr_Init_OnLoad's PUSH_ATTRS copies 0x84 bytes without scaling. Dr. Mario
//! shares the layout through ftMr_Init_OnLoadForDrMario.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{
    fox_attributes::ReflectionAttributes, special_attributes_offset, FighterDescError,
};
use melee_types::ItemKind;
type Result<T> = std::result::Result<T, FighterDescError>;

/// 0x60 bytes of specials plus the cape's 0x24-byte ReflectDesc.
pub const MARIO_ATTRIBUTES_SIZE: u32 = 0x84;
/// Offset of `cape_reflection` in ftMario_DatAttrs.
const CAPE_REFLECTION: u32 = 0x60;

#[derive(Clone, Debug, PartialEq)]
pub struct MarioAttributes {
    pub cape: CapeAttributes,
    pub super_jump_punch: SuperJumpPunchAttributes,
    pub tornado: TornadoAttributes,
    /// +0x60: the cape's reflector (ftMario_DatAttrs.cape_reflection).
    pub cape_reflection: ReflectionAttributes,
}

/// ftMario_SpecialS_DatAttrs: the cape (side special).
#[derive(Clone, Debug, PartialEq)]
pub struct CapeAttributes {
    /// +0x00: horizontal velocity divisor on entry.
    pub horizontal_velocity_decay: f32,
    /// +0x04: aerial horizontal friction.
    pub air_friction: f32,
    /// +0x08: vertical velocity of the first aerial cape.
    pub vertical_boost: f32,
    /// +0x0C: aerial gravity.
    pub gravity: f32,
    /// +0x10: aerial terminal velocity.
    pub terminal_velocity: f32,
    /// +0x14: the cape article's item kind.
    pub cape_kind: ItemKind,
}

/// ftMario_SpecialHi_DatAttrs: Super Jump Punch.
#[derive(Clone, Debug, PartialEq)]
pub struct SuperJumpPunchAttributes {
    /// +0x18: special fall air mobility.
    pub freefall_mobility: f32,
    /// +0x1C: special landing lag.
    pub landing_lag: f32,
    /// +0x20: stick range that reverses the facing.
    pub reverse_stick_range: f32,
    /// +0x24: stick range that steers the jump.
    pub momentum_stick_range: f32,
    /// +0x28: maximum steering angle, in degrees.
    pub angle_diff: f32,
    /// +0x2C: horizontal velocity scale.
    pub vel_x: f32,
    /// +0x30: gravity after the rise.
    pub gravity: f32,
    /// +0x34: velocity multiplier.
    pub vel_mul: f32,
}

/// ftMario_SpecialLw_DatAttrs: Mario Tornado.
#[derive(Clone, Debug, PartialEq)]
pub struct TornadoAttributes {
    /// +0x38: vertical velocity of an aerial tap.
    pub vel_y: f32,
    /// +0x3C: grounded horizontal momentum.
    pub momentum_x: f32,
    /// +0x40: aerial horizontal momentum.
    pub air_momentum_x: f32,
    /// +0x44: grounded momentum multiplier.
    pub momentum_x_mul: f32,
    /// +0x48: aerial momentum multiplier.
    pub air_momentum_x_mul: f32,
    /// +0x4C: friction at the end.
    pub friction_end: f32,
    /// +0x50: speciallw.unk0.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown: i32,
    /// +0x54: maximum vertical velocity from tapping.
    pub tap_y_vel_max: f32,
    /// +0x58: gravity while tapping.
    pub tap_gravity: f32,
    /// +0x5C: landing lag.
    pub landing_lag: i32,
}

pub fn read_mario_attributes(archive: &Archive) -> Result<MarioAttributes> {
    let root = archive.public("ftDataMario").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataMario".into(),
        })
    })?;
    MarioAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl MarioAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, MARIO_ATTRIBUTES_SIZE)?);
        Ok(Self {
            cape: CapeAttributes {
                horizontal_velocity_decay: r.f32(0x00)?,
                air_friction: r.f32(0x04)?,
                vertical_boost: r.f32(0x08)?,
                gravity: r.f32(0x0C)?,
                terminal_velocity: r.f32(0x10)?,
                cape_kind: ItemKind::try_from(r.s32(0x14)?)?,
            },
            super_jump_punch: SuperJumpPunchAttributes {
                freefall_mobility: r.f32(0x18)?,
                landing_lag: r.f32(0x1C)?,
                reverse_stick_range: r.f32(0x20)?,
                momentum_stick_range: r.f32(0x24)?,
                angle_diff: r.f32(0x28)?,
                vel_x: r.f32(0x2C)?,
                gravity: r.f32(0x30)?,
                vel_mul: r.f32(0x34)?,
            },
            tornado: TornadoAttributes {
                vel_y: r.f32(0x38)?,
                momentum_x: r.f32(0x3C)?,
                air_momentum_x: r.f32(0x40)?,
                momentum_x_mul: r.f32(0x44)?,
                air_momentum_x_mul: r.f32(0x48)?,
                friction_end: r.f32(0x4C)?,
                unknown: r.s32(0x50)?,
                tap_y_vel_max: r.f32(0x54)?,
                tap_gravity: r.f32(0x58)?,
                landing_lag: r.s32(0x5C)?,
            },
            cape_reflection: ReflectionAttributes::read_at(r, CAPE_REFLECTION)?,
        })
    }
}
