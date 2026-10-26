//! Two circles swept along their last frame's motion in the xy plane
//! (lbColl_800067F8), as a fighter's coin sphere meets a coin. Operations
//! follow the retail asm (80005FC0, 800067F8..80006E54).
use gekko_math::fma::{fmadd, fmadds, fmsubs};
use hsd_types::Vec3;

/// "@275" / "@276": lbcollision.c's approximatelyZero bounds.
const NEAR_ZERO: f32 = 0.00001;

fn approximately_zero(value: f32) -> bool {
    value < NEAR_ZERO && value > -NEAR_ZERO
}

/// The clamp every parameter takes: `> 1.0` and `< 0.0` against doubles,
/// which for a float are the same tests.
#[allow(clippy::manual_clamp)] // retail's two tests, in order
fn clamp_unit(value: f32) -> f32 {
    if value > 1.0 {
        1.0
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

/// lbColl_80005FC0 (80005FC0): the parameter along `start`..`end` nearest
/// `point` (clamped), and the squared xy distance from there.
fn nearest_on_segment(start: Vec3, end: Vec3, point: Vec3) -> (f32, f32) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    // 80006008 fmuls, 80006018 fmadds.
    let length = fmadds(dx, dx, dy * dy);
    // 80006030 fmuls, 80006034 fmadds.
    let along = fmadds(dx, start.x - point.x, dy * (start.y - point.y));
    let t = clamp_unit(-along / length);
    // 80006064 / 80006074: fmadds.
    let y = fmadds(dy, t, start.y) - point.y;
    let x = fmadds(dx, t, start.x) - point.x;
    // 80006084 fmuls, 80006088 fmadds.
    (t, fmadds(x, x, y * y))
}

/// The parameter along `start`..`end` nearest `point` in three dimensions
/// (800067F8's inline at 80006B6C / 80006C2C), clamped.
fn nearest_in_space(start: Vec3, end: Vec3, point: Vec3) -> f32 {
    let d = Vec3::new(end.x - start.x, end.y - start.y, end.z - start.z);
    // 80006B8C fmuls, 80006B9C / 80006BAC fmadds.
    let length = fmadds(d.z, d.z, fmadds(d.x, d.x, d.y * d.y));
    // 80006BC4 fmuls, 80006BD4 / 80006BD8 fmadds.
    let along = fmadds(
        d.z,
        start.z - point.z,
        fmadds(d.x, start.x - point.x, d.y * (start.y - point.y)),
    );
    clamp_unit(-along / length)
}

/// The two nearest points of the sweeps when they touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweptContact {
    /// e: on the first circle's sweep.
    pub first: Vec3,
    /// f: on the second circle's sweep (the coin's, where its pickup
    /// effect plays).
    pub second: Vec3,
}

