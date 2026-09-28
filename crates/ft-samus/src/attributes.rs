//! ftSs_DatAttrs, ft/kinds/ftSamus/types.h; ftData.ext_attr (+4).
//! ftSs_Init_OnLoad's PUSH_ATTRS copies 0xD4 bytes; ftSs_Init_LoadSpecialAttrs
//! scales five of them only when the fighter's y scale is not 1.
use hsd_archive::{Archive, Reader};
use hsd_types::{Vec2, Vec3};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_types::mp::FtCollisionBox;
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftSs_DatAttrs).
pub const SAMUS_ATTRIBUTES_SIZE: u32 = 0xD4;

#[derive(Clone, Debug, PartialEq)]
pub struct SamusAttributes {
    pub bomb_jump: BombJumpAttributes,
    pub charge_shot: ChargeShotAttributes,
    pub missile: MissileAttributes,
    pub screw_attack: ScrewAttackAttributes,
    pub bomb: BombAttributes,
    /// +0x84: the morph ball's environment box (ft_80082888/ft_800824A0).
    pub morph_ball_box: FtCollisionBox,
    /// +0x9C..+0xA8: the standing grab's grapple timeline (fn_800D9558).
    pub grab_beam: GrappleTimeline,
    /// +0xAC..+0xB8: the dash grab's grapple timeline (fn_800D9930).
    pub dash_grab_beam: GrappleTimeline,
    /// +0xBC..+0xC8: the aerial tether's timeline (ftCo_AirCatch_Anim).
    pub air_beam: GrappleTimeline,
    /// +0xCC.
    // TODO(meaning): no decomp reader found yet.
    pub unknown_cc: f32,
    /// +0xD0.
    // TODO(meaning): no decomp reader found yet.
    pub unknown_d0: i32,
}

/// The morph-ball launch from Samus's own bomb (ftSs_Init_80128944 and
/// ftSs_SpecialLw rows 341/342).
#[derive(Clone, Debug, PartialEq)]
pub struct BombJumpAttributes {
    /// +0x00 x0: the launch animation's start frame when the bomb's hit
    /// was the second of its kind (x5F4_arr[0].idx == 2).
    pub late_start_frame: f32,
    /// +0x04 x4: the launch angle's spread either side of vertical.
    pub angle_range: f32,
    /// +0x08 x8: launch speed.
    pub speed: f32,
    /// +0x0C xC: grounded walk multiplier while balled.
    pub ground_mobility: f32,
    /// +0x10 x10: aerial drift multiplier and launch x clamp.
    pub air_mobility: f32,
    /// +0x14 x14: stick y below which a grounded roll squats.
    pub squat_stick_y: f32,
}

/// Charge Shot (ftsamusspecialn.c).
#[derive(Clone, Debug, PartialEq)]
pub struct ChargeShotAttributes {
    /// +0x18 x18: full charge, in levels (a float compared with the
    /// integer level).
    pub max_charge: f32,
    /// +0x1C x1C: aerial recoil per charge level.
    pub recoil_per_level: f32,
    /// +0x20 x20: frames per charge level.
    pub frames_per_level: i32,
    /// +0x24 x24: landing lag after an aerial shot (0: plain fall).
    pub landing_lag: f32,
}

/// Missiles (ftsamusspecials.c).
#[derive(Clone, Debug, PartialEq)]
pub struct MissileAttributes {
    /// +0x28 x28: a stick flick newer than this (x673) fires a super missile.
    pub smash_frames: f32,
    /// +0x2C x2C: velocity divisor on entry.
    pub velocity_divisor: f32,
    /// +0x30 x30: aerial friction.
    pub air_friction: f32,
    /// +0x34 x34: missile spawn x offset, facing-scaled.
    pub spawn_offset_x: f32,
}

/// Screw Attack (ftsamusspecialhi.c).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrewAttackAttributes {
    /// +0x38 x38: grounded take-off x velocity.
    pub ground_launch_x: f32,
    /// +0x3C x3C: aerial drift acceleration.
    pub air_mobility: f32,
    /// +0x40 x40: maximum drift.
    pub air_max: f32,
    /// +0x44 x44: aerial entry y velocity.
    pub air_launch_y: f32,
    /// +0x48 x48: special-fall mobility.
    pub freefall_mobility: f32,
    /// +0x4C x4C: stick x beyond which Samus turns around.
    pub reverse_stick: f32,
    /// +0x50 x50: special landing lag.
    pub landing_lag: f32,
}

/// Bomb (ftsamusspeciallw1.c).
#[derive(Clone, Debug, PartialEq)]
pub struct BombAttributes {
    /// +0x54 x54: grounded hop y velocity.
    pub ground_hop: f32,
    /// +0x58 x58: aerial entry y velocity.
    pub air_launch_y: f32,
    /// +0x5C x5C: grounded maximum walk multiplier.
    pub ground_max: f32,
    /// +0x60 x60: aerial maximum drift multiplier.
    pub air_max: f32,
    /// +0x64 x64: grounded walk acceleration multiplier.
    pub ground_accel: f32,
    /// +0x68 x68: aerial drift acceleration multiplier.
    pub air_accel: f32,
    /// +0x6C x6C: grounded entry velocity multiplier.
    pub ground_velocity_scale: f32,
    /// +0x70 x70: aerial entry x velocity multiplier.
    pub air_velocity_scale: f32,
    /// +0x74 x74_vec: spawn offset from TopN (x facing-scaled).
    pub spawn_offset: Vec3,
    /// +0x80 x80: stick y below which a grounded bomb squats.
    pub squat_stick_y: f32,
}

