//! Complete `ftCo_DatAttrs`, `ft/types.h:679-769`. The C sizeof is 0x184:
//! 91 float components, five i32 fields, one u8 mask and three padding bytes.
//! The field docs cite the header offset; Rust grouping is independent of C layout.

use super::read::{block, required, vec3, Result};
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_types::snapshot::{Snapshot, SnapshotSink};

/// Size verified by compiling the unmodified scalar/Vec3 C declarations.
pub const FIGHTER_ATTRIBUTES_SIZE: u32 = 0x184;

#[derive(Debug, Clone, PartialEq)]
pub struct FighterAttributes {
    pub walking: WalkingAttributes,
    pub ground: GroundAttributes,
    pub running: RunningAttributes,
    pub jumping: JumpingAttributes,
    pub air: AirAttributes,
    pub combat: CombatAttributes,
    pub size: SizeAttributes,
    pub shield: ShieldAttributes,
    pub ledge: LedgeAttributes,
    pub items: ItemsAttributes,
    pub specials: SpecialsAttributes,
    pub yoshi_egg: YoshiEggAttributes,
    pub kirby_throw: KirbyThrowAttributes,
    pub landing: LandingAttributes,
    pub wall: WallAttributes,
    pub ice: IceAttributes,
    pub camera: CameraAttributes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalkingAttributes {
    /// +0x000: `walk_accel_mul` (`ft/types.h:687`).
    pub walk_accel_mul: f32,
    /// +0x004: `walk_accel_base` (`ft/types.h:688`).
    pub walk_accel_base: f32,
    /// +0x008: `walk_max_vel` (`ft/types.h:689`).
    pub walk_max_vel: f32,
    /// +0x00C: `slow_walk_max` (`ft/types.h:690`).
    pub slow_walk_max: f32,
    /// +0x010: `mid_walk_point` (`ft/types.h:691`).
    pub mid_walk_point: f32,
    /// +0x014: `fast_walk_min` (`ft/types.h:692`).
    pub fast_walk_min: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroundAttributes {
    /// +0x018: `ground_friction` (`ft/types.h:693`).
    pub ground_friction: f32,
    /// +0x034: `ground_max_horizontal_velocity` (`ft/types.h:700`).
    pub ground_max_horizontal_velocity: f32,
    /// +0x084: `standing_turn_frames` (`ft/types.h:720`).
    pub standing_turn_frames: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunningAttributes {
    /// +0x01C: `dash_initial_velocity` (`ft/types.h:694`).
    pub dash_initial_velocity: f32,
    /// +0x020: `dash_accel_mul` (`ft/types.h:695`).
    pub dash_accel_mul: f32,
    /// +0x024: `dash_accel_base` (`ft/types.h:696`).
    pub dash_accel_base: f32,
    /// +0x028: `dash_max_velocity` (`ft/types.h:697`).
    pub dash_max_velocity: f32,
    /// +0x02C: `run_animation_scaling` (`ft/types.h:698`).
    pub run_animation_scaling: f32,
    /// +0x030: `max_run_brake_frames` (`ft/types.h:699`).
    pub max_run_brake_frames: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JumpingAttributes {
    /// +0x038: `jump_startup_time` (`ft/types.h:701`).
    pub jump_startup_time: f32,
    /// +0x03C: `jump_h_initial_velocity` (`ft/types.h:702`).
    pub jump_h_initial_velocity: f32,
    /// +0x040: `jump_v_initial_velocity` (`ft/types.h:703`).
    pub jump_v_initial_velocity: f32,
    /// +0x044: `ground_to_air_jump_momentum_multiplier` (`ft/types.h:704`).
    pub ground_to_air_jump_momentum_multiplier: f32,
    /// +0x048: `jump_h_max_velocity` (`ft/types.h:705`).
    pub jump_h_max_velocity: f32,
    /// +0x04C: `hop_v_initial_velocity` (`ft/types.h:706`).
    pub hop_v_initial_velocity: f32,
    /// +0x050: `air_jump_v_multiplier` (`ft/types.h:707`).
    pub air_jump_v_multiplier: f32,
    /// +0x054: `air_jump_h_multiplier` (`ft/types.h:708`).
    pub air_jump_h_multiplier: f32,
    /// +0x058: `max_jumps` (`ft/types.h:709`).
    pub max_jumps: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirAttributes {
    /// +0x05C: `gravity` (`ft/types.h:710`).
    pub gravity: f32,
    /// +0x060: `terminal_velocity` (`ft/types.h:711`).
    pub terminal_velocity: f32,
    /// +0x064: `air_drift_stick_mul` (`ft/types.h:712`).
    pub air_drift_stick_mul: f32,
    /// +0x068: `aerial_drift_base` (`ft/types.h:713`).
    pub aerial_drift_base: f32,
    /// +0x06C: `air_drift_max` (`ft/types.h:714`).
    pub air_drift_max: f32,
    /// +0x070: `aerial_friction` (`ft/types.h:715`).
    pub aerial_friction: f32,
    /// +0x074: `fast_fall_velocity` (`ft/types.h:716`).
    pub fast_fall_velocity: f32,
    /// +0x078: `air_max_horizontal_velocity` (`ft/types.h:717`).
    pub air_max_horizontal_velocity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombatAttributes {
    /// +0x07C: `jab_2_input_window` (`ft/types.h:718`).
    pub jab_2_input_window: f32,
    /// +0x080: `jab_3_input_window` (`ft/types.h:719`).
    pub jab_3_input_window: f32,
    /// +0x098: `rapid_jab_window` (`ft/types.h:725`).
    pub rapid_jab_window: i32,
    /// +0x09C: `clank_animation_length` (`ft/types.h:726`).
    pub clank_animation_length: f32,
    /// +0x0A0: `hit_spark_variant` (`ft/types.h:727`).
    pub hit_spark_variant: i32,
    /// +0x0A4: `unused_0` (`ft/types.h:728`).
    pub unused_0: i32,
    /// +0x0E0: `kirby_b_star_damage` (`ft/types.h:736`).
    pub kirby_b_star_damage: f32,
    /// +0x180: `weight_independent_throws_mask` (`ft/types.h:768`).
    pub weight_independent_throws_mask: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SizeAttributes {
    /// +0x088: `weight` (`ft/types.h:721`).
    pub weight: f32,
    /// +0x08C: `model_scaling` (`ft/types.h:722`).
    pub model_scaling: f32,
    /// +0x0FC: `name_tag_height` (`ft/types.h:743`).
    pub name_tag_height: f32,
    /// +0x110: `trophy_scale` (`ft/types.h:748`).
    pub trophy_scale: f32,
    /// +0x160: `respawn_platform_scale` (`ft/types.h:762`).
    pub respawn_platform_scale: f32,
    /// +0x164: `warp_star_hitbox_scale` (`ft/types.h:763`).
    pub warp_star_hitbox_scale: f32,
    /// +0x168: `x168` (`ft/types.h:764`).
    // TODO(meaning): retain the header field without inventing semantics.
    pub unknown_168: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShieldAttributes {
    /// +0x090: `initial_shield_size` (`ft/types.h:723`).
    pub initial_shield_size: f32,
    /// +0x094: `shield_break_initial_velocity` (`ft/types.h:724`).
    pub shield_break_initial_velocity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LedgeAttributes {
    /// +0x0A8: `ledge_jump_horizontal_velocity` (`ft/types.h:729`).
    pub ledge_jump_horizontal_velocity: f32,
    /// +0x0AC: `ledge_jump_vertical_velocity` (`ft/types.h:730`).
    pub ledge_jump_vertical_velocity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemsAttributes {
    /// +0x0B0: `item_throw_velocity_multiplier` (`ft/types.h:731`).
    pub item_throw_velocity_multiplier: f32,
    /// +0x0B4: `heavy_throw_velocity_multiplier` (`ft/types.h:732`).
    pub heavy_throw_velocity_multiplier: f32,
    /// +0x114: `x114` (`ft/types.h:749`); bunny hood joint 4 offset,
    /// ftcommon.c:1514 and it/kinds/itrabbitc.c:81-86.
    pub bunny_hood_offset_1: Vec3,
    /// +0x120: `x120` (`ft/types.h:750`); bunny hood joint 8 offset,
    /// ftcommon.c:1514 and it/kinds/itrabbitc.c:74-79.
    pub bunny_hood_offset_2: Vec3,
    /// +0x12C: `x12C` (`ft/types.h:751`); ftcommon.c:1515.
    pub bunny_hood_scale: f32,
    /// +0x130: `x130` (`ft/types.h:752`); ftcommon.c:1600-1609.
    pub flower_flame_offset: Vec3,
    /// +0x13C: `x13C` (`ft/types.h:753`); ftcommon.c:1610.
    pub flower_flame_scale: f32,
    /// +0x140: `screw_attack_launch_velocity` (`ft/types.h:754`).
    pub screw_attack_launch_velocity: f32,
    /// +0x144: `x144` (`ft/types.h:755`).
    // TODO(meaning): retain the header field without inventing semantics.
    pub unknown_144: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialsAttributes {
    /// +0x0B8: `specials_ground_speed_retention` (`ft/types.h:733`).
    pub specials_ground_speed_retention: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct YoshiEggAttributes {
    /// +0x0BC: `ftCo_DatAttrs.xBC` (`ft/types.h:680`).
    pub size: f32,
    /// +0x0C0: `ftCo_DatAttrs.xBC` (`ft/types.h:681`).
    pub hurtbox_start: Vec3,
    /// +0x0CC: `ftCo_DatAttrs.xBC` (`ft/types.h:682`).
    pub hurtbox_end: Vec3,
    /// +0x0D8: `ftCo_DatAttrs.xBC` (`ft/types.h:683`).
    pub hurtbox_scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KirbyThrowAttributes {
    /// +0x0DC: `xDC` (`ft/types.h:735`); ftCo_ThrownKirby.c:89-92,138-139.
    pub star_scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LandingAttributes {
    /// +0x0E4: `normal_landing_lag` (`ft/types.h:737`).
    pub normal_landing_lag: f32,
    /// +0x0E8: `landingairn_lag` (`ft/types.h:738`).
    pub landingairn_lag: f32,
    /// +0x0EC: `landingairf_lag` (`ft/types.h:739`).
    pub landingairf_lag: f32,
    /// +0x0F0: `landingairb_lag` (`ft/types.h:740`).
    pub landingairb_lag: f32,
    /// +0x0F4: `landingairhi_lag` (`ft/types.h:741`).
    pub landingairhi_lag: f32,
    /// +0x0F8: `landingairlw_lag` (`ft/types.h:742`).
    pub landingairlw_lag: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WallAttributes {
    /// +0x100: `passivewall_vel_x` (`ft/types.h:744`).
    pub passivewall_vel_x: f32,
    /// +0x104: `wall_jump_horizontal_velocity` (`ft/types.h:745`).
    pub wall_jump_horizontal_velocity: f32,
    /// +0x108: `wall_jump_vertical_velocity` (`ft/types.h:746`).
    pub wall_jump_vertical_velocity: f32,
    /// +0x10C: `passiveceil_vel_x` (`ft/types.h:747`).
    pub passiveceil_vel_x: f32,
    /// +0x148: `wall_jump_min_approach_speed` (`ft/types.h:756`).
    pub wall_jump_min_approach_speed: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IceAttributes {
    /// +0x14C: `damageice_ice_size` (`ft/types.h:757`).
    pub damageice_ice_size: f32,
    /// +0x150: `x150_damageice_unk` (`ft/types.h:758`).
    // TODO(meaning): retain the header field without inventing semantics.
    pub unknown_150: f32,
    /// +0x154: `x154_damageice_unk` (`ft/types.h:759`).
    // TODO(meaning): retain the header field without inventing semantics.
    pub unknown_154: f32,
    /// +0x158: `damageicejump_vel_y` (`ft/types.h:760`).
    pub damageicejump_vel_y: f32,
    /// +0x15C: `damageicejump_vel_x_mult` (`ft/types.h:761`).
    pub damageicejump_vel_x_mult: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraAttributes {
    /// +0x16C: `camera_zoom_target_bone` (`ft/types.h:765`).
    pub camera_zoom_target_bone: i32,
    /// +0x170: `x170` (`ft/types.h:766`).
    pub zoom_offset: Vec3,
    /// +0x17C: `x17C` (`ft/types.h:767`).
    pub damage_camera_y_offset: f32,
}

/// Read the common per-character attributes at `ftData.x0` (+0, ft/types.h:613).
pub fn read_fighter_attributes(archive: &Archive, ft_data: u32) -> Result<FighterAttributes> {
    FighterAttributes::read(archive, required(archive, ft_data, 0, "ftData.x0")?)
}

impl FighterAttributes {
    /// Read an attribute block directly, retaining every scalar bit pattern.
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = block(archive, offset, FIGHTER_ATTRIBUTES_SIZE)?;
        Ok(Self {
            walking: WalkingAttributes::read(r)?,
            ground: GroundAttributes::read(r)?,
            running: RunningAttributes::read(r)?,
            jumping: JumpingAttributes::read(r)?,
            air: AirAttributes::read(r)?,
            combat: CombatAttributes::read(r)?,
            size: SizeAttributes::read(r)?,
            shield: ShieldAttributes::read(r)?,
            ledge: LedgeAttributes::read(r)?,
            items: ItemsAttributes::read(r)?,
            specials: SpecialsAttributes::read(r)?,
            yoshi_egg: YoshiEggAttributes::read(r)?,
            kirby_throw: KirbyThrowAttributes::read(r)?,
            landing: LandingAttributes::read(r)?,
            wall: WallAttributes::read(r)?,
            ice: IceAttributes::read(r)?,
            camera: CameraAttributes::read(r)?,
        })
    }
}

impl WalkingAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            walk_accel_mul: r.f32(0x000)?,
            walk_accel_base: r.f32(0x004)?,
            walk_max_vel: r.f32(0x008)?,
            slow_walk_max: r.f32(0x00C)?,
            mid_walk_point: r.f32(0x010)?,
            fast_walk_min: r.f32(0x014)?,
        })
    }
}

impl GroundAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            ground_friction: r.f32(0x018)?,
            ground_max_horizontal_velocity: r.f32(0x034)?,
            standing_turn_frames: r.f32(0x084)?,
        })
    }
}

impl RunningAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            dash_initial_velocity: r.f32(0x01C)?,
            dash_accel_mul: r.f32(0x020)?,
            dash_accel_base: r.f32(0x024)?,
            dash_max_velocity: r.f32(0x028)?,
            run_animation_scaling: r.f32(0x02C)?,
            max_run_brake_frames: r.f32(0x030)?,
        })
    }
}

impl JumpingAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            jump_startup_time: r.f32(0x038)?,
            jump_h_initial_velocity: r.f32(0x03C)?,
            jump_v_initial_velocity: r.f32(0x040)?,
            ground_to_air_jump_momentum_multiplier: r.f32(0x044)?,
            jump_h_max_velocity: r.f32(0x048)?,
            hop_v_initial_velocity: r.f32(0x04C)?,
            air_jump_v_multiplier: r.f32(0x050)?,
            air_jump_h_multiplier: r.f32(0x054)?,
            max_jumps: r.s32(0x058)?,
        })
    }
}

