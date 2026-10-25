//! Brinstar Depths (`gr/grkraid.c`, GrKr.dat, StKind 0x0F). Kraid (map 4,
//! with his effects on map 1) rises at one of six places, roars and swipes;
//! a swipe that connects in its window turns the whole stage (map 3, whose
//! six collision joints sit on bone 12 under the rotated root).
pub mod procs;
use gekko_math::HsdRng;

/// `grKraid_YakumonoParam` (grkraid.c:14-23).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    pub map_time_min: i32,
    pub map_time_max: i32,
    /// The ticks the turn takes to brake (`map_time_acl`).
    pub map_time_acl: i32,
    pub map_rot_spd_min: f32,
    pub map_rot_spd_max: f32,
    pub kraid_wait_time: i32,
    pub kraid_wait_time_add: i32,
    pub kraid_pos_x: [f32; 6],
}

/// grkraid.c:801FE4C4: `rand_range(max, min)` (gr/inlines.h).
fn rand_range(a: i32, b: i32, rng: &mut HsdRng) -> i32 {
    let draw = |n: i32, rng: &mut HsdRng| if n != 0 { rng.randi(n) } else { 0 };
    if a > b {
        b + draw(a - b, rng)
    } else if a < b {
        a + draw(b - a, rng)
    } else {
        a
    }
}

/// The stage's turn (`grKraid_GroundVars` on map 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    /// x0: 0 still, 1 turning.
    pub turning: bool,
    /// x1: animation 1 plays on bone 18's subtree.
    pub shaking: bool,
    /// x4: the swipe's direction (+1 or -1), 0 while still.
    pub direction: f32,
    /// x8: degrees per tick.
    pub speed: f32,
    /// xC: braking per tick.
    pub braking: f32,
    /// x10: ticks before the braking starts.
    pub delay: i32,
}

/// The stage proc's engine half.
pub trait TurnEngine {
    /// grAnime_801C7FF8(gobj, 18, 7, index, 0.0, 1.0).
    fn play_shake(&mut self, index: usize);
    /// grAnime_801C83D0(gobj, 18, 7).
    fn shake_ended(&self) -> bool;
    /// HSD_JObjAddRotationZ on bone 0, then Camera_RequestQuake(Loop).
    fn turn(&mut self, radians: f32);
}

/// Animation 1 shakes the stage while it turns; 0 is the still pose.
pub const SHAKE: usize = 1;
pub const STILL: usize = 0;
/// `3.14159265f / 180` (0x801FE5D0's constant; 3.14159265f is
/// `f32::consts::PI`).
const DEGREES: f32 = core::f32::consts::PI / 180.0;
/// grKraid_801FE6D8: the speed per unit of the hand's distance from the
/// middle is `(max / min) / 132 * min`.
const SPEED_SCALE: f32 = 132.0;

impl Turn {
    pub fn new() -> Self {
        Self {
            turning: false,
            shaking: false,
            direction: 0.0,
            speed: 0.0,
            braking: 0.0,
            delay: 0,
        }
    }

    /// grKraid_801FE6D8 (0x801FE6D8): a swipe from a hand at `hand_x`
    /// starts a turn unless one is running. Returns whether it did (its
    /// sound and Camera_RequestQuake(Medium) at the hand follow).
    pub fn swipe(&mut self, parameters: &Parameters, hand_x: f32, direction: f32) -> bool {
        if self.direction != 0.0 {
            return false;
        }
        self.direction = direction;
        let (min, max) = (parameters.map_rot_spd_min, parameters.map_rot_spd_max);
        // retail 0x801FE750..0x801FE76C: fdivs, fdivs, fmuls, fmuls.
        let scale = max / min / SPEED_SCALE * min;
        let distance = if hand_x < 0.0 { -hand_x } else { hand_x };
        self.speed = scale * distance;
        if self.speed < min {
            self.speed = min;
        }
        if self.speed > max {
            self.speed = max;
        }
        // retail 0x801FE7B8: fmuls; 0x801FE7E4: fdivs by the int as float.
        self.speed *= self.direction;
        self.braking = self.speed / parameters.map_time_acl as f32;
        true
    }

