//! lbVector_EulerAnglesFromONB (8000DF0C) and PartialONB (8000DFF4).
use crate::{
    dynamics::arithmetic,
    trigf::{asinf, atan2f},
};
use hsd_types::Vec3;

/// Ordered cross/normalize followed by scalar inverse trig; no fused sites in
/// either retail wrapper. PSVECCrossProduct's fusion is in the shared helper.
pub fn from_partial_basis(forward: Vec3, up: Vec3) -> Vec3 {
    let right = arithmetic::cross(up, forward);
    if right.z == -1.0 {
        Vec3::new(atan2f(up.x, up.y), std::f32::consts::FRAC_PI_2, 0.0)
    } else if right.z == 1.0 {
        Vec3::new(atan2f(-up.x, up.y), -std::f32::consts::FRAC_PI_2, 0.0)
    } else {
        Vec3::new(
            atan2f(up.z, forward.z),
            asinf(-right.z),
            atan2f(right.y, right.x),
        )
    }
}
