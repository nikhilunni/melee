//! The ordinary (non-deferred displacement) `Fighter_procUpdate` path.
use super::FighterPhysics;
use gekko_math::fma::fmadds;
use hsd_types::Vec3;

#[derive(Clone, Copy, Debug, Default)]
pub struct VelocityBlend {
    /// `dmg.x1948`, Fighter +0x1948; zero disables interpolation.
    pub duration: i32,
    /// `dmg.x194C`, Fighter +0x194C.
    pub remaining: i32,
    /// `xA4_unk_vel`, Fighter +0xA4.
    pub origin: Vec3,
}
impl VelocityBlend {
    /// `Fighter_procUpdate`, fighter.c:2306-2326, 0x8006BC24..BCB4.
    /// Only the temporary integration velocity changes, not `self_vel`.
    pub fn apply(&mut self, velocity: Vec3) -> Vec3 {
        if self.duration == 0 {
            return velocity;
        }
        let progress = 1.0 - self.remaining as f32 / self.duration as f32;
        let result = Vec3::new(
            // retail 0x8006BC7C / 0x8006BC90: fmadds.
            fmadds(progress, velocity.x - self.origin.x, self.origin.x),
            fmadds(progress, velocity.y - self.origin.y, self.origin.y),
            velocity.z,
        );
        self.remaining = self.remaining.wrapping_sub(1);
        if self.remaining == 0 {
            self.duration = 0;
        }
        result
    }
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    // PSVECAdd (0x80342D54): ps_add, no multiplication or contraction.
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

/// `Fighter_procUpdate` (0x8006B82C), fighter.c:2292-2394.
/// Moving-floor displacement is applied by the caller after this function.
/// Retail omits the C `cur_pos.y += 0` and `cur_pos.z += 0` statements.
pub fn integrate_velocity(state: &mut FighterPhysics) {
    // retail 0x8006BBDC/BBE4: two fadds, preserving the parentheses.
    state.ground_velocity += state.ground_acceleration + state.secondary_ground_acceleration;
    state.ground_acceleration = 0.0;
    state.secondary_ground_acceleration = 0.0;
    state.self_velocity = add(state.self_velocity, state.animation_velocity);
    state.animation_velocity = Vec3::ZERO;
    let velocity = state.velocity_blend.apply(state.self_velocity);
    // retail 0x8006BCC0/BCD0: only x and z nudge additions are present.
    state.position.x += state.player_nudge.x;
    state.position.z += state.player_nudge.y;
    state.position = add(state.position, velocity);
    state.position.x += state.knockback_velocity.x;
    state.position.y += state.knockback_velocity.y;
    state.position = add(state.position, state.shield_knockback_velocity);
}

/// `Fighter_procUpdate` 0x8006BE48..BE7C: moving floor, then wind.
// Keep this concrete integration body in melee-ft. Rust's cross-crate
// inlining heuristic otherwise emits another copy in each special family.
#[inline(never)]
pub fn integrate_environment(state: &mut FighterPhysics, floor_speed: Option<Vec3>, wind: Vec3) {
    if let Some(speed) = floor_speed {
        state.position = add(state.position, speed);
    }
    state.position = add(state.position, wind);
}
