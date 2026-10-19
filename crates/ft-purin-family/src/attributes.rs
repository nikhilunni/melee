//! Rollout's attribute block: Jigglypuff's ext_attr +34..+D8 (ftPurin/types.h)
//! and the same 0xA8 bytes at Kirby's +2B4..+358 (`specialn_pr_*`,
//! ftKirby/types.h). Offsets below are from the block's start; the
//! comments give Jigglypuff's.
use hsd_archive::Reader;
use melee_ft::desc::FighterDescError;

/// The block's length.
pub const ROLLOUT_ATTRIBUTES_SIZE: u32 = 0xA8;

#[derive(Clone, Debug, PartialEq)]
pub struct RolloutAttributes {
    /// +0x34: frames of rolling.
    pub duration: i32,
    /// +0x38: roll frames a hit costs.
    pub hit_frame_cost: i32,
    /// +0x3C: the roll's gravity.
    pub gravity: f32,
    /// +0x40: the roll's terminal speed.
    pub terminal_velocity: f32,
    /// +0x44: x1C on the ground.
    pub ground_acceleration: f32,
    /// +0x4C: the first ground speed limit.
    pub ground_speed_limit: f32,
    /// +0x50: the second ground speed limit.
    pub slope_speed_limit: f32,
    /// +0x54: x1C in the air.
    pub air_acceleration: f32,
    /// +0x58: the aerial roll's deceleration.
    pub air_deceleration: f32,
    /// +0x5C: the aerial roll's minimum speed.
    pub air_minimum_speed: f32,
    /// +0x68: the stick deflection that turns the roll.
    pub reverse_stick_threshold: f32,
    /// +0x6C: the turn's spin.
    pub turn_rotation_speed: f32,
    /// +0x70: frames between the turn's dust.
    pub turn_effect_interval: i32,
    /// +0x74: above this speed a turn ignores floor edges.
    pub landing_speed_threshold: f32,
    /// +0x78: the share of vertical speed a floor bounce keeps.
    pub bounce_vertical_multiplier: f32,
    /// +0x7C: below this bounce speed the roll lands.
    pub minimum_bounce_speed: f32,
    /// +0x80: added to the speed for the damage.
    pub damage_speed_offset: f32,
    /// +0x84: the damage per unit of speed.
    pub damage_multiplier: f32,
    /// +0x88: the share of speed a hit bounce keeps.
    pub hit_horizontal_multiplier: f32,
    /// +0x8C: a hit bounce's upward speed.
    pub hit_vertical_speed: f32,
    /// +0x90: the end's horizontal damping.
    pub rebound_horizontal_multiplier: f32,
    /// +0x94: the aerial end's vertical damping.
    pub rebound_vertical_multiplier: f32,
    /// +0x98: the rolling spin.
    pub release_rotation_speed: f32,
    /// +0x9C: frames before the hitbox may hit again.
    pub hitbox_refresh_interval: i32,
    /// +0xA0: the charge at the start.
    pub initial_charge: f32,
    /// +0xA4: the full charge.
    pub maximum_charge: f32,
    /// +0xA8: the charge per frame held.
    pub charge_per_tick: f32,
    /// +0xAC: the charge's spin, in degrees per unit.
    pub charge_rotation_multiplier: f32,
    /// +0xB4: the charge lost per frame of rolling.
    pub charge_decay: f32,
    /// +0xB8: below this charge the roll ends.
    pub minimum_charge: f32,
    /// +0xBC: the aerial spin's multiplier.
    pub air_rotation_multiplier: f32,
    /// +0xC0: the speed per unit of charge.
    pub charge_speed_multiplier: f32,
    /// +0xC4: the turn's friction multiplier.
    pub ground_friction_multiplier: f32,
    /// +0xC8: how much a slope adds or takes.
    pub slope_influence: f32,
    /// +0xCC: below this speed the hitbox is off.
    pub minimum_hitbox_speed: f32,
    /// +0xD0: the share of the turn's entry speed that ends the turn.
    pub turn_speed_ratio: f32,
    /// +0xD4: a wall's damping of charge and speed.
    pub wall_rebound_multiplier: f32,
    /// +0xD8: the landing lag after the roll; zero lands in Wait.
    pub landing_lag: f32,
}

impl RolloutAttributes {
    /// The block at `base` (Jigglypuff's +0x34) of an attribute reader.
    pub fn read(r: &Reader<'_>, base: u32) -> Result<Self, FighterDescError> {
        let s32 = |offset: u32| r.s32(base + offset - 0x34);
        let f32 = |offset: u32| r.f32(base + offset - 0x34);
        Ok(Self {
            duration: s32(0x34)?,
            hit_frame_cost: s32(0x38)?,
            gravity: f32(0x3C)?,
            terminal_velocity: f32(0x40)?,
            ground_acceleration: f32(0x44)?,
            ground_speed_limit: f32(0x4C)?,
            slope_speed_limit: f32(0x50)?,
            air_acceleration: f32(0x54)?,
            air_deceleration: f32(0x58)?,
            air_minimum_speed: f32(0x5C)?,
            reverse_stick_threshold: f32(0x68)?,
            turn_rotation_speed: f32(0x6C)?,
            turn_effect_interval: s32(0x70)?,
            landing_speed_threshold: f32(0x74)?,
            bounce_vertical_multiplier: f32(0x78)?,
            minimum_bounce_speed: f32(0x7C)?,
            damage_speed_offset: f32(0x80)?,
            damage_multiplier: f32(0x84)?,
            hit_horizontal_multiplier: f32(0x88)?,
            hit_vertical_speed: f32(0x8C)?,
            rebound_horizontal_multiplier: f32(0x90)?,
            rebound_vertical_multiplier: f32(0x94)?,
            release_rotation_speed: f32(0x98)?,
            hitbox_refresh_interval: s32(0x9C)?,
            initial_charge: f32(0xA0)?,
            maximum_charge: f32(0xA4)?,
            charge_per_tick: f32(0xA8)?,
            charge_rotation_multiplier: f32(0xAC)?,
            charge_decay: f32(0xB4)?,
            minimum_charge: f32(0xB8)?,
            air_rotation_multiplier: f32(0xBC)?,
            charge_speed_multiplier: f32(0xC0)?,
            ground_friction_multiplier: f32(0xC4)?,
            slope_influence: f32(0xC8)?,
            minimum_hitbox_speed: f32(0xCC)?,
            turn_speed_ratio: f32(0xD0)?,
            wall_rebound_multiplier: f32(0xD4)?,
            landing_lag: f32(0xD8)?,
        })
    }
}
