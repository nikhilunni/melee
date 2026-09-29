//! ftIceClimberAttributes (ft/kinds/ftPopo/types.h), 0x15C bytes read from
//! `ftDataPopo` or `ftDataNana`'s ext_attr (+4). Both climbers PUSH_ATTRS
//! the same layout (ftPp_Init_OnLoad, ftPp_Init_OnLoadForNana); the two
//! archives carry identical values. Fields join this struct as the code
//! that reads them is ported.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftIceClimberAttributes).
pub const ICE_CLIMBER_ATTRIBUTES_SIZE: u32 = 0x15C;

#[derive(Clone, Debug, PartialEq)]
pub struct IceClimberAttributes {
    /// +0x00: Popo's fp->x40, the spawn and revival offset along his facing
    /// (ftCommon_800804EC).
    pub leader_spawn_offset: f32,
    /// +0x04: the lift of a first aerial Ice Shot (ftPp_SpecialAirN_Enter).
    pub air_lift: f32,
    /// +0x08: an aerial Ice Shot's landing lag (ftPp_SpecialAirN_Coll).
    pub ice_shot_landing_lag: f32,
    /// +0x0C / +0x10: where the ice block forms, along the facing and above
    /// TopN (ftPp_SpecialN_8011F500).
    pub ice_reach: f32,
    pub ice_height: f32,
    /// +0x20..+0x70: the Squall Hammer (ftpopospecials.c).
    pub squall: SquallAttributes,
    /// +0xC4: Nana's fp->x40 (ftNn_Init_OnLoad).
    pub partner_spawn_offset: f32,
    /// +0xC8: Nana's dmg.armor0, set on every reset (ftNn_Init_OnDeath).
    pub partner_armor: f32,
    /// +0xD0: how near Nana must stand to join Popo's Squall Hammer
    /// (ftNn_Init_80123954, squared against the scale).
    pub squall_join_distance: f32,
    /// +0x12C: Nana's FallSpecial landing lag when her Squall Hammer ends
    /// in the air (ftPp_SpecialS_1_Anim).
    pub partner_squall_landing_lag: f32,
    /// The Belay's attributes (+0x74..+0xB4, +0x130..+0x14C).
    pub belay: BelayAttributes,
}

/// ftIceClimberAttributes +0x20..+0x70: the Squall Hammer. `solo` values
/// apply to SpecialS1/SpecialAirS1 (Popo alone), `linked` to SpecialS2/
/// SpecialAirS2 (Nana with him).
#[derive(Clone, Debug, PartialEq)]
pub struct SquallAttributes {
    /// +0x20 / +0x24: the aerial entry's vertical speed.
    pub air_entry_rise: [f32; 2],
    /// +0x28: the grounded entry's speed along the facing.
    pub ground_entry_speed: f32,
    /// +0x2C: the aerial entry's horizontal speed along the facing.
    pub air_entry_speed: f32,
    /// +0x30 / +0x34: the stick's acceleration on the ground / in the air.
    pub ground_steer: f32,
    pub air_steer: f32,
    /// +0x38 / +0x3C: the speed cap on the ground / in the air.
    pub ground_speed_max: f32,
    pub air_speed_max: f32,
    /// +0x40: the stick's dead zone for steering.
    pub steer_threshold: f32,
    /// +0x44: a wall reverses the speed scaled by this ...
    pub wall_rebound: f32,
    /// +0x48: ... unless it is slower than this, which it rebounds at.
    pub wall_rebound_min: f32,
    /// +0x4C / +0x50: gravity, solo and linked, for the first +0x5C frames.
    pub gravity: [f32; 2],
    /// +0x54 / +0x58: terminal velocity, solo and linked.
    pub terminal_velocity: [f32; 2],
    /// +0x5C: frames of the move's own gravity before the ordinary one.
    pub gravity_frames: f32,
    /// +0x60 / +0x64: the rise of a B press, solo and linked.
    pub press_rise: [f32; 2],
    /// +0x68: frames between rises (a press counts once more have passed).
    pub press_interval: i32,
    /// +0x6C: the floor slope's pull (ftPp_SpecialS1_Phys: fmadds with the
    /// floor normal's x).
    pub slope_pull: f32,
    /// +0x70: FallSpecial landing lag when Popo's move ends in the air.
    pub landing_lag: f32,
}