    /// grKraid_801FE440 (0x801FE440) before its lb_800115F4 and
    /// Ground_801C2FE0.
    pub fn tick(&mut self, parameters: &Parameters, rng: &mut HsdRng, engine: &mut impl TurnEngine) {
        if !self.turning {
            if self.direction != 0.0 {
                self.delay = rand_range(parameters.map_time_max, parameters.map_time_min, rng);
                self.turning = true;
                engine.play_shake(SHAKE);
                // Ground_801C53EC(420007): sound only.
                self.shaking = true;
            }
        } else {
            if self.delay == 0 {
                // retail 0x801FE560: fsubs.
                self.speed -= self.braking;
            } else {
                self.delay -= 1;
            }
            let braking = if self.braking < 0.0 { -self.braking } else { self.braking };
            let speed = if self.speed < 0.0 { -self.speed } else { self.speed };
            if speed < braking {
                self.braking = 0.0;
                self.speed = 0.0;
                self.direction = 0.0;
                self.turning = false;
            } else {
                // retail 0x801FE5D0: fmuls.
                engine.turn(self.speed * DEGREES);
            }
        }
        if self.shaking && engine.shake_ended() {
            engine.play_shake(STILL);
            self.shaking = false;
        }
    }
}

impl Default for Turn {
    fn default() -> Self {
        Self::new()
    }
}

/// Kraid's phase (`grKraid_GroundVars2.x0` on map 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KraidPhase {
    Hidden = 0,
    Rising = 1,
    Roaring = 2,
    Swiping = 3,
    Sinking = 4,
}
impl KraidPhase {
    pub fn from_saved(value: i8) -> Option<Self> {
        Some(match value {
            0 => Self::Hidden,
            1 => Self::Rising,
            2 => Self::Roaring,
            3 => Self::Swiping,
            4 => Self::Sinking,
            _ => return None,
        })
    }
}

/// Kraid (`grKraid_GroundVars2`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kraid {
    pub phase: KraidPhase,
    /// x1: the animation Kraid and his effects play (grKraid_801FF068).
    pub animation: i8,
    /// x2: the place he last rose at.
    pub place: i8,
    /// x3, x4: the leftmost and rightmost places.
    pub leftmost: i8,
    pub rightmost: i8,
    /// x5: the next rise may be anywhere but the last place.
    pub anywhere: bool,
    /// x8: 1.0 swipes right (animation 6), 0.0 left (5).
    pub side: f32,
    /// xC.
    pub timer: i32,
}

/// Animations of Kraid and his effects (maps 4 and 1).
const RISE: i8 = 3;
const SINK: i8 = 4;
const SWIPE_5: i8 = 5;
const ROARS: i32 = 3;
/// Each swipe's window and the tick its hand lands from
/// (grKraid_801FF0E0: 0x9E for animation 5, 0x82 for 6).
const SWIPE_5_WINDOW: (i32, i32, i32) = (147, 169, 0x9E);
const SWIPE_6_WINDOW: (i32, i32, i32) = (123, 138, 0x82);
/// The hands (x10, x14): bone 31 turns the stage one way (+1), 47 the
/// other (-1).
pub const RIGHT_HAND: usize = 31;
pub const LEFT_HAND: usize = 47;

/// What one Kraid step asks of maps 4 and 1, in C order: the rise, a
/// hand landing (grKraid_801FE6D8), then a new animation or the hiding.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KraidStep {
    /// grKraid_801FEE54 moved both models to this x; both are shown.
    pub rise: Option<f32>,
    /// grKraid_801FE6D8(hand, direction).
    pub swipe: Option<(usize, f32)>,
    /// grKraid_801FF068(animation) on maps 4 and 1.
    pub play: Option<i8>,
    /// Both models hidden.
    pub hide: bool,
}

impl Kraid {
    /// grKraid_801FE818 (0x801FE818): the outermost places and one wait.
    pub fn create(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        let mut leftmost = 0;
        let mut rightmost = 0;
        let mut lowest = 0.0;
        let mut highest = 0.0;
        for (index, &x) in parameters.kraid_pos_x.iter().enumerate() {
            if x < lowest {
                leftmost = index as i8;
                lowest = x;
            } else if x > highest {
                highest = x;
                rightmost = index as i8;
            }
        }
        Self {
            phase: KraidPhase::Hidden,
            animation: 0,
            place: 0,
            leftmost,
            rightmost,
            anywhere: true,
            side: 0.0,
            timer: Self::wait(parameters, rng),
        }
    }

