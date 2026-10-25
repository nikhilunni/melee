//! Onett (`gr/gronett.c`, GrOt.dat, StKind 0x09). Map 5 is the street with
//! the two springy awnings and the chimney smoke; map 4 the drugstore roof,
//! which collapses after enough landings and is rebuilt; map 3 the cars,
//! four models each with a stage material item that carries the hit, one
//! lane driving right to left through the stage (warned by a sign, framed by
//! a camera subject) and one in the background left to right.
pub mod cars;
pub mod procs;
use gekko_math::HsdRng;

/// `grOnett_StageParam` (gronett.c:64-91, GrOt.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x00: the push a landing gives an awning (`awning_initial`).
    pub awning_initial: f32,
    /// +0x04: the awning's top speed.
    pub max_velocity: f32,
    /// +0x08 / +0x0C: below these it settles.
    pub vel_threshold: f32,
    pub pos_threshold: f32,
    /// +0x10: the spring's damping.
    pub damping: f32,
    /// +0x14 / +0x18: the spring.
    pub spring_force: f32,
    pub spring_constant: f32,
    /// +0x1C: the awning's largest displacement.
    pub max_displacement: f32,
    /// +0x20: each caller's sag (`awning_delta`).
    pub awning_delta: f32,
    /// +0x24 / +0x28: the smoke's pause; +0x2C / +0x30: its puff.
    pub smoke_pause: [f32; 2],
    pub smoke_puff: [f32; 2],
    /// +0x34 / +0x38: landings that crack and break the roof.
    pub roof_crack: f32,
    pub roof_break: f32,
    /// +0x3C / +0x40: the broken roof's wait, `lo + (hi - lo) * rand`.
    pub roof_wait: [f32; 2],
    /// +0x44: a car's wait before it is announced.
    pub car_wait: i32,
    /// +0x48: the announcement's length.
    pub car_announce: i32,
    /// +0x4C / +0x50: the delay for a player who is not leading, and for
    /// the last.
    pub car_delay_middle: i32,
    pub car_delay_last: i32,
    /// +0x54 / +0x58: a car's speed, `x54 + x58 * rand`.
    pub car_speed_base: f32,
    pub car_speed_range: f32,
    /// +0x5C: a struck car's spin, degrees per tick.
    pub car_spin: f32,
    /// +0x60: the warning sign's ticks (un_802FD604).
    pub warning_ticks: i32,
    /// +0x64: the camera's right edge plus this is where the horn sounds.
    pub horn_lead: f32,
}

/// `rand_range(a, b)` (gr/inlines.h:193).
pub fn rand_range(a: i32, b: i32, rng: &mut HsdRng) -> i32 {
    let draw = |n: i32, rng: &mut HsdRng| if n != 0 { rng.randi(n) } else { 0 };
    if a > b {
        b + draw(a - b, rng)
    } else if a < b {
        a + draw(b - a, rng)
    } else {
        a
    }
}

/// One awning (`grOnett_Awning`, map 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Awning {
    /// The force last applied (`initial`).
    pub force: f32,
    /// The bone's translation y at creation.
    pub initial_y: f32,
    /// This tick's sag from the callers.
    pub accumulator: f32,
    /// The spring's position.
    pub velocity: f32,
    pub counter_prev: i32,
    pub counter: i32,
    /// A caller landed this tick.
    pub flag: bool,
    pub cooldown: i32,
}

impl Awning {
    pub fn new(initial_y: f32) -> Self {
        Self {
            force: 0.0,
            initial_y,
            accumulator: 0.0,
            velocity: 0.0,
            counter_prev: 0,
            counter: 0,
            flag: false,
            cooldown: 0,
        }
    }

    /// grOnett_801E54B4 (0x801E54B4), joints 0 and 1's floor callback, for
    /// a fighter or item (`coll->x34_flags.b1234` 1..3).
    pub fn touch(&mut self, ground_kind: i32, p: &Parameters) {
        if ground_kind == 1 {
            self.flag = true;
            self.force = p.awning_initial;
        }
        self.accumulator += p.awning_delta;
        self.counter += 1;
    }

    /// grOnett_801E5214 (0x801E5214) for one awning: the spring, then the
    /// bone's y. Returns the y and whether it puffs (grLib_801C9808(0x7533)).
    pub fn step(&mut self, p: &Parameters) -> (f32, bool) {
        use gekko_math::fma::{fmadds, fnmsubs};
        if self.accumulator < -p.max_displacement {
            self.accumulator = -p.max_displacement;
        }
        let displacement = self.accumulator;
        let error = displacement - self.velocity;
        let mut force = self.force + p.spring_constant;
        let magnitude = if error < 0.0 { -error } else { error };
        let ratio = magnitude / p.max_displacement;
        // 0x801E52A4: compared with a double 0.0.
        force = if f64::from(error) < 0.0 {
            // 0x801E52B4 fnmsubs.
            fnmsubs(p.spring_force, ratio, force)
        } else {
            // 0x801E52C0 fmadds.
            fmadds(p.spring_force, ratio, force)
        };
        force -= p.spring_constant;
        force *= 1.0 - p.damping;
        if force > p.max_velocity {
            force = p.max_velocity;
        } else if force < -p.max_velocity {
            force = -p.max_velocity;
        }
        let gap = self.velocity - displacement;
        let gap = if gap < 0.0 { -gap } else { gap };
        let speed = if force < 0.0 { -force } else { force };
        if gap < p.pos_threshold && speed < p.vel_threshold {
            self.velocity = displacement;
            self.force = 0.0;
        } else {
            self.force = force;
            self.velocity += force;
            let limit = p.max_displacement;
            if self.velocity > limit {
                self.velocity = limit;
            } else if self.velocity < -limit {
                self.velocity = -limit;
            }
        }
        let y = self.velocity + (self.initial_y + displacement);
        let puff = self.cooldown == 0 && (self.counter_prev != self.counter || self.flag);
        if puff {
            self.cooldown = 10;
        }
        self.accumulator = 0.0;
        self.counter_prev = self.counter;
        self.counter = 0;
        self.flag = false;
        if self.cooldown != 0 {
            self.cooldown -= 1;
        }
        (y, puff)
    }
}

