//! MarsAttributes, ft/kinds/ftMars/types.h; ftData.ext_attr (+4).
//! Roy shares the layout via ftMs_Init_OnLoadForRoy. No arithmetic on load.
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;
pub const MARS_ATTRIBUTES_SIZE: u32 = 0x98;
#[derive(Clone, Debug, PartialEq)]
pub struct MarsAttributes {
    pub shield_breaker: ShieldBreakerAttributes,
    pub dancing_blade: DancingBladeAttributes,
    pub dolphin_slash: DolphinSlashAttributes,
    pub counter: CounterAttributes,
    /// +0x64: AbsorbDesc used as the Counter collision volume.
    pub counter_volume: CounterVolume,
    /// +0x78: SwordAttrs consumed by ftafterimage.c.
    pub sword: SwordTrailAttributes,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ShieldBreakerAttributes {
    /// +0x00, ftMars/types.h; ftmarsspecialn.c:154 multiplies by 30 ticks.
    pub maximum_charge_levels: i32,
    /// +0x04, ftMars/types.h.
    pub base_damage: i32,
    /// +0x08, ftMars/types.h; damage added per 30-tick charge level.
    pub damage_per_level: i32,
    /// +0x0C, ftMars/types.h.
    pub momentum_divisor: f32,
    /// +0x10, ftMars/types.h.
    pub friction: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DancingBladeAttributes {
    /// +0x14, ftMars/types.h.
    pub momentum_divisor: f32,
    /// +0x18, ftMars/types.h.
    pub air_friction: f32,
    /// +0x1C, ftMars/types.h.
    pub first_air_vertical_speed: f32,
    /// +0x20, ftMars/types.h.
    pub fall_acceleration: f32,
    /// +0x24, ftMars/types.h.
    pub terminal_velocity: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DolphinSlashAttributes {
    /// +0x28, ftMars/types.h.
    pub freefall_mobility: f32,
    /// +0x2C, ftMars/types.h.
    pub landing_lag: f32,
    /// +0x30, ftMars/types.h.
    pub reverse_stick_threshold: f32,
    /// +0x34, ftMars/types.h.
    pub angle_stick_threshold: f32,
    /// +0x38, ftMars/types.h.
    pub maximum_angle: f32,
    /// +0x3C, ftMars/types.h.
    pub startup_momentum_multiplier: f32,
    /// +0x40, ftMars/types.h.
    pub ending_momentum_multiplier: f32,
    /// +0x44, ftMars/types.h.
    pub fall_acceleration: f32,
    /// +0x48, ftMars/types.h.
    pub terminal_velocity: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CounterAttributes {
    /// +0x4C, ftMars/types.h.
    pub momentum_divisor: f32,
    /// +0x50, ftMars/types.h.
    pub air_friction: f32,
    /// +0x54, ftMars/types.h.
    pub fall_acceleration: f32,
    /// +0x58, ftMars/types.h.
    pub terminal_velocity: f32,
    /// +0x5C, ftMars/types.h.
    pub damage_multiplier: f32,
    /// +0x60, ftMars/types.h.
    pub collision_multiplier: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CounterVolume {
    /// +0x64, AbsorbDesc +0.
    pub bone: i32,
    /// +0x68, AbsorbDesc +4.
    pub offset: Vec3,
    /// +0x74, AbsorbDesc +10.
    pub radius: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SwordTrailAttributes {
    /// +0x78/+0x7C, SwordAttrs +0/+4: trail interpolation parameters.
    pub interpolation: [f32; 2],
    /// +0x80..+0x88, SwordAttrs +8..+10; byte parameters, not padding.
    // TODO(meaning): color and trail byte semantics are not fully named in types.h.
    pub appearance: [u8; 9],
    /// +0x8C, SwordAttrs +14; ftafterimage.c:447 selects this joint.
    pub bone: i32,
    /// +0x90/+0x94, SwordAttrs +18/+1C; ftafterimage.c:445-446.
    pub endpoints: [f32; 2],
}
pub fn read_mars_attributes(archive: &Archive) -> Result<MarsAttributes> {
    let root = archive.public("ftDataMars").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataMars".into(),
        })
    })?;
    MarsAttributes::read(archive, special_attributes_offset(archive, root)?)
}
impl MarsAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, MARS_ATTRIBUTES_SIZE)?);
        Ok(Self {
            shield_breaker: ShieldBreakerAttributes {
                maximum_charge_levels: r.s32(0x0)?,
                base_damage: r.s32(0x4)?,
                damage_per_level: r.s32(0x8)?,
                momentum_divisor: r.f32(0xC)?,
                friction: r.f32(0x10)?,
            },
            dancing_blade: DancingBladeAttributes {
                momentum_divisor: r.f32(0x14)?,
                air_friction: r.f32(0x18)?,
                first_air_vertical_speed: r.f32(0x1C)?,
                fall_acceleration: r.f32(0x20)?,
                terminal_velocity: r.f32(0x24)?,
            },
            dolphin_slash: DolphinSlashAttributes {
                freefall_mobility: r.f32(0x28)?,
                landing_lag: r.f32(0x2C)?,
                reverse_stick_threshold: r.f32(0x30)?,
                angle_stick_threshold: r.f32(0x34)?,
                maximum_angle: r.f32(0x38)?,
                startup_momentum_multiplier: r.f32(0x3C)?,
                ending_momentum_multiplier: r.f32(0x40)?,
                fall_acceleration: r.f32(0x44)?,
                terminal_velocity: r.f32(0x48)?,
            },
            counter: CounterAttributes {
                momentum_divisor: r.f32(0x4C)?,
                air_friction: r.f32(0x50)?,
                fall_acceleration: r.f32(0x54)?,
                terminal_velocity: r.f32(0x58)?,
                damage_multiplier: r.f32(0x5C)?,
                collision_multiplier: r.f32(0x60)?,
            },
            counter_volume: CounterVolume {
                bone: r.s32(0x64)?,
                offset: Vec3::new(r.f32(0x68)?, r.f32(0x6C)?, r.f32(0x70)?),
                radius: r.f32(0x74)?,
            },
            sword: SwordTrailAttributes {
                interpolation: [r.f32(0x78)?, r.f32(0x7C)?],
                appearance: r.slice(0x80, 9)?.try_into().unwrap(),
                bone: r.s32(0x8C)?,
                endpoints: [r.f32(0x90)?, r.f32(0x94)?],
            },
        })
    }
}
