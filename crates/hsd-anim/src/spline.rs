//! HSD spline.c: points along a spline and its arc-length parameter.
//! Every multiply-add follows retail splGetSplinePoint (0x80378A94) and
//! splArcLengthGetParameter (0x80378F38).
use gekko_math::fma::{fmadds, fmsubs, fnmsubs};
use hsd_archive::desc::spline::{Spline, SplineKind};
use hsd_types::Vec3;

/// spline.c's literals (.sdata2 @13..@19, @91, @101, @178, @179).
const ONE_SIXTH: f32 = 0.166_666_67;
const SIMPSON_STEPS: f32 = 0.125;
const NEAR_ZERO: f32 = -0.001;
const TOLERANCE: f32 = 0.000_01;

fn point(p: &hsd_archive::desc::Vec3) -> Vec3 {
    Vec3::new(p.x, p.y, p.z)
}

/// The four-weight blend every cubic branch ends with: retail multiplies
/// the second point first, then adds the first, third and fourth with
/// fmadds (e.g. 0x80378BC8..0x80378BE0).
fn blend(cp: &[hsd_archive::desc::Vec3], w: [f32; 4]) -> Vec3 {
    let axis = |a: fn(&hsd_archive::desc::Vec3) -> f32| {
        let v = a(&cp[1]) * w[1];
        let v = fmadds(a(&cp[0]), w[0], v);
        let v = fmadds(a(&cp[2]), w[2], v);
        fmadds(a(&cp[3]), w[3], v)
    };
    Vec3::new(axis(|p| p.x), axis(|p| p.y), axis(|p| p.z))
}

/// Only the linear paths and Jungle Japes' B-spline PATH joint have
/// retail witnesses. Other retail curve branches stay fail-closed.
fn require_witnessed_curve(spline: &Spline) {
    if matches!(spline.kind, SplineKind::Bezier | SplineKind::Cardinal) {
        unimplemented!(
            "splGetSplinePoint / splArcLengthGetParameter (spline.c:82-135,178): {:?} needs a retail witness",
            spline.kind
        );
    }
}

/// splGetSplinePoint (0x80378A94): the point at parameter `u` in [0, 1];
/// `None` outside it (the C leaves its output unwritten).
pub fn spline_point(spline: &Spline, u: f32) -> Option<Vec3> {
    require_witnessed_curve(spline);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let last = spline.knots as i32 - 1;
    if u >= 1.0 {
        // 80378DD8: the end of the curve.
        let index = last as usize;
        return Some(match spline.kind {
            SplineKind::Linear => point(&spline.points[index]),
            SplineKind::Bezier => point(&spline.points[index * 3]),
            SplineKind::BSpline => {
                // splGetBSplinePoint(cv[n - 1], 1.0), folded: weights 0,
                // 1/6, 4/6 (3 * 1 + 1, times 1/6), 1/6.
                let cp = &spline.points[index - 1..];
                let w2 = ONE_SIXTH * fmadds(3.0, 1.0 + (-1.0 + 1.0), 1.0);
                let axis = |a: fn(&hsd_archive::desc::Vec3) -> f32| {
                    let v = a(&cp[1]) * ONE_SIXTH;
                    let v = fmadds(a(&cp[0]), 0.0, v);
                    let v = fmadds(a(&cp[2]), w2, v);
                    fmadds(a(&cp[3]), ONE_SIXTH, v)
                };
                Vec3::new(axis(|p| p.x), axis(|p| p.y), axis(|p| p.z))
            }
            SplineKind::Cardinal => point(&spline.points[index + 1]),
        });
    }
    // 80378AC8..80378B14: fsubs of the int's float, fmuls, fctiwz, extsh.
    let scaled = u * last as f32;
    let index = gekko_math::msl::fctiwz(scaled) as i16;
    let t = scaled - index as f32;
    let index = index as usize;
    Some(match spline.kind {
        SplineKind::Linear => {
            let a = spline.points[index];
            let b = spline.points[index + 1];
            // 80378B58/6C/80: fmadds after separately rounded differences.
            Vec3::new(
                fmadds(t, b.x - a.x, a.x),
                fmadds(t, b.y - a.y, a.y),
                fmadds(t, b.z - a.z, a.z),
            )
        }
        SplineKind::Bezier => {
            // 80378B8C: splGetBezierPoint.
            let u_1 = 1.0 - t;
            let u2 = t * t;
            let u_12 = u_1 * u_1;
            let bez1 = (3.0 * t) * u_12;
            let bez0 = u_12 * u_1;
            let bez2 = (3.0 * u2) * u_1;
            let bez3 = u2 * t;
            blend(&spline.points[index * 3..], [bez0, bez1, bez2, bez3])
        }
        SplineKind::BSpline => {
            // 80378C34: splGetBSplinePoint.
            let u2 = t * t;
            let u_1 = 1.0 - t;
            let u3 = u2 * t;
            // 80378C5C fmsubs: 3 * u3 - 6 * u2.
            let inner = fmsubs(3.0, u3, 6.0 * u2);
            let b0 = u_1 * (u_1 * (ONE_SIXTH * u_1));
            let b1 = ONE_SIXTH * (4.0 + inner);
            let b3 = ONE_SIXTH * u3;
            // 80378C9C fmadds: 3 * (u + (-u3 + u2)) + 1.
            let b2 = ONE_SIXTH * fmadds(3.0, t + (-u3 + u2), 1.0);
            blend(&spline.points[index..], [b0, b1, b2, b3])
        }
        SplineKind::Cardinal => {
            // 80378D04: splGetCardinalPoint.
            let tension = spline.tension;
            let u2 = t * t;
            let u3 = u2 * t;
            // 80378D34 fnmsubs: 3 - 2 * tension.
            let three_less = fnmsubs(2.0, tension, 3.0);
            // 80378D3C fmadds: 2 * u2 - u3.
            let car0 = tension * (fmadds(2.0, u2, -u3) - t);
            // 80378D40 fmadds: (2 - tension) * u3 + (tension - 3) * u2.
            let car1 = 1.0 + fmadds(2.0 - tension, u3, (tension - 3.0) * u2);
            // 80378D64 / 80378D70 fmadds.
            let car2 = fmadds(
                tension,
                t,
                fmadds(tension - 2.0, u3, three_less * u2),
            );
            let car3 = tension * (u3 - u2);
            blend(&spline.points[index..], [car0, car1, car2, car3])
        }
    })
}

