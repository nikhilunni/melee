//! Onett's cars (map 3, grOnett_801E43E0): lane A drives right to left
//! through the stage with a hitbox, lane B left to right in the background.
use super::Parameters;
use gekko_math::HsdRng;

/// The cars' stage-material item states: 0 harmless, `car + 2` lane A's hit.
pub const HARMLESS: u16 = 0;
/// Where lane A starts and both lanes end.
pub const RIGHT_END: f32 = 726.0;
pub const LEFT_END: f32 = -642.0;
/// Lane A's depth and lane B's.
pub const LANE_A_Z: f32 = 30.0;
pub const LANE_B_Z: f32 = 56.0;
/// Lane A's warning point and lane B's.
const LANE_A_WARNING_X: f32 = 580.0;
const LANE_B_SOUND_X: f32 = -513.0;
/// A fighter at or below this height is on the street.
const STREET_HEIGHT: f32 = 1.0;

/// `GroundVars_onettcar` (map 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cars {
    /// x0 b0: set until the stage starts (grOnett_801E41B0 clears it).
    pub waiting: bool,
    /// Lane A's car and lane B's (`curr_car`, `next_car`; 4 is none).
    pub current: i8,
    pub next: i8,
    pub state_a: i8,
    pub state_b: i8,
    /// x108: lane A's timer; x110 its approach phase.
    pub timer_a: i32,
    pub approach: i32,
    pub speed_a: f32,
    pub timer_b: i32,
    pub speed_b: f32,
    pub sub_state_b: i32,
}

/// What a car step asks of the stage, in retail order.
pub trait CarEngine {
    fn translate_x(&self, car: usize) -> f32;
    fn set_translate_x(&mut self, car: usize, x: f32);
    fn set_translate_z(&mut self, car: usize, z: f32);
    /// The car's child joint (its body) about y.
    fn set_body_rotation_y(&mut self, car: usize, radians: f32);
    fn add_body_rotation_y(&mut self, car: usize, radians: f32);
    fn set_hidden(&mut self, car: usize, hidden: bool);
    /// Item_80268E5C(car's item, state, ITEM_ANIM_UPDATE).
    fn item_state(&mut self, car: usize, state: u16);
    /// grLib_801C9808(0x2C, 0, car_jobjs2[car]): a struck car's smoke.
    fn smoke(&mut self, car: usize);
    /// Camera_800307D0's (left, center, right).
    fn ground_view(&self) -> (f32, f32, f32);
    /// Each fighter's y and standing (gm_8016C6C0), in GObj list order.
    fn fighters(&self) -> &[(f32, i32)];
    /// Ground_801C5784: lane A's camera subject turns on at 1 and 2.
    fn set_camera_phase(&mut self, phase: i32);
    /// The shared random stream (the smoke's particles draw from it too).
    fn rng(&mut self) -> &mut HsdRng;
}

impl Cars {
    /// grOnett_801E41C8 (0x801E41C8): HSD_Randi(4), overwritten at once.
    pub fn new(rng: &mut HsdRng) -> Self {
        rng.randi(4);
        Self {
            waiting: true,
            current: 4,
            next: 4,
            state_a: 0,
            state_b: 0,
            timer_a: 0,
            approach: 0,
            speed_a: 0.0,
            timer_b: 0,
            speed_b: 0.0,
            sub_state_b: 0,
        }
    }

    /// grOnett_801E5030 (0x801E5030), the cars' damage-dealt callback.
    pub fn struck_something(&mut self, rng: &mut HsdRng) {
        if self.state_a == 4 {
            // HSD_Randi(2) picks the horn.
            rng.randi(2);
        } else if rng.randi(2) != 0 {
            let random = rng.randf();
            self.speed_a += 0.5 + random;
            let speed = if self.speed_a < 0.0 { -self.speed_a } else { self.speed_a };
            if speed < 0.5 {
                self.speed_a = 0.5;
            }
            self.state_a = 4;
            self.approach = 4;
            self.timer_a = 1;
            // The horn: HSD_Randi(2), then Ground_801C5544 and the sound.
            rng.randi(2);
        } else {
            self.state_a = 6;
        }
    }

    /// grOnett_801E43E0 (0x801E43E0).
    pub fn step<E: CarEngine>(&mut self, p: &Parameters, engine: &mut E) {
        if self.waiting {
            return;
        }
        let (_, center, right) = engine.ground_view();
        let saved = self.current;
        if !self.step_a(saved, p, center, right, engine) {
            return;
        }
        self.step_b(p, engine);
    }

