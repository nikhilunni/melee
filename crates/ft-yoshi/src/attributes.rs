//! ftYoshi/types.h: ftData.ext_attr (+4), 0x138 bytes.
//! The overlapping ftYs_DatAttrs names the Egg Throw fields hidden by padding
//! in ftYoshiAttributes; both describe the same archive block.
use hsd_archive::{Archive, Reader};
use hsd_types::Vec2;
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;
pub const YOSHI_ATTRIBUTES_SIZE: u32 = 0x138;
#[derive(Clone, Debug, PartialEq)]
pub struct YoshiAttributes {
    pub double_jump: DoubleJumpAttributes,
    pub egg_lay: EggLayAttributes,
    pub egg_roll: EggRollAttributes,
    pub egg_throw: EggThrowAttributes,
    pub ground_pound: GroundPoundAttributes,
    /// +0x0C: filled from egg material AObj end frames by OnLoad.
    pub shield_material_frames: f32,
    /// +0x120: ftCo_CaptureWait uses this hurtbox scale.
    pub captured_hurtbox_scale: f32,
    /// +0x124/+0x128: Catch frames [start, end) in which a catch skips
    /// ahead in CatchPull (fn_800D9CE8).
    pub catch_pull_window: [f32; 2],
    /// +0x12C..137: CatchPull start frame per whole frame into that window.
    pub catch_pull_start_frames: [u8; 12],
}
#[derive(Clone, Debug, PartialEq)]
pub struct DoubleJumpAttributes {
    /// +0x00, ftYoshi/types.h; ftCo_JumpAerial.c.
    pub turn_frames: i32,
    /// +0x04, ftYoshi/types.h; ftCo_JumpAerial.c.
    pub reverse_threshold: f32,
    /// +0x08, ftYoshi/types.h; ftCo_JumpAerial.c.
    pub armor: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EggLayAttributes {
    /// +0x10, ftYoshi/types.h; ftyoshispecialn.c.
    pub horizontal_speed: f32,
    /// +0x14, ftYoshi/types.h; ftyoshispecialn.c.
    pub vertical_speed: f32,
    /// +0x18, ftYoshi/types.h; ftyoshispecialn.c.
    pub damage_behavior: f32,
    /// +0x1C, ftYoshi/types.h; ftyoshispecialn.c.
    pub wobble_parameter: f32,
    /// +0x20, ftYoshi/types.h; ftyoshispecialn.c.
    pub capture_parameter: f32,
    /// +0x24, ftYoshi/types.h; ftyoshispecialn.c.
    pub duration: f32,
    /// +0x28, ftYoshi/types.h; ftyoshispecialn.c.
    pub escape_frames_per_tick: f32,
    /// +0x2C, ftYoshi/types.h; ftyoshispecialn.c.
    pub mash_frame_reduction: f32,
    /// +0x30, ftYoshi/types.h; ftyoshispecialn.c.
    pub mash_animation_duration: f32,
    /// +0x34, ftYoshi/types.h; ftyoshispecialn.c.
    pub mash_animation_rate: f32,
    /// +0x38, ftYoshi/types.h; ftyoshispecialn.c.
    pub exit_intangibility_frames: i32,
    /// +0x3C, ftYoshi/types.h; ftyoshispecialn.c.
    pub release_velocity: Vec2,
    /// +0x44, ftYoshi/types.h; ftyoshispecialn.c.
    pub damage_frame_reduction: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EggRollAttributes {
    /// +0x48, ftYoshi/types.h; ftyoshispecials.c.
    pub duration: i32,
    /// +0x4C, ftYoshi/types.h; ftyoshispecials.c.
    pub minimum_duration: i32,
    /// +0x50, ftYoshi/types.h; ftyoshispecials.c.
    pub collision_frame_cost: i32,
    /// +0x54, ftYoshi/types.h; ftyoshispecials.c.
    pub ground_start_speed: f32,
    /// +0x58, ftYoshi/types.h; ftyoshispecials.c.
    pub air_start_speed: f32,
    /// +0x5C, ftYoshi/types.h; ftyoshispecials.c.
    pub hop_speed: f32,
    /// +0x60, ftYoshi/types.h; ftyoshispecials.c.
    pub start_rotation_speed: f32,
    /// +0x64, ftYoshi/types.h; ftyoshispecials.c.
    pub landing_speed: f32,
    /// +0x68, ftYoshi/types.h; ftyoshispecials.c.
    pub smash_speed_multiplier: f32,
    /// +0x6C (specials_start_gravity): the hop and aerial roll's gravity.
    pub start_gravity: f32,
    /// +0x70 (specials_start_terminal_vel).
    pub start_terminal_velocity: f32,
    /// +0x74, ftYoshi/types.h; ftyoshispecials.c.
    pub ground_acceleration: f32,
    /// +0x78: slowing toward the target speed on the ground.
    pub ground_deceleration: f32,
    /// +0x7C, ftYoshi/types.h; ftyoshispecials.c.
    pub ground_target_speed: f32,
    /// +0x80, ftYoshi/types.h; ftyoshispecials.c.
    pub ground_maximum_speed: f32,
    /// +0x84, ftYoshi/types.h; ftyoshispecials.c.
    pub air_acceleration: f32,
    /// +0x88, ftYoshi/types.h; ftyoshispecials.c.
    pub air_stick_acceleration: f32,
    /// +0x8C, ftYoshi/types.h; ftyoshispecials.c.
    pub air_target_speed: f32,
    /// +0x90, ftYoshi/types.h; ftyoshispecials.c.
    pub air_maximum_speed: f32,
    /// +0x94, ftYoshi/types.h; ftyoshispecials.c.
    pub slope_multiplier: f32,
    /// +0x98, ftYoshi/types.h; ftyoshispecials.c.
    pub smash_window: f32,
    /// +0x9C, ftYoshi/types.h; ftyoshispecials.c.
    pub steer_threshold: f32,
    /// +0xA0, ftYoshi/types.h; ftyoshispecials.c.
    pub rotation_speed: f32,
    /// +0xA4, ftYoshi/types.h; ftyoshispecials.c.
    pub effect_interval: i32,
    /// +0xA8: above this ground speed the turn ignores floor edges.
    pub edge_ignore_speed: f32,
    /// +0xAC: horizontal speed kept (reversed) off a wall.
    pub wall_bounce_multiplier: f32,
    /// +0xB0, ftYoshi/types.h; ftyoshispecials.c.
    pub wall_bounce_vertical_speed: f32,
    /// +0xB4: vertical speed kept off the floor.
    pub floor_bounce_multiplier: f32,
    /// +0xB8, ftYoshi/types.h; ftyoshispecials.c.
    pub minimum_bounce_speed: f32,
    /// +0xBC, ftYoshi/types.h; ftyoshispecials.c.
    pub landing_stick_multiplier: f32,
    /// +0xC0: hitbox damage at zero speed, before the multiplier.
    pub damage_base: f32,
    /// +0xC4: hitbox damage per unit of (base + speed).
    pub damage_multiplier: f32,
    /// +0xC8, ftYoshi/types.h; ftyoshispecials.c.
    pub unknown_animation_parameter: f32,
    /// +0xCC: speed lost when the roll's hit lands (fn_8012EFF4).
    pub hit_deceleration: f32,
    /// +0xD0, ftYoshi/types.h; ftyoshispecials.c.
    pub end_horizontal_multiplier: f32,
    /// +0xD4, ftYoshi/types.h; ftyoshispecials.c.
    pub end_vertical_multiplier: f32,
    /// +0xD8, ftYoshi/types.h; ftyoshispecials.c.
    pub rolling_rotation_multiplier: f32,
    /// +0xDC: frames between hit group toggles (re-hit window).
    pub group_toggle_frames: i32,
    /// +0xE0, ftYoshi/types.h; ftyoshispecials.c.
    pub air_steer_multiplier: f32,
    /// +0xE4, ftYoshi/types.h; ftyoshispecials.c.
    pub maximum_tilt: f32,
    /// +0xE8, ftYoshi/types.h; ftyoshispecials.c.
    pub landing_lag: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EggThrowAttributes {
    /// +0xEC, ftYoshi/types.h; ftyoshispecialhi.c.
    pub angle_stick_divisor: f32,
    /// +0xF0, ftYoshi/types.h; ftyoshispecialhi.c.
    pub angle_range: f32,
    /// +0xF4, ftYoshi/types.h; ftyoshispecialhi.c.
    pub minimum_angle_adjustment: f32,
    /// +0xF8, ftYoshi/types.h; ftyoshispecialhi.c.
    pub base_angle: f32,
    /// +0xFC, ftYoshi/types.h; ftyoshispecialhi.c.
    pub base_speed: f32,
    /// +0x100, ftYoshi/types.h; ftyoshispecialhi.c.
    pub speed_per_charge_frame: f32,
    /// +0x104, ftYoshi/types.h; ftyoshispecialhi.c.
    pub spawn_offset_x: f32,
    /// +0x108, ftYoshi/types.h; ftyoshispecialhi.c.
    pub spawn_offset_y: f32,
    /// +0x10C, ftYoshi/types.h; ftyoshispecialhi.c.
    pub base_spin: f32,
    /// +0x110, ftYoshi/types.h; ftyoshispecialhi.c.
    pub spin_per_charge_frame: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GroundPoundAttributes {
    /// +0x114, ftYoshi/types.h; ftyoshispeciallw.c.
    pub fall_speed: f32,
    /// +0x118, ftYoshi/types.h; ftyoshispeciallw.c.
    pub star_offset: Vec2,
}
pub fn read_yoshi_attributes(archive: &Archive) -> Result<YoshiAttributes> {
    let root = archive.public("ftDataYoshi").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataYoshi".into(),
        })
    })?;
    YoshiAttributes::read(archive, special_attributes_offset(archive, root)?)
}
impl YoshiAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, YOSHI_ATTRIBUTES_SIZE)?);
        Ok(Self {
            double_jump: DoubleJumpAttributes {
                turn_frames: r.s32(0x0)?,
                reverse_threshold: r.f32(0x4)?,
                armor: r.f32(0x8)?,
            },
            egg_lay: EggLayAttributes {
                horizontal_speed: r.f32(0x10)?,
                vertical_speed: r.f32(0x14)?,
                damage_behavior: r.f32(0x18)?,
                wobble_parameter: r.f32(0x1C)?,
                capture_parameter: r.f32(0x20)?,
                duration: r.f32(0x24)?,
                escape_frames_per_tick: r.f32(0x28)?,
                mash_frame_reduction: r.f32(0x2C)?,
                mash_animation_duration: r.f32(0x30)?,
                mash_animation_rate: r.f32(0x34)?,
                exit_intangibility_frames: r.s32(0x38)?,
                release_velocity: Vec2::new(r.f32(0x3C)?, r.f32(0x40)?),
                damage_frame_reduction: r.f32(0x44)?,
            },
            egg_roll: EggRollAttributes {
                duration: r.s32(0x48)?,
                minimum_duration: r.s32(0x4C)?,
                collision_frame_cost: r.s32(0x50)?,
                ground_start_speed: r.f32(0x54)?,
                air_start_speed: r.f32(0x58)?,
                hop_speed: r.f32(0x5C)?,
                start_rotation_speed: r.f32(0x60)?,
                landing_speed: r.f32(0x64)?,
                smash_speed_multiplier: r.f32(0x68)?,
                start_gravity: r.f32(0x6C)?,
                start_terminal_velocity: r.f32(0x70)?,
                ground_acceleration: r.f32(0x74)?,
                ground_deceleration: r.f32(0x78)?,
                ground_target_speed: r.f32(0x7C)?,
                ground_maximum_speed: r.f32(0x80)?,
                air_acceleration: r.f32(0x84)?,
                air_stick_acceleration: r.f32(0x88)?,
                air_target_speed: r.f32(0x8C)?,
                air_maximum_speed: r.f32(0x90)?,
                slope_multiplier: r.f32(0x94)?,
                smash_window: r.f32(0x98)?,
                steer_threshold: r.f32(0x9C)?,
                rotation_speed: r.f32(0xA0)?,
                effect_interval: r.s32(0xA4)?,
                edge_ignore_speed: r.f32(0xA8)?,
                wall_bounce_multiplier: r.f32(0xAC)?,
                wall_bounce_vertical_speed: r.f32(0xB0)?,
                floor_bounce_multiplier: r.f32(0xB4)?,
                minimum_bounce_speed: r.f32(0xB8)?,
                landing_stick_multiplier: r.f32(0xBC)?,
                damage_base: r.f32(0xC0)?,
                damage_multiplier: r.f32(0xC4)?,
                unknown_animation_parameter: r.f32(0xC8)?,
                hit_deceleration: r.f32(0xCC)?,
                end_horizontal_multiplier: r.f32(0xD0)?,
                end_vertical_multiplier: r.f32(0xD4)?,
                rolling_rotation_multiplier: r.f32(0xD8)?,
                group_toggle_frames: r.s32(0xDC)?,
                air_steer_multiplier: r.f32(0xE0)?,
                maximum_tilt: r.f32(0xE4)?,
                landing_lag: r.f32(0xE8)?,
            },
            egg_throw: EggThrowAttributes {
                angle_stick_divisor: r.f32(0xEC)?,
                angle_range: r.f32(0xF0)?,
                minimum_angle_adjustment: r.f32(0xF4)?,
                base_angle: r.f32(0xF8)?,
                base_speed: r.f32(0xFC)?,
                speed_per_charge_frame: r.f32(0x100)?,
                spawn_offset_x: r.f32(0x104)?,
                spawn_offset_y: r.f32(0x108)?,
                base_spin: r.f32(0x10C)?,
                spin_per_charge_frame: r.f32(0x110)?,
            },
            ground_pound: GroundPoundAttributes {
                fall_speed: r.f32(0x114)?,
                star_offset: Vec2::new(r.f32(0x118)?, r.f32(0x11C)?),
            },
            shield_material_frames: r.f32(0xC)?,
            captured_hurtbox_scale: r.f32(0x120)?,
            catch_pull_window: [r.f32(0x124)?, r.f32(0x128)?],
            catch_pull_start_frames: r.slice(0x12C, 12)?.try_into().unwrap(),
        })
    }
}
