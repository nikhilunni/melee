//! Body separation, ftcommon.c:720-888. Stage adjacency is supplied by the owner.
use gekko_math::{fma::fmadds, msl::fabsf};
use hsd_types::{Vec2, Vec3};
/// ftData.x50 and PlCo.x450..460, scaled by ftChangeParam.
pub struct OverlapParameters {
    pub center: f32,
    pub half_width: f32,
    pub horizontal_step: f32,
    pub depth_step: f32,
    pub depth_limit: f32,
    /// PlCo +45C: a partner's (Nana's) depth step, also taken when she
    /// overlaps her player's own fighter (ftCommon_8007DFD0).
    pub partner_depth_step: f32,
    /// PlCo +460: a partner's depth limit.
    pub partner_depth_limit: f32,
}

/// The read-only fighter-list fields used by ftCommon_8007DD7C.
#[derive(Clone, Copy, Debug, Default)]
pub struct OverlapBody {
    pub position: Vec3,
    pub center: f32,
    pub half_width: f32,
    pub facing: f32,
    pub floor: i32,
    pub eligible: bool,
    pub linked: bool,
    pub ignore_others: bool,
    pub hitlag: bool,
    /// player_id: fighters of one player never push each other
    /// (ftLib_80086FD4).
    pub player: u8,
    /// x221F_b4: a player's partner (Nana). Nobody else is pushed by her; she
    /// keeps a depth offset from her own player's fighter instead.
    pub secondary: bool,
}
/// Whether `other`'s floor is `body`'s or one next to it.
fn adjacent(
    body: &OverlapBody,
    other: &OverlapBody,
    adjacent_floors: impl Fn(i32) -> [i32; 2],
) -> bool {
    if body.floor == other.floor {
        return true;
    }
    let neighbors = adjacent_floors(body.floor);
    other.floor == neighbors[0] || other.floor == neighbors[1]
}

/// ftCommon_8007DD7C/8007DFD0's horizontal separation: both centers fuse
/// before the subtraction (retail 8007DE5C/8007DE64, 8007E070/8007E07C).
fn separation(body: &OverlapBody, other: &OverlapBody) -> f32 {
    fmadds(body.center, body.facing, body.position.x)
        - fmadds(other.facing, other.center, other.position.x)
}

/// ftCommon_8007E0E4 (8007E0E4), called after each fighter's Anim callback.
/// ftCommon_8007DD7C (8007DD7C) visits fighters in entity-list order; a
/// partner first runs ftCommon_8007DFD0 (8007DFD0) against her player's own
/// fighter (`owner`, Player_GetEntity).
pub fn nudge(
    slot: usize,
    owner: usize,
    bodies: &[OverlapBody],
    params: &OverlapParameters,
    adjacent_floors: impl Fn(i32) -> [i32; 2],
) -> Vec2 {
    let body = &bodies[slot];
    let mut velocity = Vec2::ZERO;
    if !body.eligible || body.hitlag {
        return velocity;
    }
    let (depth_step, depth_limit) = if body.secondary {
        (params.partner_depth_step, params.partner_depth_limit)
    } else {
        (params.depth_step, params.depth_limit)
    };
    if !body.ignore_others && body.secondary {
        // ftCommon_8007DFD0: only the own fighter's x221F_b3 and ground
        // state gate this check (8007DFFC..8007E014).
        let own = &bodies[owner];
        if !own.disabled_for_partner()
            && adjacent(body, own, &adjacent_floors)
            && fabsf(separation(body, own)) < body.half_width + own.half_width
        {
            velocity.y -= params.partner_depth_step;
        }
    }
    if !body.ignore_others {
        // phi_r28: this fighter, or another of its player's, has been passed.
        let mut passed = false;
        for (other_slot, other) in bodies.iter().enumerate() {
            if other_slot == slot || other.player == body.player {
                passed = true;
                continue;
            }
            if !other.eligible || other.linked || other.secondary {
                continue;
            }
            if !adjacent(body, other, &adjacent_floors) {
                continue;
            }
            let separation = separation(body, other);
            if fabsf(separation) < body.half_width + other.half_width {
                let negative = if separation != 0.0 {
                    separation < 0.0
                } else {
                    passed
                };
                velocity.x += if negative {
                    -params.horizontal_step
                } else {
                    params.horizontal_step
                };
                let depth = body.position.z - other.position.z;
                let negative_depth = if depth != 0.0 { depth < 0.0 } else { negative };
                velocity.y += if negative_depth {
                    -params.depth_step
                } else {
                    params.depth_step
                };
            }
        }
    }
    let depth = body.position.z;
    if velocity.y == 0.0 && depth != 0.0 {
        velocity.y = if depth < 0.0 { depth_step } else { -depth_step };
    }
    // 8007E0E4 --fused: none; additions stay separately rounded.
    if (velocity.y > 0.0 && depth < 0.0 && depth + velocity.y >= 0.0)
        || (velocity.y < 0.0 && depth > 0.0 && depth + velocity.y <= 0.0)
    {
        velocity.y = -depth;
    }
    if depth + velocity.y > depth_limit {
        velocity.y = depth_limit - depth;
    } else if depth + velocity.y < -depth_limit {
        velocity.y = -depth_limit - depth;
    }
    // ftcommon.c:878: a partner is never pushed sideways.
    if body.secondary {
        velocity.x = 0.0;
    }
    velocity
}

impl OverlapBody {
    /// ftCommon_8007DFD0's view of the player's own fighter: x221F_b3 or
    /// airborne. `eligible` is exactly `!x221F_b3 && grounded`.
    fn disabled_for_partner(&self) -> bool {
        !self.eligible
    }
}
