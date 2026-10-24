//! Kongo Jungle N64, gr/groldkongo.c (NTSC 1.02): the barrel cannon that
//! drifts under the stage, turns to random directions and fires what it
//! catches; and a background bird at random intervals.
//!
//! The barrel is map 1 (stageGObj1_GObjProc, 0x8020F888): its joint
//! animation carries it along its path at a rate the controller ramps up
//! and down, descendant 1 is the cannon (rotation Z is its aim), and every
//! step publishes the cannon's world position and aim (Ground_801C4D70) for
//! the fighters' catch check (grOldKongo_80210454) and the caught fighter
//! (ftCo_8009EC44). Map 3's platforms carry collision joints 0 and 1
//! (grOk_803E6580). The bird is map 2 (stageGObj2_GObjProc, 0x802100FC).
pub mod procs;
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::fctiwz,
    HsdRng,
};
use hsd_types::Vec3;

/// `grOldKongo_YakumonoParam` (GrOk.dat `yakumono_param`).
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0x2, +0x0 (s16): `rand_range`'s bounds of the bird's wait.
    pub bird_wait: [i32; 2],
    /// +0x4: the bird's height range around 70.
    pub bird_height_range: f32,
    /// +0x8, +0xC: the frames a caught fighter waits before the shot, as
    /// `a + (b - a) * HSD_Randf()`.
    pub shot_delay: [f32; 2],
    /// +0x10: the catch radius.
    pub catch_radius: f32,
    /// +0x18, +0x14: `rand_range`'s bounds of the wait before a turn
    /// (floats retail truncates with fctiwz).
    pub turn_wait: [f32; 2],
    /// +0x1C: the turn's acceleration, in degrees per frame per frame.
    pub turn_acceleration: f32,
    /// +0x20: the turn's top speed, in degrees per frame.
    pub turn_speed: f32,
    /// +0x28, +0x24: `rand_range`'s bounds of a turn's spin-up.
    pub turn_spin: [f32; 2],
    /// +0x2C..+0x3A (s16): the weights of the eight aims, from down-left
    /// round to down.
    pub aim_weights: [i16; 8],
    /// +0x40, +0x3C (s32): `rand_range`'s bounds of a stop.
    pub stop_interval: [i32; 2],
    /// +0x44: the path speed's change per frame.
    pub move_acceleration: f32,
    /// +0x48: the path speed's maximum (the animation rate).
    pub move_speed: f32,
    /// +0x50, +0x4C (s32): `rand_range`'s bounds of a drift at full speed.
    pub drift_frames: [i32; 2],
    /// +0x54..+0x68: the shot's damage, angle, growth, set knockback, base
    /// knockback and element.
    pub shot_damage: i32,
    pub shot_angle: i32,
    pub shot_growth: i32,
    pub shot_set_knockback: i32,
    pub shot_base_knockback: i32,
    pub shot_element: i32,
    /// +0x6C: the material program grMaterial_801C9604 starts on a catch.
    pub catch_material: i32,
}

/// `rand_range(a, b)` (gr/inlines.h) on integer bounds, retail's order.
fn rand_range(rng: &mut HsdRng, [a, b]: [i32; 2]) -> i32 {
    let draw = |rng: &mut HsdRng, n: i32| if n != 0 { rng.randi(n) } else { 0 };
    if a > b {
        b + draw(rng, a - b)
    } else if a < b {
        a + draw(rng, b - a)
    } else {
        a
    }
}
/// `rand_range` over float fields: each bound truncates (fctiwz) first.
fn rand_range_f(rng: &mut HsdRng, [a, b]: [f32; 2]) -> i32 {
    rand_range(rng, [fctiwz(a), fctiwz(b)])
}

/// `MTXDegToRad`: one fmuls by @475.
fn radians(degrees: f32) -> f32 {
    const DEGREES_TO_RADIANS: f32 = 0.017453292;
    DEGREES_TO_RADIANS * degrees
}
/// @478 and @479, compared and added in double precision.
const TAU: f64 = std::f64::consts::TAU;
/// @483: a shot leaves at a right angle to the aim.
const QUARTER_TURN: f64 = std::f64::consts::FRAC_PI_2;
/// @484.
const RADIANS_TO_DEGREES: f32 = 57.29578;
/// grOldKongo_80210650's aims (@651..@657), in weight order.
const AIMS: [f32; 8] = [
    2.3561945,
    std::f32::consts::FRAC_PI_2,
    std::f32::consts::FRAC_PI_4,
    0.0,
    -std::f32::consts::FRAC_PI_4,
    -std::f32::consts::FRAC_PI_2,
    -2.3561945,
    -std::f32::consts::PI,
];
/// Shots leave this many frames after the shot animation starts
/// (`hit_timer++ > 10`).
const SHOT_WINDUP: i16 = 10;

