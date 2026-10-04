//! ftNessAttributes, ft/kinds/ftNess/types.h; ftData.ext_attr (+4).
//! ftNs_Init_OnLoad's PUSH_ATTRS copies 0xDC bytes.
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_ft::desc::{
    fox_attributes::ReflectionAttributes, special_attributes_offset, FighterDescError,
};
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftNessAttributes): 0xB8 bytes and the bat's 0x24-byte ReflectDesc.
pub const NESS_ATTRIBUTES_SIZE: u32 = 0xDC;

#[derive(Clone, Debug, PartialEq)]
pub struct NessAttributes {
    pub pk_flash: PkFlashAttributes,
    pub pk_fire: PkFireAttributes,
    pub pk_thunder: PkThunderAttributes,
    pub pk_thunder_2: PkThunder2Attributes,
    pub magnet: MagnetAttributes,
    pub yoyo: YoyoAttributes,
    /// +0xB8 xB8_BASEBALL_BAT: the forward smash's reflect bubble.
    pub bat_reflection: ReflectionAttributes,
}

/// PK Flash (ftnessspecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct PkFlashAttributes {
    /// +0x00 x0_PKFLASH_TIMER1_LOOPFRAMES: frames Ness holds the loop on
    /// the ground once the flash is gone.
    pub ground_loop_frames: i32,
    /// +0x04 x4_PKFLASH_TIMER2_LOOPFRAMES: the same in the air.
    pub air_loop_frames: i32,
    /// +0x08 x8_PKFLASH_GRAVITY_DELAY: frames before the aerial move falls.
    pub gravity_delay: i32,
    /// +0x0C xC_PKFLASH_MINCHARGEFRAMES: frames before a released B
    /// detonates the flash.
    pub release_delay: i32,
    /// +0x10 x10_PKFLASH_UNK1: aerial entry x velocity divisor.
    pub entry_velocity_divisor: f32,
    /// +0x14 x14_PKFLASH_FALL_ACCEL.
    pub fall_acceleration: f32,
    /// +0x18 x18_PKFLASH_UNK2: the fall's terminal speed.
    pub terminal_velocity: f32,
    /// +0x1C x1C_PKFLASH_LANDING_LAG (0: plain fall).
    pub landing_lag: f32,
}

/// PK Fire (ftnessspecials.c).
#[derive(Clone, Debug, PartialEq)]
pub struct PkFireAttributes {
    /// +0x20 x20_PKFIRE_AERIAL_LAUNCH_TRAJECTORY, radians.
    pub air_angle: f32,
    /// +0x24 x24_PKFIRE_AERIAL_VELOCITY.
    pub air_speed: f32,
    /// +0x28 x28_PKFIRE_GROUNDED_LAUNCH_TRAJECTORY, radians.
    pub ground_angle: f32,
    /// +0x2C x2C_PKFIRE_GROUNDED_VELOCITY.
    pub ground_speed: f32,
    /// +0x30 x30_PKFIRE_SPAWN_X, +0x34 x34_PKFIRE_SPAWN_Y.
    pub spawn_x: f32,
    pub spawn_y: f32,
    /// +0x38 x38_PKFIRE_LANDING_LAG.
    pub landing_lag: f32,
}

/// PK Thunder while Ness steers it (ftnessspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct PkThunderAttributes {
    /// +0x3C x3C_PK_THUNDER_UNK1: aerial entry x velocity divisor.
    pub entry_velocity_divisor: f32,
    /// +0x40 x40_PK_THUNDER_LOOP1: frames Ness holds the loop once the
    /// thunder is gone.
    pub loop_frames: i32,
    /// +0x44 x44_PK_THUNDER_LOOP2.
    pub loop_frames_2: i32,
    /// +0x48 x48_PK_THUNDER_GRAVITY_DELAY.
    pub gravity_delay: i32,
    /// +0x4C x4C_PK_THUNDER_UNK2: the fall's terminal speed.
    pub terminal_velocity: f32,
    /// +0x50 x50_PK_THUNDER_FALL_ACCEL.
    pub fall_acceleration: f32,
}

/// PK Thunder 2, the self-hit launch (ftnessspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct PkThunder2Attributes {
    /// +0x54 x54_PK_THUNDER_2_MOMENTUM.
    pub speed: f32,
    /// +0x58 x58_PK_THUNDER_2_UNK1.
    // TODO(meaning): read by the launch's animation-rate code.
    pub unknown_58: f32,
    /// +0x5C x5C_PK_THUNDER_2_DECELERATION_RATE.
    pub deceleration: f32,
    /// +0x60 x60_PK_THUNDER_2_KNOCKDOWN_ANGLE, degrees.
    pub knockdown_angle: f32,
    /// +0x64 x64_PK_THUNDER_2_WALLHUG_ANGLE, degrees.
    pub wall_hug_angle: f32,
    /// +0x68 x68_PK_THUNDER_2_UNK2.
    // TODO(meaning): the rebound's speed scale.
    pub unknown_68: f32,
    /// +0x6C x6C_PK_THUNDER_2_FREEFALL_ANIM_BLEND.
    pub fall_blend: f32,
    /// +0x70 x70_PK_THUNDER_2_LANDING_LAG.
    pub landing_lag: f32,
}

