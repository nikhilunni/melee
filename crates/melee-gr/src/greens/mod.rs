//! Green Greens (`gr/grgreens.c`, GrGr.dat, StKind 0x11).
//!
//! Map 5 is Whispy Woods' controller (map 4 his face, animated in step):
//! he idles, blows wind across one half of the stage (a fighter device,
//! ftCo_800C06E8 type 9, and a directional dynamics field) or drops
//! apples. Map 6 holds the thirty block slots of two 3x5 grids on either
//! side; blocks fall in from the top on a timer, stack, and break when hit
//! (bomb blocks explode). Each live block is a map-1 or map-3 GObj with a
//! stage material item carrying its hurt capsule.
pub mod blocks;
pub mod procs;
pub mod whispy;

/// `grGreens_YakumonoParam` (grgreens.c, GrGr.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0 / +0x4: the wait before the next falling block, `randrange`.
    pub block_wait: [i32; 2],
    /// +0x18, +0x14, +0x10, +0xC, +0x8: a column's weight by the row of its
    /// topmost block (0 the bottom).
    pub column_weights: [i32; 5],
    /// +0x1C: an empty column's weight.
    pub empty_column_weight: i32,
    /// +0x20: one block in this many is a bomb.
    pub bomb_odds: i32,
    /// +0x24 / +0x28: a block's and a bomb's hurt radius, times the map
    /// scale.
    pub block_radius: i32,
    pub bomb_radius: i32,
    /// +0x2C / +0x30: a falling block's top speed and acceleration.
    pub fall_speed_max: f32,
    pub fall_acceleration: f32,
    /// +0x34 / +0x38: Whispy's idle time, `randrange(x38, x34)`.
    pub idle_wait: [i32; 2],
    /// +0x3C: the wind's push.
    pub wind_speed: f32,
    /// +0x40..+0x4C: the wind's rectangle (mirrored for the left side).
    pub wind_left: f32,
    pub wind_right: f32,
    pub wind_top: f32,
    pub wind_bottom: f32,
    /// +0x50: the inhale's loops.
    pub inhale_loops: f32,
    /// +0x54: the blow's loops; +0x58: the loops after which he stops
    /// early when the fighters moved to his other side.
    pub blow_loops: f32,
    pub blow_turn_loops: f32,
    /// +0x5C / +0x60: the apples, `randrange(x60, x5C - 1)` plus one.
    pub apple_count: [i32; 2],
    /// +0x64 / +0x68: the first apple's tick and the ticks between them.
    pub apple_start: i32,
    pub apple_interval: i32,
    /// +0x6C / +0x70: an apple's height range.
    pub apple_height: [f32; 2],
    /// +0x74 / +0x78: it_802EE200's odds (an apple that stays; a healing
    /// one, items on only).
    pub apple_kept: f32,
    pub apple_heal: f32,
}

/// grgreens.c's `randrange(min, max)`: equal ends draw nothing.
pub fn randrange(min: i32, max: i32, rng: &mut gekko_math::HsdRng) -> i32 {
    crate::rand::range(rng, [min, max])
}