    /// Lane A; false when the step returns early.
    fn step_a<E: CarEngine>(
        &mut self,
        saved: i8,
        p: &Parameters,
        center: f32,
        right: f32,
        engine: &mut E,
    ) -> bool {
        let car = saved as usize;
        match self.state_a {
            0 => {
                let old = self.current;
                while self.current == old || self.current == self.next {
                    self.current = engine.rng().randi(4) as i8;
                }
                let car = self.current as usize;
                engine.set_translate_x(car, RIGHT_END);
                engine.set_translate_z(car, LANE_A_Z);
                engine.set_body_rotation_y(car, 0.0);
                self.state_a = 1;
                self.timer_a = p.car_wait;
                engine.set_camera_phase(0);
            }
            1 => {
                if self.timer_a != 0 {
                    self.timer_a -= 1;
                } else {
                    self.timer_a = p.car_announce;
                    let random = engine.rng().randf();
                    // 0x801E46C8 fmadds.
                    self.speed_a = gekko_math::fma::fmadds(p.car_speed_range, random, p.car_speed_base);
                    if self.speed_a > 0.0 {
                        self.speed_a *= -1.0;
                    }
                    self.state_a = 2;
                    engine.item_state(car, saved as u16 + 2);
                }
            }
            2 => {
                if self.timer_a != 0 {
                    let count = engine.fighters().len() as i32;
                    self.timer_a -= 1;
                    for fighter in 0..count as usize {
                        let (y, rank) = engine.fighters()[fighter];
                        if y <= STREET_HEIGHT {
                            if count == 1 || rank == 0 {
                                self.timer_a = 0;
                                engine.set_hidden(car, false);
                                self.state_a = 5;
                            } else if count == rank + 1 {
                                self.timer_a = p.car_delay_last;
                                self.state_a = 3;
                            } else {
                                self.timer_a = p.car_delay_middle;
                                self.state_a = 3;
                            }
                            self.approach = 0;
                        }
                    }
                } else {
                    self.timer_a = 0;
                    self.approach = 0;
                    engine.set_hidden(car, false);
                    self.state_a = 5;
                    engine.set_camera_phase(1);
                }
            }
            3 => {
                if self.timer_a != 0 {
                    self.timer_a -= 1;
                } else {
                    self.timer_a = 0;
                    self.approach = 0;
                    engine.set_hidden(car, false);
                    self.state_a = 5;
                    engine.set_camera_phase(1);
                }
            }
            4 => {
                engine.add_body_rotation_y(car, 0.017_453_292 * p.car_spin);
                engine.smoke(car);
                let x = engine.translate_x(car) + self.speed_a;
                engine.set_translate_x(car, x);
                if engine.translate_x(car) <= LEFT_END {
                    self.finish_a(car, engine);
                    return false;
                }
            }
            5 | 6 => {
                let x = engine.translate_x(car) + self.speed_a;
                engine.set_translate_x(car, x);
                match self.approach {
                    0 => {
                        if engine.translate_x(car) <= LANE_A_WARNING_X {
                            // grOnett_801E5194's sound, un_802FD604's sign.
                            self.approach = 1;
                        }
                    }
                    // Case 1 falls through into case 2's test.
                    1 | 2 => {
                        if self.approach == 1
                            && engine.translate_x(car) <= right + p.horn_lead
                            && engine.fighters().iter().any(|&(y, _)| y <= STREET_HEIGHT)
                        {
                            // The horn: HSD_Randi(3), then the sound.
                            engine.rng().randi(3);
                            self.approach = 2;
                            engine.set_camera_phase(2);
                        }
                        if engine.translate_x(car) <= 100.0 + center {
                            self.approach = 4;
                        }
                    }
                    _ => {}
                }
                if engine.translate_x(car) <= LEFT_END {
                    self.finish_a(car, engine);
                    return false;
                }
            }
            _ => {}
        }
        true
    }

    fn finish_a<E: CarEngine>(&mut self, car: usize, engine: &mut E) {
        engine.set_hidden(car, true);
        self.state_a = 0;
        engine.item_state(car, HARMLESS);
    }

    /// Lane B.
    fn step_b<E: CarEngine>(&mut self, p: &Parameters, engine: &mut E) {
        let car = self.next as usize;
        match self.state_b {
            0 => {
                let next = self.next;
                while self.next == next || self.next == self.current {
                    self.next = engine.rng().randi(4) as i8;
                }
                let car = self.next as usize;
                engine.set_translate_x(car, LEFT_END);
                engine.set_translate_z(car, LANE_B_Z);
                engine.set_body_rotation_y(car, core::f64::consts::PI as f32);
                self.state_b = 1;
                self.timer_b = p.car_wait;
            }
            1 => {
                if self.timer_b != 0 {
                    self.timer_b -= 1;
                    return;
                }
                self.timer_b = p.car_announce;
                let random = engine.rng().randf();
                // 0x801E4E98 fmadds.
                self.speed_b = gekko_math::fma::fmadds(p.car_speed_range, random, p.car_speed_base);
                self.state_b = 2;
            }
            2 => {
                if self.timer_b != 0 {
                    self.timer_b -= 1;
                    return;
                }
                self.timer_b = 0;
                engine.set_hidden(car, false);
                self.sub_state_b = 0;
                self.state_b = 5;
                engine.item_state(car, HARMLESS);
            }
            5 => {
                let x = engine.translate_x(car) + self.speed_b;
                engine.set_translate_x(car, x);
                if self.sub_state_b == 0 && engine.translate_x(car) >= LANE_B_SOUND_X {
                    // grOnett_801E5194's sound.
                    self.sub_state_b = 4;
                }
                if engine.translate_x(car) >= RIGHT_END {
                    engine.set_hidden(car, true);
                    self.state_b = 0;
                }
            }
            _ => {}
        }
    }
}
