//! Whispy Woods (map 5, grGreens_802139C4 / 80213C10; map 4 plays the
//! same animations): idling, blowing (the wind device and a directional
//! field each blowing loop) and dropping apples, in the order grGr_803E7734
//! cycles through.
use super::{randrange, Parameters};
use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;
use melee_lb::radial_force::DirectionalGust;

/// `range != 0 ? HSD_Randi(range) : 0`.
pub fn randi_or_zero(range: i32, rng: &mut HsdRng) -> i32 {
    if range != 0 {
        rng.randi(range)
    } else {
        0
    }
}

/// grGr_803E7734: the modes in turn (0 idle, 1 blow, 2 apples).
const MODES: [i32; 10] = [0, 1, 0, 1, 0, 2, 0, 1, 0, 2];
/// grGr_803E775C (and grGr_803E77B4, the same for map 4): the idle
/// animation for each of the eight idle draws.
const IDLE: [usize; 8] = [0, 0, 0, 14, 0, 0, 0, 14];
/// grGr_803E777C (and 803E77D4): the blow's animations by phase and side.
const BLOW: [[usize; 2]; 7] = [[1, 8], [15, 16], [2, 9], [3, 10], [4, 11], [5, 12], [6, 13]];
/// The apples' animation.
const APPLES: usize = 7;
/// grGr_803E780C: the field's direction for each side.
const DIRECTIONS: [Vec3; 2] = [Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
/// lb_80011A50(dir, 0xF, 0.5, 0.0, 0.0, ...): each blowing loop's field.
const FIELD_FRAMES: i32 = 0xF;
const FIELD_STRENGTH: f32 = 0.5;
/// An apple's x reach either side and its depth.
const APPLE_REACH: f32 = 40.0;
const APPLE_DEPTH: f32 = -40.0;

/// Whispy's controller (`greens2`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Whispy {
    /// x0: the place in [`MODES`]; x4 its mode.
    pub turn: i32,
    pub mode: i32,
    /// x8: the idle draw, or the blow's phase.
    pub phase: i32,
    /// xC: a countdown or a loop count.
    pub timer: i32,
    /// x10: the mode just began.
    pub entering: bool,
    /// x14: the side he blows to (0 left, 1 right).
    pub side: i32,
    /// x18: the wind device's direction this tick (0 none, 1 left, 2 right).
    pub wind: i32,
    /// x20 / x24: apples left (dropping while not negative), their side.
    pub apples: i32,
    pub apple_side: i32,
}

/// What a Whispy step asks of the stage, in retail order.
pub trait WhispyEngine {
    /// grAnime_801C8138 on map 5, then on map 4.
    fn play(&mut self, animation: usize);
    /// grAnime_801C83D0(gobj, 0, 7) / grAnime_801C84A4(gobj, 0, 7) on map 5.
    fn ended(&self) -> bool;
    fn rewound(&self) -> bool;
    /// ftLib_800864A8(map 5's translation, NULL) == 1.0 (may draw).
    fn fighters_to_the_right(&mut self) -> bool;
    /// The shared random stream.
    fn rng(&mut self) -> &mut HsdRng;
    /// Camera_RequestQuake(Loop).
    fn quake(&mut self);
    /// lb_80011A50.
    fn field(&mut self, gust: DirectionalGust);
    /// it_802EE200(gobj, &pos, x74, x78).
    fn apple(&mut self, position: Vec3);
}

