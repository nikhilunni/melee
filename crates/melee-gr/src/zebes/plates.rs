//! The breakable plates under the two upper parts: grZebes_801DA528, run
//! by map 6's proc (grZebes_801D881C) for the left plate and map 7's
//! (grZebes_801D925C) for the right one, and their material items'
//! hit callbacks (fn_801DA9D8 / fn_801DA9F0).
//!
//! An intact plate wobbles (two bones' translation x) when hit and heals a
//! point of damage every `plate_heal_interval` ticks; its crack animation
//! shows the damage. Past `plate_break_damage` it breaks (the rest of the
//! crack plays, its hits stop), waits `plate_wait`, and grows back.
use super::Parameters;
use gekko_math::{fma::fmadds, HsdRng};

/// The plate's phase (`grZe_AcidState.x00` / `x01`, u8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatePhase {
    /// Whole: wobbling, healing, cracking under damage.
    Whole = 0,
    /// Breaking: the crack plays out.
    Breaking = 1,
    /// Gone for `plate_wait`.
    Gone = 2,
    /// Growing back.
    Growing = 3,
}

impl PlatePhase {
    pub fn from_saved(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Whole,
            1 => Self::Breaking,
            2 => Self::Gone,
            3 => Self::Growing,
            _ => return None,
        })
    }
}

/// The animation a whole plate's crack scrubs through, and the one that
/// grows it back (grZebes_801D881C / 801D925C pass 1 and 2).
pub const CRACK_ANIMATION: usize = 1;
pub const GROWTH_ANIMATION: usize = 2;

/// One plate (`grZe_AcidState`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plate {
    /// x00: the phase entered, `None` (0xFF) before the first tick.
    pub phase: Option<PlatePhase>,
    /// x01: the phase the next tick enters.
    pub next: PlatePhase,
    /// x02 (s16): the healing clock while whole, the wait while gone.
    pub timer: i16,
    /// x04: the wobbling bones' translation x at creation.
    pub base_x: f32,
    /// x08 / x0C: the wobble.
    pub offset: f32,
    pub velocity: f32,
    /// x10: the damage taken.
    pub damage: f32,
}

/// What a plate step asks of its model and material item, through
/// [`PlateEngine`] in C order.
pub trait PlateEngine {
    /// grAnime_801C7FF8(gobj, crack bone, 1, animation, 0.0, 1.0).
    fn play(&mut self, animation: usize);
    /// grAnime_801C78FC(gobj, crack bone, 1): the crack stops looping.
    fn stop_looping(&mut self);
    /// grAnime_801C83D0(gobj, crack bone, 1).
    fn ended(&self) -> bool;
    /// grAnime_801C7B24(gobj, crack bone, 1, frame).
    fn request_frame(&mut self, frame: f32);
    /// HSD_JObjSetTranslateX on both wobbling bones.
    fn set_wobble(&mut self, x: f32);
    /// grMaterial_801C8E08 / 801C8E28 on the plate's item.
    fn set_hits(&mut self, on: bool);
}

/// The crack's frame once broken (`@296`).
const BROKEN_FRAME: f32 = 30.0;

impl Plate {
    /// grZebes_801D8644 / 801D9100: x00 = 0xFF, everything still.
    pub fn new(base_x: f32) -> Self {
        Self {
            phase: None,
            next: PlatePhase::Whole,
            timer: 0,
            base_x,
            offset: 0.0,
            velocity: 0.0,
            damage: 0.0,
        }
    }

    /// fn_801DA9D8 / fn_801DA9F0: a hit on the plate's item adds its damage
    /// (xCA0) and sets the wobble's speed to the launch's x (it_8027B798).
    pub fn hit(&mut self, damage: f32, launch_x: f32) {
        self.damage += damage;
        self.velocity = launch_x;
    }

    /// grZebes_801DA528 (0x801DA528): enter a new phase, then run it.
    /// Returns the phase it ran (x00).
    pub fn step(
        &mut self,
        parameters: &Parameters,
        rng: &mut HsdRng,
        engine: &mut impl PlateEngine,
    ) -> PlatePhase {
        if self.phase != Some(self.next) {
            self.phase = Some(self.next);
            self.enter(parameters, rng, engine);
        }
        let phase = self.phase.expect("the plate entered a phase");
        match phase {
            PlatePhase::Whole => self.whole(parameters, engine),
            PlatePhase::Breaking => {
                if engine.ended() {
                    self.next = PlatePhase::Gone;
                }
            }
            PlatePhase::Gone => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer < 0 {
                    self.next = PlatePhase::Growing;
                }
            }
            PlatePhase::Growing => {
                if engine.ended() {
                    self.next = PlatePhase::Whole;
                }
            }
        }
        phase
    }

    fn enter(&mut self, parameters: &Parameters, rng: &mut HsdRng, engine: &mut impl PlateEngine) {
        match self.next {
            PlatePhase::Whole => {
                engine.play(CRACK_ANIMATION);
                engine.set_hits(true);
                self.timer = 0;
                self.offset = 0.0;
                self.velocity = 0.0;
            }
            PlatePhase::Breaking => {
                engine.stop_looping();
                engine.set_hits(false);
            }
            PlatePhase::Gone => {
                // retail 0x801DA5F8: fmadds, fctiwz.
                let [low, high] = parameters.plate_wait;
                self.timer = gekko_math::msl::fctiwz(fmadds(high - low, rng.randf(), low)) as i16;
            }
            PlatePhase::Growing => {
                engine.play(GROWTH_ANIMATION);
                engine.stop_looping();
                self.damage = 0.0;
            }
        }
    }

    /// Phase 0: heal, wobble, crack.
    fn whole(&mut self, parameters: &Parameters, engine: &mut impl PlateEngine) {
        let elapsed = self.timer;
        self.timer = self.timer.wrapping_add(1);
        if f32::from(elapsed) > parameters.plate_heal_interval {
            self.timer = 0;
            self.damage -= 1.0;
            if self.damage < 0.0 {
                self.damage = 0.0;
            }
        }
        let rest = parameters.plate_rest_offset;
        if self.offset > rest {
            self.velocity -= parameters.plate_spring;
        } else if self.offset < -rest {
            self.velocity += parameters.plate_spring;
        }
        // retail 0x801DA724 fsubs, 0x801DA728 fmuls.
        self.velocity *= 1.0 - parameters.plate_damping;
        let top = parameters.plate_max_speed;
        if self.velocity > top {
            self.velocity = top;
        } else if self.velocity < -top {
            self.velocity = -top;
        }
        self.offset += self.velocity;
        let limit = parameters.plate_max_offset;
        if self.offset > limit {
            self.offset = limit;
        } else if self.offset < -limit {
            self.offset = -limit;
        }
        let offset = if self.offset < 0.0 {
            -self.offset
        } else {
            self.offset
        };
        if offset < rest {
            let speed = if self.velocity < 0.0 {
                -self.velocity
            } else {
                self.velocity
            };
            if speed < parameters.plate_rest_speed {
                self.offset = 0.0;
                self.velocity = 0.0;
            }
        }
        engine.set_wobble(self.base_x + self.offset);
        let threshold = parameters.plate_break_damage;
        let frame = if self.damage > threshold {
            self.next = PlatePhase::Breaking;
            // Ground_801C53EC(400006) and ftLib_80086C9C(1, 0x5A): a sound
            // and every controller's rumble.
            BROKEN_FRAME
        } else {
            // retail 0x801DA924 fmuls, 0x801DA92C fdiv by the double 30.0.
            (f64::from(self.damage * threshold) / 30.0) as f32
        };
        engine.request_frame(frame);
    }
}
