//! Body separation, ftcommon.c:720-888. Stage adjacency is supplied by the owner.
use gekko_math::{fma::fmadds, msl::fabsf};
use hsd_types::{Vec2, Vec3};
/// ftData.x50 and PlCo.x450..458, scaled by ftChangeParam.
pub struct OverlapParameters {
    pub center: f32,
    pub half_width: f32,
    pub horizontal_step: f32,
    pub depth_step: f32,
    pub depth_limit: f32,
}

/// The read-only fighter-list fields used by ftCommon_8007DD7C.
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
}
/// ftCommon_8007E0E4 (8007E0E4), called after each fighter's Anim callback.
/// ftCommon_8007DD7C (8007DD7C) visits fighters in entity-list order.
pub fn nudge(
    slot: usize,
    bodies: &[OverlapBody],
    params: &OverlapParameters,
    adjacent_floors: impl Fn(i32) -> [i32; 2],
) -> Vec2 {
    let body = &bodies[slot];
    let mut velocity = Vec2::ZERO;
    if !body.eligible || body.hitlag {
        return velocity;
    }
    if !body.ignore_others {
        for (other_slot, other) in bodies.iter().enumerate() {
            if other_slot == slot || !other.eligible || other.linked {
                continue;
            }
            if body.floor != other.floor {
                let neighbors = adjacent_floors(body.floor);
                if other.floor != neighbors[0] && other.floor != neighbors[1] {
                    continue;
                }
            }
            // retail 8007DE5C/8007DE64: both centers fuse before subtraction.
            let separation = fmadds(body.center, body.facing, body.position.x)
                - fmadds(other.facing, other.center, other.position.x);
            if fabsf(separation) < body.half_width + other.half_width {
                let negative = if separation != 0.0 {
                    separation < 0.0
                } else {
                    other_slot > slot
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
        velocity.y = if depth < 0.0 {
            params.depth_step
        } else {
            -params.depth_step
        };
    }
    // 8007E0E4 --fused: none; additions stay separately rounded.
    if (velocity.y > 0.0 && depth < 0.0 && depth + velocity.y >= 0.0)
        || (velocity.y < 0.0 && depth > 0.0 && depth + velocity.y <= 0.0)
    {
        velocity.y = -depth;
    }
    if depth + velocity.y > params.depth_limit {
        velocity.y = params.depth_limit - depth;
    } else if depth + velocity.y < -params.depth_limit {
        velocity.y = -params.depth_limit - depth;
    }
    velocity
}