/// The barrel's aim controller (xC4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    /// 0: braking, then waiting for the next turn.
    Waiting = 0,
    /// 1: spinning up.
    SpinningUp = 1,
    /// 2: spinning, still short of the braking point.
    Spinning = 2,
    /// 3: spinning, past it once.
    Approaching = 3,
}
/// The barrel's path controller (xC8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drift {
    Stopped = 0,
    Accelerating = 1,
    Cruising = 2,
    Decelerating = 3,
}
/// The barrel's catch (xC6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Catch {
    Empty = 0,
    /// 1: holding a fighter until the shot delay runs out (or A/B).
    Holding = 1,
    /// 2: the shot animation starts.
    Firing = 2,
    /// 3: winding up to the shot.
    WindingUp = 3,
    /// 4: fired; empty next frame.
    Fired = 4,
}
impl Turn {
    pub fn from_saved(value: i16) -> Option<Self> {
        Some(match value {
            0 => Self::Waiting,
            1 => Self::SpinningUp,
            2 => Self::Spinning,
            3 => Self::Approaching,
            _ => return None,
        })
    }
}
impl Drift {
    pub fn from_saved(value: i16) -> Option<Self> {
        Some(match value {
            0 => Self::Stopped,
            1 => Self::Accelerating,
            2 => Self::Cruising,
            3 => Self::Decelerating,
            _ => return None,
        })
    }
}
impl Catch {
    pub fn from_saved(value: i16) -> Option<Self> {
        Some(match value {
            0 => Self::Empty,
            1 => Self::Holding,
            2 => Self::Firing,
            3 => Self::WindingUp,
            4 => Self::Fired,
            _ => return None,
        })
    }
}

/// `grOldKongo` barrel Ground state (`gp->u.taru`).
#[derive(Clone, Debug, PartialEq)]
pub struct Barrel {
    pub turn: Turn,
    pub catch: Catch,
    pub drift: Drift,
    /// xCA (s16): frames until a held fighter is fired.
    pub shot_delay: i16,
    /// xCC (s16): the aim controller's timer.
    pub turn_timer: i16,
    /// xCE (s16): the path controller's timer.
    pub drift_timer: i16,
    /// xD0 (s16): frames since the shot animation started.
    pub windup: i16,
    /// xD4: the held fighter (its fighter-list index), or none.
    pub held: Option<usize>,
    /// xD8: the aim being turned to.
    pub target_aim: f32,
    /// xDC: the aim.
    pub aim: f32,
    /// xE0: the spin-up's signed acceleration.
    pub spin_acceleration: f32,
    /// xE4: the spin speed.
    pub spin: f32,
    /// xEC: the path speed (the barrel animation's rate).
    pub speed: f32,
}

/// A shot (ftCo_8009EC70's arguments): from `position`, angle in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub fighter: usize,
    pub position: Vec3,
    pub angle: f32,
}

/// What a barrel step asks of the engine, in retail's order.
pub trait BarrelEngine {
    /// HSD_JObjSetRotationZ(cannon, aim), then lb_8000B1CC's world position
    /// of the cannon (descendant 1).
    fn aim_cannon(&mut self, aim: f32) -> Vec3;
    /// grAnime_801C7A04(gobj, 0, 7, rate): every animation's rate.
    fn set_rate(&mut self, rate: f32);
    /// grAnime_801C7FF8(gobj, 2, 7, 2, 0.0, 1.0), then grMaterial_801C95C4.
    fn start_shot_animation(&mut self);
    /// Whether the held fighter's GObj is still a fighter (p_link 8).
    fn holds_fighter(&self, fighter: usize) -> bool;
}