impl AirAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            gravity: r.f32(0x05C)?,
            terminal_velocity: r.f32(0x060)?,
            air_drift_stick_mul: r.f32(0x064)?,
            aerial_drift_base: r.f32(0x068)?,
            air_drift_max: r.f32(0x06C)?,
            aerial_friction: r.f32(0x070)?,
            fast_fall_velocity: r.f32(0x074)?,
            air_max_horizontal_velocity: r.f32(0x078)?,
        })
    }
}

impl CombatAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            jab_2_input_window: r.f32(0x07C)?,
            jab_3_input_window: r.f32(0x080)?,
            rapid_jab_window: r.s32(0x098)?,
            clank_animation_length: r.f32(0x09C)?,
            hit_spark_variant: r.s32(0x0A0)?,
            unused_0: r.s32(0x0A4)?,
            kirby_b_star_damage: r.f32(0x0E0)?,
            weight_independent_throws_mask: r.u8(0x180)?,
        })
    }
}

impl SizeAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            weight: r.f32(0x088)?,
            model_scaling: r.f32(0x08C)?,
            name_tag_height: r.f32(0x0FC)?,
            trophy_scale: r.f32(0x110)?,
            respawn_platform_scale: r.f32(0x160)?,
            warp_star_hitbox_scale: r.f32(0x164)?,
            unknown_168: r.f32(0x168)?,
        })
    }
}

