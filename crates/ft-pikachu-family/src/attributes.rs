//! ftPikachuAttributes, ft/kinds/ftPikachu/types.h; ftData.ext_attr (+4).
//! ftPk_Init_LoadSpecialAttrs (80124700) copies 0xF8 bytes, then scales
//! the six collision-box floats at +E0 by the player's y scale
//! (Fighter.x34_scale.y) when it is not 1, which a plain versus match never
//! sets. Pichu shares the layout (ftPk_Init_OnLoadForPichu).
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_types::mp::FtCollisionBox;
type Result<T> = std::result::Result<T, FighterDescError>;

pub const PIKACHU_ATTRIBUTES_SIZE: u32 = 0xF8;

#[derive(Clone, Debug, PartialEq)]
pub struct PikachuAttributes {
    pub thunder_jolt: ThunderJoltAttributes,
    pub skull_bash: SkullBashAttributes,
    pub quick_attack: QuickAttackAttributes,
    pub thunder: ThunderAttributes,
    /// +0xE0: the environment box Quick Attack collides with (ft_80082888,
    /// ft_8008239C).
    pub quick_attack_box: FtCollisionBox,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThunderJoltAttributes {
    /// +0x00: the grounded jolt's spawn offset (x by facing), scaled by y scale.
    pub ground_spawn_offset: hsd_types::Vec2,
    /// +0x08: the aerial jolt's spawn offset.
    pub air_spawn_offset: hsd_types::Vec2,
    /// +0x10: landing lag of the aerial jolt's ftCo_80096900 (zero: Fall).
    pub air_landing_lag: f32,
    /// +0x14: specialn_itkind, the item both jolts spawn.
    pub ground_item: u32,
    /// +0x18: specialairn_itkind, registered by OnLoad but never spawned.
    pub air_item: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkullBashAttributes {
    /// +0x1C: a side-B entered within this many frames of the stick's
    /// tilt (fp->x673) starts charged (ftPk_SpecialN_80124DC8).
    pub smash_input_window: f32,
    /// +0x20: the charge a smash input starts with.
    pub smash_input_charge: f32,
    /// +0x24: the charge that releases on its own.
    pub maximum_charge: f32,
    /// +0x28: launch damage at no charge.
    pub base_damage: f32,
    /// +0x2C: launch damage per charge frame.
    pub damage_per_charge: f32,
    /// +0x30: divides the entry velocity.
    pub entry_velocity_divisor: f32,
    /// +0x34: specials_start_friction, the start's ground and air friction.
    pub start_friction: f32,
    /// +0x38: specials_start_gravity, the aerial start's gravity once
    /// its script allows falling.
    pub start_gravity: f32,
    /// +0x3C: launch speed at no charge.
    pub launch_speed: f32,
    /// +0x40: launch speed per charge frame.
    pub launch_speed_per_charge: f32,
    /// +0x44: the launch's vertical speed scale.
    pub launch_vertical_speed: f32,
    /// +0x48: flight gravity before the script's flag.
    pub flight_gravity: f32,
    /// +0x4C: flight terminal velocity.
    pub flight_terminal_velocity: f32,
    /// +0x50: divides the velocity entering the end motion.
    pub end_velocity_divisor: f32,
    /// +0x54: friction of the end motion (and of flight after the flag).
    pub end_friction: f32,
    /// +0x58: gravity of the end motion (and of flight after the flag).
    pub end_gravity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuickAttackAttributes {
    /// +0x5C: frames the aerial start hangs before gravity applies.
    pub air_start_hang_frames: i32,
    /// +0x60: frames each zip travels.
    pub zip_frames: i32,
    /// +0x64: gravity of the aerial start.
    pub air_start_gravity: f32,
    /// +0x68: the grounded zip's pose angle offset.
    pub ground_angle_offset: f32,
    /// +0x6C: the grounded zip's XRotN scale.
    pub ground_scale: hsd_types::Vec3,
    /// +0x78: the aerial zip's pose angle offset.
    pub air_angle_offset: f32,
    /// +0x7C: the aerial zip's XRotN scale.
    pub air_scale: hsd_types::Vec3,
    /// +0x88: zip frames before a floor contact may cancel the zip
    /// regardless of the platform-drop check.
    pub landing_frames: f32,
    /// +0x8C: minimum stick magnitude for a directed zip.
    pub minimum_stick: f32,
    /// +0x90: zip speed per stick magnitude.
    pub speed_per_stick: f32,
    /// +0x94: zip speed at no stick magnitude.
    pub base_speed: f32,
    /// +0x98: the second zip's speed multiplier.
    pub second_zip_multiplier: f32,
    /// +0x9C: the aerial end's drift limit, times the common air_drift_max.
    pub end_drift_multiplier: f32,
    /// +0xA0: the angle past a right angle (degrees) at which a floor
    /// stops the zip; ftCommon_HandleTeleportCollisions reads it too.
    pub floor_angle_degrees: i32,
    /// +0xA4: the velocity multiplier entering the end motion.
    pub end_velocity_multiplier: f32,
    /// +0xA8: minimum angle between the two zips' sticks (degrees).
    pub second_zip_angle_degrees: i32,
    /// +0xAC: the aerial end's FallSpecial drift multiplier.
    pub fall_special_drift: f32,
    /// +0xB0: the landing lag after the aerial end.
    pub landing_lag: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThunderAttributes {
    /// +0xB4: vertical speed when the aerial bolt strikes Pikachu.
    pub air_hit_vertical_speed: f32,
    /// +0xB8: gravity of the aerial strike.
    pub air_hit_gravity: f32,
    /// +0xBC: height above Pikachu the bolt's tip is tested at.
    pub hit_height: f32,
    /// +0xC0: the bolt's initial vertical velocity.
    pub bolt_velocity: f32,
    /// +0xC4: horizontal reach of the strike test.
    pub hit_width: f32,
    /// +0xC8: vertical reach of the strike test.
    pub hit_range: f32,
    /// +0xCC: the cloud effect's height above the bolt's spawn.
    pub cloud_height: f32,
    /// +0xD0: the bolt's spawn height above Pikachu.
    pub spawn_height: f32,
    /// +0xD4, +0xD8: it_802B1DF8's integer parameters.
    pub bolt_parameters: [i32; 2],
    /// +0xDC: the bolt item kind (OnLoad registers it).
    pub bolt_item: u32,
}

/// ftData.ext_attr of the fighter data at public symbol `data_symbol`
/// (`ftDataPikachu`, or `ftDataPichu` through ftPk_Init_OnLoadForPichu).
pub fn read(archive: &Archive, data_symbol: &str) -> Result<PikachuAttributes> {
    let root = archive.public(data_symbol).ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: data_symbol.into(),
        })
    })?;
    PikachuAttributes::read(archive, special_attributes_offset(archive, root)?)
}
impl PikachuAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, PIKACHU_ATTRIBUTES_SIZE)?);
        let vec2 = |at: u32| -> Result<hsd_types::Vec2> {
            Ok(hsd_types::Vec2::new(r.f32(at)?, r.f32(at + 4)?))
        };
        let vec3 = |at: u32| -> Result<hsd_types::Vec3> {
            Ok(hsd_types::Vec3::new(
                r.f32(at)?,
                r.f32(at + 4)?,
                r.f32(at + 8)?,
            ))
        };
        let i32_at = |at: u32| -> Result<i32> { Ok(r.u32(at)? as i32) };
        Ok(Self {
            thunder_jolt: ThunderJoltAttributes {
                ground_spawn_offset: vec2(0x0)?,
                air_spawn_offset: vec2(0x8)?,
                air_landing_lag: r.f32(0x10)?,
                ground_item: r.u32(0x14)?,
                air_item: r.u32(0x18)?,
            },
            skull_bash: SkullBashAttributes {
                smash_input_window: r.f32(0x1C)?,
                smash_input_charge: r.f32(0x20)?,
                maximum_charge: r.f32(0x24)?,
                base_damage: r.f32(0x28)?,
                damage_per_charge: r.f32(0x2C)?,
                entry_velocity_divisor: r.f32(0x30)?,
                start_friction: r.f32(0x34)?,
                start_gravity: r.f32(0x38)?,
                launch_speed: r.f32(0x3C)?,
                launch_speed_per_charge: r.f32(0x40)?,
                launch_vertical_speed: r.f32(0x44)?,
                flight_gravity: r.f32(0x48)?,
                flight_terminal_velocity: r.f32(0x4C)?,
                end_velocity_divisor: r.f32(0x50)?,
                end_friction: r.f32(0x54)?,
                end_gravity: r.f32(0x58)?,
            },
            quick_attack: QuickAttackAttributes {
                air_start_hang_frames: i32_at(0x5C)?,
                zip_frames: i32_at(0x60)?,
                air_start_gravity: r.f32(0x64)?,
                ground_angle_offset: r.f32(0x68)?,
                ground_scale: vec3(0x6C)?,
                air_angle_offset: r.f32(0x78)?,
                air_scale: vec3(0x7C)?,
                landing_frames: r.f32(0x88)?,
                minimum_stick: r.f32(0x8C)?,
                speed_per_stick: r.f32(0x90)?,
                base_speed: r.f32(0x94)?,
                second_zip_multiplier: r.f32(0x98)?,
                end_drift_multiplier: r.f32(0x9C)?,
                floor_angle_degrees: i32_at(0xA0)?,
                end_velocity_multiplier: r.f32(0xA4)?,
                second_zip_angle_degrees: i32_at(0xA8)?,
                fall_special_drift: r.f32(0xAC)?,
                landing_lag: r.f32(0xB0)?,
            },
            thunder: ThunderAttributes {
                air_hit_vertical_speed: r.f32(0xB4)?,
                air_hit_gravity: r.f32(0xB8)?,
                hit_height: r.f32(0xBC)?,
                bolt_velocity: r.f32(0xC0)?,
                hit_width: r.f32(0xC4)?,
                hit_range: r.f32(0xC8)?,
                cloud_height: r.f32(0xCC)?,
                spawn_height: r.f32(0xD0)?,
                bolt_parameters: [i32_at(0xD4)?, i32_at(0xD8)?],
                bolt_item: r.u32(0xDC)?,
            },
            quick_attack_box: FtCollisionBox {
                top: r.f32(0xE0)?,
                bottom: r.f32(0xE4)?,
                left: vec2(0xE8)?,
                right: vec2(0xF0)?,
            },
        })
    }
}
