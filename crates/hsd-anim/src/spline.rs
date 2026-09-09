//! Linear branches of HSD spline.c. Cubic paths remain outside this port.
use gekko_math::fma::fmadds;
use hsd_archive::desc::spline::LinearSpline;
use hsd_types::Vec3;

/// splArcLengthGetParameter (0x80378F38), then splGetSplinePoint (0x80378A94).
/// Keep both divisions and the multiply back into control-point space: their
/// rounding survives even for a straight segment.
pub fn linear_point(spline: &LinearSpline, distance: f32) -> Vec3 {
    let spans = (spline.points.len() - 1) as f32;
    let u = if distance <= 0.0 {
        0.0
    } else if distance >= 1.0 {
        1.0
    } else {
        let mut i = 0;
        while spline.lengths[i + 1] < distance {
            i += 1;
        }
        let fraction = (distance - spline.lengths[i]) / (spline.lengths[i + 1] - spline.lengths[i]);
        (fraction + i as f32) / spans
    };
    if u == 1.0 {
        let p = spline.points.last().unwrap();
        return Vec3::new(p.x, p.y, p.z);
    }
    let segment = u * spans;
    let i = segment as usize;
    let t = segment - i as f32;
    let a = spline.points[i];
    let b = spline.points[i + 1];
    // Retail 80378B58/6C/80: fmadds after separately rounded differences.
    Vec3::new(
        fmadds(t, b.x - a.x, a.x),
        fmadds(t, b.y - a.y, a.y),
        fmadds(t, b.z - a.z, a.z),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsd_archive::desc::Vec3 as Point;
    #[test]
    fn nonuniform_path_uses_length_fraction_and_clamps_endpoints() {
        let spline = LinearSpline {
            points: vec![
                Point {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point {
                    x: 2.0,
                    y: 6.0,
                    z: 0.0,
                },
            ],
            lengths: vec![0.0, 0.25, 1.0],
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
}
