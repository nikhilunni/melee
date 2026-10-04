//! ftKoopaAttributes, ft/kinds/ftKoopa/types.h; ftData.ext_attr (+4).
//! ftKp_Init_OnLoad's PUSH_ATTRS copies 0xA0 bytes.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftKoopaAttributes).
pub const KOOPA_ATTRIBUTES_SIZE: u32 = 0xA0;

#[derive(Clone, Debug, PartialEq)]
pub struct KoopaAttributes {
    /// +0x00 x0: dmg.armor0, set on every reset (ftKp_Init_OnDeath). Zero
    /// in NTSC 1.02's PlKp.dat.
    pub armor: f32,
    pub fire_breath: FireBreathAttributes,
    pub klaw: KlawAttributes,
    pub fortress: FortressAttributes,
    pub bomb: BombAttributes,
}

/// Fire Breath (ftkoopaspecialn.c). The breath has two fuels: the flame's
/// reach (u.kp.x222C) and its lifetime (u.kp.x2230); both drain by one a
/// tick while breathing and refill outside the move.
#[derive(Clone, Debug, PartialEq)]
pub struct FireBreathAttributes {
    /// +0x04 x4: ticks of breath before releasing B may end it.
    pub minimum_ticks: i32,
    /// +0x08 x8: reach regained per tick outside the move.
    pub reach_recharge: f32,
    /// +0x0C xC: lifetime regained per tick outside the move.
    pub life_recharge: f32,
    /// +0x10 x10 / +0x14 x14: the reach's full and spent values.
    pub reach_max: f32,
    pub reach_min: f32,
    /// +0x18 x18 / +0x1C x1C: the lifetime's full and spent values.
    pub life_max: f32,
    pub life_min: f32,
    /// +0x20 x20: ticks between camera quakes.
    pub quake_interval: i32,
    /// +0x24 x24 / +0x28 x28: the flame's spawn offset from the mouth
    /// (part 48), facing- and scale-scaled.
    pub spawn_offset_x: f32,
    pub spawn_offset_y: f32,
}

/// Koopa Klaw (ftkoopaspecials.c, ftCo_CaptureKoopa.c).
#[derive(Clone, Debug, PartialEq)]
pub struct KlawAttributes {
    /// +0x2C x2C: the bite's damage to the held victim.
    pub bite_damage: u32,
    /// +0x30 x30: stick x past which a flick throws.
    pub throw_stick: f32,
    /// +0x34 x34 / +0x38 x38: the victim's struggle shake per stick unit
    /// and its limit (ftCo_800BC4A8).
    pub shake_scale: f32,
    pub shake_limit: f32,
    /// +0x3C x3C: the victim's animation rate while mashing.
    pub mash_rate: f32,
    /// +0x40 x40: ticks one mash input keeps that rate.
    pub mash_ticks: f32,
    /// +0x44 x44: grab timer taken per mash input (ftCommon_GrabMash).
    pub mash_escape: f32,
    /// +0x48 x48: grab timer taken per tick.
    pub hold_decay: f32,
    /// +0x4C x4C: the grab timer's start (ftCommon_InitGrab).
    pub hold_time: f32,
    /// +0x50: unread.
    pub unused_50: u32,
}

/// Whirling Fortress (ftkoopaspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct FortressAttributes {
    /// +0x54 x54: the aerial start's rise speed.
    pub air_rise: f32,
    /// +0x58 x58 / +0x5C x5C: gravity and terminal velocity while spinning.
    pub gravity: f32,
    pub terminal_velocity: f32,
    /// +0x60 x60: ground speed limit (and the aerial entry's clamp).
    pub ground_max: f32,
    /// +0x64 x64: air speed limit.
    pub air_max: f32,
    /// +0x68 x68 / +0x6C x6C: stick acceleration on the ground and in air.
    pub ground_accel: f32,
    pub air_accel: f32,
    /// +0x70 x70: landing window (the comparison retail makes with it can
    /// never hold).
    pub landing_window: f32,
    /// +0x74: unread.
    pub unused_74: f32,
    /// +0x78 x78: frames rewound by the never-taken landing branch.
    pub landing_rewind: f32,
    /// +0x7C x7C: landing lag after the aerial spin (0: plain fall).
    pub landing_lag: f32,
}

/// Bowser Bomb (ftkoopaspeciallw.c).
#[derive(Clone, Debug, PartialEq)]
pub struct BombAttributes {
    /// +0x80 x80 / +0x84 x84: the aerial entry's velocity multipliers.
    pub entry_scale_x: f32,
    pub entry_scale_y: f32,
    /// +0x88 x88: air friction during the drop.
    pub air_friction: f32,
    /// +0x8C x8C / +0x90 x90: gravity and terminal velocity before the drop.
    pub gravity: f32,
    pub terminal_velocity: f32,
    /// +0x94 x94: the drop's velocity (negative).
    pub drop_velocity: f32,
    /// +0x98 / +0x9C: unread by ftKoopa.
    pub unused_98: f32,
    pub unused_9c: f32,
}

pub fn read_koopa_attributes(archive: &Archive) -> Result<KoopaAttributes> {
    let root = archive.public("ftDataKoopa").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataKoopa".into(),
        })
    })?;
    KoopaAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl KoopaAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, KOOPA_ATTRIBUTES_SIZE)?);
        Ok(Self {
            armor: r.f32(0x00)?,
            fire_breath: FireBreathAttributes {
                minimum_ticks: r.s32(0x04)?,
                reach_recharge: r.f32(0x08)?,
                life_recharge: r.f32(0x0C)?,
                reach_max: r.f32(0x10)?,
                reach_min: r.f32(0x14)?,
                life_max: r.f32(0x18)?,
                life_min: r.f32(0x1C)?,
                quake_interval: r.s32(0x20)?,
                spawn_offset_x: r.f32(0x24)?,
                spawn_offset_y: r.f32(0x28)?,
            },
            klaw: KlawAttributes {
                bite_damage: r.u32(0x2C)?,
                throw_stick: r.f32(0x30)?,
                shake_scale: r.f32(0x34)?,
                shake_limit: r.f32(0x38)?,
                mash_rate: r.f32(0x3C)?,
                mash_ticks: r.f32(0x40)?,
                mash_escape: r.f32(0x44)?,
                hold_decay: r.f32(0x48)?,
                hold_time: r.f32(0x4C)?,
                unused_50: r.u32(0x50)?,
            },
            fortress: FortressAttributes {
                air_rise: r.f32(0x54)?,
                gravity: r.f32(0x58)?,
                terminal_velocity: r.f32(0x5C)?,
                ground_max: r.f32(0x60)?,
                air_max: r.f32(0x64)?,
                ground_accel: r.f32(0x68)?,
                air_accel: r.f32(0x6C)?,
                landing_window: r.f32(0x70)?,
                unused_74: r.f32(0x74)?,
                landing_rewind: r.f32(0x78)?,
                landing_lag: r.f32(0x7C)?,
            },
            bomb: BombAttributes {
                entry_scale_x: r.f32(0x80)?,
                entry_scale_y: r.f32(0x84)?,
                air_friction: r.f32(0x88)?,
                gravity: r.f32(0x8C)?,
                terminal_velocity: r.f32(0x90)?,
                drop_velocity: r.f32(0x94)?,
                unused_98: r.f32(0x98)?,
                unused_9c: r.f32(0x9C)?,
            },
        })
    }
}