impl ShieldAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            initial_shield_size: r.f32(0x090)?,
            shield_break_initial_velocity: r.f32(0x094)?,
        })
    }
}

impl LedgeAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            ledge_jump_horizontal_velocity: r.f32(0x0A8)?,
            ledge_jump_vertical_velocity: r.f32(0x0AC)?,
        })
    }
}

impl ItemsAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            item_throw_velocity_multiplier: r.f32(0x0B0)?,
            heavy_throw_velocity_multiplier: r.f32(0x0B4)?,
            bunny_hood_offset_1: vec3(r, 0x114)?,
            bunny_hood_offset_2: vec3(r, 0x120)?,
            bunny_hood_scale: r.f32(0x12C)?,
            flower_flame_offset: vec3(r, 0x130)?,
            flower_flame_scale: r.f32(0x13C)?,
            screw_attack_launch_velocity: r.f32(0x140)?,
            unknown_144: r.f32(0x144)?,
        })
    }
}

impl SpecialsAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            specials_ground_speed_retention: r.f32(0x0B8)?,
        })
    }
}

impl YoshiEggAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            size: r.f32(0x0BC)?,
            hurtbox_start: vec3(r, 0x0C0)?,
            hurtbox_end: vec3(r, 0x0CC)?,
            hurtbox_scale: r.f32(0x0D8)?,
        })
    }
}

