//! Grounded Wait physics. Call `begin_tick` at s_link 1, `step_wait` at 4,
//! and `collision::ground::map_wait` at 6. Animation owns skeleton stepping.
//! See README.md for the exact proc subset and unsupported outer callbacks.

pub mod friction;
pub mod grounded;
pub mod integrate;

use hsd_types::{Vec2, Vec3};
use melee_types::GroundOrAir;

/// Motion fields of `Fighter` (`ft/types.h`), independent of retail layout.
#[derive(Clone, Debug)]
pub struct FighterPhysics {
    /// `cur_pos`, +0xB0.
    pub position: Vec3,
    /// `prev_pos`, +0xBC. Copied at s_link 1, before animation.
    pub previous_position: Vec3,
    /// `pos_delta`, +0xC8.
    pub position_delta: Vec3,
    /// `self_vel`, +0x80.
    pub self_velocity: Vec3,
    /// `x8c_kb_vel`, +0x8C.
    pub knockback_velocity: Vec3,
    /// `x98_atk_shield_kb`, +0x98.
    pub shield_knockback_velocity: Vec3,
    /// `x74_anim_vel`, +0x74. Wait overwrites this with ground acceleration;
    /// it does not consume the animation's TransN offset.
    pub animation_velocity: Vec3,
    /// `gr_vel`, +0xEC.
    pub ground_velocity: f32,
    /// `xE4_ground_accel_1`, +0xE4.
    pub ground_acceleration: f32,
    /// `xE8_ground_accel_2`, +0xE8.
    pub secondary_ground_acceleration: f32,
    /// `xF0_ground_kb_vel`, +0xF0.
    pub ground_knockback_velocity: f32,
    /// `xF4_ground_attacker_shield_kb_vel`, +0xF4.
    pub ground_shield_knockback_velocity: f32,
    /// `xF8_playerNudgeVel`, +0xF8; x is horizontal, y is depth.
    pub player_nudge: Vec2,
    /// `ground_or_air`, +0xE0.
    pub ground_or_air: GroundOrAir,
    /// `x1968_jumpsUsed`, +0x1968; unchanged by supported Wait.
    pub jumps_used: u8,
    /// `dmg.x1830_percent`, +0x1830; unchanged by this path.
    pub percent: f32,
    /// `facing_dir`, +0x2C, always +1 or -1.
    pub facing: f32,
    /// `dmg.x1948/x194C` and `xA4_unk_vel` interpolation state.
    pub velocity_blend: integrate::VelocityBlend,
    /// `shield_hit.skip_update_pos`, +0x19C4 bit 7 (C first bitfield).
    pub shield_position_cached: bool,
}

impl FighterPhysics {
    /// An already grounded Wait fighter. Spawn owns finding its initial floor.
    pub fn standing(position: Vec3, facing: f32) -> Self {
        assert!(facing == 1.0 || facing == -1.0);
        Self {
            position,
            previous_position: position,
            position_delta: Vec3::ZERO,
            self_velocity: Vec3::ZERO,
            knockback_velocity: Vec3::ZERO,
            shield_knockback_velocity: Vec3::ZERO,
            animation_velocity: Vec3::ZERO,
            ground_velocity: 0.0,
            ground_acceleration: 0.0,
            secondary_ground_acceleration: 0.0,
            ground_knockback_velocity: 0.0,
            ground_shield_knockback_velocity: 0.0,
            player_nudge: Vec2::ZERO,
            ground_or_air: GroundOrAir::Ground,
            jumps_used: 0,
            percent: 0.0,
            facing,
            velocity_blend: Default::default(),
            shield_position_cached: false,
        }
    }

    /// `Fighter_8006A360` (0x8006A360), fighter.c:1449-1453, s_link 1.
    pub fn begin_tick(&mut self) {
        self.position_delta = Vec3::new(
            self.position.x - self.previous_position.x,
            self.position.y - self.previous_position.y,
            self.position.z - self.previous_position.z,
        );
        self.previous_position = self.position;
    }
}
