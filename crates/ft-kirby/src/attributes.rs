//! ftKirby/types.h ftKb_DatAttrs: ftData.ext_attr (+4), 0x424 bytes.
//! The first 0x34 bytes are the shared Fighter_x2D0_t multijump data
//! (ft/types.h), read with its consumers' types: `lwz` for the turn count
//! and action ids, `lfs` for the impulses (ftCo_800D74A4).
use hsd_archive::{Archive, Reader};
use hsd_types::Vec2;
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_ft::fighter::multi_jump::MultiJumpAttributes;
use melee_types::mp::FtCollisionBox;

type Result<T> = std::result::Result<T, FighterDescError>;

pub const KIRBY_ATTRIBUTES_SIZE: u32 = 0x424;

/// ftKb_DatAttrs. Groups are added with the states that read them.
#[derive(Clone, Debug, PartialEq)]
pub struct KirbyAttributes {
    /// +00..+30: the midair jumps; `first_actions` are the bare (341) and
    /// helmeted (346) families.
    pub multi_jump: MultiJumpAttributes,
    /// +34 (`jumpaerial_unk`, a halfword: 40 on the disc): the Inhale's
    /// loop lasts at least this many frames (u.kb.xE4).
    pub inhale_minimum_frames: i16,
    /// +38..+C8: the Inhale's catch, the mouthful, the swallow and the spit.
    pub inhale: InhaleAttributes,
    /// +CC, +D0: Hammer.
    pub hammer: HammerAttributes,
    /// +D4..+E4: Final Cutter.
    pub final_cutter: FinalCutterAttributes,
}

/// ftKb_DatAttrs +CC, +D0 (`specials_*`).
#[derive(Clone, Debug, PartialEq)]
pub struct HammerAttributes {
    /// +CC: the upward speed of the first aerial Hammer of a flight.
    pub aerial_lift: f32,
    /// +D0: the landing lag of an aerial Hammer that reaches the floor.
    pub landing_lag: f32,
}

/// ftKb_DatAttrs +38..+C8 (`specialn_*`), named for what the code does with
/// them (the header's names are guesses). +A4..+AC place the star of a lost
/// ability (it_802ADA1C), which is not ported.
#[derive(Clone, Debug, PartialEq)]
pub struct InhaleAttributes {
    /// +38, +3C: the mouth from Kirby's position, ahead and up.
    pub mouth_offset: Vec2,
    /// +40: the caught fighter shrinks inside this distance from the mouth.
    pub shrink_radius: f32,
    /// +44: the share of its size it has lost at the mouth, and grows back
    /// out of a star.
    pub shrink: f32,
    /// +48, +4C: how far the wind moves it each tick, across and up.
    pub pull_step: Vec2,
    /// +50: within this distance of the mouth Kirby closes it.
    pub close_distance: f32,
    /// +54: taken from the hold timer per mash input.
    pub hold_mash_decrement: f32,
    /// +58: taken from the hold timer each tick.
    pub hold_decay: f32,
    /// +5C: the hold timer.
    pub hold_time: f32,
    /// +60: taken from a star's timer per mash input.
    pub star_mash_decrement: f32,
    /// +64: taken from a star's timer each tick.
    pub star_decay: f32,
    /// +68: the spat star's timer.
    pub spit_star_time: f32,
    /// +6C: the copy star's timer.
    pub copy_star_time: f32,
    /// +70: the frames over which a fighter out of a star grows back.
    pub regrow_frames: f32,
    /// +74: the stick deflection that turns Kirby round with a mouthful.
    pub turn_stick_range: f32,
    /// +78: the downward deflection that swallows.
    pub swallow_stick_range: f32,
    /// +7C: the walk's acceleration multiplier with a mouthful.
    pub walk_multiplier: f32,
    /// +80: the jump's multiplier with a mouthful.
    pub jump_multiplier: f32,
    /// +84: the speed a tap of the held fighter's stick gives Kirby.
    pub steer_speed: f32,
    /// +88: the spat star's speed.
    pub spit_speed: f32,
    /// +8C: taken off the spat star's speed each tick.
    pub spit_deceleration: f32,
    /// +90: the copy star's angle, in radians.
    pub copy_star_angle: f32,
    /// +94: the copy star's speed.
    pub copy_star_speed: f32,
    /// +98: taken off the copy star's speed each tick.
    pub copy_star_deceleration: f32,
    /// +9C, +A0: a fighter's velocity out of a spat star.
    pub star_exit_velocity: Vec2,
    /// +B0: a hit takes the copied ability with odds of one in this.
    pub ability_loss_odds: f32,
    /// +B4..+C8: a star's environment box before the fighter's own scale.
    pub star_box: FtCollisionBox,
}