/// Grapple beam frames: fire, extend, retract and put away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrappleTimeline {
    /// Spawn the beam (it_802B7C18).
    pub spawn: i32,
    /// Throw it forward (it_802BAAE4).
    pub extend: i32,
    /// Pull it back (it_802BAA58).
    pub retract: i32,
    /// Remove it (it_802B7B84).
    pub remove: i32,
}

pub fn read_samus_attributes(archive: &Archive) -> Result<SamusAttributes> {
    let root = archive.public("ftDataSamus").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataSamus".into(),
        })
    })?;
    SamusAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl SamusAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, SAMUS_ATTRIBUTES_SIZE)?);
        let timeline = |base: u32| -> Result<GrappleTimeline> {
            Ok(GrappleTimeline {
                spawn: r.s32(base)?,
                extend: r.s32(base + 4)?,
                retract: r.s32(base + 8)?,
                remove: r.s32(base + 0xC)?,
            })
        };
        Ok(Self {
            bomb_jump: BombJumpAttributes {
                late_start_frame: r.f32(0x00)?,
                angle_range: r.f32(0x04)?,
                speed: r.f32(0x08)?,
                ground_mobility: r.f32(0x0C)?,
                air_mobility: r.f32(0x10)?,
                squat_stick_y: r.f32(0x14)?,
            },
            charge_shot: ChargeShotAttributes {
                max_charge: r.f32(0x18)?,
                recoil_per_level: r.f32(0x1C)?,
                frames_per_level: r.s32(0x20)?,
                landing_lag: r.f32(0x24)?,
            },
            missile: MissileAttributes {
                smash_frames: r.f32(0x28)?,
                velocity_divisor: r.f32(0x2C)?,
                air_friction: r.f32(0x30)?,
                spawn_offset_x: r.f32(0x34)?,
            },
            screw_attack: ScrewAttackAttributes {
                ground_launch_x: r.f32(0x38)?,
                air_mobility: r.f32(0x3C)?,
                air_max: r.f32(0x40)?,
                air_launch_y: r.f32(0x44)?,
                freefall_mobility: r.f32(0x48)?,
                reverse_stick: r.f32(0x4C)?,
                landing_lag: r.f32(0x50)?,
            },
            bomb: BombAttributes {
                ground_hop: r.f32(0x54)?,
                air_launch_y: r.f32(0x58)?,
                ground_max: r.f32(0x5C)?,
                air_max: r.f32(0x60)?,
                ground_accel: r.f32(0x64)?,
                air_accel: r.f32(0x68)?,
                ground_velocity_scale: r.f32(0x6C)?,
                air_velocity_scale: r.f32(0x70)?,
                spawn_offset: Vec3::new(r.f32(0x74)?, r.f32(0x78)?, r.f32(0x7C)?),
                squat_stick_y: r.f32(0x80)?,
            },
            morph_ball_box: FtCollisionBox {
                top: r.f32(0x84)?,
                bottom: r.f32(0x88)?,
                left: Vec2::new(r.f32(0x8C)?, r.f32(0x90)?),
                right: Vec2::new(r.f32(0x94)?, r.f32(0x98)?),
            },
            grab_beam: timeline(0x9C)?,
            dash_grab_beam: timeline(0xAC)?,
            air_beam: timeline(0xBC)?,
            unknown_cc: r.f32(0xCC)?,
            unknown_d0: r.s32(0xD0)?,
        })
    }
}

/// itSamusGrappleAttributes' base words (PlSs.dat ftData.x48_items[3], the
/// Article's +4), which it_802B75FC scales into the rope's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrappleArticle {
    /// x0: what a bounce keeps of the tip's velocity.
    pub bounce: f32,
    /// xC: the rope's links at scale 1.
    pub links: i32,
    /// x10 / x14: the longest and shortest span between links.
    pub span: f32,
    pub min_span: f32,
    /// x18: the throw's speed.
    pub throw_speed: f32,
    /// x1C: the hanging rope's gravity.
    pub gravity: f32,
    /// x20.
    // TODO(meaning): read only into x48, which the grabs do not use.
    pub x20: f32,
    /// x24: the reel-in speed (x4C).
    pub retract_speed: f32,
    /// x28.
    // TODO(meaning): read only into x50, which the grabs do not use.
    pub x28: f32,
    /// x2C: the pull's reel speed (x54).
    pub reel_speed: f32,
    /// x30: the loose tip's air friction (x58).
    pub friction: f32,
    /// x5C / x60: a grab's and the aerial tether's coefficient.
    pub grab_coefficient: f32,
    pub air_coefficient: f32,
}

/// ftSs_Init_OnLoad's ftData.x48_items[3]: the grapple beam's attributes.
pub fn read_grapple_article(archive: &Archive) -> Result<GrappleArticle> {
    let root = archive.public("ftDataSamus").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataSamus".into(),
        })
    })?;
    let r = archive.reader();
    let items = r.u32(root + 0x48)?;
    let article = r.u32(items + 3 * 4)?;
    let base = r.u32(article + 4)?;
    Ok(GrappleArticle {
        bounce: r.f32(base)?,
        links: r.s32(base + 0xC)?,
        span: r.f32(base + 0x10)?,
        min_span: r.f32(base + 0x14)?,
        throw_speed: r.f32(base + 0x18)?,
        gravity: r.f32(base + 0x1C)?,
        x20: r.f32(base + 0x20)?,
        retract_speed: r.f32(base + 0x24)?,
        x28: r.f32(base + 0x28)?,
        reel_speed: r.f32(base + 0x2C)?,
        friction: r.f32(base + 0x30)?,
        grab_coefficient: r.f32(base + 0x5C)?,
        air_coefficient: r.f32(base + 0x60)?,
    })
}