impl Barrel {
    /// stageGObj1_OnInit (0x8020F6E4): the turn wait, then the first
    /// drift's length.
    pub fn create(p: &Parameters, rng: &mut HsdRng) -> Self {
        let turn_timer = rand_range_f(rng, p.turn_wait) as i16;
        let drift_timer = rand_range(rng, p.drift_frames) as i16;
        Self {
            turn: Turn::Waiting,
            catch: Catch::Empty,
            drift: Drift::Cruising,
            shot_delay: 0,
            turn_timer,
            drift_timer,
            windup: 0,
            held: None,
            target_aim: 0.0,
            aim: 0.0,
            spin_acceleration: 0.0,
            spin: 0.0,
            speed: p.move_speed,
        }
    }

    /// stageGObj1_GObjProc (0x8020F888). Returns the cannon's position
    /// (Ground_801C4D70's) and a shot, if one leaves.
    pub fn tick(
        &mut self,
        p: &Parameters,
        rng: &mut HsdRng,
        engine: &mut impl BarrelEngine,
    ) -> (Vec3, Option<Shot>) {
        self.turn_step(p, rng);
        // 0x8020FBF8 fadds, then Ground_WrapAngle in double.
        self.aim += self.spin;
        let aim = f64::from(self.aim);
        if aim > TAU {
            self.aim = (aim - TAU) as f32;
        } else if aim < -TAU {
            self.aim = (aim + TAU) as f32;
        }
        let position = engine.aim_cannon(self.aim);
        self.drift_step(p, rng);
        engine.set_rate(self.speed);
        let shot = self.catch_step(position, engine);
        (position, shot)
    }

    fn turn_step(&mut self, p: &Parameters, rng: &mut HsdRng) {
        match self.turn {
            Turn::Spinning | Turn::Approaching => {
                let acceleration = radians(p.turn_acceleration);
                // 0x8020F8F8..0x8020F900: fdivs, fmuls, fmuls.
                let braking = 0.5 * (self.spin * (self.spin / acceleration));
                let mut remaining = if self.spin > 0.0 {
                    self.target_aim - self.aim
                } else if self.spin < 0.0 {
                    self.aim - self.target_aim
                } else {
                    // HSD_ASSERT(0x18C): never with a spin under way.
                    panic!("groldkongo.c:396: a turning barrel without spin")
                };
                if remaining < 0.0 {
                    remaining = (f64::from(remaining) + TAU) as f32;
                }
                let speed = if self.spin < 0.0 {
                    -self.spin
                } else {
                    self.spin
                };
                if remaining < braking || remaining < speed {
                    if self.turn == Turn::Approaching {
                        self.turn = Turn::Waiting;
                    }
                } else if self.turn == Turn::Spinning {
                    self.turn = Turn::Approaching;
                }
            }
            Turn::Waiting => {
                let acceleration = radians(p.turn_acceleration);
                if self.spin > 0.0 {
                    if self.spin < acceleration {
                        self.spin = 0.0;
                        self.aim = self.target_aim;
                    } else {
                        self.spin -= acceleration;
                    }
                } else if self.spin < 0.0 {
                    if self.spin > -acceleration {
                        self.spin = 0.0;
                        self.aim = self.target_aim;
                    } else {
                        self.spin += acceleration;
                    }
                }
                self.turn_timer = self.turn_timer.wrapping_sub(1);
                if self.turn_timer < 0 {
                    self.turn = Turn::SpinningUp;
                    let acceleration = radians(p.turn_acceleration);
                    self.spin_acceleration = if rng.randi(2) != 0 {
                        acceleration
                    } else {
                        -acceleration
                    };
                    self.turn_timer = rand_range_f(rng, p.turn_spin) as i16;
                }
            }
            Turn::SpinningUp => {
                self.spin += self.spin_acceleration;
                let top = radians(p.turn_speed);
                if self.spin > top {
                    self.spin = top;
                } else if self.spin < -top {
                    self.spin = -top;
                }
                let old = self.turn_timer;
                self.turn_timer = old.wrapping_sub(1);
                if old < 0 {
                    self.turn = Turn::Spinning;
                    self.turn_timer = rand_range_f(rng, p.turn_wait) as i16;
                    self.target_aim = choose_aim(p, rng);
                }
            }
        }
    }

