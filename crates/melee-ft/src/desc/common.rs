//! M3 subset of PlCo `p_ftCommonData`. Root slot 0 is installed by
//! `Fighter_LoadCommonData` (fighter.c:182-188); it is not ftCo_DatAttrs.
//! Wait animation selection uses ftData.x24, not this block (ftwaitanim.c:47-100).
//! Wait's shield-position invalidation reads no common attributes
//! (ftcoll.c:3082-3085). ECB creation uses ftData.x44; the ledge-check path
//! additionally reads common +1CC (ft_081B.c:142-147).

use super::read::{block, public, required, Result};
use hsd_archive::reader::add_offset;
use hsd_archive::Archive;

/// Neutral input thresholds used by Wait's movement predicates and pad
/// decoding. Attack/special/shield state physics are outside this subset.
#[derive(Debug, Clone, PartialEq)]
pub struct IdleInputAttributes {
    /// ft/types.h:54, +0.
    pub horizontal_stick_deadzone: f32,
    /// ft/types.h:55, +4.
    pub vertical_stick_deadzone: f32,
    /// ft/types.h:56, +8.
    pub horizontal_stick_smash_deadzone: f32,
    /// ft/types.h:57, +C.
    pub vertical_stick_smash_deadzone: f32,
    /// ft/types.h:58, +10.
    pub analog_shoulder_deadzone: f32,
    /// ft/types.h:59, +14.
    pub z_press_analog_value: f32,
    /// ft/types.h:60, +18.
    pub shield_press_threshold: f32,
    /// ft/types.h:63, +24; ftwalkcommon.c.
    pub walk_stick_threshold: f32,
    /// ft/types.h:67, +34 (x34); ftCo_Turn.c:32.
    pub turn_stick_threshold: f32,
    /// ft/types.h:69, +3C; ftCo_Dash.c:35.
    pub dash_smash_stick_threshold: f32,
    /// ft/types.h:70, +40; ftCo_Dash.c:37.
    pub dash_smash_window: i32,
    /// ft/types.h:82, +70; ftCo_Jump.c:33.
    pub tap_jump_threshold: f32,
    /// ft/types.h:83, +74; ftCo_Jump.c:34.
    pub tap_jump_window: i32,
    /// ft/types.h:90, +90 (x90); ftCo_Squat.c:38.
    pub squat_stick_threshold: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommonFighterData {
    pub input: IdleInputAttributes,
    /// ft/types.h:81, +6C; Wait Phys -> ft_084E.c:42-53.
    pub friction_when_above_walk_speed: f32,
    /// ft/types.h:143, +164 (x164); ground entry, ftcommon.c:197-201.
    pub ground_knockback_speed_limit: f32,
    /// +200 (x200); Fighter_procUpdate grounded knockback friction.
    pub ground_knockback_friction_multiplier: f32,
    /// +3EC; Fighter_procUpdate attacker shield knockback friction.
    pub shield_ground_friction_multiplier: f32,
    /// +1FC (x1FC); air friction above the air drift maximum, ftcommon.c:283-308.
    pub over_drift_air_friction: f32,
    /// ft/types.h:169, +1CC (x1CC); ft_081B.c:142-147.
    pub ledge_snap_height_multiplier: f32,
    /// ft/types.h:552, +804 (x804); grounded pose clamp, ft_0899.c:225.
    pub ground_pose_max_angle_degrees: f32,
    /// +808 (x808): the offset from HipN that ftAnim_8006DF0C keeps the
    /// model's translation joint at while a fighter with x2221_b2 hangs
    /// from a captor.
    pub pinned_hip_offset: hsd_types::Vec3,
}

pub fn read_common_data(archive: &Archive) -> Result<CommonFighterData> {
    let root = public(archive, "ftLoadCommonData")?;
    CommonFighterData::read(archive, required(archive, root, 0, "p_ftCommonData")?)
}

impl CommonFighterData {
    /// Read the selected scalar fields, including the distant grounded-pose
    /// clamp. Uninterpreted fields are not exposed as an opaque float array.
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = block(archive, offset, 0x1D0)?;
        Ok(Self {
            input: IdleInputAttributes {
                horizontal_stick_deadzone: r.f32(0)?,
                vertical_stick_deadzone: r.f32(4)?,
                horizontal_stick_smash_deadzone: r.f32(8)?,
                vertical_stick_smash_deadzone: r.f32(0xC)?,
                analog_shoulder_deadzone: r.f32(0x10)?,
                z_press_analog_value: r.f32(0x14)?,
                shield_press_threshold: r.f32(0x18)?,
                walk_stick_threshold: r.f32(0x24)?,
                turn_stick_threshold: r.f32(0x34)?,
                dash_smash_stick_threshold: r.f32(0x3C)?,
                dash_smash_window: r.s32(0x40)?,
                tap_jump_threshold: r.f32(0x70)?,
                tap_jump_window: r.s32(0x74)?,
                squat_stick_threshold: r.f32(0x90)?,
            },
            friction_when_above_walk_speed: r.f32(0x6C)?,
            ground_knockback_speed_limit: r.f32(0x164)?,
            ground_knockback_friction_multiplier: archive
                .reader()
                .f32(add_offset(offset, 0x200)?)?,
            shield_ground_friction_multiplier: archive.reader().f32(add_offset(offset, 0x3EC)?)?,
            over_drift_air_friction: archive.reader().f32(add_offset(offset, 0x1FC)?)?,
            ledge_snap_height_multiplier: r.f32(0x1CC)?,
            ground_pose_max_angle_degrees: archive.reader().f32(add_offset(offset, 0x804)?)?,
            pinned_hip_offset: hsd_types::Vec3::new(
                archive.reader().f32(add_offset(offset, 0x808)?)?,
                archive.reader().f32(add_offset(offset, 0x80C)?)?,
                archive.reader().f32(add_offset(offset, 0x810)?)?,
            ),
        })
    }
}

/// PlCo movement parameters, loaded at the archive boundary.
#[derive(Clone, Copy, Debug)]
pub struct MovementParameters {
    /// +28/+2C: speed fractions selecting Middle/Fast animation.
    pub middle_threshold: f32,
    pub fast_threshold: f32,
    /// +30: approach-to-target acceleration gain.
    pub acceleration_taper: f32,
    /// +440: target-speed animation estimate on slippery ground.
    pub slippery_animation_multiplier: f32,
    /// +94: SquatRv release threshold.
    pub squat_release_threshold: f32,
    /// +464/+468/+470: platform-drop stick threshold, input age and delay.
    /// +468 is a float: ftCo_80099F1C compares the u8 tilt age as a float
    /// (retail 0x80099F58 lfs, 0x80099F68 fcmpo).
    pub platform_drop_threshold: f32,
    pub platform_drop_window: f32,
    pub platform_drop_delay: f32,
    /// +46C: initial downward velocity on entering Pass.
    pub platform_drop_velocity: f32,
}
