//! The acid (map 8): grZebes_801D9798 (creation) and grZebes_801D99E0
//! (its proc). The acid waits out a level's delay (shaking the camera for
//! its last 120 ticks), draws its next height, accelerates toward it up
//! to a top speed that lets it brake in time, brakes, and takes the next
//! row of the table.
use super::{AcidLevel, Parameters, ACID_LEVELS, ACID_SURFACE_OFFSET};
use gekko_math::{
    fma::{fmadds, fnmsub},
    msl::sqrtf,
    HsdRng,
};

/// The acid's phase (`grZebes_GroundVars.xC8`, an s32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcidPhase {
    /// Created: nothing until the stage starts (grZebes_801D9758).
    Idle = 0,
    /// Counting down the level's delay.
    Waiting = 1,
    /// The target was drawn this tick; it starts next tick.
    Starting = 2,
    /// Accelerating toward the target.
    Moving = 3,
    /// Braking.
    Stopping = 4,
}

impl AcidPhase {
    pub fn from_saved(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::Idle,
            1 => Self::Waiting,
            2 => Self::Starting,
            3 => Self::Moving,
            4 => Self::Stopping,
            _ => return None,
        })
    }
}

/// The acid (`u.zebes5` on map 8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Acid {
    /// xC4 (s16): the level table's row.
    pub row: i16,
    /// xC6 (s16): the delay while waiting, ticks moved while moving.
    pub timer: i16,
    /// xC8.
    pub phase: AcidPhase,
    /// xCC: the top speed for this move.
    pub max_speed: f32,
    /// xD0: the speed.
    pub speed: f32,
    /// xD4: the height it moves to.
    pub target: f32,
    /// xD8: the height (map 8's bone 0 translation y).
    pub level: f32,
}

/// The waiting acid shakes the camera below this many ticks left.
const WARNING_TICKS: i16 = 0x78;
/// The moving acid shakes the camera for its first 60 ticks.
const MOVING_QUAKE_TICKS: i16 = 60;
/// grZebes_801D99E0: a move with no real root has no speed limit.
const UNLIMITED: f32 = 999.0;

/// The level's delay: `rand_range(max, min)` plus the row's base
/// (grZebes_801D9798 / 801D99E0's state 4).
fn delay(row: &AcidLevel, rng: &mut HsdRng) -> i16 {
    let delay = crate::onett::rand_range(i32::from(row.delay_max), i32::from(row.delay_min), rng);
    (i32::from(row.base) + delay) as i16
}

/// The camera shake the acid asks for (Camera_RequestQuake(Loop)).
pub trait Quake {
    fn quake(&mut self);
}

impl Acid {
    /// grZebes_801D9798 (0x801D9798): row 0's delay, then the starting
    /// height from the table's last row (0x801D98F8 fmadds).
    pub fn create(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        let levels = &parameters.acid_levels;
        let timer = delay(&levels[0], rng);
        let mut last = 0;
        while last < ACID_LEVELS - 1 && !levels[last + 1].is_end() {
            last += 1;
        }
        let level = fmadds(
            parameters.acid_spread,
            rng.randf(),
            f32::from(levels[last].level),
        );
        Self {
            row: 0,
            timer,
            phase: AcidPhase::Idle,
            max_speed: 0.0,
            speed: 0.0,
            target: level,
            level,
        }
    }

    /// `-55.0 + level` (0x801D9F00 fadds): what Ground_801C438C publishes
    /// as the stage's floor.
    pub fn surface(&self) -> f32 {
        ACID_SURFACE_OFFSET + self.level
    }

    /// grZebes_801D9758, deferred to the stage's start: the countdown
    /// begins.
    pub fn start(&mut self) {
        self.phase = AcidPhase::Waiting;
    }