/// ftKb_DatAttrs +D4..+E8 (`specialhi_*`). +E8 (`specialhi_unk`, 0 on the
/// disc) has no reader in ftkirbyspecialhi.c.
#[derive(Clone, Debug, PartialEq)]
pub struct FinalCutterAttributes {
    /// +D4: the scale on the rise's upward TransN speed.
    pub vertical_momentum: f32,
    /// +D8: the scale on PlCo's aerial drift acceleration.
    pub horizontal_momentum: f32,
    /// +DC, +E0: the beam's spawn point from TopN, ahead and up.
    pub projectile_spawn_x: f32,
    pub projectile_spawn_y: f32,
    /// +E4: the stick deflection that turns Kirby round at the start.
    pub reverse_stick_range: f32,
}

pub fn read_kirby_attributes(archive: &Archive) -> Result<KirbyAttributes> {
    let root = archive.public("ftDataKirby").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataKirby".into(),
        })
    })?;
    KirbyAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl KirbyAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, KIRBY_ATTRIBUTES_SIZE)?);
        Ok(Self {
            multi_jump: MultiJumpAttributes {
                turn_frames: r.s32(0)?,
                reverse_threshold: r.f32(4)?,
                horizontal_impulse: r.f32(8)?,
                acceleration_multiplier: r.f32(0xC)?,
                speed_multiplier: r.f32(0x10)?,
                vertical_impulses: [
                    r.f32(0x14)?,
                    r.f32(0x18)?,
                    r.f32(0x1C)?,
                    r.f32(0x20)?,
                    r.f32(0x24)?,
                ],
                state_count: r.s32(0x28)?,
                first_actions: [r.s32(0x2C)?, r.s32(0x30)?],
            },
            inhale_minimum_frames: r.s16(0x34)?,
            inhale: InhaleAttributes {
                mouth_offset: Vec2::new(r.f32(0x38)?, r.f32(0x3C)?),
                shrink_radius: r.f32(0x40)?,
                shrink: r.f32(0x44)?,
                pull_step: Vec2::new(r.f32(0x48)?, r.f32(0x4C)?),
                close_distance: r.f32(0x50)?,
                hold_mash_decrement: r.f32(0x54)?,
                hold_decay: r.f32(0x58)?,
                hold_time: r.f32(0x5C)?,
                star_mash_decrement: r.f32(0x60)?,
                star_decay: r.f32(0x64)?,
                spit_star_time: r.f32(0x68)?,
                copy_star_time: r.f32(0x6C)?,
                regrow_frames: r.f32(0x70)?,
                turn_stick_range: r.f32(0x74)?,
                swallow_stick_range: r.f32(0x78)?,
                walk_multiplier: r.f32(0x7C)?,
                jump_multiplier: r.f32(0x80)?,
                steer_speed: r.f32(0x84)?,
                spit_speed: r.f32(0x88)?,
                spit_deceleration: r.f32(0x8C)?,
                copy_star_angle: r.f32(0x90)?,
                copy_star_speed: r.f32(0x94)?,
                copy_star_deceleration: r.f32(0x98)?,
                star_exit_velocity: Vec2::new(r.f32(0x9C)?, r.f32(0xA0)?),
                ability_loss_odds: r.f32(0xB0)?,
                star_box: FtCollisionBox {
                    top: r.f32(0xB4)?,
                    bottom: r.f32(0xB8)?,
                    left: Vec2::new(r.f32(0xBC)?, r.f32(0xC0)?),
                    right: Vec2::new(r.f32(0xC4)?, r.f32(0xC8)?),
                },
            },
            hammer: HammerAttributes {
                aerial_lift: r.f32(0xCC)?,
                landing_lag: r.f32(0xD0)?,
            },
            final_cutter: FinalCutterAttributes {
                vertical_momentum: r.f32(0xD4)?,
                horizontal_momentum: r.f32(0xD8)?,
                projectile_spawn_x: r.f32(0xDC)?,
                projectile_spawn_y: r.f32(0xE0)?,
                reverse_stick_range: r.f32(0xE4)?,
            },
        })
    }
}