impl KirbyThrowAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            star_scale: r.f32(0x0DC)?,
        })
    }
}

impl LandingAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            normal_landing_lag: r.f32(0x0E4)?,
            landingairn_lag: r.f32(0x0E8)?,
            landingairf_lag: r.f32(0x0EC)?,
            landingairb_lag: r.f32(0x0F0)?,
            landingairhi_lag: r.f32(0x0F4)?,
            landingairlw_lag: r.f32(0x0F8)?,
        })
    }
}

impl WallAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            passivewall_vel_x: r.f32(0x100)?,
            wall_jump_horizontal_velocity: r.f32(0x104)?,
            wall_jump_vertical_velocity: r.f32(0x108)?,
            passiveceil_vel_x: r.f32(0x10C)?,
            wall_jump_min_approach_speed: r.f32(0x148)?,
        })
    }
}

impl IceAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            damageice_ice_size: r.f32(0x14C)?,
            unknown_150: r.f32(0x150)?,
            unknown_154: r.f32(0x154)?,
            damageicejump_vel_y: r.f32(0x158)?,
            damageicejump_vel_x_mult: r.f32(0x15C)?,
        })
    }
}

impl CameraAttributes {
    fn read(r: Reader<'_>) -> Result<Self> {
        Ok(Self {
            camera_zoom_target_bone: r.s32(0x16C)?,
            zoom_offset: vec3(r, 0x170)?,
            damage_camera_y_offset: r.f32(0x17C)?,
        })
    }
}