    fn drift_step(&mut self, p: &Parameters, rng: &mut HsdRng) {
        match self.drift {
            Drift::Stopped => {
                let old = self.drift_timer;
                self.drift_timer = old.wrapping_sub(1);
                if old < 0 {
                    self.drift = Drift::Accelerating;
                }
            }
            Drift::Accelerating => {
                self.speed += p.move_acceleration;
                if self.speed > p.move_speed {
                    self.speed = p.move_speed;
                    self.drift_timer = rand_range(rng, p.drift_frames) as i16;
                    self.drift = Drift::Cruising;
                }
            }
            Drift::Cruising => {
                let old = self.drift_timer;
                self.drift_timer = old.wrapping_sub(1);
                if old < 0 {
                    self.drift = Drift::Decelerating;
                }
            }
            Drift::Decelerating => {
                self.speed -= p.move_acceleration;
                if self.speed < 0.0 {
                    self.speed = 0.0;
                    self.drift_timer = rand_range(rng, p.stop_interval) as i16;
                    self.drift = Drift::Stopped;
                }
            }
        }
    }

    fn catch_step(&mut self, position: Vec3, engine: &mut impl BarrelEngine) -> Option<Shot> {
        // The C falls through from 1 to 2 to 3: a holding barrel whose
        // fighter is gone empties and still runs the shot animation
        // (0x8020FEB8..0x8020FEC0 branch into case 2).
        let mut state = self.catch;
        if state == Catch::Holding {
            if self.held.is_none() {
                self.catch = Catch::Empty;
            } else {
                let old = self.shot_delay;
                self.shot_delay = old.wrapping_sub(1);
                if old >= 0 {
                    return None;
                }
                self.catch = Catch::Firing;
            }
            state = Catch::Firing;
        }
        if state == Catch::Firing {
            engine.start_shot_animation();
            self.windup = 0;
            self.catch = Catch::WindingUp;
            state = Catch::WindingUp;
        }
        match state {
            Catch::WindingUp => {
                let old = self.windup;
                self.windup = old.wrapping_add(1);
                if old <= SHOT_WINDUP {
                    return None;
                }
                // 0x8020FFC0: fadd in double, frsp; wrap in double.
                let mut angle = (QUARTER_TURN + f64::from(self.aim)) as f32;
                if angle < 0.0 {
                    angle = (f64::from(angle) + TAU) as f32;
                } else if f64::from(angle) > TAU {
                    angle = (f64::from(angle) - TAU) as f32;
                }
                let degrees = RADIANS_TO_DEGREES * angle;
                let fighter = self.held.expect("groldkongo.c: a shot without a fighter");
                self.catch = Catch::Fired;
                engine.holds_fighter(fighter).then_some(Shot {
                    fighter,
                    position,
                    angle: degrees,
                })
            }
            Catch::Fired => {
                self.catch = Catch::Empty;
                None
            }
            _ => None,
        }
    }

    /// grOldKongo_80210454 (0x80210454), the catch device's callback: an
    /// empty barrel catches a fighter whose position is within the catch
    /// radius of the cannon, drawing the shot delay. Returns whether it
    /// caught (the caller then plays the catch: animation 1 on the
    /// cannon, the material, effect 0x405 at the fighter, the rumble).
    pub fn try_catch(
        &mut self,
        p: &Parameters,
        cannon: Vec3,
        fighter: usize,
        fighter_position: Vec3,
        rng: &mut HsdRng,
    ) -> bool {
        if self.catch != Catch::Empty {
            return false;
        }
        // 0x802104B0..0x802104E0: separate fsubs, fmuls and fadds.
        let dx = cannon.x - fighter_position.x;
        let dy = cannon.y - fighter_position.y;
        let dz = cannon.z - fighter_position.z;
        let distance = dz * dz + (dx * dx + dy * dy);
        // 0x802104E8: fcmpo; bge (a NaN distance catches).
        if distance >= p.catch_radius * p.catch_radius {
            return false;
        }
        let r = rng.randf();
        // retail 0x80210510: fmadds (b - a) * r + a, then fctiwz.
        let [a, b] = p.shot_delay;
        self.shot_delay = fctiwz(fmadds(b - a, r, a)) as i16;
        self.held = Some(fighter);
        self.catch = Catch::Holding;
        true
    }

    /// grOldKongo_802105AC (0x802105AC), Ground_801C4DD0: A or B in the
    /// barrel fires early.
    pub fn fire_early(&mut self) {
        if self.catch == Catch::Holding {
            self.catch = Catch::Firing;
        }
    }