    fn wait(parameters: &Parameters, rng: &mut HsdRng) -> i32 {
        let add = parameters.kraid_wait_time_add;
        parameters.kraid_wait_time + if add != 0 { rng.randi(add) } else { 0 }
    }

    /// grKraid_801FEE54 (0x801FEE54): where he rises, and which way he
    /// swipes.
    fn choose_place(&mut self, parameters: &Parameters, rng: &mut HsdRng) -> f32 {
        let place = if self.anywhere {
            loop {
                let place = rng.randi(6);
                if i32::from(self.place) != place {
                    break place as i8;
                }
            }
        } else if rng.randi(2) != 0 {
            self.leftmost
        } else {
            self.rightmost
        };
        self.anywhere = false;
        self.place = place;
        let x = parameters.kraid_pos_x[place as usize];
        self.side = if x == 0.0 {
            if rng.randi(2) != 0 {
                1.0
            } else {
                0.0
            }
        } else if x < 0.0 {
            0.0
        } else {
            1.0
        };
        x
    }

    /// grKraid_801FEA00 (0x801FEA00). `turning` is map 3's
    /// `(int) x4 != 0`; `ended` is grAnime_801C83D0(gobj, 0, 7) on map 4.
    pub fn tick(
        &mut self,
        parameters: &Parameters,
        turning: bool,
        ended: bool,
        rng: &mut HsdRng,
    ) -> KraidStep {
        let mut step = KraidStep::default();
        match self.phase {
            KraidPhase::Hidden => {
                if self.timer <= 0 {
                    if !turning {
                        step.rise = Some(self.choose_place(parameters, rng));
                        self.play(RISE, &mut step);
                        // Ground_801C53EC(420001), (420008): sounds only.
                        self.phase = KraidPhase::Rising;
                    }
                } else {
                    self.timer -= 1;
                }
            }
            KraidPhase::Rising => {
                if ended {
                    let roar = rng.randi(ROARS) as i8;
                    self.play(roar, &mut step);
                    self.timer = 0;
                    self.phase = KraidPhase::Roaring;
                }
            }
            KraidPhase::Roaring => {
                // Sounds at fixed ticks of each roar (Ground_801C53EC,
                // Ground_801C5440/5544): no game state.
                if ended {
                    // retail: fadds 5.0, fctiwz into grKraid_801FF068's int.
                    self.play(gekko_math::msl::fctiwz(self.side + 5.0) as i8, &mut step);
                    self.timer = 0;
                    self.phase = KraidPhase::Swiping;
                }
                self.timer += 1;
            }
            KraidPhase::Swiping => {
                let (window, hand, direction) = if self.animation == SWIPE_5 {
                    (SWIPE_5_WINDOW, LEFT_HAND, -1.0)
                } else {
                    (SWIPE_6_WINDOW, RIGHT_HAND, 1.0)
                };
                let (first, last, lands) = window;
                // Ground_801C53EC(420003) at the window's first tick. The
                // landing's rumble (ftLib_80086C9C) is controller output.
                if !turning && self.timer >= first && self.timer <= last && self.timer >= lands {
                    self.anywhere = true;
                    self.side = 1.0;
                    step.swipe = Some((hand, direction));
                }
                self.timer += 1;
                if ended {
                    self.play(SINK, &mut step);
                    // Ground_801C53EC(420002), (420008): sounds only.
                    self.phase = KraidPhase::Sinking;
                }
            }
            KraidPhase::Sinking => {
                if ended {
                    self.timer = Self::wait(parameters, rng);
                    self.phase = KraidPhase::Hidden;
                    step.hide = true;
                }
            }
        }
        step
    }

    /// grKraid_801FF068 (0x801FF068): x1 of maps 4 and 1.
    fn play(&mut self, animation: i8, step: &mut KraidStep) {
        self.animation = animation;
        step.play = Some(animation);
    }
}