impl Snapshot for FighterAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("walking", &self.walking);
        sink.field("ground", &self.ground);
        sink.field("running", &self.running);
        sink.field("jumping", &self.jumping);
        sink.field("air", &self.air);
        sink.field("combat", &self.combat);
        sink.field("size", &self.size);
        sink.field("shield", &self.shield);
        sink.field("ledge", &self.ledge);
        sink.field("items", &self.items);
        sink.field("specials", &self.specials);
        sink.field("yoshi_egg", &self.yoshi_egg);
        sink.field("kirby_throw", &self.kirby_throw);
        sink.field("landing", &self.landing);
        sink.field("wall", &self.wall);
        sink.field("ice", &self.ice);
        sink.field("camera", &self.camera);
    }
}

impl Snapshot for WalkingAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("walk_accel_mul", &self.walk_accel_mul);
        sink.field("walk_accel_base", &self.walk_accel_base);
        sink.field("walk_max_vel", &self.walk_max_vel);
        sink.field("slow_walk_max", &self.slow_walk_max);
        sink.field("mid_walk_point", &self.mid_walk_point);
        sink.field("fast_walk_min", &self.fast_walk_min);
    }
}

impl Snapshot for GroundAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("ground_friction", &self.ground_friction);
        sink.field(
            "ground_max_horizontal_velocity",
            &self.ground_max_horizontal_velocity,
        );
        sink.field("standing_turn_frames", &self.standing_turn_frames);
    }
}

