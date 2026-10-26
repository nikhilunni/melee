//! Venom (`gr/grvenom.c`, GrVe.dat, StKind 0x16).
//!
//! - Map 5 is the Great Fox the fighters stand on (collision joints 0 and
//!   1 on its root, which bobs with its looping animations); its proc also
//!   runs the Arwing spawner ([`arwings::Spawner`], grVenom_8020362C).
//! - Map 7 is the flight through Venom: one long looping animation whose
//!   frame drives the sequence's flags ([`sequence`]): which Arwings may
//!   come, and the tunnel's floor (collision joint 2 on map 9, from frame
//!   4400 to 6100).
//! - An Arwing (or Wolfen) is three dynamic maps ([`arwings`]): its flight
//!   path (map 2, one of eleven animations), the ship on that path (map 3
//!   for an Arwing, 6 for a Wolfen) and, for the near passes (types 1..7),
//!   the platform riding its wing (map 10, collision joint 4; map 13,
//!   joint 3, for a Wolfen). The far passes (types 8..11) fire lasers at
//!   the fighters (It_Kind_Arwing_Laser).
//! - Maps 0 and 4 are scenery; map 0 carries the stage's markers.
//!
//! Display only and not ported: the lights and fog (grVenom_80204B88,
//! Ground_801C4E70's fog range, map 7's lights restarting with its loop),
//! map 7's visibility culling, the ship's colour overlay when it fires, the
//! sounds and the shadow test (grVenom_80206D7C).
pub mod arwings;
pub mod procs;
pub mod sequence;

/// `grVenom_YakumonoParam` (grvenom.c:38-50, GrVe.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x00 / +0x04: the first Arwing's wait (grVenom_80203B18's
    /// `inlineA0`).
    pub first_wait: [f32; 2],
    /// +0x08 / +0x0C: the wait after an Arwing, drawn every tick one lives.
    pub wait: [f32; 2],
    /// +0x10: the chance a pass is a Wolfen (`HSD_Randf() > x10` picks an
    /// Arwing).
    pub wolfen_chance: f32,
    /// +0x2C: ticks between a near ship's maneuvers.
    pub maneuver_wait: f32,
    /// +0x34: the ships' and platforms' scale (times the map scale).
    pub ship_scale: f32,
}