/// ftIceClimberAttributes x74..xB4 (Popo's Belay) and x130..x14C (Nana's
/// flight up the rope).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BelayAttributes {
    /// +0x74 / +0x78: the special fall's drift scale and landing lag
    /// (ftCo_80096900) after an aerial Belay.
    pub fall_mobility: f32,
    pub landing_lag: f32,
    /// +0x7C: Nana joins from no farther than this (checkNanaInRange).
    pub partner_reach: f32,
    /// +0x80: the stick's x past this turns the climber (the IASA) or
    /// drifts him (the climb's physics).
    pub stick_threshold: f32,
    /// +0x84 / +0x88: the entry divides the ground speed, or the aerial
    /// velocity's x and y, by these.
    pub entry_speed_divisor_x: f32,
    pub entry_speed_divisor_y: f32,
    /// +0x8C / +0x90: gravity and terminal speed while throwing Nana.
    pub throw_gravity: f32,
    pub throw_terminal_velocity: f32,
    /// +0x94 / +0x98: Popo's climb speed toward Nana, and the distance
    /// that adds one more unit of it (ftPp_SpecialS_80120E68).
    pub climb_base_speed: f32,
    pub climb_distance_per_speed: f32,
    /// +0x9C / +0xA0: gravity and terminal speed while climbing.
    pub climb_gravity: f32,
    pub climb_terminal_velocity: f32,
    /// +0xA4: the solo Belay's lift (ftPp_SpecialAirHiStart_1_Anim).
    pub solo_lift: f32,
    /// +0xA8 / +0xAC: gravity and terminal speed of the solo Belay.
    pub solo_gravity: f32,
    pub solo_terminal_velocity: f32,
    /// +0xB0 / +0xB4: the climb's drift, as scales of the common aerial
    /// drift acceleration and maximum.
    pub climb_drift_scale: f32,
    pub climb_drift_max_scale: f32,
    /// +0x130 / +0x134: Nana's special fall drift scale and landing lag.
    pub partner_fall_mobility: f32,
    pub partner_landing_lag: f32,
    /// +0x138: the stick's x past this drifts the flung Nana.
    pub partner_stick_threshold: f32,
    /// +0x13C / +0x140: Nana's launch speed and angle (radians).
    pub partner_launch_speed: f32,
    pub partner_launch_angle: f32,
    /// +0x144 / +0x148: gravity and terminal speed of the flung Nana.
    pub partner_gravity: f32,
    pub partner_terminal_velocity: f32,
    /// +0x14C: the share of her speed Nana keeps turning off a wall.
    pub partner_wall_rebound: f32,
}

