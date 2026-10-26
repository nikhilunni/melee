//! Brinstar (`gr/grzebes.c`, GrZe.dat, StKind 0x06).
//!
//! - Map 8 is the acid ([`acid`]): it waits, then moves to the next level
//!   of the `grZe_YakumonoParam` table, accelerating and braking; its
//!   surface (`stage_info.x724`) hurts fighters below it
//!   (grZebes_801DCBFC) and splashes where a fighter enters.
//! - Map 6 is the upper part on the left, its top platform (bone 14) and
//!   the breakable plate under it; map 7 the right part with its plate
//!   ([`plates`]). A plate wobbles under hits, breaks past a damage total,
//!   stays away a while and grows back.
//! - Between the two parts hangs a chain of organic bubbles ([`bubbles`],
//!   grZe_8049F170): slots 0 and 6 are anchors on the two parts, 1..5 the
//!   chain the top platform's collision line follows (joint 0), 7..19
//!   loose bubbles that drift up into gaps. Any hit on their shared
//!   material item pops one. When the chain has broken and every chain
//!   bubble is gone, the top platform sinks, waits and comes back up,
//!   growing a new chain ([`island`]).
//! - Map 4 rides the top platform and plays its two animations in turn;
//!   map 1 reacts to the acid's height; both are display only, but map 4
//!   draws its waits.
//! - Below the parts, lines of the two models cross the acid: each tick a
//!   crossing draws and splashes ([`splash`], grZakoGenerator_801CA43C).
pub mod acid;
pub mod bubbles;
pub mod island;
pub mod plates;
pub mod procs;
pub mod scenery;
pub mod splash;

/// One row of the acid's level table (`grZe_AcidLevelEntry`, +0xA0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcidLevel {
    /// +0: ticks before the acid moves, added to the drawn delay.
    pub base: i16,
    /// +2 / +4: the delay's range (`rand_range(max, min)`).
    pub delay_min: i16,
    pub delay_max: i16,
    /// +6: the level the acid moves to, before the spread.
    pub level: i16,
}

impl AcidLevel {
    /// An all-zero row ends the table.
    pub fn is_end(&self) -> bool {
        self.base == 0 && self.delay_min == 0 && self.delay_max == 0 && self.level == 0
    }
}

/// The level table's rows (`xA0_entries[30]`).
pub const ACID_LEVELS: usize = 30;

/// `grZe_YakumonoParam` (grzebes.c:83-119, GrZe.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x00 / +0x04: the sunk top platform's wait, `x00 + (x04 - x00) * rand`.
    pub island_wait: [f32; 2],
    /// +0x08 / +0x0C: the wait between two bubble pulses.
    pub pulse_wait: [f32; 2],
    /// +0x10: ticks between two steps of the chain's regrowth.
    pub regrowth_step: i32,
    /// +0x2C: the acid's hit (an archive offset to an lbColl_80008D30_arg1
    /// record, the pointer grZebes_801DCBFC hands the collision device).
    pub acid_hit: u32,
    /// +0x30: a plate's largest wobble.
    pub plate_max_offset: f32,
    /// +0x34: a plate's top wobble speed.
    pub plate_max_speed: f32,
    /// +0x38: the wobble's pull back once past `plate_rest_offset`.
    pub plate_spring: f32,
    /// +0x3C: the wobble's damping (`speed *= 1 - x3C`).
    pub plate_damping: f32,
    /// +0x40 / +0x44: below these speed and offset the plate settles.
    pub plate_rest_speed: f32,
    pub plate_rest_offset: f32,
    /// +0x48: ticks between two points of a plate's damage healing.
    pub plate_heal_interval: f32,
    /// +0x4C: the damage that breaks a plate (also its crack's scale).
    pub plate_break_damage: f32,
    /// +0x50 / +0x54: a broken plate's wait.
    pub plate_wait: [f32; 2],
    /// +0x58 / +0x5C: a regrown chain bubble's size.
    pub regrowth_size: [f32; 2],
    /// +0x60 / +0x64: ticks between two loose bubbles (floats the asm
    /// converts with fctiwz).
    pub bubble_wait: [f32; 2],
    /// +0x68: a bubble's growth per tick toward its size.
    pub growth: f32,
    /// +0x6C / +0x70: two bubbles attract between half these times their
    /// sizes' sum.
    pub attract_far: f32,
    pub attract_near: f32,
    /// +0x74: the chain's link length; half of it times the sizes' sum is
    /// where two bubbles push apart.
    pub link: f32,
    /// +0x78: the attraction's strength.
    pub attraction: f32,
    /// +0x80: the most a chain link pulls in a tick.
    pub max_pull: f32,
    /// +0x84: every bubble's sink per tick.
    pub sink: f32,
    /// +0x88: a loose bubble's top speed.
    pub max_speed: f32,
    /// +0x8C / +0x90: the acid's highest and lowest level.
    pub acid_top: f32,
    pub acid_bottom: f32,
    /// +0x94: the spread added to a level, `x94 * rand`.
    pub acid_spread: f32,
    /// +0x98: the acid's top speed; +0x9C its acceleration.
    pub acid_max_speed: f32,
    pub acid_acceleration: f32,
    pub acid_levels: [AcidLevel; ACID_LEVELS],
}

/// grZebes_801DA0C4 / grZebes_801DA254 recolour the fog and a light with
/// the acid's height: display only.
pub const ACID_SURFACE_OFFSET: f32 = -55.0;
