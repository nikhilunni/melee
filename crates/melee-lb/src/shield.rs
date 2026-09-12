//! Projectile deflection geometry, lbcollision.c and lbvector.c.
use gekko_math::{
    fma::{fmadds, fmsubs, fnmsubs},
    msl::{sqrtf, sqrtf_accurate},
};
use hsd_types::{Mtx, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct ShieldDeflection {
    pub normal: Vec3,
    pub angle: f32,
}

/// lbVector_AngleXY (8000D790): unfused squared lengths, four Newton steps,
/// then the fused XY dot (8000D8BC). Ordered clamps retain NaN behavior.
#[allow(clippy::manual_clamp)]
fn angle_xy(a: Vec3, b: Vec3) -> f32 {
    let length = sqrtf_accurate(a.x * a.x + a.y * a.y) * sqrtf_accurate(b.x * b.x + b.y * b.y);
    if length == 0.0 {
        return 0.0;
    }
    let mut cosine = fmadds(a.x, b.x, a.y * b.y) / length;
    if cosine > 1.0 {
        cosine = 1.0;
    }
    if cosine < -1.0 {
        cosine = -1.0;
    }
    crate::trigf::acosf(cosine)
}

/// lbColl_800077A0 (800077A0), reached through lbColl_80007DD8 (80007DD8).
/// The unused surface-position output is omitted; normal and incidence retain
/// the retail arithmetic. Positions are world space; matrix scales the shield.
pub fn deflection(
    center: Vec3,
    matrix: &Mtx,
    start: Vec3,
    end: Vec3,
    radius: f32,
    projectile_radius: f32,
) -> ShieldDeflection {
    let delta = Vec3::new(end.x - start.x, end.y - start.y, end.z - start.z);
    if delta == Vec3::ZERO {
        return ShieldDeflection {
            normal: Vec3::ZERO,
            angle: std::f32::consts::PI,
        };
    }
    let mut edge = Vec3::ZERO;
    let mut origin = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(matrix, &Vec3::new(radius, 0.0, 0.0), &mut edge);
    hsd_anim::mtx::mtx_mult_vec(matrix, &Vec3::ZERO, &mut origin);
    let scale = Vec3::new(edge.x - origin.x, edge.y - origin.y, edge.z - origin.z);
    // retail 80007898/9C: y square, then x/z FMAs; inline three-step sqrt.
    let distance = sqrtf(fmadds(
        scale.z,
        scale.z,
        fmadds(scale.x, scale.x, scale.y * scale.y),
    ));
    let total_radius = distance + projectile_radius;
    let offset = Vec3::new(start.x - center.x, start.y - center.y, start.z - center.z);
    // retail 80007900..24: unfused squared travel length, z added last.
    let travel = (delta.x * delta.x + delta.y * delta.y) + delta.z * delta.z;
    let fraction = if travel < 0.00001 && travel > -0.00001 {
        0.0
    } else {
        // retail 80007984/90, 998/A4: preserve the separately doubled axes.
        let offset_squared = fmadds(
            offset.z,
            offset.z,
            fmadds(offset.x, offset.x, offset.y * offset.y),
        );
        let linear = fmadds(
            2.0 * delta.z,
            offset.z,
            fmadds(2.0 * delta.x, offset.x, (2.0 * delta.y) * offset.y),
        );
        // retail 800079A0/A8/AC: fnmsubs, rounded multiply, then fmsubs.
        let mut discriminant = fmsubs(
            linear,
            linear,
            (4.0 * travel) * fnmsubs(total_radius, total_radius, offset_squared),
        );
        if discriminant < 0.0 {
            discriminant = 0.0;
        }
        (-linear - sqrtf(discriminant)) / (2.0 * travel)
    };
    // retail 80007A48/60/78: fused point followed by separate subtraction.
    let radial = Vec3::new(
        fmadds(fraction, delta.x, start.x) - center.x,
        fmadds(fraction, delta.y, start.y) - center.y,
        fmadds(fraction, delta.z, start.z) - center.z,
    );
    let mut normal = Vec3::ZERO;
    hsd_anim::mtx::vec_normalize(&radial, &mut normal);
    ShieldDeflection {
        normal,
        angle: angle_xy(normal, delta),
    }
}
