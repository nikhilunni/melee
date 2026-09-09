//! `ftFox_DatAttrs`, ft/kinds/ftFox/types.h:76-149, reached via
//! ftData.ext_attr (+4, ft/types.h:614). No scaling or arithmetic on load.

use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_types::ItemKind;

type Result<T> = std::result::Result<T, FighterDescError>;

/// 0xB0 bytes of specials plus 0x24-byte ReflectDesc (lb/types.h:115-126).
pub const FOX_ATTRIBUTES_SIZE: u32 = 0xD4;

#[derive(Debug, Clone, PartialEq)]
pub struct FoxAttributes {
    pub blaster: BlasterAttributes,
    pub illusion: IllusionAttributes,
    pub fire_fox: FireFoxAttributes,
    pub reflector: ReflectorAttributes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlasterAttributes {
    /// +0x00, ft/kinds/ftFox/types.h:79.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown_1: f32,
    /// +0x04, ft/kinds/ftFox/types.h:80.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown_2: f32,
    /// +0x08, ft/kinds/ftFox/types.h:81.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown_3: f32,
    /// +0x0C, ft/kinds/ftFox/types.h:82.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown_4: f32,
    /// +0x10, ft/kinds/ftFox/types.h:84.
    pub angle: f32,
    /// +0x14, ft/kinds/ftFox/types.h:86.
    pub velocity: f32,
    /// +0x18, ft/kinds/ftFox/types.h:87.
    pub landing_lag: f32,
    /// +0x1C, ft/kinds/ftFox/types.h:88.
    pub shot_item_kind: ItemKind,
    /// +0x20, ft/kinds/ftFox/types.h:89.
    pub gun_item_kind: ItemKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IllusionAttributes {
    /// +0x24, ft/kinds/ftFox/types.h:93.
    pub gravity_delay: f32,
    /// +0x28, ft/kinds/ftFox/types.h:95.
    pub startup_momentum_divisor: f32,
    /// +0x2C, ft/kinds/ftFox/types.h:96.
    pub startup_air_friction: f32,
    /// +0x30, ft/kinds/ftFox/types.h:97.
    pub startup_fall_acceleration: f32,
    /// +0x34, ft/kinds/ftFox/types.h:98.
    pub ground_end_vel_x: f32,
    /// +0x38, ft/kinds/ftFox/types.h:99.
    pub ground_friction: f32,
    /// +0x3C, ft/kinds/ftFox/types.h:100.
    pub air_end_vel_x: f32,
    /// +0x40, ft/kinds/ftFox/types.h:101.
    pub end_air_friction: f32,
    /// +0x44, ft/kinds/ftFox/types.h:102. Header says FALL_ACCEL, but
    /// ftfoxspecials.c:575 assigns this to gravityDelay.
    pub end_gravity_delay: f32,
    /// +0x48, ft/kinds/ftFox/types.h:103. Header says TERMINAL_VELOCITY,
    /// but ftfoxspecials.c:532 passes it as acceleration to ftCommon_Fall.
    pub end_fall_acceleration: f32,
    /// +0x4C, ft/kinds/ftFox/types.h:104.
    pub freefall_mobility: f32,
    /// +0x50, ft/kinds/ftFox/types.h:105.
    pub landing_lag: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FireFoxAttributes {
    /// +0x54, ft/kinds/ftFox/types.h:109.
    pub gravity_delay: f32,
    /// +0x58, ft/kinds/ftFox/types.h:110.
    pub vel_x: f32,
    /// +0x5C, ft/kinds/ftFox/types.h:111.
    pub air_momentum_preserve_x: f32,
    /// +0x60, ft/kinds/ftFox/types.h:112.
    pub fall_accel: f32,
    /// +0x64, ft/kinds/ftFox/types.h:113.
    pub direction_stick_min: f32,
    /// +0x68, ft/kinds/ftFox/types.h:117.
    pub duration: f32,
    /// +0x6C, ft/kinds/ftFox/types.h:118.
    pub bounce_var: i32,
    /// +0x70, ft/kinds/ftFox/types.h:119.
    pub duration_end: f32,
    /// +0x74, ft/kinds/ftFox/types.h:120.
    pub speed: f32,
    /// +0x78, ft/kinds/ftFox/types.h:121.
    pub reverse_accel: f32,
    /// +0x7C, ft/kinds/ftFox/types.h:122.
    pub ground_momentum_end: f32,
    /// +0x80, ft/kinds/ftFox/types.h:123.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown_2: f32,
    /// +0x84, ft/kinds/ftFox/types.h:124.
    pub bound_vel_x: f32,
    /// +0x88, ft/kinds/ftFox/types.h:127.
    pub facing_stick_min: f32,
    /// +0x8C, ft/kinds/ftFox/types.h:130.
    pub freefall_mobility: f32,
    /// +0x90, ft/kinds/ftFox/types.h:131.
    pub landing_lag: f32,
    /// +0x94, ft/kinds/ftFox/types.h:132.
    pub bound_angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectorAttributes {
    /// +0x98, ft/kinds/ftFox/types.h:141.
    pub release_lag: f32,
    /// +0x9C, ft/kinds/ftFox/types.h:143.
    pub turn_frames: f32,
    /// +0xA0, ft/kinds/ftFox/types.h:144.
    // TODO(meaning): the decomp has not named this parameter.
    pub unknown_1: f32,
    /// +0xA4, ft/kinds/ftFox/types.h:145.
    pub gravity_delay: i32,
    /// +0xA8, ft/kinds/ftFox/types.h:146.
    pub momentum_preserve_x: f32,
    /// +0xAC, ft/kinds/ftFox/types.h:147.
    pub fall_accel: f32,
    /// +0xB0, ft/kinds/ftFox/types.h:148.
    pub reflection: ReflectionAttributes,
}

/// `ReflectDesc`, lb/types.h:115-126. Offsets are relative to the descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionAttributes {
    /// +0, lb/types.h:116; direct joint index (ftcoll.c:3200).
    pub joint: u32,
    /// +4, lb/types.h:117.
    pub max_damage: i32,
    /// +8, lb/types.h:118.
    pub offset: Vec3,
    /// +14, lb/types.h:119.
    pub size: f32,
    /// +18, lb/types.h:120.
    pub damage_multiplier: f32,
    /// +1C, lb/types.h:121.
    pub speed_multiplier: f32,
    /// +20, lb/types.h:123-125. Kept as a byte, not normalized to bool.
    pub skip_ownership_change: u8,
}

pub fn read_fox_attributes(archive: &Archive) -> Result<FoxAttributes> {
    let root = archive.public("ftDataFox").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataFox".into(),
        })
    })?;
    FoxAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl FoxAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, FOX_ATTRIBUTES_SIZE)?);
        Ok(Self {
            blaster: BlasterAttributes::read(r)?,
            illusion: IllusionAttributes::read(r)?,
            fire_fox: FireFoxAttributes::read(r)?,
            reflector: ReflectorAttributes::read(r)?,
        })
    }
}