impl Whispy {
    /// grGreens_802139C4 (0x802139C4): two draws (the idle wait, then the
    /// side).
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        let timer = randrange(p.idle_wait[1], p.idle_wait[0], rng);
        Self {
            turn: 0,
            mode: 0,
            phase: 0,
            timer,
            entering: true,
            side: rng.randi(2),
            wind: 0,
            apples: 0,
            apple_side: 0,
        }
    }

    fn next_mode(&mut self) {
        self.turn = (self.turn + 1) % 10;
        self.mode = MODES[self.turn as usize];
        self.entering = true;
    }

    /// The blow's field over the side he faces.
    fn field(&self, p: &Parameters) -> DirectionalGust {
        let side = self.side as usize;
        let rectangle = if side == 0 {
            [-p.wind_right, p.wind_top, -p.wind_left, p.wind_bottom]
        } else {
            [p.wind_left, p.wind_top, p.wind_right, p.wind_bottom]
        };
        DirectionalGust {
            direction: DIRECTIONS[side],
            frames: FIELD_FRAMES,
            strength: FIELD_STRENGTH,
            decay: 0.0,
            phase_step: 0.0,
            rectangle,
        }
    }

    fn play_blow<E: WhispyEngine>(&self, engine: &mut E) {
        engine.play(BLOW[self.phase as usize][self.side as usize]);
    }

    /// grGreens_80213C10 (0x80213C10), once the stage started.
    pub fn step<E: WhispyEngine>(&mut self, p: &Parameters, engine: &mut E) {
        match self.mode {
            0 => self.idle(p, engine),
            1 => self.blow(p, engine),
            2 => self.drop_apples(p, engine),
            _ => {}
        }
    }

    fn idle<E: WhispyEngine>(&mut self, p: &Parameters, engine: &mut E) {
        if self.entering {
            self.entering = false;
            self.phase = engine.rng().randi(8);
            self.timer = randrange(p.idle_wait[1], p.idle_wait[0], engine.rng());
            engine.play(IDLE[self.phase as usize]);
            return;
        }
        self.timer -= 1;
        if engine.rewound() {
            if self.timer > 0 {
                self.phase = engine.rng().randi(8);
                engine.play(IDLE[self.phase as usize]);
                return;
            }
            self.next_mode();
        }
    }

    fn blow<E: WhispyEngine>(&mut self, p: &Parameters, engine: &mut E) {
        if self.entering {
            self.entering = false;
            self.phase = 0;
            self.side = i32::from(engine.fighters_to_the_right());
            self.timer = 0;
            self.play_blow(engine);
            return;
        }
        match self.phase {
            0 | 1 | 5 => {
                if engine.ended() {
                    self.phase += 1;
                    self.play_blow(engine);
                }
            }
            2 => {
                if engine.rewound() {
                    self.timer += 1;
                    if self.timer as f32 > p.inhale_loops {
                        self.phase += 1;
                        self.timer = 0;
                        self.play_blow(engine);
                    }
                }
            }
            3 => {
                if engine.ended() {
                    self.phase += 1;
                    // Map 5's animation, the field, then map 4's.
                    self.play_blow(engine);
                    engine.field(self.field(p));
                }
            }
            4 => {
                engine.quake();
                self.wind = self.side + 1;
                if engine.rewound() {
                    self.timer += 1;
                    let timer = self.timer as f32;
                    if timer > p.blow_loops
                        || (timer > p.blow_turn_loops
                            && self.side != i32::from(engine.fighters_to_the_right()))
                    {
                        self.phase += 1;
                        self.timer = 0;
                        self.play_blow(engine);
                        return;
                    }
                    engine.field(self.field(p));
                }
            }
            6 if engine.ended() => self.next_mode(),
            _ => {}
        }
    }

    fn drop_apples<E: WhispyEngine>(&mut self, p: &Parameters, engine: &mut E) {
        if self.entering {
            self.entering = false;
            self.phase = 0;
            self.timer = 0;
            engine.play(APPLES);
            self.apples = randrange(p.apple_count[1], p.apple_count[0] - 1, engine.rng());
            self.apple_side = engine.rng().randi(2);
        } else if engine.ended() {
            if self.apples < 0 {
                self.next_mode();
            } else {
                engine.play(APPLES);
            }
        } else if self.apples >= 0
            && self.timer >= p.apple_start
            && (self.timer - p.apple_start) % p.apple_interval == 0
        {
            let sign = if self.apple_side == 0 { -1.0 } else { 1.0 };
            let x = sign * (APPLE_REACH * engine.rng().randf());
            let random = engine.rng().randf();
            // retail 0x802145F0: fmadds.
            let y = fmadds(p.apple_height[1] - p.apple_height[0], random, p.apple_height[0]);
            engine.apple(Vec3::new(x, y, APPLE_DEPTH));
            self.apples -= 1;
            self.apple_side = (self.apple_side + 1) & 1;
        }
        self.timer += 1;
    }
}
