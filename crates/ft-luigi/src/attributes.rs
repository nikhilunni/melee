//! ftLuigiAttributes, ft/kinds/ftLuigi/types.h; ftData.ext_attr (+4).
//! ftLg_Init_OnLoad's PUSH_ATTRS copies 0x98 bytes without scaling.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

pub const LUIGI_ATTRIBUTES_SIZE: u32 = 0x98;

#[derive(Clone, Debug, PartialEq)]
pub struct LuigiAttributes {
    pub green_missile: GreenMissileAttributes,
    pub super_jump_punch: SuperJumpPunchAttributes,
    pub cyclone: CycloneAttributes,
}

/// Green Missile (side special), +0x00..+0x4C.
#[derive(Clone, Debug, PartialEq)]
pub struct GreenMissileAttributes {
    /// +0x00.
    // TODO(meaning): no retail reader found.
    pub unknown: f32,
    /// +0x04: a side special entered within this many frames of the stick
    /// crossing (x673) starts pre-charged.
    pub smash_window: f32,
    /// +0x08: charge frames a smash-input start begins with.
    pub smash_charge: f32,
    /// +0x0C: charge frames that launch on their own.
    pub max_charge_frames: f32,
    /// +0x10: base damage of the launch hitbox.
    pub base_damage: f32,
    /// +0x14: damage per charge frame.
    pub damage_per_charge: f32,
    /// +0x18: horizontal speed divisor on entry.
    pub entry_speed_divisor: f32,
    /// +0x1C: wind-up friction (ground and air).
    pub windup_friction: f32,
    /// +0x20: wind-up gravity once the script allows it.
    pub windup_gravity: f32,
    /// +0x24: launch speed at no charge.
    pub launch_speed: f32,
    /// +0x28: launch speed per charge frame.
    pub launch_speed_per_charge: f32,
    /// +0x2C: launch vertical speed at full charge.
    pub launch_vertical_speed: f32,
    /// +0x30: gravity while flying.
    pub flight_gravity: f32,
    /// +0x34: terminal velocity while flying.
    pub flight_terminal_velocity: f32,
    /// +0x38: horizontal speed divisor when the flight ends.
    pub end_speed_divisor: f32,
    /// +0x3C: friction once the flight slows, and in the end states.
    pub end_friction: f32,
    /// +0x40: gravity once the flight slows, and in the aerial end.
    pub end_gravity: f32,
    /// +0x44: HSD_Randi's range (truncated by fctiwz); zero misfires.
    pub misfire_odds: f32,
    /// +0x48: misfire launch speed.
    pub misfire_speed: f32,
    /// +0x4C: misfire vertical speed.
    pub misfire_vertical_speed: f32,
}

/// Super Jump Punch, +0x50..+0x6C (Mario's layout).
#[derive(Clone, Debug, PartialEq)]
pub struct SuperJumpPunchAttributes {
    /// +0x50: special fall air mobility.
    pub freefall_mobility: f32,
    /// +0x54: special landing lag.
    pub landing_lag: f32,
    /// +0x58: stick range that reverses the facing.
    pub reverse_stick_range: f32,
    /// +0x5C: stick range that steers the jump.
    pub momentum_stick_range: f32,
    /// +0x60: maximum steering angle, in degrees.
    pub angle_diff: f32,
    /// +0x64: horizontal velocity scale of an aerial start.
    pub vel_x: f32,
    /// +0x68: gravity before the rise.
    pub gravity: f32,
    /// +0x6C: velocity multiplier of the rise.
    pub vel_mul: f32,
}

/// Luigi Cyclone, +0x70..+0x94.
#[derive(Clone, Debug, PartialEq)]
pub struct CycloneAttributes {
    /// +0x70: vertical velocity on entry.
    pub tap_momentum: f32,
    /// +0x74: grounded horizontal speed.
    pub momentum_x_ground: f32,
    /// +0x78: aerial horizontal speed.
    pub momentum_x_air: f32,
    /// +0x7C: grounded acceleration.
    pub momentum_x_mul_ground: f32,
    /// +0x80: aerial acceleration.
    pub momentum_x_mul_air: f32,
    /// +0x84: deceleration per frame at the end.
    pub friction_end: f32,
    /// +0x88: one less than the unread motion scratch value.
    // TODO(meaning): SpecialLw.unk is written but never read.
    pub unknown: i32,
    /// +0x8C: the rise a B tap gives.
    pub tap_y_vel_max: f32,
    /// +0x90: gravity while rising.
    pub tap_gravity: f32,
    /// +0x94: landing lag; zero falls instead.
    pub landing_lag: i32,
}

pub fn read_luigi_attributes(archive: &Archive) -> Result<LuigiAttributes> {
    let root = archive.public("ftDataLuigi").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataLuigi".into(),
        })
    })?;
    LuigiAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl LuigiAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, LUIGI_ATTRIBUTES_SIZE)?);
        Ok(Self {
            green_missile: GreenMissileAttributes {
                unknown: r.f32(0x00)?,
                smash_window: r.f32(0x04)?,
                smash_charge: r.f32(0x08)?,
                max_charge_frames: r.f32(0x0C)?,
                base_damage: r.f32(0x10)?,
                damage_per_charge: r.f32(0x14)?,
                entry_speed_divisor: r.f32(0x18)?,
                windup_friction: r.f32(0x1C)?,
                windup_gravity: r.f32(0x20)?,
                launch_speed: r.f32(0x24)?,
                launch_speed_per_charge: r.f32(0x28)?,
                launch_vertical_speed: r.f32(0x2C)?,
                flight_gravity: r.f32(0x30)?,
                flight_terminal_velocity: r.f32(0x34)?,
                end_speed_divisor: r.f32(0x38)?,
                end_friction: r.f32(0x3C)?,
                end_gravity: r.f32(0x40)?,
                misfire_odds: r.f32(0x44)?,
                misfire_speed: r.f32(0x48)?,
                misfire_vertical_speed: r.f32(0x4C)?,
            },
            super_jump_punch: SuperJumpPunchAttributes {
                freefall_mobility: r.f32(0x50)?,
                landing_lag: r.f32(0x54)?,
                reverse_stick_range: r.f32(0x58)?,
                momentum_stick_range: r.f32(0x5C)?,
                angle_diff: r.f32(0x60)?,
                vel_x: r.f32(0x64)?,
                gravity: r.f32(0x68)?,
                vel_mul: r.f32(0x6C)?,
            },
            cyclone: CycloneAttributes {
                tap_momentum: r.f32(0x70)?,
                momentum_x_ground: r.f32(0x74)?,
                momentum_x_air: r.f32(0x78)?,
                momentum_x_mul_ground: r.f32(0x7C)?,
                momentum_x_mul_air: r.f32(0x80)?,
                friction_end: r.f32(0x84)?,
                unknown: r.s32(0x88)?,
                tap_y_vel_max: r.f32(0x8C)?,
                tap_gravity: r.f32(0x90)?,
                landing_lag: r.s32(0x94)?,
            },
        })
    }
}