/// The chimney smoke (map 5: x FC timer, x100 generator).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Smoke {
    pub timer: i32,
    pub puffing: bool,
}

/// What grOnett_801E5538 asks of the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmokeStep {
    Nothing,
    /// grLib_801C9834 on the generator.
    Stop,
    /// grLib_801C96F8(0x7530, 0x1E, bone 6 at (15.75, 40, 28.25)).
    Start,
}

impl Smoke {
    /// grOnett_801E3A34's `rand_range(x28, x24)`.
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            timer: pause(p, rng),
            puffing: false,
        }
    }
    /// grOnett_801E5538 (0x801E5538).
    pub fn step(&mut self, p: &Parameters, rng: &mut HsdRng) -> SmokeStep {
        let timer = self.timer;
        self.timer -= 1;
        if timer >= 0 {
            return SmokeStep::Nothing;
        }
        if self.puffing {
            self.timer = pause(p, rng);
            self.puffing = false;
            SmokeStep::Stop
        } else {
            self.timer = rand_range(
                gekko_math::msl::fctiwz(p.smoke_puff[1]),
                gekko_math::msl::fctiwz(p.smoke_puff[0]),
                rng,
            );
            self.puffing = true;
            SmokeStep::Start
        }
    }
}

fn pause(p: &Parameters, rng: &mut HsdRng) -> i32 {
    rand_range(
        gekko_math::msl::fctiwz(p.smoke_pause[1]),
        gekko_math::msl::fctiwz(p.smoke_pause[0]),
        rng,
    )
}

/// The drugstore roof (map 4, `GroundVars_onett_building`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roof {
    pub state: i32,
    pub next_state: i32,
    pub hit_count: i32,
    pub frame: i32,
    pub timer: i32,
}

/// What a roof step does to the stage.
pub trait RoofEngine {
    /// grAnime_801C83D0(gobj, 0, 7).
    fn animation_ended(&self) -> bool;
    /// grAnime_801C7FF8(gobj, 0, 7, index, 0.0, 1.0).
    fn play(&mut self, index: usize);
    /// mpLib_80057BC0 / mpJointListAdd on joint 3 or 4.
    fn set_solid(&mut self, joint: i32, solid: bool);
    /// HSD_JObjSetFlagsAll(bones 5, 2, 4, HIDDEN), or ClearFlagsAll on the
    /// whole model.
    fn blink(&mut self, hidden: bool);
}

impl Roof {
    /// grOnett_801E3CE4's state: -1, heading for 0.
    pub fn new() -> Self {
        Self {
            state: -1,
            next_state: 0,
            hit_count: 0,
            frame: 0,
            timer: 0,
        }
    }

    /// grOnett_801E40E4 (0x801E40E4), joints 3 and 4's floor callback.
    pub fn land(&mut self, ground_kind: i32, p: &Parameters) {
        if ground_kind != 1 {
            return;
        }
        self.hit_count += 1;
        match self.state {
            0 => {
                self.next_state = if (self.hit_count as f32) < p.roof_crack { 1 } else { 2 };
            }
            3 => {
                self.next_state = if (self.hit_count as f32) < p.roof_break { 4 } else { 5 };
            }
            _ => {}
        }
    }

    /// grOnett_801E3DA0 (0x801E3DA0) before its Ground_801C2FE0.
    pub fn step<E: RoofEngine>(&mut self, p: &Parameters, rng: &mut HsdRng, engine: &mut E) {
        self.frame += 1;
        match self.state {
            7 => {
                engine.blink(self.timer & 2 != 0);
                let timer = self.timer;
                self.timer -= 1;
                if timer < 0 {
                    engine.blink(false);
                    self.next_state = 0;
                }
            }
            1 => {
                if engine.animation_ended() {
                    self.next_state = 0;
                }
            }
            2 | 4 => {
                if engine.animation_ended() {
                    self.next_state = 3;
                }
            }
            5 => {
                if engine.animation_ended() {
                    let random = rng.randf();
                    let lo = p.roof_wait[0];
                    let range = p.roof_wait[1] - lo;
                    // 0x801E3F5C fmadds, then fctiwz.
                    self.timer = gekko_math::msl::fctiwz(gekko_math::fma::fmadds(range, random, lo));
                    self.next_state = 6;
                }
                if self.frame == 0x14 {
                    engine.set_solid(3, false);
                } else if self.frame == 0x5A {
                    engine.set_solid(4, false);
                }
            }
            6 => {
                let timer = self.timer;
                self.timer -= 1;
                if timer < 0 {
                    self.next_state = 7;
                    self.timer = 0x3C;
                }
            }
            _ => {}
        }
        if self.state != self.next_state {
            self.state = self.next_state;
            match self.state {
                7 => {
                    engine.play(0);
                    engine.set_solid(3, true);
                    engine.set_solid(4, true);
                }
                1 => engine.play(1),
                2 => engine.play(3),
                4 => engine.play(4),
                5 => engine.play(5),
                6 => self.hit_count = 0,
                _ => {}
            }
            self.frame = 0;
        }
    }
}

impl Default for Roof {
    fn default() -> Self {
        Self::new()
    }
}
