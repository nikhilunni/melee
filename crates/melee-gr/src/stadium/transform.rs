//! The transformation controller (map 2, `grStadium_801D4548`): the base
//! arena sinks, a form rises in its place, and later the base returns.
use super::Parameters;
use crate::rand::range;
use gekko_math::HsdRng;

/// A form's map id (`grStadium_GroundVars::xDE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Fire = 3,
    Grass = 4,
    Base = 5,
    Rock = 6,
    Water = 9,
}
impl Form {
    pub fn map(self) -> u8 {
        self as u8
    }
    pub fn from_map(map: i16) -> Option<Self> {
        Some(match map {
            3 => Self::Fire,
            4 => Self::Grass,
            5 => Self::Base,
            6 => Self::Rock,
            9 => Self::Water,
            _ => return None,
        })
    }
}

/// `grStadium_GroundVars::xDC`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Counting down the current form's duration.
    Waiting = 0,
}
impl Phase {
    pub fn from_saved(value: i16) -> Option<Self> {
        (value == 0).then_some(Self::Waiting)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transformation {
    /// Map 2's xC4_b0: set at creation, cleared by the deferred start
    /// callback fn_801D13C8; while set the controller does not run.
    pub waiting_for_start: bool,
    pub phase: Phase,
    /// xD8: the phase's counter.
    pub timer: i32,
    /// xDE.
    pub form: Form,
    /// xE0; `None` is retail's -1.
    pub previous: Option<Form>,
    /// xE2.
    pub before_previous: Option<Form>,
}

impl Transformation {
    /// `grStadium_801D13E0` (0x801D13E0): the first base-form duration.
    pub fn new(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            waiting_for_start: true,
            phase: Phase::Waiting,
            timer: range(rng, parameters.base_frames),
            form: Form::Base,
            previous: None,
            before_previous: None,
        }
    }

    /// `grStadium_801D4548` (0x801D4548), outside Training mode.
    pub fn tick(&mut self) {
        match self.phase {
            Phase::Waiting => {
                // Signed post-decrement: a timer of zero waits one more tick.
                let old = self.timer;
                self.timer -= 1;
                if old < 0 {
                    unimplemented!("grpstadium.c:2061-2100: choose the next form");
                }
            }
        }
    }
}
