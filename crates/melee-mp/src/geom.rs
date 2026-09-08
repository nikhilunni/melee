//! Segment intersection primitives and the line-to-line remap from
//! `mplib.c`. Pure functions; no map state.

use gekko_math::fma::{fmadd, fmsub};
use gekko_math::msl::{fabs, fabsf};
use hsd_anim::mtx::vec_normalize;
use hsd_types::Vec3;

/// `SQ(x)` from the decomp macros: `x * x` in the operand's type.
#[inline]
pub(crate) fn sq(x: f32) -> f32 {
    x * x
}

/// The outward normal the map hands back for a line from `(x0, y0)` to
/// `(x1, y1)`: `(-(y1 - y0), x1 - x0, 0)` through `PSVECNormalize`. This
/// exact sequence appears in `mpLib_8004DD90_Floor` (`mplib.c:1166`), the
/// `mpCheck*` sweeps, and `mpLineGetNormal` (`mplib.c:4514`).
pub(crate) fn line_normal(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec3 {
    let mut n = Vec3::new(-(y1 - y0), x1 - x0, 0.0);
    let src = n;
    vec_normalize(&src, &mut n);
    n
}

/// `mpRemap2d` (static, `mplib.c:1066`): remap point `p` from line `a` to
/// line `b`, moving it by the same amount the line moved this frame. Used by
/// the `*Remap` sweeps and `mpGetSpeed`. Returns `(x_out, y_out)`.
///
/// The clamp keeps the C's `if / else if` shape rather than `f64::clamp` so
/// the transcription reads against the source.
#[allow(clippy::too_many_arguments, clippy::manual_clamp)]
pub fn remap_2d(
    ax0: f32,
    ay0: f32,
    ax1: f32,
    ay1: f32,
    bx0: f32,
    by0: f32,
    bx1: f32,
    by1: f32,
    px: f32,
    py: f32,
) -> (f32, f32) {
    let dx: f64 = f64::from(ax1 - ax0);
    let dy: f64 = f64::from(ay1 - ay0);
    let f30: f32 = px - ax0;
    let f29: f32 = py - ay0;
    // retail 0x8004DCA0, 0x8004DCB0: fmul + fmadd.
    let dist2: f64 = fmadd(dy, dy, dx * dx);
    if fabs(dist2) > 0.0001 {
        // how far along line a is point p
        // retail 0x8004DCE8, 0x8004DCF0: fmul + fmadd.
        let mut t: f64 = fmadd(dy, f64::from(f29), dx * f64::from(f30)) / dist2;
        if t > 1.0 {
            t = 1.0;
        } else if t < 0.0 {
            t = 0.0;
        }
        // retail 0x8004DD30/34/38/3C: fmadd (two per coordinate), then frsp.
        let start_x = fmadd(1.0 - t, f64::from(bx0 - ax0), f64::from(px));
        let start_y = fmadd(1.0 - t, f64::from(by0 - ay0), f64::from(py));
        let x = fmadd(t, f64::from(bx1 - ax1), start_x);
        let y = fmadd(t, f64::from(by1 - ay1), start_y);
        (x as f32, y as f32)
    } else {
        // The C uses ax0 in both terms here; transcribed as is.
        (
            px + (bx0 - ax0) + (bx1 - ax0),
            py + (by0 - ay0) + (by1 - ay0),
        )
    }
}

/// `mpLineIntersection` (static, retail `0x8004E97C`, `mplib.c:1398`):
/// direction-dependent intersection of map segment `a` with sweep segment
/// `b`. Returns the point on `a`, or `None` when `b` misses `a`, runs
/// parallel to it, or approaches from behind.
#[allow(clippy::too_many_arguments)]
pub fn line_intersection(
    a0x: f32,
    a0y: f32,
    a1x: f32,
    a1y: f32,
    b0x: f32,
    b0y: f32,
    b1x: f32,
    b1y: f32,
) -> Option<(f32, f32)> {
    let mut b1_below_a = false;
    let mut b2_above_a = false;

    // b entirely left/right of a
    if a0x <= a1x {
        if (b0x < a0x && b1x < a0x) || (a1x < b0x && a1x < b1x) {
            return None;
        }
    } else if (b0x < a1x && b1x < a1x) || (a0x < b0x && a0x < b1x) {
        return None;
    }

    // b entirely above/below a
    if a0y <= a1y {
        if (b0y < a0y && b1y < a0y) || (a1y < b0y && a1y < b1y) {
            return None;
        }
    } else if (b0y < a1y && b1y < a1y) || (a0y < b0y && a0y < b1y) {
        return None;
    }

    let ah: f64 = f64::from(a1y - a0y);
    let d0x: f64 = f64::from(b0x - a0x);
    let aw: f64 = f64::from(a1x - a0x);
    let d0y: f64 = f64::from(b0y - a0y);
    // retail 0x8004EA60/64: fmul + fmsub.
    let hs_b0_a: f64 = fmsub(aw, d0y, ah * d0x);

    if hs_b0_a < 0.0 {
        if hs_b0_a < -0.1 {
            return None;
        }
        b1_below_a = true;
    }

    let d1x: f64 = f64::from(b1x - a1x);
    let d1y: f64 = f64::from(b1y - a1y);

    // retail 0x8004EA94/98: fmul + fmsub.
    let hs_b1_a: f64 = fmsub(aw, d1y, ah * d1x);
    if hs_b1_a > 0.0 {
        if hs_b1_a > 0.1 {
            return None;
        }
        b2_above_a = true;
    }

    // check if a and b are colinear
    if hs_b0_a == 0.0 && hs_b1_a == 0.0 {
        return None;
    }

    // retail 0x8004EAD8/DC: fmul + fmsub.
    let det: f64 = fmsub(d0x, d1y, d0y * d1x);
    if det < hs_b0_a {
        if det < hs_b1_a {
            return None;
        }
    } else if det > hs_b0_a && det > hs_b1_a {
        return None;
    }

    let bw: f64 = f64::from(b1x - b0x);
    let bh: f64 = f64::from(b1y - b0y);
    if !((bw == 0.0 && bh == 0.0) || (b1_below_a && b2_above_a) || (hs_b0_a >= 0.0 && b2_above_a)) {
        // retail 0x8004EB50/58: fmul + fmsub.
        let area: f64 = fmsub(bw, ah, bh * aw);

        // `ABS(area) > 0.0001F`: the constant is a float promoted to double.
        if fabs(area) > f64::from(0.0001f32) {
            // barycentric weight
            // retail 0x8004EB80/88: fmul + fmsub, then fdiv.
            let t: f64 = fmsub(bw, d0y, bh * d0x) / area;
            let (int_x, int_y) = if t > 0.0 {
                if t < 1.0 {
                    // retail 0x8004EBA4/A8: fmadd, then frsp.
                    (
                        fmadd(aw, t, f64::from(a0x)) as f32,
                        fmadd(ah, t, f64::from(a0y)) as f32,
                    )
                } else {
                    (a1x, a1y)
                }
            } else {
                (a0x, a0y)
            };
            return Some((int_x, int_y));
        }
    }
    None
}

/// `mpLineIntersectionH` (retail `0x8004EBF8`, `mplib.c:1507`): intersection
/// of horizontal map segment `a` (from `(a0x, a0y)` to `(a1x, a0y)`) with
/// sweep segment `b`.
#[allow(clippy::too_many_arguments)]
pub fn line_intersection_h(
    a0x: f32,
    a0y: f32,
    a1x: f32,
    b0x: f32,
    b0y: f32,
    b1x: f32,
    b1y: f32,
) -> Option<(f32, f32)> {
    let min_ax: f32;
    let max_ax: f32;
    if a0x < a1x {
        if (b0x < a0x && b1x < a0x) || (a1x < b0x && a1x < b1x) {
            return None;
        }
        if f64::from(b0y - a0y) < -0.0001 || f64::from(b1y - a0y) > 0.0001 {
            return None;
        }
        min_ax = a0x;
        max_ax = a1x;
    } else {
        if (b0x < a1x && b1x < a1x) || (a0x < b0x && a0x < b1x) {
            return None;
        }
        if f64::from(b1y - a0y) < -0.0001 || f64::from(b0y - a0y) > 0.0001 {
            return None;
        }
        min_ax = a1x;
        max_ax = a0x;
    }
    let dby: f64 = f64::from(b1y - b0y);
    let dbx: f64 = f64::from(b1x - b0x);
    if fabs(dby) < 0.0001 {
        return None;
    }
    // retail 0x8004ECE8/F4: fdiv + fmadd, then frsp at 0x8004ED48.
    let mut new_x: f64 = fmadd(dbx / dby, f64::from(a0y - b0y), f64::from(b0x));
    let dx: f64 = new_x - f64::from(min_ax);
    if dx < 0.0 {
        if dx < -0.1 {
            return None;
        }
        new_x = f64::from(min_ax);
    }
    if new_x - f64::from(max_ax) > 0.0 {
        if new_x - f64::from(max_ax) > 0.1 {
            return None;
        }
        new_x = f64::from(max_ax);
    }
    Some((new_x as f32, a0y))
}

/// `mpLineIntersectionV` (retail `0x80050068`, `mplib.c:2234`): intersection
/// of vertical map segment `a` (from `(a0x, a0y)` to `(a0x, a1y)`) with
/// sweep segment `b`.
#[allow(clippy::too_many_arguments)]
pub fn line_intersection_v(
    a0x: f32,
    a0y: f32,
    a1y: f32,
    b0x: f32,
    b0y: f32,
    b1x: f32,
    b1y: f32,
) -> Option<(f32, f32)> {
    let min_ay: f32;
    let max_ay: f32;
    if a0y < a1y {
        if (b0y < a0y && b1y < a0y) || (a1y < b0y && a1y < b1y) {
            return None;
        }
        if f64::from(b1x - a0x) < -0.0001 || f64::from(b0x - a0x) > 0.0001 {
            return None;
        }
        min_ay = a0y;
        max_ay = a1y;
    } else {
        if (b0y < a1y && b1y < a1y) || (a0y < b0y && a0y < b1y) {
            return None;
        }
        if f64::from(b0x - a0x) < -0.0001 || f64::from(b1x - a0x) > 0.0001 {
            return None;
        }
        min_ay = a1y;
        max_ay = a0y;
    }
    let dby: f64 = f64::from(b1y - b0y);
    let dbx: f64 = f64::from(b1x - b0x);
    if fabs(dbx) < 0.0001 {
        return None;
    }
    // retail 0x80050158/64: fdiv + fmadd, then frsp at 0x800501B8.
    let mut new_y: f64 = fmadd(dby / dbx, f64::from(a0x - b0x), f64::from(b0y));
    let mut dy: f64 = new_y - f64::from(min_ay);
    if dy < 0.0 {
        if dy < -0.1 {
            return None;
        }
        new_y = f64::from(min_ay);
    }
    dy = new_y - f64::from(max_ay);
    if dy > 0.0 {
        if dy > 0.1 {
            return None;
        }
        new_y = f64::from(max_ay);
    }
    Some((a0x, new_y as f32))
}

/// `ABS(a - b) > 0.0001` as the sweeps write it: a float difference
/// compared against a double constant.
#[inline]
pub(crate) fn differs_by_more_than_1e4(a: f32, b: f32) -> bool {
    f64::from(fabsf(a - b)) > 0.0001
}
