//! ftCaptain_DatAttrs, ft/kinds/ftCaptain/types.h; ftData.ext_attr (+4).
//! OnLoad/LoadSpecialAttrs copy 0x8C bytes without scaling. Ganondorf shares
//! the layout through OnLoadForGanon; his behavior is outside this crate.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

pub const CAPTAIN_ATTRIBUTES_SIZE: u32 = 0x8C;

#[derive(Clone, Debug, PartialEq)]
pub struct CaptainAttributes {
    pub falcon_punch: FalconPunchAttributes,
    pub raptor_boost: RaptorBoostAttributes,
    pub falcon_dive: FalconDiveAttributes,
    pub falcon_kick: FalconKickAttributes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FalconPunchAttributes {
    /// +0x00, ftCaptain/types.h. Minimum stick range for the downward angle.
    pub downward_stick_threshold: f32,
    /// +0x04, ftCaptain/types.h. Minimum stick range for the upward angle.
    pub upward_stick_threshold: f32,
    /// +0x08, ftCaptain/types.h. Maximum punch angle change.
    pub maximum_angle: f32,
    /// +0x0C, ftCaptain/types.h. Aerial punch launch speed.
    pub aerial_speed: f32,
    /// +0x10, ftCaptain/types.h. Applied to both aerial velocity components.
    pub momentum_multiplier: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RaptorBoostAttributes {
    /// +0x14, ftCaptain/types.h. Ground velocity multiplier on detection.
    pub ground_hit_speed_multiplier: f32,
    /// +0x18, ftCaptain/types.h. Aerial vertical acceleration.
    pub gravity: f32,
    /// +0x1C, ftCaptain/types.h. Aerial vertical speed limit.
    pub terminal_velocity: f32,
    /// +0x20, ftCaptain/types.h. +20..34: specials_unk0..5, unused per types.dox.
    pub unused_parameters: [f32; 6],
    /// +0x38, ftCaptain/types.h. Landing lag after the startup misses.
    pub miss_landing_lag: f32,
    /// +0x3C, ftCaptain/types.h. Landing lag after the lunge hits.
    pub hit_landing_lag: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FalconDiveAttributes {
    /// +0x40, ftCaptain/types.h. Multiplies common air_drift_stick_mul.
    pub air_acceleration_multiplier: f32,
    /// +0x44, ftCaptain/types.h. Multiplies common air_drift_max.
    pub horizontal_speed_multiplier: f32,
    /// +0x48, ftCaptain/types.h. Special-fall mobility after animation end.
    pub freefall_mobility: f32,
    /// +0x4C, ftCaptain/types.h. Special landing lag.
    pub landing_lag: f32,
    /// +0x50, ftCaptain/types.h. +50/+54: specialhi_unk0/1; no consumers in the pinned C.
    pub unused_parameters: [f32; 2],
    /// +0x58, ftCaptain/types.h. Absolute horizontal input needed to reverse facing.
    pub reverse_stick_threshold: f32,
    /// +0x5C, ftCaptain/types.h. specialhi_unk2: converted into cmd_vars[1] on entry.
    pub initial_command_value: f32,
    /// +0x60, ftCaptain/types.h. Gravity after the throw command.
    pub catch_gravity: f32,
    /// +0x64, ftCaptain/types.h. specialhi_air_var: copied to the u16 motion counter on entry.
    pub initial_air_counter: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FalconKickAttributes {
    /// +0x68, ftCaptain/types.h. x68: unknown in types.dox, no consumers in the pinned C.
    pub unused_parameter: f32,
    /// +0x6C, ftCaptain/types.h. speciallw_unk1: no consumers in the pinned C.
    pub unused_word: u32,
    /// +0x70, ftCaptain/types.h. Aerial flame particle angle before radians conversion.
    pub flame_angle_degrees: f32,
    /// +0x74, ftCaptain/types.h. Multiplies retained friction on each eligible damage callback.
    pub on_hit_speed_multiplier: f32,
    /// +0x78, ftCaptain/types.h. Inclusive counter bound in ftCa_SpecialHi_800E400C.
    pub hit_slowdown_counter_limit: i32,
    /// +0x7C, ftCaptain/types.h. Rate of the grounded ending animation.
    pub ground_ending_animation_rate: f32,
    /// +0x80, ftCaptain/types.h. Rate of the aerial kick landing animation.
    pub landing_animation_rate: f32,
    /// +0x84, ftCaptain/types.h. Multiplies common ground friction during the ground ending.
    pub ground_traction_multiplier: f32,
    /// +0x88, ftCaptain/types.h. Multiplies common ground friction after the aerial kick lands.
    pub air_landing_traction_multiplier: f32,
}

pub fn read_captain_attributes(archive: &Archive) -> Result<CaptainAttributes> {
    let root = archive.public("ftDataCaptain").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataCaptain".into(),
        })
    })?;
    CaptainAttributes::read(archive, special_attributes_offset(archive, root)?)
}
impl CaptainAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, CAPTAIN_ATTRIBUTES_SIZE)?);
        Ok(Self {
            falcon_punch: FalconPunchAttributes {
                downward_stick_threshold: r.f32(0x0)?,
                upward_stick_threshold: r.f32(0x4)?,
                maximum_angle: r.f32(0x8)?,
                aerial_speed: r.f32(0xC)?,
                momentum_multiplier: r.f32(0x10)?,
            },
            raptor_boost: RaptorBoostAttributes {
                ground_hit_speed_multiplier: r.f32(0x14)?,
                gravity: r.f32(0x18)?,
                terminal_velocity: r.f32(0x1C)?,
                unused_parameters: [
                    r.f32(0x20)?,
                    r.f32(0x24)?,
                    r.f32(0x28)?,
                    r.f32(0x2C)?,
                    r.f32(0x30)?,
                    r.f32(0x34)?,
                ],
                miss_landing_lag: r.f32(0x38)?,
                hit_landing_lag: r.f32(0x3C)?,
            },
            falcon_dive: FalconDiveAttributes {
                air_acceleration_multiplier: r.f32(0x40)?,
                horizontal_speed_multiplier: r.f32(0x44)?,
                freefall_mobility: r.f32(0x48)?,
                landing_lag: r.f32(0x4C)?,
                unused_parameters: [r.f32(0x50)?, r.f32(0x54)?],
                reverse_stick_threshold: r.f32(0x58)?,
                initial_command_value: r.f32(0x5C)?,
                catch_gravity: r.f32(0x60)?,
                initial_air_counter: r.s32(0x64)?,
            },
            falcon_kick: FalconKickAttributes {
                unused_parameter: r.f32(0x68)?,
                unused_word: r.u32(0x6C)?,
                flame_angle_degrees: r.f32(0x70)?,
                on_hit_speed_multiplier: r.f32(0x74)?,
                hit_slowdown_counter_limit: r.s32(0x78)?,
                ground_ending_animation_rate: r.f32(0x7C)?,
                landing_animation_rate: r.f32(0x80)?,
                ground_traction_multiplier: r.f32(0x84)?,
                air_landing_traction_multiplier: r.f32(0x88)?,
            },
        })
    }
}
