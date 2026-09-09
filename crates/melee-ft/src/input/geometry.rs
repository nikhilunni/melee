//! lb_8000D148 (lb/lb_00CE.c:160-247), restricted to the origin query used
//! by Fighter_Spaghetti_8006AD10. Kept here until melee-lb owns this helper.
use super::pad::Stick;
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::sqrtf,
};

pub fn crosses_stick_circle(previous: Stick, current: Stick, radius: f32) -> bool {
    let dx = current.x - previous.x;
    let dy = previous.y - current.y;
    // retail 0x8000D150/0x8000D164: fmuls then fmsubs.
    let cross = fmsubs(previous.x, current.y, previous.y * current.x);
    // retail 0x8000D15C/0x8000D168: fmuls then fmadds.
    let distance_squared = fmadds(dx, dx, dy * dy);
    if distance_squared < 0.00001 {
        return false;
    }
    // sqrtf uses the same three double fnmsub Newton steps as retail
    // 0x8000D1A0/0x8000D1B0/0x8000D1C0.
    let distance = sqrtf(distance_squared);
    // retail 0x8000D1E0/0x8000D1E8/0x8000D1EC. Do not elide zero products.
    let mut numerator = cross + fmadds(dx, 0.0, dy * 0.0);
    if numerator < 0.0 {
        numerator = -numerator;
    }
    if numerator / distance <= radius {
        let (px, py) = (previous.x - 0.0, previous.y - 0.0);
        let (cx, cy) = (current.x - 0.0, current.y - 0.0);
        // retail 0x8000D218..0x8000D238: separate fmuls/fadds, NOT fused.
        let before = px * px + py * py;
        let limit = radius * radius;
        let after = cx * cx + cy * cy;
        if before < limit {
            if after > limit {
                return true;
            }
            if after < limit {
                return false;
            }
            return true;
        }
        if before > limit && after > limit {
            return (previous.x > 0.0 && current.x < 0.0)
                || (previous.x < 0.0 && current.x > 0.0)
                || (previous.y > 0.0 && current.y < 0.0)
                || (previous.y < 0.0 && current.y > 0.0);
        }
        return true;
    }
    false
}
