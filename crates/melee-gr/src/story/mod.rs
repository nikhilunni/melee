//! Yoshi's Story callbacks, gr/grstory.c. The stage clock advances through
//! map animations; Randall's puff timer is separate from his path phase.
pub mod procs;
use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;

/// grStory_801E366C: common bank particle for Randall's trailing puff.
pub const PUFF_PARTICLE: u32 = 44;
const PUFF_MINIMUM_DELAY: i32 = 10;
const PUFF_DELAY_RANGE: i32 = 20;
/// Retail overwrites its random timer with this fixed delay.
const SHY_GUY_DELAY: i32 = 120;
/// itHeiho_UnkMotion1_Phys starts the upward escape after frame 960.
const SHY_GUY_FLIGHT_FRAMES: u16 = 960;
const SHY_GUY_STAGGER: i32 = 25;

#[derive(Clone, Debug)]
pub struct Parameters {
    pub timer_minimum: f32,
    pub timer_range: i32,
    pub group_rarity: i32,
    pub heights: [f32; 6],
}
#[derive(Clone, Debug)]
pub struct ShyGuySpawn {
    pub position: Vec3,
    pub speed_variant: i32,
    pub delay: i32,
}
#[derive(Clone, Debug)]
pub struct Story {
    pub parameters: Parameters,
    pub puff_timer: i16,
    pub shy_timer: i32,
    pub previous_pattern: i8,
    pub spawn_count: i8,
    pub lights: Vec<crate::battle::lights::Light>,
    /// Spawn requests retained as stage-owned outputs. The no-hit idle interval
    /// stays inside Heiho's 960-frame flight before its upward escape begins.
    pub shy_guys: Vec<ShyGuySpawn>,
    pub occupied_frames: u16,
}
impl Story {
    /// grStory_801E3234 / grStory_801E3370: Ground creation starts the
    /// Shy Guy schedule at 120 and Randall's puff timer at zero.
    pub fn initialize(parameters: Parameters, rng: &mut HsdRng) -> Self {
        // Retail 801E32B8 draws even though 801E3304 overwrites the timer.
        // 801E32F0 fadds, 801E32F4 fctiwz; no multiply-add contraction.
        let _discarded_timer =
            (parameters.timer_minimum + randi(rng, parameters.timer_range) as f32) as i32;
        Self {
            parameters,
            puff_timer: 0,
            shy_timer: SHY_GUY_DELAY,
            previous_pattern: 0,
            spawn_count: 0,
            lights: Vec::new(),
            shy_guys: Vec::new(),
            occupied_frames: 0,
        }
    }

    /// grStory_801E366C (0x801E366C): signed short post-decrement.
    pub fn tick_puff(&mut self, rng: &mut HsdRng) -> bool {
        let old = self.puff_timer;
        self.puff_timer = self.puff_timer.wrapping_sub(1);
        if old >= 0 {
            return false;
        }
        self.puff_timer = (rng.randi(PUFF_DELAY_RANGE) + PUFF_MINIMUM_DELAY) as i16;
        true
    }
    /// grStory_801E3418 (0x801E3418): all draws, including overwritten
    /// timer/count selections and the jitter after the final spawn.
    pub fn tick_shy_guys(&mut self, rng: &mut HsdRng) {
        if !self.shy_guys.is_empty() {
            self.occupied_frames += 1;
            assert!(
                self.occupied_frames <= SHY_GUY_FLIGHT_FRAMES,
                "itheiho.c:167-182: post-flight escape requires item simulation"
            );
            return;
        }
        if self.shy_timer != 0 {
            self.shy_timer -= 1;
            return;
        }
        // Retail 801E34C0 fadds, then fctiwz. The store is overwritten by 120.
        self.shy_timer =
            (self.parameters.timer_minimum + randi(rng, self.parameters.timer_range) as f32) as i32;
        self.shy_timer = SHY_GUY_DELAY;
        let pattern = loop {
            let pattern = rng.randi(6) as i8;
            if pattern != self.previous_pattern {
                break pattern;
            }
        };
        self.previous_pattern = pattern;
        let height = self.parameters.heights[pattern as usize];
        let mut position = Vec3::new(if pattern < 3 { -292.0 } else { 304.0 }, height, 2.0);
        let speed_variant = rng.randi(3);
        self.spawn_count = spawn_count(rng, self.parameters.group_rarity);
        self.spawn_count = spawn_count(rng, 2);
        for index in 0..self.spawn_count {
            self.shy_guys.push(ShyGuySpawn {
                position,
                speed_variant,
                delay: i32::from(index) * SHY_GUY_STAGGER,
            });
            // Retail 801E3614 fsubs, 801E3624 fmuls, 801E362C fmadds.
            position.y = fmadds(3.0, 2.0 * (rng.randf() - 0.5), height);
        }
    }
}
fn randi(rng: &mut HsdRng, maximum: i32) -> i32 {
    if maximum == 0 {
        0
    } else {
        rng.randi(maximum)
    }
}
fn spawn_count(rng: &mut HsdRng, rarity: i32) -> i8 {
    if randi(rng, rarity) == 0 {
        (rng.randi(3) + 3) as i8
    } else {
        1
    }
}