impl Snapshot for RunningAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("dash_initial_velocity", &self.dash_initial_velocity);
        sink.field("dash_accel_mul", &self.dash_accel_mul);
        sink.field("dash_accel_base", &self.dash_accel_base);
        sink.field("dash_max_velocity", &self.dash_max_velocity);
        sink.field("run_animation_scaling", &self.run_animation_scaling);
        sink.field("max_run_brake_frames", &self.max_run_brake_frames);
    }
}

impl Snapshot for JumpingAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("jump_startup_time", &self.jump_startup_time);
        sink.field("jump_h_initial_velocity", &self.jump_h_initial_velocity);
        sink.field("jump_v_initial_velocity", &self.jump_v_initial_velocity);
        sink.field(
            "ground_to_air_jump_momentum_multiplier",
            &self.ground_to_air_jump_momentum_multiplier,
        );
        sink.field("jump_h_max_velocity", &self.jump_h_max_velocity);
        sink.field("hop_v_initial_velocity", &self.hop_v_initial_velocity);
        sink.field("air_jump_v_multiplier", &self.air_jump_v_multiplier);
        sink.field("air_jump_h_multiplier", &self.air_jump_h_multiplier);
        sink.field("max_jumps", &self.max_jumps);
    }
}

impl Snapshot for AirAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("gravity", &self.gravity);
        sink.field("terminal_velocity", &self.terminal_velocity);
        sink.field("air_drift_stick_mul", &self.air_drift_stick_mul);
        sink.field("aerial_drift_base", &self.aerial_drift_base);
        sink.field("air_drift_max", &self.air_drift_max);
        sink.field("aerial_friction", &self.aerial_friction);
        sink.field("fast_fall_velocity", &self.fast_fall_velocity);
        sink.field(
            "air_max_horizontal_velocity",
            &self.air_max_horizontal_velocity,
        );
    }
}

impl Snapshot for CombatAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("jab_2_input_window", &self.jab_2_input_window);
        sink.field("jab_3_input_window", &self.jab_3_input_window);
        sink.field("rapid_jab_window", &self.rapid_jab_window);
        sink.field("clank_animation_length", &self.clank_animation_length);
        sink.field("hit_spark_variant", &self.hit_spark_variant);
        sink.field("unused_0", &self.unused_0);
        sink.field("kirby_b_star_damage", &self.kirby_b_star_damage);
        sink.field(
            "weight_independent_throws_mask",
            &self.weight_independent_throws_mask,
        );
    }
}

