//! Corneria (`gr/grcorneria.c`, GrCn.dat, StKind 0x07).
//!
//! - Map 3 is the Great Fox the fighters stand on (collision joints 3 and
//!   4 on its root). Once the countdown releases it, it drifts
//!   ([`great_fox::Drift`], grCorneria_801E1970), lowers itself over the
//!   sea ([`great_fox::Descent`], grCorneria_801E2228), fires its cannon
//!   ([`great_fox::Cannon`], grCorneria_801E1348) and runs the Arwing
//!   spawner ([`arwings::Spawner`], grCorneria_801DCE1C). Map 0 carries the
//!   stage's markers and copies its translation (grCorneria_801E1878), so
//!   the camera range and the blast zones move with it.
//! - Maps 8, 9 and 4 are the city, the coast and the sea scrolling past
//!   ([`backgrounds`], grCorneria_801DFC98 / 801DFF20 / 801E01A8).
//! - An Arwing (or Wolfen) is a path (map 1), a ship (map 2, a Wolfen's
//!   map 10) and, for a near pass, the platform on its wing (map 13 with
//!   collision joint 0; map 16 with joint 5 for a Wolfen).
//!
//! Display only and not ported: the lights per background
//! (grCorneria_801E2110), map 7's fog, the sounds and the communicator
//! (map 12, a 1P stage's or the Star Fox taunt's).
pub mod arwings;
pub mod backgrounds;
pub mod great_fox;
pub mod procs;

/// `grCorneria_YakumonoParam` (grcorneria.c:41-72, GrCn.dat
/// `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x00 / +0x04: the drift's wait between its draws.
    pub drift_wait: [f32; 2],
    /// +0x08: the drift's acceleration (and its turn at a limit).
    pub drift_accel: f32,
    /// +0x0C: the drift's top speed on each axis.
    pub drift_speed: f32,
    /// +0x10 / +0x14: how far the drift strays in x and y.
    pub drift_limit: [f32; 2],
    /// +0x18: the cannon's rest is `x1C + HSD_Randi(x18)` ticks.
    pub cannon_rest_range: f32,
    pub cannon_rest: f32,
    /// +0x20: the longest the aimed cannon waits for a fighter.
    pub cannon_aim: f32,
    /// +0x24: the chance a cycle is one big shot.
    pub cannon_burst_chance: f32,
    /// +0x28 / +0x2C: the charge before the big shot or the volley.
    pub cannon_burst_charge: f32,
    pub cannon_volley_charge: f32,
    /// +0x30 / +0x34: the volley's shot count range.
    pub cannon_volley_shots: [f32; 2],
    /// +0x38: the beam damage that destroys the cannons.
    pub cannon_health: f32,
    /// +0x3C / +0x40: the first Arwing's wait.
    pub first_wait: [f32; 2],
    /// +0x44 / +0x48: the wait after an Arwing, drawn every tick one lives.
    pub wait: [f32; 2],
    /// +0x4C: the chance a pass is a Wolfen (`HSD_Randf() > x4C` picks an
    /// Arwing).
    pub wolfen_chance: f32,
    /// +0x68: ticks between a near ship's maneuvers.
    pub maneuver_wait: f32,
    /// +0x70: the ships' and their paths' scale (times the map scale).
    pub ship_scale: f32,
    /// +0x74 / +0x78: a ship may fire every `x74`th tick, one time in
    /// `x78`.
    pub fire_period: i32,
    pub fire_odds: i32,
    /// +0x84: the ship's colour flash when it fires (display).
    pub flash: i32,
    /// +0x88: the backgrounds' scroll per tick.
    pub scroll_speed: f32,
}

/// GrCn.dat's values, for the unit tests.
#[cfg(test)]
pub(crate) fn test_parameters() -> Parameters {
    Parameters {
        drift_wait: [30.0, 180.0],
        drift_accel: 0.002,
        drift_speed: 0.1,
        drift_limit: [20.0, 20.0],
        cannon_rest_range: 1200.0,
        cannon_rest: 1200.0,
        cannon_aim: 180.0,
        cannon_burst_chance: 0.5,
        cannon_burst_charge: 160.0,
        cannon_volley_charge: 160.0,
        cannon_volley_shots: [8.0, 12.0],
        cannon_health: 200.0,
        first_wait: [600.0, 600.0],
        wait: [960.0, 1800.0],
        wolfen_chance: 0.15,
        maneuver_wait: 120.0,
        ship_scale: 0.5,
        fire_period: 16,
        fire_odds: 8,
        flash: 390640,
        scroll_speed: 2.0,
    }
}