/// lbColl_800067F8 (800067F8): whether a circle of `first_radius` swept
/// from `first_from` to `first_to` comes within reach of one of
/// `second_radius` swept from `second_from` to `second_to`. The nearest
/// points are written only once the bounding tests pass; retail's callers
/// read them only on contact.
pub fn swept_circles_touch(
    first_from: Vec3,
    first_to: Vec3,
    second_from: Vec3,
    second_to: Vec3,
    first_radius: f32,
    second_radius: f32,
) -> Option<SweptContact> {
    // 80006820 fadds.
    let reach = first_radius + second_radius;
    let (a, b, c, d) = (first_from, first_to, second_from, second_to);
    // The bounding tests on each axis (800068B0..800069E0).
    let outside = |a: f32, b: f32, c: f32, d: f32| {
        if a > b {
            let high = a + reach;
            if high < c && high < d {
                return true;
            }
            let low = b - reach;
            low > c && low > d
        } else {
            let low = a - reach;
            if low > c && low > d {
                return true;
            }
            let high = b + reach;
            high < c && high < d
        }
    };
    if outside(a.x, b.x, c.x, d.x) || outside(a.y, b.y, c.y, d.y) {
        return None;
    }
    // 800069E4..80006A44.
    let ba = (b.x - a.x, b.y - a.y);
    let dc = (d.x - c.x, d.y - c.y);
    let ac = (a.x - c.x, a.y - c.y);
    let ba_dc = fmadds(ba.0, dc.0, ba.1 * dc.1);
    let dc_dc = fmadds(dc.0, dc.0, dc.1 * dc.1);
    let ba_ba = fmadds(ba.0, ba.0, ba.1 * ba.1);
    let dc_ac = fmadds(dc.0, ac.0, dc.1 * ac.1);
    let ba_ac = fmadds(ba.0, ac.0, ba.1 * ac.1);
    let determinant = fmsubs(ba_ba, dc_dc, ba_dc * ba_dc);
    let (along_first, along_second) = if approximately_zero(dc_dc) {
        if approximately_zero(ba_ba) {
            (0.0, 0.0)
        } else {
            (clamp_unit(-ba_ac / ba_ba), 0.0)
        }
    } else if approximately_zero(determinant) {
        // Parallel: the end of the first sweep nearer the second's middle.
        // 80006B10 / 80006B18: fmadd in double with 0.5, then frsp.
        let middle_y = fmadd(0.5, f64::from(dc.1), f64::from(c.y)) as f32;
        let middle_x = fmadd(0.5, f64::from(dc.0), f64::from(c.x)) as f32;
        let (ay, by) = (a.y - middle_y, b.y - middle_y);
        let (ax, bx) = (a.x - middle_x, b.x - middle_x);
        if fmadds(ax, ax, ay * ay) < fmadds(bx, bx, by * by) {
            (0.0, nearest_in_space(c, d, a))
        } else {
            (1.0, nearest_in_space(c, d, b))
        }
    } else {
        // 80006CCC..80006CE4: fmuls, fmsubs, fdivs.
        let first = fmsubs(ba_dc, dc_ac, dc_dc * ba_ac) / determinant;
        let second = fmsubs(ba_ba, dc_ac, ba_dc * ba_ac) / determinant;
        // 80006CE8..80006D0C: four ordered tests (a NaN passes them all).
        #[allow(clippy::manual_range_contains)]
        let beyond = first > 1.0 || first < 0.0 || second > 1.0 || second < 0.0;
        if beyond {
            let (end_first, distance_first, along_second) = if first < 0.0 {
                let (t, distance) = nearest_on_segment(c, d, a);
                (0.0, distance, t)
            } else {
                let (t, distance) = nearest_on_segment(c, d, b);
                (1.0, distance, t)
            };
            let (end_second, (t, distance_second)) = if second < 0.0 {
                (0.0, nearest_on_segment(a, b, c))
            } else {
                (1.0, nearest_on_segment(a, b, d))
            };
            if distance_first < distance_second {
                (end_first, along_second)
            } else {
                (t, end_second)
            }
        } else {
            (first, second)
        }
    };
    // 80006DB4..80006DD8: fmadds.
    let first = Vec3::new(
        fmadds(ba.0, along_first, a.x),
        fmadds(ba.1, along_first, a.y),
        0.0,
    );
    let second = Vec3::new(
        fmadds(dc.0, along_second, c.x),
        fmadds(dc.1, along_second, c.y),
        0.0,
    );
    let dy = first.y - second.y;
    let dx = first.x - second.x;
    // 80006DC0 fmuls; 80006E00 fmuls, 80006E04 fmadds; 80006E08 fcmpo, bge.
    let distance = fmadds(dx, dx, dy * dy);
    if reach * reach < distance {
        return None;
    }
    Some(SweptContact { first, second })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32) -> Vec3 {
        Vec3::new(x, y, 0.0)
    }

    #[test]
    fn still_circles_touch_within_their_summed_radii() {
        let cases = [
            (at(0.0, 0.0), at(3.0, 0.0), true),
            (at(0.0, 0.0), at(3.0, 0.5), false),
            (at(0.0, 0.0), at(0.0, -2.5), true),
            (at(0.0, 0.0), at(10.0, 10.0), false),
        ];
        for (first, second, touch) in cases {
            let contact = swept_circles_touch(first, first, second, second, 2.0, 1.0);
            assert_eq!(contact.is_some(), touch, "{first:?} {second:?}");
        }
    }

    #[test]
    fn a_sweep_through_a_still_circle_touches_it() {
        let contact = swept_circles_touch(at(-10.0, 0.0), at(10.0, 0.0), at(0.0, 0.5), at(0.0, 0.5), 0.25, 0.5)
            .expect("the sweep passes through");
        assert_eq!(contact.first, at(0.0, 0.0));
        assert_eq!(contact.second, at(0.0, 0.5));
    }

    #[test]
    fn parallel_sweeps_measure_from_the_nearer_end() {
        // Both sweeps run along x, a unit apart.
        let contact =
            swept_circles_touch(at(0.0, 0.0), at(4.0, 0.0), at(5.0, 1.0), at(9.0, 1.0), 1.0, 1.0)
                .expect("ends within reach");
        assert_eq!(contact.first, at(4.0, 0.0));
        assert_eq!(contact.second, at(5.0, 1.0));
    }
}