impl Snapshot for SizeAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("weight", &self.weight);
        sink.field("model_scaling", &self.model_scaling);
        sink.field("name_tag_height", &self.name_tag_height);
        sink.field("trophy_scale", &self.trophy_scale);
        sink.field("respawn_platform_scale", &self.respawn_platform_scale);
        sink.field("warp_star_hitbox_scale", &self.warp_star_hitbox_scale);
        sink.field("unknown_168", &self.unknown_168);
    }
}

impl Snapshot for ShieldAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("initial_shield_size", &self.initial_shield_size);
        sink.field(
            "shield_break_initial_velocity",
            &self.shield_break_initial_velocity,
        );
    }
}

impl Snapshot for LedgeAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field(
            "ledge_jump_horizontal_velocity",
            &self.ledge_jump_horizontal_velocity,
        );
        sink.field(
            "ledge_jump_vertical_velocity",
            &self.ledge_jump_vertical_velocity,
        );
    }
}

impl Snapshot for ItemsAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field(
            "item_throw_velocity_multiplier",
            &self.item_throw_velocity_multiplier,
        );
        sink.field(
            "heavy_throw_velocity_multiplier",
            &self.heavy_throw_velocity_multiplier,
        );
        sink.field("bunny_hood_offset_1", &self.bunny_hood_offset_1);
        sink.field("bunny_hood_offset_2", &self.bunny_hood_offset_2);
        sink.field("bunny_hood_scale", &self.bunny_hood_scale);
        sink.field("flower_flame_offset", &self.flower_flame_offset);
        sink.field("flower_flame_scale", &self.flower_flame_scale);
        sink.field(
            "screw_attack_launch_velocity",
            &self.screw_attack_launch_velocity,
        );
        sink.field("unknown_144", &self.unknown_144);
    }
}

impl Snapshot for SpecialsAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field(
            "specials_ground_speed_retention",
            &self.specials_ground_speed_retention,
        );
    }
}

impl Snapshot for YoshiEggAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("size", &self.size);
        sink.field("hurtbox_start", &self.hurtbox_start);
        sink.field("hurtbox_end", &self.hurtbox_end);
        sink.field("hurtbox_scale", &self.hurtbox_scale);
    }
}

impl Snapshot for KirbyThrowAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("star_scale", &self.star_scale);
    }
}

impl Snapshot for LandingAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("normal_landing_lag", &self.normal_landing_lag);
        sink.field("landingairn_lag", &self.landingairn_lag);
        sink.field("landingairf_lag", &self.landingairf_lag);
        sink.field("landingairb_lag", &self.landingairb_lag);
        sink.field("landingairhi_lag", &self.landingairhi_lag);
        sink.field("landingairlw_lag", &self.landingairlw_lag);
    }
}

impl Snapshot for WallAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("passivewall_vel_x", &self.passivewall_vel_x);
        sink.field(
            "wall_jump_horizontal_velocity",
            &self.wall_jump_horizontal_velocity,
        );
        sink.field(
            "wall_jump_vertical_velocity",
            &self.wall_jump_vertical_velocity,
        );
        sink.field("passiveceil_vel_x", &self.passiveceil_vel_x);
        sink.field(
            "wall_jump_min_approach_speed",
            &self.wall_jump_min_approach_speed,
        );
    }
}

impl Snapshot for IceAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("damageice_ice_size", &self.damageice_ice_size);
        sink.field("unknown_150", &self.unknown_150);
        sink.field("unknown_154", &self.unknown_154);
        sink.field("damageicejump_vel_y", &self.damageicejump_vel_y);
        sink.field("damageicejump_vel_x_mult", &self.damageicejump_vel_x_mult);
    }
}

impl Snapshot for CameraAttributes {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("camera_zoom_target_bone", &self.camera_zoom_target_bone);
        sink.field("zoom_offset", &self.zoom_offset);
        sink.field("damage_camera_y_offset", &self.damage_camera_y_offset);
    }
}
