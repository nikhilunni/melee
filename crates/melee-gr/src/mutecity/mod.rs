//! Mute City (`gr/grmutecity.c`, GrMc.dat, StKind 0x0A). Map 0x1E is the
//! city and the track, its animation the platform's 3600-frame route, which
//! a command script (grMc_803E34E0) follows: it stops the route where the
//! platform sets down on the track, swaps the platform's collision, moves
//! the camera and blast-zone limits, and raises the track's own floor
//! (collision joint 4, placed every tick where the car splines cross the
//! stage's plane). Map 0x1D is the platform, map 2 the thirty F-Zero cars
//! (grFZeroCar_801CAFBC), which race along the two splines with a stage
//! material item each while they pass through the stage.
pub mod cars;
pub mod procs;
pub mod script;
pub mod track;

/// `grMc_YakumonoParam` (grmutecity.c:303-317, GrMc.dat `yakumono_param`):
/// the floats the cars read. +0x00..+0x0C are color-overlay scripts and
/// line dynamics, read only by drawing and buried fighters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x2C: the damage past which a car smokes (generator 0x116).
    pub smoke_damage: f32,
    /// +0x30: the damage past which it explodes.
    pub explode_damage: f32,
    /// +0x34 / +0x38 / +0x3C: a car's top speed (spline parameter per
    /// tick); the five leaders', the five trailers'.
    pub top_speed: f32,
    pub leader_top_speed: f32,
    pub trailer_top_speed: f32,
    /// +0x40: the acceleration of a car with nobody close ahead.
    pub acceleration: f32,
    /// +0x44: a lane change.
    pub lane_step: f32,
    /// +0x48: the drift back towards the middle lanes.
    pub lane_centering: f32,
    /// +0x4C: the braking behind a close car.
    pub braking: f32,
}
