//! Dream Land N64, gr/groldpupupu.c (NTSC 1.02).
pub mod flyby;
pub mod procs;
use crate::wind::{Gust, Wind};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_lb::radial_force::DirectionalGust;

// Animation tables grOp_804D48C8/D0/D8, indexed by left/right facing.
const BLINK_ANIMATIONS: [usize; 2] = [4, 0];
const BLOW_ANIMATIONS: [usize; 2] = [5, 1];
const TURN_ANIMATIONS: [usize; 2] = [2, 3];
// grOldPupupu_802113E0: wind is enabled strictly between these timer values.
const WIND_START: i32 = 45;
const WIND_END: i32 = 320;
// grOldPupupu_802113E0: a dynamics gust every this many wind ticks.
const GUST_INTERVAL: i32 = 10;
const GUST_FRAMES: i32 = 15;
const GUST_STRENGTH: f32 = 0.5;

#[derive(Clone, Debug)]
pub struct Parameters {
    pub background_delay: [i16; 2],
    pub background_height: i16,
    pub wind_delay: [i32; 2],
    pub wind_speed: f32,
    pub right_bounds: [f32; 2],
    pub left_bounds: [f32; 2],
    pub vertical_bounds: [f32; 2],
    pub blink_delay: [i32; 2],
}
/// The cycle step (xC4) whose state is the wind.
const BLOWING_STEP: i32 = 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Waiting,
    Turning,
    Blowing,
}
impl Phase {
    pub fn from_saved(value: i32) -> Self {
        match value {
            0 => Self::Waiting,
            1 => Self::Turning,
            2 => Self::Blowing,
            _ => panic!("invalid Whispy phase {value}"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Pupupu {
    pub parameters: Parameters,
    pub phase: Phase,
    pub cycle: i32,
    pub blink_timer: i32,
    pub timer: i32,
    pub entering: bool,
    pub facing_right: bool,
    pub wind: i32,
    pub elapsed: i32,
    pub background_timer: i16,
    pub secondary: Option<(Phase, bool)>,
    pub lights: Vec<crate::battle::lights::Light>,
    /// "Frozen Stages" (0x803E67E0, the third entry of the cycle's state
    /// table grOp_803E67D8, is 0): the cycle's blowing step is a second
    /// wait, so Whispy turns to the fighters and waits again.
    pub wind_disabled: bool,
}
impl Pupupu {
    /// grOldPupupu_8021119C / 80210C7C: Whispy is created before the
    /// background spawn controller. Both ranges are exclusive at the top.
    pub fn initialize(parameters: Parameters, rng: &mut HsdRng) -> Self {
        let timer = range(rng, parameters.wind_delay);
        let background_timer = range(rng, parameters.background_delay.map(i32::from)) as i16;
        Self {
            parameters,
            phase: Phase::Waiting,
            cycle: 0,
            blink_timer: 0,
            timer,
            entering: true,
            facing_right: true,
            wind: 0,
            elapsed: 0,
            background_timer,
            secondary: Some((Phase::Waiting, true)),
            lights: Vec::new(),
            wind_disabled: false,
        }
    }
    fn sync_secondary(&mut self) {
        self.secondary = Some((self.phase, self.facing_right));
    }
    fn advance(&mut self) {
        self.cycle = (self.cycle + 1) % 3;
        // grOp_803E67D8 = { 0, 1, 2 }: the cycle step is its own state,
        // except under the frozen-stage code, whose table ends in 0.
        self.phase = if self.wind_disabled && self.cycle == BLOWING_STEP {
            Phase::Waiting
        } else {
            Phase::from_saved(self.cycle)
        };
        self.entering = true;
        self.sync_secondary();
    }
    /// grOldPupupu_802113E0 (0x802113E0). Returns the replacement map-7
    /// animation; the caller evaluates its frame zero immediately. The retail
    /// function has no fused arithmetic: timers and random ranges are integers.
    pub fn tick_whispy(
        &mut self,
        ended: bool,
        fighter_sides: i32,
        rng: &mut HsdRng,
    ) -> Option<usize> {
        self.elapsed = self.elapsed.wrapping_add(1);
        self.wind = 0;
        let side = usize::from(self.facing_right);
        match self.phase {
            Phase::Waiting => {
                let mut animation = None;
                if self.entering {
                    self.entering = false;
                    self.timer = range(rng, self.parameters.wind_delay);
                    animation = Some(BLINK_ANIMATIONS[side]);
                    self.sync_secondary();
                    self.blink_timer = range(rng, self.parameters.blink_delay);
                }
                self.timer -= 1;
                // Attaching/evaluating frame zero clears the old completion flag.
                if ended && animation.is_none() {
                    if self.timer > 0 {
                        self.blink_timer -= 1;
                        if self.blink_timer <= 0 {
                            animation = Some(BLINK_ANIMATIONS[side]);
                            self.blink_timer = range(rng, self.parameters.blink_delay);
                        }
                        self.sync_secondary();
                    } else {
                        self.advance();
                    }
                }
                animation
            }
            Phase::Turning => {
                if self.entering {
                    // ftLib_800864A8 (0x800864A8): signed side vote, tie draw.
                    let right = if fighter_sides == 0 {
                        rng.randi(2) != 0
                    } else {
                        fighter_sides > 0
                    };
                    self.entering = false;
                    self.blink_timer = 0;
                    self.timer = 0;
                    if self.facing_right == right {
                        self.phase = Phase::Blowing;
                        self.advance();
                        Some(BLOW_ANIMATIONS[side])
                    } else {
                        self.facing_right = right;
                        self.sync_secondary();
                        Some(TURN_ANIMATIONS[usize::from(right)])
                    }
                } else {
                    if ended {
                        self.advance();
                    }
                    None
                }
            }
            Phase::Blowing => {
                let animation = if self.entering {
                    self.entering = false;
                    self.blink_timer = 0;
                    self.timer = 0;
                    self.sync_secondary();
                    Some(BLOW_ANIMATIONS[side])
                } else {
                    None
                };
                if !ended || animation.is_some() {
                    self.timer += 1;
                    if self.timer > WIND_START && self.timer < WIND_END {
                        self.wind = side as i32 + 1;
                    }
                } else {
                    self.advance();
                }
                animation
            }
        }
    }
    /// grOldPupupu_80210A24 (0x80210A24): secondary tree animation.
    pub fn take_secondary_animation(&mut self) -> Option<usize> {
        self.secondary.take().map(|(phase, right)| match phase {
            Phase::Waiting => BLINK_ANIMATIONS, Phase::Turning => TURN_ANIMATIONS, Phase::Blowing => BLOW_ANIMATIONS,
        }[usize::from(right)])
    }
    /// grOldPupupu_80211C1C (0x80211C1C): the cloud responds to the
    /// pending blow phase, after map 7 and before map 1 in creation order.
    pub fn cloud_animation(&self) -> Option<usize> {
        (self.phase == Phase::Blowing && self.entering).then_some(usize::from(!self.facing_right))
    }
    /// grOldPupupu_802113E0's blowing tail: every tenth counter value of the
    /// wind interval also pushes dynamic bones (lb_80011A50, 0x80011A50) in
    /// the same rectangle for 15 pool ticks, strength 0.5, no decay.
    /// Valid right after `tick_whispy`, like `wind`.
    pub fn dynamics_gust(&self) -> Option<DirectionalGust> {
        if self.wind == 0 || self.timer % GUST_INTERVAL != 0 {
            return None;
        }
        let p = &self.parameters;
        // grOp_803E67E4 by facing; the rectangle is (x10, x14, x18, x1C).
        let (direction, [left, right]) = if self.facing_right {
            (Vec3::new(1.0, 0.0, 0.0), p.right_bounds)
        } else {
            (
                Vec3::new(-1.0, 0.0, 0.0),
                [p.left_bounds[1], p.left_bounds[0]],
            )
        };
        Some(DirectionalGust {
            direction,
            frames: GUST_FRAMES,
            strength: GUST_STRENGTH,
            decay: 0.0,
            phase_step: 0.0,
            rectangle: [left, p.vertical_bounds[0], right, p.vertical_bounds[1]],
        })
    }
    /// fn_802112F4 (0x802112F4): the gust this tick's Whispy state selects
    /// (+0xDC: 1 blows left, 2 blows right).
    pub fn wind(&self) -> Wind {
        let (x_bounds, speed) = match self.wind {
            1 => (self.parameters.left_bounds, -self.parameters.wind_speed),
            2 => (self.parameters.right_bounds, self.parameters.wind_speed),
            _ => return Wind::CALM,
        };
        Wind::gust(Gust {
            velocity: Vec3::new(speed, 0.0, 0.0),
            x_bounds,
            y_bounds: self.parameters.vertical_bounds,
        })
    }
}
pub use crate::rand::range;

#[cfg(test)]
mod tests {
    use super::*;
    fn stage() -> Pupupu {
        Pupupu::initialize(
            Parameters {
                background_delay: [3000, 4000],
                background_height: 30,
                wind_delay: [600, 1200],
                wind_speed: 0.2,
                right_bounds: [-17.0, 76.0],
                left_bounds: [-18.0, -74.0],
                vertical_bounds: [40.0, -10.0],
                blink_delay: [180, 360],
            },
            &mut HsdRng::new(1),
        )
    }
    #[test]
    fn reversed_ranges_match_and_equal_bounds_do_not_draw() {
        let mut a = HsdRng::new(123);
        let mut b = a;
        assert_eq!(range(&mut a, [600, 1200]), range(&mut b, [1200, 600]));
        assert_eq!(a.seed, b.seed);
        assert_eq!(range(&mut a, [9, 9]), 9);
        assert_eq!(a.seed, b.seed);
    }
    #[test]
    fn wind_begins_after_45_and_stops_at_320_with_strict_bounds() {
        let mut stage = stage();
        stage.phase = Phase::Blowing;
        stage.entering = false;
        stage.facing_right = false;
        stage.timer = 44;
        let mut rng = HsdRng::new(9);
        stage.tick_whispy(false, 0, &mut rng);
        assert_eq!(stage.wind().at(Vec3::new(-46.6, 30.1, 0.0)), Vec3::ZERO);
        stage.tick_whispy(false, 0, &mut rng);
        assert_eq!(
            stage.wind().at(Vec3::new(-46.6, 30.1, 0.0)).x.to_bits(),
            (-0.2_f32).to_bits()
        );
        for p in [
            Vec3::new(-74.0, 30.1, 0.0),
            Vec3::new(-18.0, 30.1, 0.0),
            Vec3::new(-46.6, 40.0, 0.0),
        ] {
            assert_eq!(stage.wind().at(p), Vec3::ZERO);
        }
        stage.timer = 319;
        stage.tick_whispy(false, 0, &mut rng);
        assert_eq!(stage.wind, 0);
        assert_eq!(rng.seed, 9);
    }
    #[test]
    fn the_frozen_stage_waits_again_where_it_would_blow() {
        // Whispy already faces the fighters, so the turn goes straight to
        // the cycle's third step: the wind, or under the code a new wait.
        for (frozen, phase) in [(false, Phase::Blowing), (true, Phase::Waiting)] {
            let mut stage = stage();
            stage.wind_disabled = frozen;
            stage.cycle = 1;
            stage.phase = Phase::Turning;
            stage.entering = true;
            let mut rng = HsdRng::new(9);
            stage.tick_whispy(false, 1, &mut rng);
            assert_eq!((stage.cycle, stage.phase, rng.seed), (2, phase, 9));
            // The new wait draws its two timers; the wind draws nothing.
            stage.tick_whispy(false, 1, &mut rng);
            assert_eq!(rng.seed != 9, frozen);
            assert_eq!(stage.wind, 0);
        }
    }
    #[test]
    fn blink_delay_counts_only_after_animation_completion() {
        let mut stage = stage();
        stage.entering = false;
        stage.timer = 3;
        stage.blink_timer = 2;
        let mut rng = HsdRng::new(9);
        assert_eq!(stage.tick_whispy(false, 0, &mut rng), None);
        assert_eq!(stage.blink_timer, 2);
        assert_eq!(stage.tick_whispy(true, 0, &mut rng), None);
        assert_eq!(stage.blink_timer, 1);
        assert_eq!(stage.tick_whispy(true, 0, &mut rng), None);
        assert_eq!(stage.phase, Phase::Turning);
        assert!(stage.entering);
        assert_eq!(rng.seed, 9);
    }
}
