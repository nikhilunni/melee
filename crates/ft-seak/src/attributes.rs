//! ftSeakAttributes, ft/kinds/ftSeak/types.h; ftData.ext_attr (+4).
//! ftSk_Init_OnLoad's PUSH_ATTRS copies 0x74 bytes without scaling.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

pub const SEAK_ATTRIBUTES_SIZE: u32 = 0x74;

#[derive(Clone, Debug, PartialEq)]
pub struct SeakAttributes {
    pub needles: NeedleAttributes,
    pub chain: ChainAttributes,
    pub vanish: VanishAttributes,
    pub transform: TransformAttributes,
}

/// Needle Storm (ftseakspecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct NeedleAttributes {
    /// +0x00: grounded throw offset ahead of Sheik, times her scale.
    pub ground_offset_x: f32,
    /// +0x04: grounded throw height.
    pub ground_offset_y: f32,
    /// +0x08: aerial throw offset ahead of Sheik.
    pub air_offset_x: f32,
    /// +0x0C: aerial throw height.
    pub air_offset_y: f32,
    /// +0x10: landing lag of an aerial throw that lands; zero lands in Fall.
    pub landing_lag: f32,
}

/// Chain (ftseakspecials.c). The counters compare against the motion's
/// integer frame count, converted to float.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainAttributes {
    /// +0x14: frames of the loop before a released B retracts.
    pub minimum_loop_frames: f32,
    /// +0x18: frames the chain's hitboxes stay live after it moves fast.
    pub hit_frames: f32,
    /// +0x1C: start frame that spawns the chain.
    pub spawn_frame: f32,
    /// +0x20: start frame after which the loop begins.
    pub start_frames: f32,
    /// +0x24: end frame that begins reeling the chain in (it_802BCF84).
    pub retract_frame: f32,
    /// +0x28: end frame that removes the chain (it_802BB20C).
    pub remove_frame: f32,
}

/// Vanish (ftseakspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct VanishAttributes {
    /// +0x2C: the aerial start's vertical velocity.
    pub air_start_velocity_y: f32,
    /// +0x30: the aerial start's gravity.
    pub air_start_gravity: f32,
    /// +0x34: the aerial start's terminal velocity.
    pub air_start_terminal_velocity: f32,
    /// +0x38: frames spent travelling while invisible.
    pub travel_frames: i32,
    /// +0x3C: travel frames before a platform can stop the aerial travel.
    pub platform_frames: f32,
    /// +0x40: stick magnitude below which Sheik travels straight up.
    pub stick_threshold: f32,
    /// +0x44: travel speed added per unit of stick magnitude.
    pub speed_per_stick: f32,
    /// +0x48: base travel speed.
    pub base_speed: f32,
    /// +0x4C: special-fall horizontal speed, times air_drift_max.
    pub freefall_speed_multiplier: f32,
    /// +0x50: ftCommon_HandleTeleportCollisions's angle clamp: a wall or
    /// ceiling ends the aerial travel when met at more than 90 plus this
    /// many degrees.
    pub surface_angle_degrees: i32,
    /// +0x54: velocity kept when the travel ends.
    pub end_velocity_multiplier: f32,
    /// +0x58: special-fall air mobility.
    pub freefall_mobility: f32,
    /// +0x5C: special landing lag.
    pub landing_lag: f32,
}

/// Transform (ftseakspeciallw.c).
#[derive(Clone, Debug, PartialEq)]
pub struct TransformAttributes {
    /// +0x60: horizontal velocity divisor on entry.
    pub horizontal_velocity_divisor: f32,
    /// +0x64: vertical velocity divisor on entry.
    pub vertical_velocity_divisor: f32,
    /// +0x68: aerial gravity.
    pub gravity: f32,
    /// +0x6C: aerial terminal velocity.
    pub terminal_velocity: f32,
    /// +0x70: frame the arriving form's second motion starts at.
    pub finish_start_frame: f32,
}

pub fn read_seak_attributes(archive: &Archive) -> Result<SeakAttributes> {
    let root = archive.public("ftDataSeak").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataSeak".into(),
        })
    })?;
    SeakAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl SeakAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, SEAK_ATTRIBUTES_SIZE)?);
        Ok(Self {
            needles: NeedleAttributes {
                ground_offset_x: r.f32(0x00)?,
                ground_offset_y: r.f32(0x04)?,
                air_offset_x: r.f32(0x08)?,
                air_offset_y: r.f32(0x0C)?,
                landing_lag: r.f32(0x10)?,
            },
            chain: ChainAttributes {
                minimum_loop_frames: r.f32(0x14)?,
                hit_frames: r.f32(0x18)?,
                spawn_frame: r.f32(0x1C)?,
                start_frames: r.f32(0x20)?,
                retract_frame: r.f32(0x24)?,
                remove_frame: r.f32(0x28)?,
            },
            vanish: VanishAttributes {
                air_start_velocity_y: r.f32(0x2C)?,
                air_start_gravity: r.f32(0x30)?,
                air_start_terminal_velocity: r.f32(0x34)?,
                travel_frames: r.s32(0x38)?,
                platform_frames: r.f32(0x3C)?,
                stick_threshold: r.f32(0x40)?,
                speed_per_stick: r.f32(0x44)?,
                base_speed: r.f32(0x48)?,
                freefall_speed_multiplier: r.f32(0x4C)?,
                surface_angle_degrees: r.s32(0x50)?,
                end_velocity_multiplier: r.f32(0x54)?,
                freefall_mobility: r.f32(0x58)?,
                landing_lag: r.f32(0x5C)?,
            },
            transform: TransformAttributes {
                horizontal_velocity_divisor: r.f32(0x60)?,
                vertical_velocity_divisor: r.f32(0x64)?,
                gravity: r.f32(0x68)?,
                terminal_velocity: r.f32(0x6C)?,
                finish_start_frame: r.f32(0x70)?,
            },
        })
    }
}