/// The integrand of the arc length: the square root of a segment's
/// speed-squared polynomial at `t`, nudged to zero just below it
/// (retail 0x803790B4..0x803790DC, inlined at each use).
fn speed(c: &[f32; 5], t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let v = c[1] * t3;
    let v = fmadds(c[0], t4, v);
    let v = fmadds(c[2], t2, v);
    let v = fmadds(c[3], t, v);
    let mut result = c[4] + v;
    if result < 0.0 && result > NEAR_ZERO {
        result = 0.0;
    }
    gekko_math::msl::sqrtf(result)
}

/// splArcLengthGetParameter (0x80378F38): the parameter at `distance`, a
/// share of the spline's length in [0, 1].
pub fn arc_length_parameter(spline: &Spline, distance: f32) -> f32 {
    require_witnessed_curve(spline);
    if distance <= 0.0 {
        return 0.0;
    }
    if distance >= 1.0 {
        return 1.0;
    }
    let mut index = 0;
    while spline.lengths[index + 1] < distance {
        index += 1;
    }
    let result = match spline.kind {
        SplineKind::Linear => {
            (distance - spline.lengths[index])
                / (spline.lengths[index + 1] - spline.lengths[index])
        }
        _ => {
            // Bisection on Simpson's rule over the segment (eight panels).
            let coefficients = &spline.polynomials[index];
            let mut remaining = spline.total_length * (distance - spline.lengths[index]);
            let mut start = 0.0f32;
            let mut end = 1.0f32;
            let mut middle = 0.0f32;
            loop {
                let mut gap = start - end;
                if gap < 0.0 {
                    gap = -gap;
                }
                if gap < TOLERANCE {
                    break;
                }
                middle = (start + end) * 0.5;
                let dx = (middle - start) * SIMPSON_STEPS;
                let mut sum = 0.0f32;
                let mut t = start + dx;
                for step in 2..=8 {
                    // 803790E0 / 80379134: fmadds by 4 (even) or 2 (odd).
                    let weight = if step & 1 == 0 { 4.0 } else { 2.0 };
                    sum = fmadds(weight, speed(coefficients, t), sum);
                    t += dx;
                }
                let at_start = speed(coefficients, start);
                let at_middle = speed(coefficients, middle);
                // 803791E4..803791F0: fadds, fadds, fmuls, fdivs.
                let simpson = (dx * ((sum + at_start) + at_middle)) / 3.0;
                if remaining < TOLERANCE + simpson {
                    end = middle;
                } else {
                    start = middle;
                    remaining -= simpson;
                }
            }
            middle
        }
    };
    // 8037922C..8037926C: (result + idx) / ((float) numcv - 1).
    (result + index as f32) / (spline.knots as f32 - 1.0)
}

/// splArcLengthPoint (0x803792C8): the point `distance` of the way along.
pub fn arc_length_point(spline: &Spline, distance: f32) -> Option<Vec3> {
    spline_point(spline, arc_length_parameter(spline, distance))
}