    /// grOldKongo_802105C8 (0x802105C8), Ground_801C4E20 from the held
    /// fighter's death2_cb: a fighter still in the fighter list lets go.
    /// Returns whether the material resets (grMaterial_801C95C4).
    pub fn release(&mut self, still_fighter: bool) -> bool {
        if self.catch != Catch::Holding {
            return false;
        }
        assert!(self.held.is_some(), "groldkongo.c:751: gp->u.taru.keep");
        if still_fighter {
            self.catch = Catch::Empty;
            self.held = None;
            return true;
        }
        false
    }
}

/// grOldKongo_80210650 (0x80210650): a weighted random aim.
fn choose_aim(p: &Parameters, rng: &mut HsdRng) -> f32 {
    let total: i32 = p.aim_weights.iter().map(|&w| i32::from(w)).sum();
    let mut r = if total != 0 { rng.randi(total) } else { 0 };
    for (weight, aim) in p.aim_weights.iter().zip(AIMS) {
        r -= i32::from(*weight);
        if r < 0 {
            return aim;
        }
    }
    panic!("groldkongo.c:786: no aim chosen")
}

/// The bird: `gp->u.unk.xC4`, frames until it flies (0 while flying).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bird {
    pub timer: i32,
}
/// The bird's base height and depth (@587, @589), and the side bound
/// (@590 / @589 as X).
const BIRD_HEIGHT: f32 = 70.0;
pub const BIRD_DEPTH: f32 = -200.0;
pub const BIRD_SIDE: f32 = 200.0;

/// What a bird step asks of map 2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BirdStep {
    Idle,
    /// grAnime_801C8138 restarts the flight at this height, entering from
    /// the right (descendant 3 shows) or the left (descendant 1).
    Fly {
        height: f32,
        from_right: bool,
    },
    /// The flight ended: hide the model.
    Land,
}

impl Bird {
    /// stageGObj2_OnInit (0x8021005C).
    pub fn create(p: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            timer: rand_range(rng, p.bird_wait),
        }
    }
    /// stageGObj2_GObjProc (0x802100FC). `ended` is grAnime_801C83D0(gobj,
    /// 0, 7), read only while flying.
    pub fn tick(&mut self, p: &Parameters, ended: bool, rng: &mut HsdRng) -> BirdStep {
        if self.timer > 0 {
            self.timer -= 1;
            if self.timer != 0 {
                return BirdStep::Idle;
            }
            let r = rng.randf();
            // retail 0x80210168 fmsubs 2.0 * r - 1.0, 0x80210174 fmadds
            // range * that + 70.0.
            let height = fmadds(p.bird_height_range, fmsubs(2.0, r, 1.0), BIRD_HEIGHT);
            let from_right = rng.randi(2) != 0;
            return BirdStep::Fly { height, from_right };
        }
        if ended {
            self.timer = rand_range(rng, p.bird_wait);
            return BirdStep::Land;
        }
        BirdStep::Idle
    }
}

/// Kongo Jungle N64's stage state.
#[derive(Clone, Debug)]
pub struct OldKongo {
    pub parameters: Parameters,
    pub barrel: Barrel,
    pub bird: Bird,
    /// Ground_801C4D70's registry: the cannon's position and aim as the
    /// barrel's last step left them (stage_info x730, x73C).
    pub cannon: Vec3,
    pub cannon_aim: f32,
    /// The catch device (ftCo_800C0764 from grOldKongo_8020F4E8, the
    /// stage's on_start) exists once the countdown releases the match.
    pub catch_enabled: bool,
}

impl OldKongo {
    /// grOldKongo_8020F46C's creation draws in map order: the barrel's
    /// (map 1), then the bird's (map 2). `cannon` is the cannon's world
    /// position at creation (stageGObj1_OnInit's Ground_801C4D70).
    pub fn create(parameters: Parameters, rng: &mut HsdRng) -> Self {
        let barrel = Barrel::create(&parameters, rng);
        let bird = Bird::create(&parameters, rng);
        Self {
            parameters,
            barrel,
            bird,
            cannon: Vec3::new(0.0, 0.0, 0.0),
            cannon_aim: 0.0,
            catch_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests;
