//! Fourside (`gr/grfourside.c`, GrFs.dat, StKind 0x12): the crane whose
//! iron beam is a platform (map 1), the helicopter in the background
//! (map 3) and the UFO that lands as a platform over one of three
//! buildings (map 5). Each keeps its state in its Ground's `u` union; the
//! three read each other's.
pub mod crane;
pub mod procs;
pub mod ufo;
use gekko_math::HsdRng;

/// `grFourside_YakumonoParam` (grfourside.c:28-50). `heli_stay_time` (+0x08)
/// is never read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    pub heli_wait: i32,
    pub heli_wait_add: i32,
    pub crane_wait: i32,
    pub crane_wait_add: i32,
    pub crane_iron_wait: i32,
    pub crane_iron_wait_add: i32,
    /// `crane_iron_up_min`, `crane_iron_up_max`.
    pub crane_iron_up: [f32; 2],
    /// `crane_iron_down_min`, `crane_iron_down_max`.
    pub crane_iron_down: [f32; 2],
    pub crane_iron_speed: f32,
    pub crane_iron_stop_acceleration: f32,
    pub ufo_wait: i32,
    /// `ufo_cs_offs`: how far above the camera's top the UFO starts
    /// being framed.
    pub ufo_camera_offset: f32,
    pub ufo_stay_time: i32,
    pub ufo_stay_time_add: i32,
    /// +0x44: failed tries after which the UFO uses the second odds.
    pub ufo_challenge: i16,
    /// +0x46: 1 in this many while a fighter stands on the main building.
    pub ufo_standing_odds: i16,
    /// +0x48: 1 in this many once `ufo_challenge` tries have failed.
    pub ufo_challenged_odds: i16,
}

/// The `base + (add != 0 ? HSD_Randi(add) : 0)` waits grfourside.c draws.
pub(crate) fn wait(base: i32, add: i32, rng: &mut HsdRng) -> i32 {
    base + if add != 0 { rng.randi(add) } else { 0 }
}

/// The helicopter's phase (`fourside2.x0`, map 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeliPhase {
    Waiting = 0,
    Arriving = 1,
    Hovering = 2,
    Leaving = 3,
}
impl HeliPhase {
    pub fn from_saved(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Waiting,
            1 => Self::Arriving,
            2 => Self::Hovering,
            3 => Self::Leaving,
            _ => return None,
        })
    }
}

/// The helicopter (`grFourside_GroundVars2` on map 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heli {
    pub phase: HeliPhase,
    /// x1: the arrival came below the camera's top (its sound played).
    pub descended: bool,
    /// x4.
    pub timer: i32,
}

/// What one helicopter step asks of its model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeliStep {
    None,
    /// HSD_JObjClearFlagsAll(hidden), grAnime_801C8138(gobj, 3, 0).
    Arrive,
    /// grAnime_801C8138(gobj, 3, 2).
    Leave,
    /// HSD_JObjSetFlagsAll(hidden).
    Hide,
}

impl Heli {
    /// grFourside_801F3C40 (0x801F3C40): hidden, one wait drawn.
    pub fn create(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            phase: HeliPhase::Waiting,
            descended: false,
            timer: wait(parameters.heli_wait, parameters.heli_wait_add, rng),
        }
    }

    /// grFourside_801F3F10 (0x801F3F10): the helicopter is in the way of
    /// the UFO's first two buildings.
    pub fn busy(&self) -> bool {
        (self.phase == HeliPhase::Arriving && !self.descended)
            || (self.phase == HeliPhase::Leaving && self.timer <= 700)
    }

    /// grFourside_801F3CC8 (0x801F3CC8). `ufo_away` is the UFO's
    /// `x1 == 2 || x0 == 0` (read through the `fourside2` view of map 5);
    /// `below_top` tests the helicopter's bone 2 against the camera's top
    /// plus 50; `ended` is grAnime_801C83D0(gobj, 0, 7).
    pub fn tick(
        &mut self,
        parameters: &Parameters,
        ufo_away: bool,
        below_top: impl FnOnce() -> bool,
        ended: bool,
        rng: &mut HsdRng,
    ) -> HeliStep {
        match self.phase {
            HeliPhase::Waiting => {
                if self.timer <= 0 {
                    if ufo_away {
                        self.descended = false;
                        self.phase = HeliPhase::Arriving;
                        return HeliStep::Arrive;
                    }
                } else {
                    self.timer -= 1;
                }
            }
            HeliPhase::Arriving => {
                // Ground_801C53EC(0x704E1) when it descends: sound only.
                if !self.descended && below_top() {
                    self.descended = true;
                }
                if ended {
                    self.phase = HeliPhase::Hovering;
                }
            }
            HeliPhase::Hovering => {
                if self.timer <= 0 {
                    if ufo_away {
                        // Ground_801C53EC(0x704E2): sound only.
                        self.phase = HeliPhase::Leaving;
                        self.timer = 0;
                        return HeliStep::Leave;
                    }
                } else {
                    self.timer -= 1;
                }
            }
            HeliPhase::Leaving => {
                let mut step = HeliStep::None;
                if ended {
                    self.phase = HeliPhase::Waiting;
                    self.timer = wait(parameters.heli_wait, parameters.heli_wait_add, rng);
                    step = HeliStep::Hide;
                }
                // grfourside.c:746: counts on after the next wait is drawn.
                self.timer += 1;
                return step;
            }
        }
        HeliStep::None
    }
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct Fourside {
    pub parameters: Parameters,
    pub crane: crane::Crane,
    pub heli: Heli,
    pub ufo: ufo::Ufo,
}
