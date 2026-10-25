//! The barrel cannon (map 5, grKongo_801D55D8 / 801D577C).
//!
//! The barrel rests, spins up in a random direction, coasts to a weighted
//! random facing and rests again. Its arrow's animation rate breathes on
//! its own timer. A fighter it catches (fn_801D8134) or a Klaptrap
//! (grKongo_801D8078) stays a random time, then is launched along the
//! barrel's facing.
use super::{Parameters, DEG_TO_RAD};
use crate::rand::range;
use gekko_math::{fma::fmadds, msl::fctiwz, HsdRng};

/// gp+C4 (s16): the spin's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spin {
    /// 0: slowing to rest, then counting the rest down.
    Rest = 0,
    /// 1: accelerating.
    Accelerate = 1,
    /// 2: coasting toward the chosen facing.
    Coast = 2,
    /// 3: braking onto it.
    Brake = 3,
}
impl Spin {
    pub fn from_saved(value: i16) -> Self {
        match value {
            0 => Self::Rest,
            1 => Self::Accelerate,
            2 => Self::Coast,
            3 => Self::Brake,
            _ => panic!("invalid Kongo barrel spin phase {value}"),
        }
    }
}

/// gp+D0 (`u.taru.keep`): what the barrel holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// A fighter, by its index in the fighter list (p_link 8).
    Fighter(usize),
    /// A Klaptrap item, by its id (p_link 9).
    Item(u32),
}

/// gp+C6 (s16): what the barrel holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// 0: empty.
    Empty = 0,
    /// 1: holding, counting down.
    Holding = 1,
    /// 2: launching this frame.
    Launch = 2,
    /// 3: waiting for the launched Klaptrap to leave its reach.
    Clearing = 3,
}

/// The barrel (`GroundVars_kongo` / `kongo2` / `kongo3` on map 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Barrel {
    pub spin: Spin,
    pub hold: Hold,
    /// gp+C8 (s16): the arrow rate's phase (0 dim wait, 1 brighten, 2
    /// bright wait, 3 dim).
    pub rate_phase: i16,
    /// gp+CA (s16): frames the held one stays.
    pub hold_timer: i16,
    /// gp+CC (s16): the spin phase's timer.
    pub spin_timer: i16,
    /// gp+CE (s16): the rate phase's timer.
    pub rate_timer: i16,
    /// gp+D4: the facing the coast heads for.
    pub target: f32,
    /// gp+D8: the barrel's rotation z.
    pub angle: f32,
    /// gp+DC: the spin's acceleration (signed).
    pub acceleration: f32,
    /// gp+E0: the angular speed.
    pub speed: f32,
    /// gp+E8: the arrow animation's rate.
    pub rate: f32,
    /// gp+D0.
    pub kept: Option<Kept>,
}

/// `2 * M_PI` as a double (.sdata2 @504).
const TAU: f64 = std::f64::consts::TAU;

/// rand_range over two fctiwz'd float bounds (grkongo.c's inline).
fn range_f(rng: &mut HsdRng, [a, b]: [f32; 2]) -> i16 {
    range(rng, [fctiwz(a), fctiwz(b)]) as i16
}

/// One barrel step's requests to the engine.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BarrelStep {
    /// HSD_JObjSetRotationZ of descendant 1, then lb_8000B1CC of it for
    /// Ground_801C4D70.
    pub angle: f32,
    /// grAnime_801C7A04(gobj, 0, 7, rate).
    pub rate: f32,
}

