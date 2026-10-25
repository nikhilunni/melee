//! The crane (map 1, `grFourside_CraneVars`): its arm swings out and back
//! on animation 0, and its iron beam, a platform (collision joint 3 on bone
//! 7), is raised and lowered by translating bone 4.
use super::{wait, Parameters};
use gekko_math::HsdRng;

/// The beam's bone (Ground_801C3FA4(gobj, 4)).
pub const IRON_BONE: usize = 4;
/// Ticks the arm's swing plays before it stops (grfourside.c:342, 351).
const SWING_FRAMES: i32 = 300;
/// The return swing's first frame in animation 0.
const RETURN_SWING_FRAME: f32 = 400.0;
/// grFourside_801F3154: the beam rests this far below its model height.
const REST_DROP: f32 = 10.0;

/// `foursideCrane.x0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CranePhase {
    Waiting = 0,
    SwingingOut = 1,
    SwingingBack = 2,
    Swung = 3,
    IronWaiting = 4,
    Lowering = 5,
    Raising = 6,
    Settling = 7,
}
impl CranePhase {
    pub fn from_saved(value: u8) -> Option<Self> {
        use CranePhase::*;
        [
            Waiting,
            SwingingOut,
            SwingingBack,
            Swung,
            IronWaiting,
            Lowering,
            Raising,
            Settling,
        ]
        .get(usize::from(value))
        .copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crane {
    pub phase: CranePhase,
    /// x1.b0: the arm is out (the next swing plays from frame 400).
    pub out: bool,
    /// x1.b1: the last move lowered the beam (only logged).
    pub lowered: bool,
    /// x4.
    pub timer: i32,
    /// x8: the beam's rest height.
    pub rest_y: f32,
    /// xC: the beam's height, written to bone 4 every tick.
    pub y: f32,
    /// x10: where the beam is going.
    pub target_y: f32,
    /// x14: the settling stops once its acceleration exceeds this.
    pub settle_limit: f32,
    /// x18.
    pub speed: f32,
    /// x1C.
    pub acceleration: f32,
}

/// The arm's animation, as the step asks for it.
pub trait CraneEngine {
    /// grAnime_801C7B24(gobj, 0, 7, frame): every AObj requested at `frame`.
    fn request_frame(&mut self, frame: f32);
    /// grAnime_801C7A04(gobj, 0, 7, rate).
    fn set_rate(&mut self, rate: f32);
}

/// `|max - min|` truncated (fctiwz), the range a beam move draws from.
fn range(min: f32, max: f32) -> i32 {
    // retail 0x801F3460: fsubs, then fneg when negative, fctiwz.
    let mut span = max - min;
    if span < 0.0 {
        span = -span;
    }
    gekko_math::msl::fctiwz(span)
}

/// The distance of one beam move: a draw over the range plus `|min|`.
fn distance(min: f32, max: f32, rng: &mut HsdRng) -> f32 {
    let range = range(min, max);
    // retail 0x801F34C0: the draw (or zero) as a float, fsubs of the
    // magic double.
    let drawn = if range != 0 { rng.randi(range) } else { 0 } as f32;
    let mut magnitude = min;
    if magnitude < 0.0 {
        magnitude = -magnitude;
    }
    // retail 0x801F34CC: fadds.
    drawn + magnitude
}

impl Crane {
    /// grFourside_801F3154 (0x801F3154): one wait drawn; the beam's height
    /// from its model, less ten.
    pub fn create(parameters: &Parameters, iron_y: f32, rng: &mut HsdRng) -> Self {
        let timer = wait(parameters.crane_wait, parameters.crane_wait_add, rng);
        let rest_y = iron_y - REST_DROP;
        Self {
            phase: CranePhase::Waiting,
            out: false,
            lowered: false,
            timer,
            rest_y,
            y: rest_y,
            target_y: rest_y,
            settle_limit: 0.0,
            speed: 0.0,
            acceleration: 0.0,
        }
    }

    /// grFourside_801F3274 (0x801F3274) before it moves bone 4 to
    /// [`Self::y`] and updates the collision (Ground_801C2FE0).
    pub fn tick(
        &mut self,
        parameters: &Parameters,
        rng: &mut HsdRng,
        engine: &mut impl CraneEngine,
    ) {
        let p = parameters;
        match self.phase {
            CranePhase::Waiting => {
                if self.timer <= 0 {
                    engine.request_frame(if self.out { RETURN_SWING_FRAME } else { 0.0 });
                    engine.set_rate(1.0);
                    self.timer = 0;
                    self.phase = if self.out {
                        CranePhase::SwingingBack
                    } else {
                        CranePhase::SwingingOut
                    };
                } else {
                    self.timer -= 1;
                }
            }
            CranePhase::SwingingOut | CranePhase::SwingingBack => {
                if self.timer >= SWING_FRAMES {
                    engine.set_rate(0.0);
                    self.out = self.phase == CranePhase::SwingingOut;
                    self.phase = CranePhase::Swung;
                } else {
                    self.timer += 1;
                }
            }
            CranePhase::Swung => {
                self.timer = wait(p.crane_iron_wait, p.crane_iron_wait_add, rng);
                self.phase = CranePhase::IronWaiting;
            }
            CranePhase::IronWaiting => {
                if self.timer <= 0 {
                    self.target_y = self.rest_y;
                    if self.y > self.rest_y {
                        let [min, max] = p.crane_iron_down;
                        // retail 0x801F34E0: fsubs.
                        self.target_y -= distance(min, max, rng);
                        self.lowered = true;
                        self.phase = CranePhase::Lowering;
                    } else {
                        let [min, max] = p.crane_iron_up;
                        // retail 0x801F35A0: fadds.
                        self.target_y += distance(min, max, rng);
                        self.lowered = false;
                        self.phase = CranePhase::Raising;
                    }
                } else {
                    self.timer -= 1;
                }
            }
            CranePhase::Lowering => {
                self.y -= p.crane_iron_speed;
                if self.y <= self.target_y {
                    self.y = self.target_y;
                    self.start_settling(p);
                }
            }
            CranePhase::Raising => {
                self.y += p.crane_iron_speed;
                // grfourside.c:410: no clamp on the way up.
                if self.y >= self.target_y {
                    self.start_settling(p);
                }
            }
            CranePhase::Settling => {
                let acceleration = self.acceleration;
                let mut magnitude = acceleration;
                if magnitude < 0.0 {
                    magnitude = -magnitude;
                }
                if magnitude > self.settle_limit {
                    self.y = self.target_y;
                    self.timer = wait(p.crane_wait, p.crane_wait_add, rng);
                    self.phase = CranePhase::Waiting;
                } else {
                    // retail 0x801F36EC, 0x801F36FC: fadds, fadds.
                    self.speed += acceleration;
                    self.y += self.speed;
                    if self.acceleration < 0.0 && self.y < self.target_y {
                        self.acceleration *= -2.0;
                    }
                    if self.acceleration > 0.0 && self.y > self.target_y {
                        self.acceleration *= -2.0;
                    }
                }
            }
        }
    }

    fn start_settling(&mut self, p: &Parameters) {
        self.settle_limit = p.crane_iron_speed;
        self.speed = p.crane_iron_speed;
        self.acceleration = p.crane_iron_stop_acceleration;
        self.phase = CranePhase::Settling;
    }
}
