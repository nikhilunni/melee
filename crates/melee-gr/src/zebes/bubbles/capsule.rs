//! grZebes_801DBB60 (0x801DBB60): the bubbles' material item takes hits on
//! one capsule around every held bubble that is not an anchor: the
//! segment between the two farthest apart, widened until it covers each.
use super::{is_anchor, Bubbles, BUBBLES};
use gekko_math::{
    fma::{fmadd, fmadds},
    msl::sqrtf,
};
use hsd_types::Vec3;

/// What grMaterial_801C8DE0 is given, or none when the item's hits stop
/// (grMaterial_801C8E28).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub ends: [Vec3; 2],
    pub radius: f32,
}

/// The pair search keeps only pairs farther apart than this squared.
const COINCIDENT: f32 = 0.0001;
/// A pair's own extent starts at this width.
const START_WIDTH: f32 = 1.0;

fn point(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

/// `(f32)(2.0 * (f64)size)` (fmul by the double 2.0, frsp).
fn diameter(size: f32) -> f32 {
    (2.0 * f64::from(size)) as f32
}

/// `sqrtf` of a squared distance the C sums with fmuls and fadds.
fn plain_length(dx: f32, dy: f32) -> f32 {
    let (dx, dy) = (dx * dx, dy * dy);
    sqrtf(dx + dy)
}

impl Bubbles {
    /// grZebes_801DBB60. `None`: no held bubble, the item's hits stop;
    /// otherwise its hits are on (grMaterial_801C8E08) on this capsule.
    pub fn capsule(&self) -> Option<Capsule> {
        let held = |index: usize| self.entries[index].is_held() && !is_anchor(index);
        let mut count = 0;
        let mut last = 0;
        for index in 0..BUBBLES {
            if held(index) {
                count += 1;
                last = index;
                if count > 2 {
                    break;
                }
            }
        }
        let single = |index: usize| {
            let bubble = &self.entries[index];
            let centre = point(bubble.x, bubble.y);
            Capsule {
                ends: [centre, centre],
                radius: diameter(bubble.size),
            }
        };
        match count {
            0 => return None,
            1 => return Some(single(last)),
            _ => {}
        }
        // The farthest pair (fsubs, fsubs, fmuls, fmuls, fadds), the larger
        // of the two remembered.
        let mut farthest = -1.0;
        let (mut x1, mut y1, mut x2, mut y2) = (0.0, 0.0, 0.0, 0.0);
        for i in 0..BUBBLES {
            if !held(i) {
                continue;
            }
            let first = self.entries[i];
            for j in i + 1..BUBBLES {
                if !held(j) {
                    continue;
                }
                let second = &self.entries[j];
                let dx = first.x - second.x;
                let dy = first.y - second.y;
                let distance = dx * dx + dy * dy;
                if farthest < distance {
                    farthest = distance;
                    x1 = first.x;
                    y1 = first.y;
                    x2 = second.x;
                    y2 = second.y;
                    last = if first.size > second.size { i } else { j };
                }
            }
        }
        if farthest < 0.0 {
            unreachable!("grzebes.c:2257: HSD_ASSERT(0x8D2, 0): no pair of held bubbles");
        }
        if farthest < COINCIDENT {
            return Some(single(last));
        }
        let mut width = START_WIDTH;
        let mut dy = y2 - y1;
        let mut dx = x2 - x1;
        // retail 0x801DBEA4: fmadds, then 1.0 / it.
        let mut inverse = 1.0 / fmadds(dx, dx, dy * dy);
        for index in 0..BUBBLES {
            if !held(index) {
                continue;
            }
            let bubble = &self.entries[index];
            let (bx, by) = (bubble.x, bubble.y);
            let from_y = by - y1;
            let from_x = bx - x1;
            // retail 0x801DBEE8..0x801DBEF0: fmuls, fmadds, fmuls.
            let t = inverse * fmadds(dx, from_x, dy * from_y);
            let squared = if t < 0.0 {
                // retail 0x801DBF00: fmadds.
                fmadds(from_x, from_x, from_y * from_y)
            } else if t > 1.0 {
                let ex = bx - x2;
                let ey = by - y2;
                let (ex, ey) = (ex * ex, ey * ey);
                ex + ey
            } else {
                // retail 0x801DBF28 / 0x801DBF2C: fmadds.
                let px = fmadds(dx, t, x1) - bx;
                let py = fmadds(dy, t, y1) - by;
                let (px, py) = (px * px, py * py);
                px + py
            };
            let distance = sqrtf(squared);
            // retail 0x801DBF94: fmadd (double), frsp.
            let reach = fmadd(2.0, f64::from(bubble.size), f64::from(distance)) as f32;
            if reach <= width {
                continue;
            }
            if t < 0.5 {
                let span = plain_length(bx - x1, by - y1);
                let grown = grown_width(span, width, bubble.size);
                if span > 0.001 {
                    let share = (grown - width) / span;
                    x1 = fmadds(share, bx - x1, x1);
                    y1 = fmadds(share, by - y1, y1);
                }
                let back_y = y1 - y2;
                let back_x = x1 - x2;
                let share = (grown - width) / sqrtf(fmadds(back_x, back_x, back_y * back_y));
                width = grown;
                x2 = fmadds(back_x, share, x2);
                y2 = fmadds(back_y, share, y2);
            } else {
                let span = plain_length(bx - x2, by - y2);
                let grown = grown_width(span, width, bubble.size);
                if span > 0.001 {
                    let share = (grown - width) / span;
                    x2 = fmadds(share, bx - x2, x2);
                    y2 = fmadds(share, by - y2, y2);
                }
                let back_y = y2 - y1;
                let back_x = x2 - x1;
                let share = (grown - width) / sqrtf(fmadds(back_x, back_x, back_y * back_y));
                width = grown;
                x1 = fmadds(back_x, share, x1);
                y1 = fmadds(back_y, share, y1);
            }
            dy = y2 - y1;
            dx = x2 - x1;
            // retail 0x801DC1E4: fmadds.
            inverse = 1.0 / fmadds(dx, dx, dy * dy);
        }
        Some(Capsule {
            ends: [point(x1, y1), point(x2, y2)],
            radius: width,
        })
    }
}

/// `0.5 * (2.0 * size + (span + width))` in double (0x801DC014 fadds,
/// 0x801DC020 fmadd, 0x801DC024 fmul, frsp).
fn grown_width(span: f32, width: f32, size: f32) -> f32 {
    let sum = span + width;
    (0.5 * fmadd(2.0, f64::from(size), f64::from(sum))) as f32
}