impl BlasterAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            unknown_1: r.f32(0x00)?,
            unknown_2: r.f32(0x04)?,
            unknown_3: r.f32(0x08)?,
            unknown_4: r.f32(0x0C)?,
            angle: r.f32(0x10)?,
            velocity: r.f32(0x14)?,
            landing_lag: r.f32(0x18)?,
            shot_item_kind: ItemKind::try_from(r.s32(0x1C)?)?,
            gun_item_kind: ItemKind::try_from(r.s32(0x20)?)?,
        })
    }
}

impl IllusionAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            gravity_delay: r.f32(0x24)?,
            startup_momentum_divisor: r.f32(0x28)?,
            startup_air_friction: r.f32(0x2C)?,
            startup_fall_acceleration: r.f32(0x30)?,
            ground_end_vel_x: r.f32(0x34)?,
            ground_friction: r.f32(0x38)?,
            air_end_vel_x: r.f32(0x3C)?,
            end_air_friction: r.f32(0x40)?,
            end_gravity_delay: r.f32(0x44)?,
            end_fall_acceleration: r.f32(0x48)?,
            freefall_mobility: r.f32(0x4C)?,
            landing_lag: r.f32(0x50)?,
        })
    }
}

impl FireFoxAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            gravity_delay: r.f32(0x54)?,
            vel_x: r.f32(0x58)?,
            air_momentum_preserve_x: r.f32(0x5C)?,
            fall_accel: r.f32(0x60)?,
            direction_stick_min: r.f32(0x64)?,
            duration: r.f32(0x68)?,
            bounce_var: r.s32(0x6C)?,
            duration_end: r.f32(0x70)?,
            speed: r.f32(0x74)?,
            reverse_accel: r.f32(0x78)?,
            ground_momentum_end: r.f32(0x7C)?,
            unknown_2: r.f32(0x80)?,
            bound_vel_x: r.f32(0x84)?,
            facing_stick_min: r.f32(0x88)?,
            freefall_mobility: r.f32(0x8C)?,
            landing_lag: r.f32(0x90)?,
            bound_angle: r.f32(0x94)?,
        })
    }
}

impl ReflectorAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            release_lag: r.f32(0x98)?,
            turn_frames: r.f32(0x9C)?,
            unknown_1: r.f32(0xA0)?,
            gravity_delay: r.s32(0xA4)?,
            momentum_preserve_x: r.f32(0xA8)?,
            fall_accel: r.f32(0xAC)?,
            reflection: ReflectionAttributes::read(r)?,
        })
    }
}

impl ReflectionAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            joint: r.u32(0xB0)?,
            max_damage: r.s32(0xB4)?,
            offset: Vec3 {
                x: r.f32(0xB8)?,
                y: r.f32(0xBC)?,
                z: r.f32(0xC0)?,
            },
            size: r.f32(0xC4)?,
            damage_multiplier: r.f32(0xC8)?,
            speed_multiplier: r.f32(0xCC)?,
            skip_ownership_change: r.u8(0xD0)?,
        })
    }
}
