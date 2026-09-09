//! Capsule contact geometry, lbcollision.c. Operations follow the retail audit.
use gekko_math::{
    fma::{fmadd, fmadds, fmsubs},
    msl::sqrtf,
};
use hsd_anim::mtx;
use hsd_types::{Mtx, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub position: Vec3,
    pub overlap: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
}
fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
// retail 80007130/7140, 7160/716C, 7164/7170: y product, x FMA, z FMA.
fn dot(a: Vec3, b: Vec3) -> f32 {
    fmadds(a.z, b.z, fmadds(a.x, b.x, a.y * b.y))
}
// retail 8000750C/751C/752C and 7500/7508/7538.
fn point(start: Vec3, delta: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        fmadds(delta.x, t, start.x),
        fmadds(delta.y, t, start.y),
        fmadds(delta.z, t, start.z),
    )
}
fn near_zero(x: f32) -> bool {
    x < 0.00001 && x > -0.00001
}
#[allow(clippy::manual_clamp)] // Keep the retail ordered comparisons visible beside the audit.
fn parameter(x: f32) -> f32 {
    if x > 1.0 {
        1.0
    } else if x < 0.0 {
        0.0
    } else {
        x
    }
}
/// lbColl_80005EBC (80005EBC): squared distance and closest segment parameter.
fn endpoint_projection(start: Vec3, end: Vec3, target: Vec3) -> (f32, f32) {
    let delta = difference(end, start);
    let offset = difference(start, target);
    // retail 80005F14/5F28, 5F4C/5F50; closest point 5F80/5F8C/5F9C.
    let t = parameter(-dot(delta, offset) / dot(delta, delta));
    let separation = difference(point(start, delta, t), target);
    // retail 80005FB0/5FB4.
    (dot(separation, separation), t)
}
/// lbColl_80006E58 (80006E58): swept hit capsule against a bone-scaled hurt capsule.
#[allow(clippy::manual_range_contains)] // A NaN parameter takes neither retail rejection branch.
pub fn capsule_contact(
    hit: Capsule,
    hurt: Capsule,
    hurt_matrix: &Mtx,
    broadphase_scale: f32,
) -> Option<Contact> {
    // retail 80006E80 fmadds. Axis rejection preserves strict comparisons.
    let radius = fmadds(hurt.radius, broadphase_scale, hit.radius);
    for (a, b, c, d) in [
        (hit.start.x, hit.end.x, hurt.start.x, hurt.end.x),
        (hit.start.y, hit.end.y, hurt.start.y, hurt.end.y),
        (hit.start.z, hit.end.z, hurt.start.z, hurt.end.z),
    ] {
        let (lo, hi) = if a > b { (b, a) } else { (a, b) };
        if (hi + radius < c && hi + radius < d) || (lo - radius > c && lo - radius > d) {
            return None;
        }
    }
    let hd = difference(hit.end, hit.start);
    let axis = difference(hurt.end, hurt.start);
    let offset = difference(hit.start, hurt.start);
    // Unlike the other dot products, hit length is explicitly UNFUSED:
    // 80007138/714C/7150 products; 7148/715C adds.
    let hit_length = hd.z * hd.z + (hd.x * hd.x + hd.y * hd.y);
    let hurt_length = dot(axis, axis);
    let segment_dot = dot(hd, axis);
    let hit_dot = dot(hd, offset);
    let hurt_dot = dot(axis, offset);
    // retail 80007174 fmsubs, segment square already rounded at 7154.
    let denominator = fmsubs(hit_length, hurt_length, segment_dot * segment_dot);
    let (mut ht, mut ut);
    if near_zero(hurt_length) {
        if near_zero(hit_length) {
            ht = 0.0;
            ut = 0.0;
        } else {
            ut = 0.0;
            ht = parameter(-hit_dot / hit_length);
        }
    } else if near_zero(denominator) {
        // C's 0.5 is double: 80007240/724C/7258 fmadd followed by frsp.
        let midpoint = Vec3::new(
            fmadd(0.5, axis.x.into(), hurt.start.x.into()) as f32,
            fmadd(0.5, axis.y.into(), hurt.start.y.into()) as f32,
            fmadd(0.5, axis.z.into(), hurt.start.z.into()) as f32,
        );
        let start_distance = difference(hit.start, midpoint);
        let end_distance = difference(hit.end, midpoint);
        // retail 80007280/7288/728C/7290.
        ht = if dot(start_distance, start_distance) < dot(end_distance, end_distance) {
            0.0
        } else {
            1.0
        };
        let selected = if ht == 0.0 { hit.start } else { hit.end };
        // inlined projection, 80007320/7324 or 73DC/73E0.
        ut = parameter(-dot(axis, difference(hurt.start, selected)) / hurt_length);
    } else {
        // retail 80007420/7424 fmsubs; right products round first.
        ht = fmsubs(segment_dot, hurt_dot, hurt_length * hit_dot) / denominator;
        ut = fmsubs(hit_length, hurt_dot, segment_dot * hit_dot) / denominator;
        if ht > 1.0 || ht < 0.0 || ut > 1.0 || ut < 0.0 {
            let he = if ht < 0.0 { 0.0 } else { 1.0 };
            let ue = if ut < 0.0 { 0.0 } else { 1.0 };
            let (hdist, up) = endpoint_projection(
                hurt.start,
                hurt.end,
                if he == 0.0 { hit.start } else { hit.end },
            );
            let (udist, hp) = endpoint_projection(
                hit.start,
                hit.end,
                if ue == 0.0 { hurt.start } else { hurt.end },
            );
            if hdist < udist {
                ht = he;
                ut = up;
            } else {
                ht = hp;
                ut = ue;
            }
        }
    }
    let hc = point(hit.start, hd, ht);
    let uc = point(hurt.start, axis, ut);
    let separation = difference(hc, uc);
    // retail 80007574/7578; sqrt refinement fnmsub at 759C/75AC/75BC.
    let distance = sqrtf(dot(separation, separation));
    if near_zero(distance) {
        return Some(Contact {
            position: hc,
            overlap: (hit.radius + hurt.radius) - distance,
        });
    }
    let mut inverse = Mtx::default();
    mtx::hsd_mtx_inverse(hurt_matrix, &mut inverse);
    let (mut local_hit, mut local_hurt) = (Vec3::ZERO, Vec3::ZERO);
    mtx::mtx_mult_vec(&inverse, &hc, &mut local_hit);
    mtx::mtx_mult_vec(&inverse, &uc, &mut local_hurt);
    let local = difference(local_hit, local_hurt);
    // retail 8000768C/7690; sqrt refinement 76B4/76C4/76D4.
    let local_distance = sqrtf(dot(local, local));
    let scaled_radius = (hurt.radius * distance) / local_distance;
    let allowed = hit.radius + scaled_radius;
    // retail 8000771C/7730/7744 fmadds.
    let position = point(uc, difference(hc, uc), scaled_radius / distance);
    if allowed < distance {
        None
    } else {
        Some(Contact {
            position,
            overlap: allowed - distance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capsule(start: [f32; 3], end: [f32; 3], radius: f32) -> Capsule {
        Capsule {
            start: Vec3::new(start[0], start[1], start[2]),
            end: Vec3::new(end[0], end[1], end[2]),
            radius,
        }
    }
    #[test]
    fn surface_contact_handles_spheres_parallel_and_crossing_segments() {
        let cases = [
            // Two spheres, and a point against a vertical capsule.
            (
                capsule([3., 0., 0.], [3., 0., 0.], 2.),
                capsule([0., 0., 0.], [0., 0., 0.], 2.),
                [2., 0., 0.],
                1.,
            ),
            (
                capsule([3., 0., 0.], [3., 0., 0.], 2.),
                capsule([0., -2., 0.], [0., 2., 0.], 2.),
                [2., 0., 0.],
                1.,
            ),
            // Parallel tie chooses the hit segment's end; crossing distance is zero.
            (
                capsule([3., -2., 0.], [3., 2., 0.], 2.),
                capsule([0., -2., 0.], [0., 2., 0.], 2.),
                [2., 2., 0.],
                1.,
            ),
            (
                capsule([-2., 0., 0.], [2., 0., 0.], 1.),
                capsule([0., -2., 0.], [0., 2., 0.], 1.),
                [0., 0., 0.],
                2.,
            ),
        ];
        for (hit, hurt, position, overlap) in cases {
            let c = capsule_contact(hit, hurt, &Mtx::IDENTITY, 3.).unwrap();
            assert_eq!(
                [c.position.x, c.position.y, c.position.z].map(f32::to_bits),
                position.map(f32::to_bits)
            );
            assert_eq!(c.overlap.to_bits(), f32::to_bits(overlap));
        }
    }
    #[test]
    fn broadphase_pass_does_not_imply_contact() {
        let hurt = capsule([0., 0., 0.], [0., 0., 0.], 1.);
        for distance in [3., 10.] {
            assert!(capsule_contact(
                capsule([distance, 0., 0.], [distance, 0., 0.], 1.),
                hurt,
                &Mtx::IDENTITY,
                3.
            )
            .is_none());
        }
    }
}