/// splArcLengthGetParameter then splGetSplinePoint for a linear path.
pub fn linear_point(spline: &Spline, distance: f32) -> Vec3 {
    arc_length_point(spline, distance).expect("a parameter in [0, 1]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsd_archive::desc::Vec3 as Point;
    fn p(x: f32, y: f32) -> Point {
        Point { x, y, z: 0.0 }
    }
    #[test]
    fn nonuniform_path_uses_length_fraction_and_clamps_endpoints() {
        let spline = Spline {
            kind: SplineKind::Linear,
            knots: 3,
            tension: 0.0,
            points: vec![p(0.0, 0.0), p(2.0, 0.0), p(2.0, 6.0)],
            total_length: 8.0,
            lengths: vec![0.0, 0.25, 1.0],
            polynomials: Vec::new(),
        };
        for (distance, expected) in [
            (-1.0, Vec3::ZERO),
            (0.125, Vec3::new(1.0, 0.0, 0.0)),
            (0.25, Vec3::new(2.0, 0.0, 0.0)),
            (0.625, Vec3::new(2.0, 3.0, 0.0)),
            (2.0, Vec3::new(2.0, 6.0, 0.0)),
        ] {
            assert_eq!(linear_point(&spline, distance), expected);
        }
    }
    fn unwitnessed_curve(kind: SplineKind) -> Spline {
        Spline {
            kind,
            knots: 2,
            tension: 0.0,
            points: vec![p(0.0, 0.0); 4],
            total_length: 3.0,
            lengths: vec![0.0, 1.0],
            polynomials: vec![[0.0; 5]],
        }
    }

    // WASI aborts on panic. wasm-check.sh also runs these four tests as
    // expected-trap subprocesses and requires the specific guard message.
    #[test]
    #[cfg_attr(
        not(target_family = "wasm"),
        should_panic(expected = "Bezier needs a retail witness")
    )]
    #[cfg_attr(
        target_family = "wasm",
        ignore = "wasm-check.sh runs this as an expected-trap subprocess"
    )]
    fn bezier_point_requires_a_retail_witness() {
        spline_point(&unwitnessed_curve(SplineKind::Bezier), 0.5);
    }
    #[test]
    #[cfg_attr(
        not(target_family = "wasm"),
        should_panic(expected = "Bezier needs a retail witness")
    )]
    #[cfg_attr(
        target_family = "wasm",
        ignore = "wasm-check.sh runs this as an expected-trap subprocess"
    )]
    fn bezier_arc_length_requires_a_retail_witness() {
        arc_length_parameter(&unwitnessed_curve(SplineKind::Bezier), 0.5);
    }
    #[test]
    #[cfg_attr(
        not(target_family = "wasm"),
        should_panic(expected = "Cardinal needs a retail witness")
    )]
    #[cfg_attr(
        target_family = "wasm",
        ignore = "wasm-check.sh runs this as an expected-trap subprocess"
    )]
    fn cardinal_point_requires_a_retail_witness() {
        spline_point(&unwitnessed_curve(SplineKind::Cardinal), 0.5);
    }
    #[test]
    #[cfg_attr(
        not(target_family = "wasm"),
        should_panic(expected = "Cardinal needs a retail witness")
    )]
    #[cfg_attr(
        target_family = "wasm",
        ignore = "wasm-check.sh runs this as an expected-trap subprocess"
    )]
    fn cardinal_arc_length_requires_a_retail_witness() {
        arc_length_parameter(&unwitnessed_curve(SplineKind::Cardinal), 0.5);
    }
}

/// lbShadow_8000E9F0 (lbshadow.c): witnessed B-spline tangent branch.
/// No endpoint adjustment occurs in retail's B-spline branch; the endpoint
/// requires an extra control point outside this admitted path interval.
pub fn bspline_tangent(spline: &Spline, u: f32) -> Vec3 {
    assert_eq!(
        spline.kind,
        SplineKind::BSpline,
        "lbShadow_8000E9F0: unwitnessed curve tangent"
    );
    assert!(
        (0.0..1.0).contains(&u),
        "lbShadow_8000E9F0: unwitnessed tangent endpoint"
    );
    let scaled = u * (spline.knots as i32 - 1) as f32;
    let index = gekko_math::msl::fctiwz(scaled) as i16;
    let t = scaled - f32::from(index);
    let square = t * t;
    // retail8000EBC0: fmsubs; EBE8: fmadds; EC00..EC5C: cp1-first blend.
    let b1 = 0.5 * fmsubs(3.0, square, 4.0 * t);
    let one_minus = 1.0 - t;
    let b0 = one_minus * (-0.5 * one_minus);
    let b2 = 0.5 * (1.0 + fmadds(-3.0, square, 2.0 * t));
    let b3 = 0.5 * square;
    blend(&spline.points[index as usize..], [b0, b1, b2, b3])
}

#[cfg(test)]
mod tangent_tests {
    use super::*;
    fn straight() -> Spline {
        Spline {
            kind: SplineKind::BSpline,
            knots: 2,
            tension: 0.0,
            points: (0..4)
                .map(|i| hsd_archive::desc::Vec3 {
                    x: (i * 2) as f32,
                    y: 0.0,
                    z: 0.0,
                })
                .collect(),
            total_length: 2.0,
            lengths: vec![0.0, 1.0],
            polynomials: vec![[0.0; 5]],
        }
    }
    #[test]
    fn cubic_tangent_preserves_a_straight_control_polygon() {
        for parameter in [0.0, 0.25, 0.5, 0.75] {
            assert_eq!(
                bspline_tangent(&straight(), parameter),
                Vec3::new(2.0, 0.0, 0.0)
            );
        }
    }
    #[test]
    #[cfg(not(target_family = "wasm"))]
    #[should_panic(expected = "unwitnessed tangent endpoint")]
    fn tangent_endpoint_requires_the_unrecorded_extra_control_point() {
        bspline_tangent(&straight(), 1.0);
    }
}
