//! ftMewtwoAttributes, ft/kinds/ftMewtwo/types.h; ftData.ext_attr (+4).
//! ftMt_Init_OnLoad's PUSH_ATTRS copies 0x88 bytes without scaling;
//! ftMt_Init_LoadSpecialAttrs scales Disable's offsets by the model scale.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{
    fox_attributes::ReflectionAttributes, special_attributes_offset, FighterDescError,
};
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftMewtwoAttributes).
pub const MEWTWO_ATTRIBUTES_SIZE: u32 = 0x88;
/// Offset of Confusion's reflector in ftMewtwoAttributes.
const CONFUSION_REFLECTION: u32 = 0x1C;

#[derive(Clone, Debug, PartialEq)]
pub struct MewtwoAttributes {
    pub shadow_ball: ShadowBallAttributes,
    pub confusion: ConfusionAttributes,
    pub teleport: TeleportAttributes,
    pub disable: DisableAttributes,
}

/// Shadow Ball (ftmewtwospecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowBallAttributes {
    /// +0x00 x0_MEWTWO_SHADOWBALL_CHARGE_CYCLES: the stored charge at which
    /// the ball is full (compared with x2234 as a float).
    pub full_charge: f32,
    /// +0x04 x4_MEWTWO_SHADOWBALL_GROUND_RECOIL_X.
    pub ground_recoil: f32,
    /// +0x08 x8_MEWTWO_SHADOWBALL_AIR_RECOIL_X.
    pub air_recoil: f32,
    /// +0x0C xC_MEWTWO_SHADOWBALL_CHARGE_ITERATIONS: loop frames per charge
    /// step.
    pub frames_per_charge: i32,
    /// +0x10 x10_MEWTWO_SHADOWBALL_RELEASE_LAG.
    pub release_lag: i32,
    /// +0x14 x14_MEWTWO_SHADOWBALL_LANDING_LAG (0: plain fall).
    pub landing_lag: f32,
}

/// Confusion (ftmewtwospecials.c).
#[derive(Clone, Debug, PartialEq)]
pub struct ConfusionAttributes {
    /// +0x18 x18_MEWTWO_CONFUSION_AIR_BOOST: the first aerial use's lift.
    pub air_boost: f32,
    /// +0x1C x1C_MEWTWO_CONFUSION_REFLECTION.
    pub reflection: ReflectionAttributes,
}

/// Teleport (ftmewtwospecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct TeleportAttributes {
    /// +0x40 x40_MEWTWO_TELEPORT_VEL_DIV_X: aerial entry x velocity divisor.
    pub air_horizontal_velocity_divisor: f32,
    /// +0x44 x44_MEWTWO_TELEPORT_VEL_DIV_Y: aerial entry y velocity divisor.
    pub air_vertical_velocity_divisor: f32,
    /// +0x48 x48_MEWTWO_TELEPORT_GRAVITY: the aerial start's gravity.
    pub air_start_gravity: f32,
    /// +0x4C x4C_MEWTWO_TELEPORT_TERMINAL_VELOCITY.
    pub air_start_terminal_velocity: f32,
    /// +0x50 x50_MEWTWO_TELEPORT_DURATION: frames spent travelling.
    pub travel_frames: i32,
    /// +0x54 x54_MEWTWO_TELEPORT_UNK2: travel frames before a platform can
    /// stop the aerial travel.
    pub platform_frames: f32,
    /// +0x58 x58_MEWTWO_TELEPORT_STICK_RANGE_MIN: stick magnitude below
    /// which Mewtwo travels straight up.
    pub stick_threshold: f32,
    /// +0x5C x5C_MEWTWO_TELEPORT_MOMENTUM: travel speed per unit of stick.
    pub speed_per_stick: f32,
    /// +0x60 x60_MEWTWO_TELEPORT_MOMENTUM_ADD: base travel speed.
    pub base_speed: f32,
    /// +0x64 x64_MEWTWO_TELEPORT_DRIFT: special-fall horizontal speed, times
    /// air_drift_max.
    pub freefall_speed_multiplier: f32,
    /// +0x68 x68_MEWTWO_TELEPORT_ANGLE_CLAMP: a wall or ceiling ends the
    /// aerial travel when met at more than 90 plus this many degrees.
    pub surface_angle_degrees: i32,
    /// +0x6C x6C_MEWTWO_TELEPORT_MOMENTUM_END_MUL: velocity kept when the
    /// travel ends.
    pub end_velocity_multiplier: f32,
    /// +0x70 x70_MEWTWO_TELEPORT_FREEFALL_MOBILITY.
    pub freefall_mobility: f32,
    /// +0x74 x74_MEWTWO_TELEPORT_LANDING_LAG.
    pub landing_lag: f32,
}

/// Disable (ftmewtwospeciallw.c).
#[derive(Clone, Debug, PartialEq)]
pub struct DisableAttributes {
    /// +0x78 x78_MEWTWO_DISABLE_GRAVITY.
    pub gravity: f32,
    /// +0x7C x7C_MEWTWO_DISABLE_TERMINAL_VELOCITY.
    pub terminal_velocity: f32,
    /// +0x80 x80_MEWTWO_DISABLE_OFFSET_X: the projectile's spawn offset
    /// ahead of Mewtwo, times the model scale (ftMt_Init_LoadSpecialAttrs).
    pub offset_x: f32,
    /// +0x84 x84_MEWTWO_DISABLE_OFFSET_Y, scaled likewise.
    pub offset_y: f32,
}

pub fn read_mewtwo_attributes(archive: &Archive) -> Result<MewtwoAttributes> {
    let root = archive.public("ftDataMewtwo").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataMewtwo".into(),
        })
    })?;
    MewtwoAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl MewtwoAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, MEWTWO_ATTRIBUTES_SIZE)?);
        Ok(Self {
            shadow_ball: ShadowBallAttributes {
                full_charge: r.f32(0x00)?,
                ground_recoil: r.f32(0x04)?,
                air_recoil: r.f32(0x08)?,
                frames_per_charge: r.s32(0x0C)?,
                release_lag: r.s32(0x10)?,
                landing_lag: r.f32(0x14)?,
            },
            confusion: ConfusionAttributes {
                air_boost: r.f32(0x18)?,
                reflection: ReflectionAttributes::read_at(r, CONFUSION_REFLECTION)?,
            },
            teleport: TeleportAttributes {
                air_horizontal_velocity_divisor: r.f32(0x40)?,
                air_vertical_velocity_divisor: r.f32(0x44)?,
                air_start_gravity: r.f32(0x48)?,
                air_start_terminal_velocity: r.f32(0x4C)?,
                travel_frames: r.s32(0x50)?,
                platform_frames: r.f32(0x54)?,
                stick_threshold: r.f32(0x58)?,
                speed_per_stick: r.f32(0x5C)?,
                base_speed: r.f32(0x60)?,
                freefall_speed_multiplier: r.f32(0x64)?,
                surface_angle_degrees: r.s32(0x68)?,
                end_velocity_multiplier: r.f32(0x6C)?,
                freefall_mobility: r.f32(0x70)?,
                landing_lag: r.f32(0x74)?,
            },
            disable: DisableAttributes {
                gravity: r.f32(0x78)?,
                terminal_velocity: r.f32(0x7C)?,
                offset_x: r.f32(0x80)?,
                offset_y: r.f32(0x84)?,
            },
        })
    }
}