    /// grZebes_801D99E0 (0x801D99E0) up to its colour updates; the caller
    /// then sets bone 0's height and publishes [`Self::surface`].
    pub fn tick(&mut self, parameters: &Parameters, rng: &mut HsdRng, quake: &mut impl Quake) {
        match self.phase {
            AcidPhase::Idle => {}
            AcidPhase::Waiting => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer < WARNING_TICKS {
                    quake.quake();
                }
                if self.timer < 0 {
                    self.draw_target(parameters, rng);
                }
            }
            AcidPhase::Starting => self.phase = AcidPhase::Moving,
            AcidPhase::Moving => self.accelerate(parameters, quake),
            AcidPhase::Stopping => self.brake(parameters, rng),
        }
    }

    /// The end of the wait: the target, clamped to the table's range, and
    /// the top speed that still brakes in time.
    fn draw_target(&mut self, parameters: &Parameters, rng: &mut HsdRng) {
        let row = parameters.acid_levels[self.row as usize];
        // retail 0x801D9AA0: fmadds.
        self.target = fmadds(parameters.acid_spread, rng.randf(), f32::from(row.level));
        if self.target > parameters.acid_top {
            self.target = parameters.acid_top;
        } else if self.target < parameters.acid_bottom {
            self.target = parameters.acid_bottom;
        }
        self.speed = 0.0;
        self.phase = AcidPhase::Starting;
        self.timer = 0;
        let mut distance = self.target - self.level;
        if distance < 0.0 {
            distance = -distance;
        }
        // retail 0x801D9B0C..0x801D9B24: fmuls, fdivs, fmuls, fsubs.
        let top = parameters.acid_max_speed;
        let discriminant = top * top - 4.0 * distance / parameters.acid_acceleration;
        if discriminant < 0.0 {
            self.max_speed = UNLIMITED;
        } else {
            let root = sqrtf(discriminant);
            // retail 0x801D9BAC fmuls, 0x801D9BB0 fmul by the double 0.5.
            let product = parameters.acid_acceleration * (top - root);
            self.max_speed = (0.5 * f64::from(product)) as f32;
            if self.max_speed < 0.0 {
                self.max_speed = UNLIMITED;
            }
        }
        // Ground_801C53EC(400000 rising / 400001 falling): sounds only.
    }

    /// The moving phase: brake once the distance left is within the
    /// braking distance (or one acceleration step).
    fn accelerate(&mut self, parameters: &Parameters, quake: &mut impl Quake) {
        self.timer = self.timer.wrapping_add(1);
        if self.timer < MOVING_QUAKE_TICKS {
            quake.quake();
        }
        let acceleration = parameters.acid_acceleration;
        let speed = self.speed;
        // retail 0x801D9C44 fdivs, 0x801D9C54 fmul (double 0.5), 0x801D9C5C /
        // 0x801D9C60 fmuls, 0x801D9C6C fnmsub (double), frsp.
        let ticks = speed / acceleration;
        let braking = fnmsub(
            0.5 * f64::from(acceleration),
            f64::from(ticks * ticks),
            f64::from(speed * ticks),
        ) as f32;
        let distance = self.target - self.level;
        let distance = if distance < 0.0 { -distance } else { distance };
        if distance < braking || distance < acceleration {
            self.phase = AcidPhase::Stopping;
        } else {
            self.speed += acceleration;
            if self.speed > self.max_speed {
                self.speed = self.max_speed;
            }
        }
        self.advance();
    }

    /// The braking phase; once stopped, the next row's delay.
    fn brake(&mut self, parameters: &Parameters, rng: &mut HsdRng) {
        let distance = self.target - self.level;
        self.speed -= parameters.acid_acceleration;
        let left = if distance < 0.0 { -distance } else { distance };
        if self.speed < 0.0 || left < self.speed {
            self.phase = AcidPhase::Waiting;
            self.speed = 0.0;
            self.row += 1;
            if self.row as usize == ACID_LEVELS
                || parameters.acid_levels[self.row as usize].is_end()
            {
                self.row = 0;
            }
            self.timer = delay(&parameters.acid_levels[self.row as usize], rng);
        }
        self.advance();
    }

    fn advance(&mut self) {
        if self.target > self.level {
            self.level += self.speed;
        } else {
            self.level -= self.speed;
        }
    }
}