impl Barrel {
    /// grKongo_801D55D8 (0x801D55D8): at rest facing up, the arrow bright.
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        let spin_timer = range_f(rng, p.rest_frames);
        let rate_timer = range(rng, p.bright_frames) as i16;
        Self {
            spin: Spin::Rest,
            hold: Hold::Empty,
            rate_phase: 2,
            hold_timer: 0,
            spin_timer,
            rate_timer,
            target: 0.0,
            angle: 0.0,
            acceleration: 0.0,
            speed: 0.0,
            rate: p.rate_top,
            kept: None,
        }
    }

    /// grKongo_801D577C (0x801D577C) up to the hold: the spin, the facing
    /// and the arrow's rate.
    pub fn step(&mut self, p: &Parameters, rng: &mut HsdRng) -> BarrelStep {
        let acceleration = DEG_TO_RAD * p.spin_acceleration;
        match self.spin {
            Spin::Coast | Spin::Brake => self.coast(acceleration),
            Spin::Rest => {
                if self.speed > 0.0 {
                    if self.speed < acceleration {
                        self.speed = 0.0;
                        self.angle = self.target;
                    } else {
                        self.speed -= acceleration;
                    }
                } else if self.speed < 0.0 {
                    if self.speed > -acceleration {
                        self.speed = 0.0;
                        self.angle = self.target;
                    } else {
                        self.speed += acceleration;
                    }
                }
                self.spin_timer = self.spin_timer.wrapping_sub(1);
                if self.spin_timer < 0 {
                    self.spin = Spin::Accelerate;
                    self.acceleration = if rng.randi(2) != 0 {
                        acceleration
                    } else {
                        -acceleration
                    };
                    self.spin_timer = range_f(rng, p.spin_frames);
                }
            }
            Spin::Accelerate => {
                self.speed += self.acceleration;
                let top = DEG_TO_RAD * p.spin_speed;
                if self.speed > top {
                    self.speed = top;
                } else if self.speed < -top {
                    self.speed = -top;
                }
                let old = self.spin_timer;
                self.spin_timer = old.wrapping_sub(1);
                if old < 0 {
                    self.spin = Spin::Coast;
                    self.spin_timer = range_f(rng, p.rest_frames);
                    self.target = direction(p, rng);
                }
            }
        }
        self.angle += self.speed;
        // Ground_WrapAngle: double compares and adds, rounded.
        if f64::from(self.angle) > TAU {
            self.angle = (f64::from(self.angle) - TAU) as f32;
        } else if f64::from(self.angle) < -TAU {
            self.angle = (f64::from(self.angle) + TAU) as f32;
        }
        self.step_rate(p, rng);
        BarrelStep {
            angle: self.angle,
            rate: self.rate,
        }
    }

    /// grkongo.c:350-384: coasting brakes once the facing is within the
    /// stopping distance (retail 801D57EC..801D57F4: fdivs, fmuls, fmuls).
    fn coast(&mut self, acceleration: f32) {
        let stopping = 0.5 * (self.speed * (self.speed / acceleration));
        let mut remaining = if self.speed > 0.0 {
            self.target - self.angle
        } else if self.speed < 0.0 {
            self.angle - self.target
        } else {
            panic!("grkongo.c:505: a coasting barrel stands still")
        };
        if remaining < 0.0 {
            remaining = (f64::from(remaining) + TAU) as f32;
        }
        let speed = if self.speed < 0.0 {
            -self.speed
        } else {
            self.speed
        };
        if remaining < stopping || remaining < speed {
            if self.spin == Spin::Brake {
                self.spin = Spin::Rest;
            }
        } else if self.spin == Spin::Coast {
            self.spin = Spin::Brake;
        }
    }

    /// grkongo.c:401-433: the arrow's rate phases.
    fn step_rate(&mut self, p: &Parameters, rng: &mut HsdRng) {
        match self.rate_phase {
            0 | 2 => {
                let old = self.rate_timer;
                self.rate_timer = old.wrapping_sub(1);
                if old < 0 {
                    self.rate_phase += 1;
                }
            }
            1 => {
                self.rate += p.rate_step;
                if self.rate > p.rate_top {
                    self.rate = p.rate_top;
                    self.rate_timer = range(rng, p.bright_frames) as i16;
                    self.rate_phase = 2;
                }
            }
            3 => {
                self.rate -= p.rate_step;
                if self.rate < 0.0 {
                    self.rate = 0.0;
                    self.rate_timer = range(rng, p.dim_frames) as i16;
                    self.rate_phase = 0;
                }
            }
            _ => {}
        }
    }

    /// The catch's hold time (fn_801D8134, grkongo.c:441): retail
    /// 0x801D81F0 (fighter) / 0x801D5DD8 (Klaptrap): fmadds
    /// (x24 - x20) * r + x20, fctiwz.
    pub fn draw_hold(p: &Parameters, rng: &mut HsdRng) -> i16 {
        let r = rng.randf();
        let [low, high] = p.hold_frames;
        fctiwz(fmadds(high - low, r, low)) as i16
    }

    /// The launch angle in degrees (grkongo.c:470-478): the barrel's
    /// facing turned a quarter, wrapped, in double, then times 57.29578.
    pub fn launch_degrees(&self) -> f32 {
        let mut angle = (std::f64::consts::FRAC_PI_2 + f64::from(self.angle)) as f32;
        if angle < 0.0 {
            angle = (f64::from(angle) + TAU) as f32;
        } else if f64::from(angle) > TAU {
            angle = (f64::from(angle) - TAU) as f32;
        }
        57.295_78 * angle
    }
}

/// grKongo_801D8314 (0x801D8314): a rest facing by the eight weights.
pub fn direction(p: &Parameters, rng: &mut HsdRng) -> f32 {
    const FACINGS: [f32; 8] = [
        2.356_194_5,
        1.570_796_4,
        std::f32::consts::FRAC_PI_4,
        0.0,
        -std::f32::consts::FRAC_PI_4,
        -1.570_796_4,
        -2.356_194_5,
        -std::f32::consts::PI,
    ];
    let w = p.direction_weights.map(i32::from);
    // 801D8328..801D8360: x46 + x48 ... + x52, then x44 added last.
    let total = w[0] + (w[1] + w[2] + w[3] + w[4] + w[5] + w[6] + w[7]);
    let mut roll = if total != 0 { rng.randi(total) } else { 0 };
    for (weight, facing) in w.iter().zip(FACINGS) {
        roll -= weight;
        if roll < 0 {
            return facing;
        }
    }
    panic!("grkongo.c:1753: no barrel facing drawn")
}
