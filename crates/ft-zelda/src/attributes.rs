//! ftZelda_DatAttrs, ft/kinds/ftZelda/types.h; ftData.ext_attr (+4).
//! ftZd_Init_OnLoad's PUSH_ATTRS copies 0x84 bytes of specials plus the
//! Nayru's Love ReflectDesc without scaling.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{
    fox_attributes::ReflectionAttributes, special_attributes_offset, FighterDescError,
};
type Result<T> = std::result::Result<T, FighterDescError>;

/// 0x84 bytes of specials plus the 0x24-byte ReflectDesc.
pub const ZELDA_ATTRIBUTES_SIZE: u32 = 0xA8;
/// Offset of the Nayru's Love reflector in ftZelda_DatAttrs.
const NAYRUS_LOVE_REFLECTION: u32 = 0x84;

#[derive(Clone, Debug, PartialEq)]
pub struct ZeldaAttributes {
    pub nayrus_love: NayrusLoveAttributes,
    pub dins_fire: DinsFireAttributes,
    pub farores_wind: FaroresWindAttributes,
    pub transform: TransformAttributes,
    /// +0x84: Nayru's Love's reflector.
    pub nayrus_love_reflection: ReflectionAttributes,
}

/// Nayru's Love (ftzeldaspecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct NayrusLoveAttributes {
    /// +0x00.
    // TODO(meaning): no consumer in the pinned C.
    pub unused: f32,
    /// +0x04: aerial frames before gravity applies.
    pub air_float_frames: i32,
    /// +0x08: aerial horizontal velocity divisor on entry.
    pub air_horizontal_velocity_divisor: f32,
    /// +0x0C: aerial gravity after the float.
    pub air_gravity: f32,
}

/// Din's Fire (ftzeldaspecials.c).
#[derive(Clone, Debug, PartialEq)]
pub struct DinsFireAttributes {
    /// +0x10..+0x1C: the motion scratch the entry copies (charge and
    /// steering counters).
    // TODO(meaning): named when Din's Fire is ported.
    pub scratch: [i32; 4],
    /// +0x20: the fireball's spawn offset ahead of Zelda's hand.
    pub spawn_offset_x: f32,
    /// +0x24: the fireball's spawn height offset.
    pub spawn_offset_y: f32,
    /// +0x28.
    // TODO(meaning): named when Din's Fire is ported.
    pub unknown_28: i32,
    /// +0x2C.
    pub unknown_2c: f32,
    /// +0x30.
    pub unknown_30: i32,
    /// +0x34: aerial landing lag; zero lands in Fall.
    pub landing_lag: f32,
}

/// Farore's Wind (ftzeldaspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct FaroresWindAttributes {
    /// +0x38: aerial horizontal velocity divisor on entry.
    pub air_horizontal_velocity_divisor: f32,
    /// +0x3C: aerial vertical velocity divisor on entry.
    pub air_vertical_velocity_divisor: f32,
    /// +0x40: the aerial start's gravity.
    pub air_start_gravity: f32,
    /// +0x44: the aerial start's terminal velocity.
    pub air_start_terminal_velocity: f32,
    /// +0x48: frames spent travelling while invisible.
    pub travel_frames: i32,
    /// +0x4C: travel frames before a platform can stop the aerial travel.
    pub platform_frames: f32,
    /// +0x50: stick magnitude below which Zelda travels straight up.
    pub stick_threshold: f32,
    /// +0x54: travel speed added per unit of stick magnitude.
    pub speed_per_stick: f32,
    /// +0x58: base travel speed.
    pub base_speed: f32,
    /// +0x5C: special-fall horizontal speed, times air_drift_max.
    pub freefall_speed_multiplier: f32,
    /// +0x60: ftCommon_HandleTeleportCollisions's angle clamp: a wall or
    /// ceiling ends the aerial travel when met at more than 90 plus this
    /// many degrees.
    pub surface_angle_degrees: i32,
    /// +0x64: velocity kept when the travel ends.
    pub end_velocity_multiplier: f32,
    /// +0x68: special-fall air mobility.
    pub freefall_mobility: f32,
    /// +0x6C: special landing lag.
    pub landing_lag: f32,
}

/// Transform (ftzeldaspeciallw.c).
#[derive(Clone, Debug, PartialEq)]
pub struct TransformAttributes {
    /// +0x70: horizontal velocity divisor on entry.
    pub horizontal_velocity_divisor: f32,
    /// +0x74: vertical velocity divisor on entry.
    pub vertical_velocity_divisor: f32,
    /// +0x78: aerial gravity.
    pub gravity: f32,
    /// +0x7C: aerial terminal velocity.
    pub terminal_velocity: f32,
    /// +0x80: frame the arriving form's second motion starts at.
    pub finish_start_frame: f32,
}

pub fn read_zelda_attributes(archive: &Archive) -> Result<ZeldaAttributes> {
    let root = archive.public("ftDataZelda").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataZelda".into(),
        })
    })?;
    ZeldaAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl ZeldaAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, ZELDA_ATTRIBUTES_SIZE)?);
        Ok(Self {
            nayrus_love: NayrusLoveAttributes {
                unused: r.f32(0x00)?,
                air_float_frames: r.s32(0x04)?,
                air_horizontal_velocity_divisor: r.f32(0x08)?,
                air_gravity: r.f32(0x0C)?,
            },
            dins_fire: DinsFireAttributes {
                scratch: [r.s32(0x10)?, r.s32(0x14)?, r.s32(0x18)?, r.s32(0x1C)?],
                spawn_offset_x: r.f32(0x20)?,
                spawn_offset_y: r.f32(0x24)?,
                unknown_28: r.s32(0x28)?,
                unknown_2c: r.f32(0x2C)?,
                unknown_30: r.s32(0x30)?,
                landing_lag: r.f32(0x34)?,
            },
            farores_wind: FaroresWindAttributes {
                air_horizontal_velocity_divisor: r.f32(0x38)?,
                air_vertical_velocity_divisor: r.f32(0x3C)?,
                air_start_gravity: r.f32(0x40)?,
                air_start_terminal_velocity: r.f32(0x44)?,
                travel_frames: r.s32(0x48)?,
                platform_frames: r.f32(0x4C)?,
                stick_threshold: r.f32(0x50)?,
                speed_per_stick: r.f32(0x54)?,
                base_speed: r.f32(0x58)?,
                freefall_speed_multiplier: r.f32(0x5C)?,
                surface_angle_degrees: r.s32(0x60)?,
                end_velocity_multiplier: r.f32(0x64)?,
                freefall_mobility: r.f32(0x68)?,
                landing_lag: r.f32(0x6C)?,
            },
            transform: TransformAttributes {
                horizontal_velocity_divisor: r.f32(0x70)?,
                vertical_velocity_divisor: r.f32(0x74)?,
                gravity: r.f32(0x78)?,
                terminal_velocity: r.f32(0x7C)?,
                finish_start_frame: r.f32(0x80)?,
            },
            nayrus_love_reflection: ReflectionAttributes::read_at(r, NAYRUS_LOVE_REFLECTION)?,
        })
    }
}
