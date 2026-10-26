//! Rainbow Cruise (`gr/grrcruise.c`, GrRc.dat, StKind 0x0B).
//!
//! Two map-3 GObjs: the first plays the cruise's camera path (a looping
//! animation, hidden), the second each tick projects that path onto the
//! stage's plane and translates every other map by the result, so the
//! ship, the sky blocks and the swinging platforms scroll past a still
//! camera. Map 1 is the ship: its deck (joint 5) tilts toward the side the
//! weight is on, seventeen cloud blocks fall a while after a fighter stands
//! on them, and twenty blocks appear when they come into the camera bounds
//! and go when they leave. Map 4 holds the three swinging platforms, map 6
//! the flag and the players' spawn points. The ship's flag is blown by a
//! wandering directional field, as Peach's Castle's are.
pub mod parts;
pub mod procs;

/// `grRCruise_YakumonoParam` (grrcruise.c, GrRc.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0: the deck's turn per unit of weight difference; +0x4 its top
    /// speed; +0x8 the speed it rights itself at.
    pub tilt_gain: f32,
    pub tilt_max: f32,
    pub tilt_return: f32,
    /// +0xC: ticks a stopped swing waits; +0x10 / +0x14 its blink's length
    /// and period; +0x18 / +0x1C the same after a full swing.
    pub swing_wait: i32,
    pub swing_blink_time: i32,
    pub swing_blink: i32,
    pub swing_end_time: i32,
    pub swing_end_blink: i32,
    /// +0x20: ticks a cloud holds a fighter before it shakes; +0x24 how long
    /// it shakes; +0x28 the shake's period; +0x2C / +0x30 its range.
    pub cloud_wait: i32,
    pub cloud_shake_time: i32,
    pub cloud_shake_period: i32,
    pub cloud_shake: [f32; 2],
    /// +0x34 / +0x38: a falling cloud's acceleration and top speed.
    pub cloud_fall_acceleration: f32,
    pub cloud_fall_max: f32,
    /// +0x3C: ticks a fallen cloud stays away; +0x40 / +0x44 its blink's
    /// length and period on return.
    pub cloud_away: i32,
    pub cloud_blink_time: i32,
    pub cloud_blink: i32,
}
