//! ftDonkeyAttributes, ft/kinds/ftDonkey/types.h; ftData.ext_attr (+4).
//! ftDk_Init_OnLoad's PUSH_ATTRS copies 0x74 bytes after writing the three
//! carry-walk animation lengths into +8..+10.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

/// ftDk_Init_OnLoad (8010D9AC): lbAnim_8001E8F8(ftData_80085E50(fp, 296..298)),
/// the animations whose lengths become x8, xC and x10.
pub const CARRY_WALK_ANIMATIONS: [i32; 3] = [296, 297, 298];

/// sizeof(ftDonkeyAttributes).
pub const DONKEY_ATTRIBUTES_SIZE: u32 = 0x74;

#[derive(Clone, Debug, PartialEq)]
pub struct DonkeyAttributes {
    pub carry: CarryAttributes,
    pub giant_punch: GiantPunchAttributes,
    pub headbutt: HeadbuttAttributes,
    /// +0x48 x48_UNKNOWN.
    // TODO(meaning): no retail reader found.
    pub unknown_48: f32,
    pub spinning_kong: SpinningKongAttributes,
    pub hand_slap: HandSlapAttributes,
}

/// The heavy-item carry and the cargo carry (ftdonkeyheavy*.c and
/// ftCo_Cargo*.c through Fighter.x2CC).
#[derive(Clone, Debug, PartialEq)]
pub struct CarryAttributes {
    /// +0x00 motion_state: the first heavy-item carry row (HeavyWait).
    pub heavy_first_state: i32,
    /// +0x04 x4_motion_state: the first cargo row (ThrowFWait0).
    pub cargo_first_state: i32,
    /// +0x08..+0x10: the slow, middle and fast carry walks' animation
    /// lengths. On disc these are placeholders; ftDk_Init_OnLoad
    /// (8010D9AC) overwrites them with the lengths of animations 296..298.
    pub walk_lengths: [f32; 3],
    /// +0x14..+0x1C x14, x18, x1C: the carry walks' speed scales
    /// (ftWalkCommon_800DFCA4's last three floats).
    pub walk_speeds: [f32; 3],
    /// +0x20 cargo_hold.x20_TURN_SPEED: frames before the turn flips facing.
    pub turn_frames: f32,
    /// +0x24 cargo_hold.x24_JUMP_STARTUP_LAG.
    pub jump_squat_frames: f32,
    /// +0x28 cargo_hold.x28_LANDING_LAG.
    pub landing_frames: f32,
}

/// Giant Punch (ftdonkeyspecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct GiantPunchAttributes {
    /// +0x2C x2C_MAX_ARM_SWINGS: the full charge.
    pub max_swings: i32,
    /// +0x30 x30_DAMAGE_PER_SWING.
    pub damage_per_swing: i32,
    /// +0x34 x34_PUNCH_HORIZONTAL_VEL, per swing.
    pub punch_speed: f32,
    /// +0x38 x38_LANDING_LAG after an aerial punch (0: plain fall).
    pub landing_lag: f32,
}

/// Headbutt (ftdonkeyspecials.c).
#[derive(Clone, Debug, PartialEq)]
pub struct HeadbuttAttributes {
    /// +0x3C x3C_MIN_STICK_X_MOMENTUM: aerial entry x velocity divisor.
    pub entry_velocity_divisor: f32,
    /// +0x40 x40_MOMENTUM_TRANSITION_MODIFIER: aerial friction.
    pub air_friction: f32,
    /// +0x44 x44_AERIAL_GRAVITY.
    pub gravity: f32,
}

/// Spinning Kong (ftdonkeyspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct SpinningKongAttributes {
    /// +0x4C x4C_AERIAL_VERTICAL_VELOCITY.
    pub air_launch_y: f32,
    /// +0x50 x50_AERIAL_GRAVITY: gravity scale until the script's flag.
    pub gravity_scale: f32,
    /// +0x54 x54_GROUNDED_HORIZONTAL_VELOCITY.
    pub ground_max: f32,
    /// +0x58 x58_AERIAL_HORIZONTAL_VELOCITY.
    pub air_max: f32,
    /// +0x5C x5C_GROUNDED_MOBILITY.
    pub ground_mobility: f32,
    /// +0x60 x60_AERIAL_MOBILITY.
    pub air_mobility: f32,
    /// +0x64 x64_LANDING_LAG (0: plain fall).
    pub landing_lag: f32,
}

/// Hand Slap's quake hitboxes (ftDk_Init_8010DB3C).
#[derive(Clone, Debug, PartialEq)]
pub struct HandSlapAttributes {
    /// +0x68 x68: spacing between the four quake points along the floor.
    pub spacing: f32,
    /// +0x6C x6C: the points' offset ahead of Donkey Kong.
    pub forward_offset: f32,
    /// +0x70 x70: the floor walk's limit (mpLib_80056C54's last argument).
    pub reach: f32,
}

pub fn read_donkey_attributes(archive: &Archive) -> Result<DonkeyAttributes> {
    let root = archive.public("ftDataDonkey").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataDonkey".into(),
        })
    })?;
    DonkeyAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl DonkeyAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, DONKEY_ATTRIBUTES_SIZE)?);
        Ok(Self {
            carry: CarryAttributes {
                heavy_first_state: r.s32(0x00)?,
                cargo_first_state: r.s32(0x04)?,
                walk_lengths: [r.f32(0x08)?, r.f32(0x0C)?, r.f32(0x10)?],
                walk_speeds: [r.f32(0x14)?, r.f32(0x18)?, r.f32(0x1C)?],
                turn_frames: r.f32(0x20)?,
                jump_squat_frames: r.f32(0x24)?,
                landing_frames: r.f32(0x28)?,
            },
            giant_punch: GiantPunchAttributes {
                max_swings: r.s32(0x2C)?,
                damage_per_swing: r.s32(0x30)?,
                punch_speed: r.f32(0x34)?,
                landing_lag: r.f32(0x38)?,
            },
            headbutt: HeadbuttAttributes {
                entry_velocity_divisor: r.f32(0x3C)?,
                air_friction: r.f32(0x40)?,
                gravity: r.f32(0x44)?,
            },
            unknown_48: r.f32(0x48)?,
            spinning_kong: SpinningKongAttributes {
                air_launch_y: r.f32(0x4C)?,
                gravity_scale: r.f32(0x50)?,
                ground_max: r.f32(0x54)?,
                air_max: r.f32(0x58)?,
                ground_mobility: r.f32(0x5C)?,
                air_mobility: r.f32(0x60)?,
                landing_lag: r.f32(0x64)?,
            },
            hand_slap: HandSlapAttributes {
                spacing: r.f32(0x68)?,
                forward_offset: r.f32(0x6C)?,
                reach: r.f32(0x70)?,
            },
        })
    }
}