/// The attributes of `symbol`'s ftData (`ftDataPopo` or `ftDataNana`).
pub fn read(archive: &Archive, symbol: &str) -> Result<IceClimberAttributes> {
    let root = archive.public(symbol).ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: symbol.into(),
        })
    })?;
    IceClimberAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl IceClimberAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(
            archive
                .reader()
                .slice(offset, ICE_CLIMBER_ATTRIBUTES_SIZE)?,
        );
        Ok(Self {
            leader_spawn_offset: r.f32(0x00)?,
            air_lift: r.f32(0x04)?,
            ice_shot_landing_lag: r.f32(0x08)?,
            ice_reach: r.f32(0x0C)?,
            ice_height: r.f32(0x10)?,
            squall: SquallAttributes {
                air_entry_rise: [r.f32(0x20)?, r.f32(0x24)?],
                ground_entry_speed: r.f32(0x28)?,
                air_entry_speed: r.f32(0x2C)?,
                ground_steer: r.f32(0x30)?,
                air_steer: r.f32(0x34)?,
                ground_speed_max: r.f32(0x38)?,
                air_speed_max: r.f32(0x3C)?,
                steer_threshold: r.f32(0x40)?,
                wall_rebound: r.f32(0x44)?,
                wall_rebound_min: r.f32(0x48)?,
                gravity: [r.f32(0x4C)?, r.f32(0x50)?],
                terminal_velocity: [r.f32(0x54)?, r.f32(0x58)?],
                gravity_frames: r.f32(0x5C)?,
                press_rise: [r.f32(0x60)?, r.f32(0x64)?],
                press_interval: r.s32(0x68)?,
                slope_pull: r.f32(0x6C)?,
                landing_lag: r.f32(0x70)?,
            },
            partner_spawn_offset: r.f32(0xC4)?,
            partner_armor: r.f32(0xC8)?,
            squall_join_distance: r.f32(0xD0)?,
            partner_squall_landing_lag: r.f32(0x12C)?,
            belay: BelayAttributes {
                fall_mobility: r.f32(0x74)?,
                landing_lag: r.f32(0x78)?,
                partner_reach: r.f32(0x7C)?,
                stick_threshold: r.f32(0x80)?,
                entry_speed_divisor_x: r.f32(0x84)?,
                entry_speed_divisor_y: r.f32(0x88)?,
                throw_gravity: r.f32(0x8C)?,
                throw_terminal_velocity: r.f32(0x90)?,
                climb_base_speed: r.f32(0x94)?,
                climb_distance_per_speed: r.f32(0x98)?,
                climb_gravity: r.f32(0x9C)?,
                climb_terminal_velocity: r.f32(0xA0)?,
                solo_lift: r.f32(0xA4)?,
                solo_gravity: r.f32(0xA8)?,
                solo_terminal_velocity: r.f32(0xAC)?,
                climb_drift_scale: r.f32(0xB0)?,
                climb_drift_max_scale: r.f32(0xB4)?,
                partner_fall_mobility: r.f32(0x130)?,
                partner_landing_lag: r.f32(0x134)?,
                partner_stick_threshold: r.f32(0x138)?,
                partner_launch_speed: r.f32(0x13C)?,
                partner_launch_angle: r.f32(0x140)?,
                partner_gravity: r.f32(0x144)?,
                partner_terminal_velocity: r.f32(0x148)?,
                partner_wall_rebound: r.f32(0x14C)?,
            },
        })
    }
}

/// ftData.x48_items[2] (ftPp_Init_OnLoad registers It_Kind_IceClimber_GumStrings
/// third): the rope article's itClimbersStringAttributes (+4).
const ROPE_ARTICLE: u32 = 2;

/// Popo's rope attributes (x0..x20 of itClimbersStringAttributes).
pub fn read_rope(
    archive: &Archive,
    symbol: &str,
) -> Result<crate::special_hi::rope::RopeAttributes> {
    let root = archive.public(symbol).ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: symbol.into(),
        })
    })?;
    let r = archive.reader();
    let items = r.u32(root + 0x48)?;
    let article = r.u32(items + ROPE_ARTICLE * 4)?;
    let special = r.u32(article + 4)?;
    let word = |offset: u32| r.u32(special + offset);
    Ok(crate::special_hi::rope::RopeAttributes {
        links: word(0x00)? as usize,
        hanging_links: word(0x04)? as i32,
        link_length: r.f32(special + 0x08)?,
        minimum_link_length: r.f32(special + 0x0C)?,
        gravity: r.f32(special + 0x14)?,
        pay_out_frame: word(0x18)? as i32,
        hang_frame: word(0x1C)? as i32,
        reel_frame: word(0x20)? as i32,
    })
}
