//! ftPurin/types.h: ftData.ext_attr (+4), 0x100 bytes.
//! The first 0x34 bytes are shared Fighter_x2D0_t multijump data (ft/types.h).
//! Its consumer types correct stale float/integer declarations in ftPurin/types.h.
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
pub struct RolloutAttributes {
    /// +0x34, ftPurin/types.h; ftpurinspecialn.c.
    pub duration: i32,
    /// +0x38, ftPurin/types.h; ftpurinspecialn.c.
    pub hit_frame_cost: i32,
    /// +0x3C, ftPurin/types.h; ftpurinspecialn.c.
    pub gravity: f32,
    /// +0x40, ftPurin/types.h; ftpurinspecialn.c.
    pub terminal_velocity: f32,
    /// +0x44, ftPurin/types.h; ftpurinspecialn.c.
    pub ground_acceleration: f32,
    /// +0x4C, ftPurin/types.h; ftpurinspecialn.c.
    pub ground_speed_limit: f32,
    /// +0x50, ftPurin/types.h; ftpurinspecialn.c.
    pub slope_speed_limit: f32,
    /// +0x54, ftPurin/types.h; ftpurinspecialn.c.
    pub air_acceleration: f32,
    /// +0x58, ftPurin/types.h; ftpurinspecialn.c.
    pub air_deceleration: f32,
    /// +0x5C, ftPurin/types.h; ftpurinspecialn.c.
    pub air_minimum_speed: f32,
    /// +0x68, ftPurin/types.h; ftpurinspecialn.c.
    pub reverse_stick_threshold: f32,
    /// +0x6C, ftPurin/types.h; ftpurinspecialn.c.
    pub turn_rotation_speed: f32,
    /// +0x70, ftPurin/types.h; ftpurinspecialn.c.
    pub turn_effect_interval: i32,
    /// +0x74, ftPurin/types.h; ftpurinspecialn.c.
    pub landing_speed_threshold: f32,
    /// +0x78, ftPurin/types.h; ftpurinspecialn.c.
    pub bounce_vertical_multiplier: f32,
    /// +0x7C, ftPurin/types.h; ftpurinspecialn.c.
    pub minimum_bounce_speed: f32,
    /// +0x80, ftPurin/types.h; ftpurinspecialn.c.
    pub damage_speed_offset: f32,
    /// +0x84, ftPurin/types.h; ftpurinspecialn.c.
    pub damage_multiplier: f32,
    /// +0x88, ftPurin/types.h; ftpurinspecialn.c.
    pub hit_horizontal_multiplier: f32,
    /// +0x8C, ftPurin/types.h; ftpurinspecialn.c.
    pub hit_vertical_speed: f32,
    /// +0x90, ftPurin/types.h; ftpurinspecialn.c.
    pub rebound_horizontal_multiplier: f32,
    /// +0x94, ftPurin/types.h; ftpurinspecialn.c.
    pub rebound_vertical_multiplier: f32,
    /// +0x98, ftPurin/types.h; ftpurinspecialn.c.
    pub release_rotation_speed: f32,
    /// +0x9C, ftPurin/types.h; ftpurinspecialn.c.
    pub hitbox_refresh_interval: i32,
    /// +0xA0, ftPurin/types.h; ftpurinspecialn.c.
    pub initial_charge: f32,
    /// +0xA4, ftPurin/types.h; ftpurinspecialn.c.
    pub maximum_charge: f32,
    /// +0xA8, ftPurin/types.h; ftpurinspecialn.c.
    pub charge_per_tick: f32,
    /// +0xAC, ftPurin/types.h; ftpurinspecialn.c.
    pub charge_rotation_multiplier: f32,
    /// +0xB4, ftPurin/types.h; ftpurinspecialn.c.
    pub charge_decay: f32,
    /// +0xB8, ftPurin/types.h; ftpurinspecialn.c.
    pub minimum_charge: f32,
    /// +0xBC, ftPurin/types.h; ftpurinspecialn.c.
    pub air_rotation_multiplier: f32,
    /// +0xC0, ftPurin/types.h; ftpurinspecialn.c.
    pub charge_speed_multiplier: f32,
    /// +0xC4, ftPurin/types.h; ftpurinspecialn.c.
    pub ground_friction_multiplier: f32,
    /// +0xC8, ftPurin/types.h; ftpurinspecialn.c.
    pub slope_influence: f32,
    /// +0xCC, ftPurin/types.h; ftpurinspecialn.c.
    pub minimum_hitbox_speed: f32,
    /// +0xD0, ftPurin/types.h; ftpurinspecialn.c.
    pub turn_speed_ratio: f32,
    /// +0xD4, ftPurin/types.h; ftpurinspecialn.c.
    pub wall_rebound_multiplier: f32,
    /// +0xD8, ftPurin/types.h; ftpurinspecialn.c.
    pub landing_lag: f32,
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
            rollout: RolloutAttributes {
                duration: r.s32(0x34)?,
                hit_frame_cost: r.s32(0x38)?,
                gravity: r.f32(0x3C)?,
                terminal_velocity: r.f32(0x40)?,
                ground_acceleration: r.f32(0x44)?,
                ground_speed_limit: r.f32(0x4C)?,
                slope_speed_limit: r.f32(0x50)?,
                air_acceleration: r.f32(0x54)?,
                air_deceleration: r.f32(0x58)?,
                air_minimum_speed: r.f32(0x5C)?,
                reverse_stick_threshold: r.f32(0x68)?,
                turn_rotation_speed: r.f32(0x6C)?,
                turn_effect_interval: r.s32(0x70)?,
                landing_speed_threshold: r.f32(0x74)?,
                bounce_vertical_multiplier: r.f32(0x78)?,
                minimum_bounce_speed: r.f32(0x7C)?,
                damage_speed_offset: r.f32(0x80)?,
                damage_multiplier: r.f32(0x84)?,
                hit_horizontal_multiplier: r.f32(0x88)?,
                hit_vertical_speed: r.f32(0x8C)?,
                rebound_horizontal_multiplier: r.f32(0x90)?,
                rebound_vertical_multiplier: r.f32(0x94)?,
                release_rotation_speed: r.f32(0x98)?,
                hitbox_refresh_interval: r.s32(0x9C)?,
                initial_charge: r.f32(0xA0)?,
                maximum_charge: r.f32(0xA4)?,
                charge_per_tick: r.f32(0xA8)?,
                charge_rotation_multiplier: r.f32(0xAC)?,
                charge_decay: r.f32(0xB4)?,
                minimum_charge: r.f32(0xB8)?,
                air_rotation_multiplier: r.f32(0xBC)?,
                charge_speed_multiplier: r.f32(0xC0)?,
                ground_friction_multiplier: r.f32(0xC4)?,
                slope_influence: r.f32(0xC8)?,
                minimum_hitbox_speed: r.f32(0xCC)?,
                turn_speed_ratio: r.f32(0xD0)?,
                wall_rebound_multiplier: r.f32(0xD4)?,
                landing_lag: r.f32(0xD8)?,
            },
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