/// PSI Magnet (ftnessspeciallw.c).
#[derive(Clone, Debug, PartialEq)]
pub struct MagnetAttributes {
    /// +0x74 x74_PSI_MAGNET_RELEASE_LAG: frames the magnet stays out with
    /// B released.
    pub release_lag: f32,
    /// +0x78 x78_PSI_MAGNET_UNK1: the turn rows' frames, which no retail
    /// code enters.
    pub turn_frames: f32,
    /// +0x7C x7C_PSI_MAGNET_UNK2: an absorb while the hit row is at or
    /// before this frame does not restart it (ftNs_AbsorbThink_DecideAction).
    pub hit_restart_frame: f32,
    /// +0x80 x80_PSI_MAGNET_UNK3: unused in retail (types.h).
    pub unused_80: f32,
    /// +0x84 x84_PSI_MAGNET_FRAMES_BEFORE_GRAVITY.
    pub gravity_delay: i32,
    /// +0x88 x88_PSI_MAGNET_MOMENTUM_PRESERVATION: aerial entry x velocity
    /// divisor.
    pub entry_velocity_divisor: f32,
    /// +0x8C x8C_PSI_MAGNET_FALL_ACCEL.
    pub fall_acceleration: f32,
    /// +0x90 x90_PSI_MAGNET_UNK4: unused in retail (types.h).
    pub unused_90: f32,
    /// +0x94 x94_PSI_MAGNET_HEAL_MUL: healed damage per absorbed damage.
    pub heal_multiplier: f32,
    /// +0x98 x98_PSI_MAGNET_ABSORPTION: the AbsorbDesc.
    pub volume: AbsorbVolume,
}

/// AbsorbDesc (lb/types.h): bone, offset and radius.
#[derive(Clone, Debug, PartialEq)]
pub struct AbsorbVolume {
    pub bone: i32,
    pub offset: Vec3,
    pub radius: f32,
}

/// The yo-yo smashes (ftnessattackhi4.c, ftnessattacklw4.c).
#[derive(Clone, Debug, PartialEq)]
pub struct YoyoAttributes {
    /// +0xAC xAC_YOYO_CHARGE_DURATION, frames.
    pub charge_frames: f32,
    /// +0xB0 xB0_YOYO_DAMAGE_MUL: the full charge's damage, in percent of
    /// the uncharged hit's above 100 (350).
    pub damage_multiplier: f32,
    /// +0xB4 xB4_YOYO_REHIT_RATE: frames before the yo-yo may hit again.
    pub rehit_frames: f32,
}

pub fn read_ness_attributes(archive: &Archive) -> Result<NessAttributes> {
    let root = archive.public("ftDataNess").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataNess".into(),
        })
    })?;
    NessAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl NessAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, NESS_ATTRIBUTES_SIZE)?);
        Ok(Self {
            pk_flash: PkFlashAttributes {
                ground_loop_frames: r.s32(0x00)?,
                air_loop_frames: r.s32(0x04)?,
                gravity_delay: r.s32(0x08)?,
                release_delay: r.s32(0x0C)?,
                entry_velocity_divisor: r.f32(0x10)?,
                fall_acceleration: r.f32(0x14)?,
                terminal_velocity: r.f32(0x18)?,
                landing_lag: r.f32(0x1C)?,
            },
            pk_fire: PkFireAttributes {
                air_angle: r.f32(0x20)?,
                air_speed: r.f32(0x24)?,
                ground_angle: r.f32(0x28)?,
                ground_speed: r.f32(0x2C)?,
                spawn_x: r.f32(0x30)?,
                spawn_y: r.f32(0x34)?,
                landing_lag: r.f32(0x38)?,
            },
            pk_thunder: PkThunderAttributes {
                entry_velocity_divisor: r.f32(0x3C)?,
                loop_frames: r.s32(0x40)?,
                loop_frames_2: r.s32(0x44)?,
                gravity_delay: r.s32(0x48)?,
                terminal_velocity: r.f32(0x4C)?,
                fall_acceleration: r.f32(0x50)?,
            },
            pk_thunder_2: PkThunder2Attributes {
                speed: r.f32(0x54)?,
                unknown_58: r.f32(0x58)?,
                deceleration: r.f32(0x5C)?,
                knockdown_angle: r.f32(0x60)?,
                wall_hug_angle: r.f32(0x64)?,
                unknown_68: r.f32(0x68)?,
                fall_blend: r.f32(0x6C)?,
                landing_lag: r.f32(0x70)?,
            },
            magnet: MagnetAttributes {
                release_lag: r.f32(0x74)?,
                turn_frames: r.f32(0x78)?,
                hit_restart_frame: r.f32(0x7C)?,
                unused_80: r.f32(0x80)?,
                gravity_delay: r.s32(0x84)?,
                entry_velocity_divisor: r.f32(0x88)?,
                fall_acceleration: r.f32(0x8C)?,
                unused_90: r.f32(0x90)?,
                heal_multiplier: r.f32(0x94)?,
                volume: AbsorbVolume {
                    bone: r.s32(0x98)?,
                    offset: Vec3::new(r.f32(0x9C)?, r.f32(0xA0)?, r.f32(0xA4)?),
                    radius: r.f32(0xA8)?,
                },
            },
            yoyo: YoyoAttributes {
                charge_frames: r.f32(0xAC)?,
                damage_multiplier: r.f32(0xB0)?,
                rehit_frames: r.f32(0xB4)?,
            },
            bat_reflection: ReflectionAttributes::read_at(r, 0xB8)?,
        })
    }
}
