//! ftGameWatchAttributes, ft/kinds/ftGameWatch/types.h; ftData.ext_attr (+4).
//! ftGw_Init_OnLoad's PUSH_ATTRS copies 0x94 bytes; ftGw_Init_LoadSpecialAttrs
//! copies them again unscaled.
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftGameWatchAttributes).
pub const GAMEWATCH_ATTRIBUTES_SIZE: u32 = 0x94;

/// Judgment's faces, 1 to 9.
pub const JUDGE_FACES: usize = 9;

#[derive(Clone, Debug, PartialEq)]
pub struct GameWatchAttributes {
    /// +0x00 x0_GAMEWATCH_WIDTH: the model's z scale (fp->x34_scale.z).
    pub width: f32,
    /// +0x04 x4_GAMEWATCH_COLOR: the four costumes' body colours (RGBA).
    pub colors: [[u8; 4]; 4],
    /// +0x14 x14_GAMEWATCH_OUTLINE: the outline colour (RGBA).
    pub outline: [u8; 4],
    pub chef: ChefAttributes,
    pub judge: JudgeAttributes,
    pub rescue: RescueAttributes,
    pub panic: PanicAttributes,
}

/// Chef (ftgamewatchspecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct ChefAttributes {
    /// +0x18 x18_GAMEWATCH_CHEF_LOOPFRAME: the frame the loop restarts at.
    pub loop_frame: f32,
    /// +0x1C x1C_GAMEWATCH_CHEF_MAX: sausages per use, as a float.
    pub max_sausages: f32,
}

/// Judgment (ftgamewatchspecials.c).
#[derive(Clone, Debug, PartialEq)]
pub struct JudgeAttributes {
    /// +0x20 x20: the entry x velocity's divisor.
    pub momentum_preserve: f32,
    /// +0x24 x24: aerial x deceleration.
    pub momentum_mul: f32,
    /// +0x28 x28: aerial fall acceleration.
    pub vel_y: f32,
    /// +0x2C x2C: aerial fall acceleration after the swing.
    pub friction1: f32,
    /// +0x30 x30: terminal velocity scale.
    pub friction2: f32,
    /// +0x34 x34_GAMEWATCH_JUDGE_ROLL: which faces can be drawn.
    pub enabled: [bool; JUDGE_FACES],
}

/// Fire (ftgamewatchspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct RescueAttributes {
    /// +0x58 x58_GAMEWATCH_RESCUE_STICK_RANGE.
    pub stick_range: f32,
    /// +0x5C x5C_GAMEWATCH_RESCUE_ANGLE_UNK, radians.
    pub angle: f32,
    /// +0x60 x60_GAMEWATCH_RESCUE_LANDING: special landing lag.
    pub landing_lag: f32,
}

/// Oil Panic (ftgamewatchspeciallw.c).
#[derive(Clone, Debug, PartialEq)]
pub struct PanicAttributes {
    /// +0x64 x64: the entry x velocity's divisor.
    pub momentum_preserve: f32,
    /// +0x68 x68: aerial x deceleration.
    pub momentum_mul: f32,
    /// +0x6C x6C: aerial fall acceleration.
    pub fall_accel: f32,
    /// +0x70 x70: terminal fall velocity.
    pub vel_y_max: f32,
    /// +0x74 x74: damage added to the spill after the formula.
    pub damage_add: f32,
    /// +0x78 x78: the absorbed damage's multiplier.
    pub damage_mul: f32,
    /// +0x7C x7C: frames a commanded turnaround takes.
    pub turn_frames: f32,
    /// +0x80 x80_GAMEWATCH_PANIC_ABSORPTION: the bucket's absorb bubble.
    pub absorb: AbsorbBubble,
}

/// AbsorbDesc (lb/types.h:128).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbsorbBubble {
    pub bone: i32,
    pub offset: Vec3,
    pub size: f32,
}

pub fn read_gamewatch_attributes(archive: &Archive) -> Result<GameWatchAttributes> {
    let root = archive.public("ftDataGamewatch").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataGamewatch".into(),
        })
    })?;
    GameWatchAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl GameWatchAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, GAMEWATCH_ATTRIBUTES_SIZE)?);
        let rgba = |at: u32| -> Result<[u8; 4]> { Ok(r.u32(at)?.to_be_bytes()) };
        let mut enabled = [false; JUDGE_FACES];
        for (i, face) in enabled.iter_mut().enumerate() {
            *face = r.s32(0x34 + 4 * i as u32)? != 0;
        }
        Ok(Self {
            width: r.f32(0x00)?,
            colors: [rgba(0x04)?, rgba(0x08)?, rgba(0x0C)?, rgba(0x10)?],
            outline: rgba(0x14)?,
            chef: ChefAttributes {
                loop_frame: r.f32(0x18)?,
                max_sausages: r.f32(0x1C)?,
            },
            judge: JudgeAttributes {
                momentum_preserve: r.f32(0x20)?,
                momentum_mul: r.f32(0x24)?,
                vel_y: r.f32(0x28)?,
                friction1: r.f32(0x2C)?,
                friction2: r.f32(0x30)?,
                enabled,
            },
            rescue: RescueAttributes {
                stick_range: r.f32(0x58)?,
                angle: r.f32(0x5C)?,
                landing_lag: r.f32(0x60)?,
            },
            panic: PanicAttributes {
                momentum_preserve: r.f32(0x64)?,
                momentum_mul: r.f32(0x68)?,
                fall_accel: r.f32(0x6C)?,
                vel_y_max: r.f32(0x70)?,
                damage_add: r.f32(0x74)?,
                damage_mul: r.f32(0x78)?,
                turn_frames: r.f32(0x7C)?,
                absorb: AbsorbBubble {
                    bone: r.s32(0x80)?,
                    offset: Vec3::new(r.f32(0x84)?, r.f32(0x88)?, r.f32(0x8C)?),
                    size: r.f32(0x90)?,
                },
            },
        })
    }
}
