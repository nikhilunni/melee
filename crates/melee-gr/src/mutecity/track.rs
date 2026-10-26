//! The track's floor under the stage (collision joint 4): where the two
//! car splines cross the stage's plane (grMuteCity_801F094C), and its
//! lines laid between those points (grMuteCity_801F0D20).
use super::cars::{point, to_world, wrapped};
use gekko_math::fma::fmadds;
use hsd_archive::desc::spline::Spline;
use hsd_types::{Mtx, Vec3};

/// The track floor's collision joint and its lines: the right end, the
/// floor's middle, the left end.
pub const JOINT: i32 = 4;
pub const RIGHT_LINE: i32 = 0x31;
pub const MIDDLE_LINE: i32 = 0x33;
pub const LEFT_LINE: i32 = 0x35;
/// The search's step along a spline.
const STEP: f32 = 0.0004;

/// The world point a spline reaches at `t` (the wrapped read, then
/// lb_8000B1CC through the spline's joint).
fn world(spline: &Spline, matrix: &Mtx, t: f32) -> Vec3 {
    to_world(matrix, wrapped(|u| point(spline, u), t))
}

/// grMuteCity_801F094C (0x801F094C): from `t`, step the parameter along
/// the spline (by 0.0004, towards the stage's plane, wrapping in [0, 1])
/// until two successive points straddle z = 0; the new parameter and the
/// last point.
pub fn cross_plane(spline: &Spline, matrix: &Mtx, mut t: f32) -> (f32, Vec3) {
    let mut here = world(spline, matrix, t);
    // 801F0A70..80: fsubs, compared with 0.0 in double, plus one.
    let mut back = t - STEP;
    if f64::from(back) < 0.0 {
        back += 1.0;
    }
    let mut behind = world(spline, matrix, back);
    let towards = if here.z > behind.z { STEP } else { -STEP };
    let away = -towards;
    while (behind.z > 0.0 && here.z > 0.0) || (behind.z < 0.0 && here.z < 0.0) {
        t += if here.z < 0.0 { towards } else { away };
        if t > 1.0 {
            t -= 1.0;
        } else if t < 0.0 {
            t += 1.0;
        }
        behind = here;
        here = world(spline, matrix, t);
    }
    (t, here)
}

/// xD0 b23: the shape of the floor's end lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ends {
    /// 0: five units out, flat.
    Flat,
    /// 1: thirty units up, walls.
    Walls,
    /// 2: ten units out along the floor, turned 15 degrees up.
    Ramps,
}
impl Ends {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 3 {
            1 => Self::Walls,
            2 => Self::Ramps,
            _ => Self::Flat,
        }
    }
}

/// What laying the floor writes to the collision.
pub trait TrackCollision {
    /// mpLib_80057424: the joint's vertices as their previous positions.
    fn snapshot(&mut self);
    /// mpLineSetPos.
    fn set_line(&mut self, line: i32, x0: f32, y0: f32, x1: f32, y1: f32);
    /// mpLib_80055E24 then mpLib_8005667C.
    fn refresh(&mut self);
}

/// `Pi` and 15 degrees (grmutecity.c's @554 and @555).
const PI: f32 = std::f32::consts::PI;
const TILT: f32 = 0.261_799_4;

/// grMuteCity_801F0D20 (0x801F0D20): the floor between the right
/// spline's crossing `right` and the left's `left`.
pub fn lay_floor(right: Vec3, left: Vec3, ends: Ends, collision: &mut impl TrackCollision) {
    collision.snapshot();
    match ends {
        Ends::Walls => {
            collision.set_line(RIGHT_LINE, right.x, 30.0 + right.y, right.x, right.y);
            collision.set_line(LEFT_LINE, left.x, left.y, left.x, 30.0 + left.y);
        }
        Ends::Ramps => {
            let angle = melee_lb::trigf::atan2f(left.y - right.y, left.x - right.x);
            let forward = TILT + angle;
            let back = (PI + angle) - TILT;
            // 801F0E0C / 801F0E2C / 801F0E48 / 801F0E64: fmadds of 10.
            let (sin_back, cos_back) = (gekko_math::msl::sinf(back), gekko_math::msl::cosf(back));
            collision.set_line(
                RIGHT_LINE,
                fmadds(10.0, cos_back, right.x),
                fmadds(10.0, sin_back, right.y),
                right.x,
                right.y,
            );
            let (sin_forward, cos_forward) =
                (gekko_math::msl::sinf(forward), gekko_math::msl::cosf(forward));
            collision.set_line(
                LEFT_LINE,
                left.x,
                left.y,
                fmadds(10.0, cos_forward, left.x),
                fmadds(10.0, sin_forward, left.y),
            );
        }
        Ends::Flat => {
            collision.set_line(RIGHT_LINE, right.x - 5.0, right.y, right.x, right.y);
            collision.set_line(LEFT_LINE, left.x, left.y, 5.0 + left.x, left.y);
        }
    }
    // lbVector_Diff, then 801F0F04..14: fmadds at 0.35 and 0.65.
    let d = Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z);
    collision.set_line(
        MIDDLE_LINE,
        fmadds(0.35, d.x, right.x),
        fmadds(0.35, d.y, right.y),
        fmadds(0.65, d.x, right.x),
        fmadds(0.65, d.y, right.y),
    );
    collision.refresh();
}
