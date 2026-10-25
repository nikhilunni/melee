//! Kongo Jungle, gr/grkongo.c (NTSC 1.02): the rope bridge of fifteen
//! planks that sag under fighters, the two swinging platform ends, the
//! rotating barrel cannon below, and the random events (logs down the
//! waterfall, background passers-by, a Klaptrap riding a path).
//!
//! Map 10 is the main stage. Its controller (grKongo_801D557C) steps the
//! planks (grKongo_801D7134), offsets the bridge's collision lines by each
//! plank's sag, swings the platform ends (grKongo_801D77E0) and runs the
//! event timer (grKongo_801D7BBC). Map 5 is the barrel (grKongo_801D577C).
pub mod procs;
pub mod planks;
pub mod barrel;
pub mod events;
pub mod objects;
pub mod rider;
pub use barrel::Barrel;
pub use planks::{Bridge, Swing};

use gekko_math::HsdRng;

/// `grKongo_YakumonoParam` (grkongo.static.h, GrKg.dat `yakumono_param`).
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0 / +0x4: bounds of the event timer.
    pub event_wait: [f32; 2],
    /// +0x8: the Klaptrap event's weight.
    pub klap_weight: f32,
    /// +0xC: the falling box's weight (items switched on only).
    pub box_weight: f32,
    /// +0x10: the stage objects' weight, split by +0x18 between the logs
    /// and the background passers-by.
    pub object_weight: f32,
    /// +0x14: the path rider's speed (grKongo_801D6668).
    pub path_speed: f32,
    /// +0x18: the logs' share of the object weight.
    pub log_share: f32,
    /// +0x1C: the chance to keep an event that repeats the last one.
    pub repeat_chance: f32,
    /// +0x20 / +0x24: bounds of the frames a caught Klaptrap stays.
    pub hold_frames: [f32; 2],
    /// +0x28: the barrel's catch radius.
    pub catch_radius: f32,
    /// +0x2C / +0x30: bounds of the barrel's rest between spins.
    pub rest_frames: [f32; 2],
    /// +0x34: the spin's acceleration, degrees.
    pub spin_acceleration: f32,
    /// +0x38: the spin's top speed, degrees.
    pub spin_speed: f32,
    /// +0x3C / +0x40: bounds of the spin's length.
    pub spin_frames: [f32; 2],
    /// +0x44..+0x52: the weights of the eight rest directions, from 135
    /// degrees down to -180.
    pub direction_weights: [i16; 8],
    /// +0x54 / +0x58 (s32): bounds of the arrow's dim wait.
    pub dim_frames: [i32; 2],
    /// +0x5C: the arrow animation's rate change per frame.
    pub rate_step: f32,
    /// +0x60: the arrow animation's top rate.
    pub rate_top: f32,
    /// +0x64 / +0x68 (s32): bounds of the arrow's bright wait.
    pub bright_frames: [i32; 2],
    /// +0x6C..+0x80: the launch hit (damage, angle, growth, set, base,
    /// element), as words over grKg_803B7FB0.
    pub launch_hit: [u32; 6],
    /// +0x84: the material script the barrel plays when it catches.
    pub catch_material: u32,
    /// +0x88 / +0x8C: a log leaves after this many landings or frames.
    pub log_landings: f32,
    pub log_frames: f32,
    /// +0x90..+0xB8: the bridge (see [`planks`]).
    pub plank_limit: f32,
    pub plank_restore: f32,
    pub plank_speed_limit: f32,
    pub plank_slack: f32,
    pub plank_pull: f32,
    pub plank_kick: f32,
    pub end_tilt: f32,
    pub end_angle_limit: f32,
    pub end_jolt: f32,
    pub swing_damping: f32,
    pub swing_limit: f32,
}

/// `MTXDegToRad`'s 0.017453292f.
pub const DEG_TO_RAD: f32 = 0.017_453_292;

/// grKongo_801D523C's creation order.
pub const MAP_ORDER: [u8; 6] = [0, 10, 5, 3, 6, 4];
/// The main stage and the barrel.
pub const MAIN: u8 = 10;
pub const BARREL: u8 = 5;

/// Kongo Jungle's stage state.
#[derive(Clone, Debug)]
pub struct Kongo {
    pub rider: Option<rider::Rider>,
    pub parameters: Parameters,
    pub bridge: Bridge,
    /// Map 10's gp+C4..+D8: the left and right platform ends.
    pub swings: [Swing; 2],
    /// Map 10's gp+E4 (s16): frames to the next event; -1 before the match
    /// starts (fn_801D542C draws it).
    pub event_timer: i16,
    /// Map 10's gp+E6 (s16): the last event's kind.
    pub last_event: i16,
    pub barrel: Barrel,
    /// Ground_801C4D70's stage_info copy of the barrel's position and angle
    /// (Ground_801C4DA0 reads it for the catch).
    pub barrel_position: hsd_types::Vec3,
    pub barrel_angle: f32,
    /// ftCo_800C0764 registered the barrel's catch at the match start.
    pub catch_active: bool,
    /// The logs afloat (maps 1 and 2).
    pub logs: [Option<objects::Log>; 2],
    /// The passers-by on screen (maps 7, 8, 9).
    pub passersby: [bool; 3],
}

/// The maps grKongo_801D7BBC creates during the match.
pub const EVENT_MAPS: [u8; 5] = [1, 2, 7, 8, 9];

impl Kongo {
    /// Whether event map `map` has a live GObj.
    pub fn live(&self, map: u8) -> bool {
        match map {
            11 => self.rider.is_some(),
            1 | 2 => self.logs[usize::from(map - 1)].is_some(),
            7..=9 => self.passersby[usize::from(map - 7)],
            _ => false,
        }
    }
}

impl Kongo {
    /// fn_801D542C (0x801D542C), deferred to the match start by
    /// Ground_801C10B8: the bridge's landing callback (the scene's) and the
    /// first event wait. retail 0x801D5468: fmadds (x4 - x0) * r + x0.
    pub fn start(&mut self, rng: &mut HsdRng) {
        let r = rng.randf();
        let [low, high] = self.parameters.event_wait;
        self.event_timer = gekko_math::msl::fctiwz(gekko_math::fma::fmadds(high - low, r, low)) as i16;
    }
}

#[cfg(test)]
mod tests;
