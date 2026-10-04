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
/// it_802D8618's stagger: each Shy Guy of a group waits 25 frames longer.
const SHY_GUY_STAGGER: f32 = 25.0;
/// grStory_801E3418: groups enter from the left below pattern 3, else the right.
const LEFT_ENTRY_X: f32 = -292.0;
const RIGHT_ENTRY_X: f32 = 304.0;
const ENTRY_Z: f32 = 2.0;
/// The vertical jitter of each Shy Guy after the first.
const HEIGHT_JITTER: f32 = 3.0;

#[derive(Clone, Debug)]
pub struct Parameters {
    pub timer_minimum: f32,
    pub timer_range: i32,
    pub group_rarity: i32,
    pub heights: [f32; 6],
}
/// it_802D8618's arguments: one Shy Guy of a group.
#[derive(Clone, Copy, Debug)]
pub struct ShyGuySpawn {
    pub group_index: i32,
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
    /// "Disable Yoshi's Story Shyguys" (0x801E3348, grStory_801E3334's
    /// `bl grStory_801E3418`, is a nop): the spawner never runs, so its
    /// timer stays where creation left it.
    pub shy_guys_disabled: bool,
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
            shy_guys_disabled: false,
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
    /// timer/count selections and the jitter after the final spawn. Nothing
    /// happens while a Shy Guy lives (it_8026B3C0). `spawn` creates each
    /// one (it_802D8618) in order, between the jitter draws.
    pub fn tick_shy_guys(
        &mut self,
        rng: &mut HsdRng,
        shy_guys_live: bool,
        mut spawn: impl FnMut(&mut HsdRng, ShyGuySpawn),
    ) {
        if self.shy_guys_disabled || shy_guys_live {
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
        let x = if pattern < 3 {
            LEFT_ENTRY_X
        } else {
            RIGHT_ENTRY_X
        };
        let mut position = Vec3::new(x, height, ENTRY_Z);
        let speed_variant = rng.randi(3);
        self.spawn_count = spawn_count(rng, self.parameters.group_rarity);
        self.spawn_count = spawn_count(rng, 2);
        for index in 0..i32::from(self.spawn_count) {
            // 25.0F * i converts to it_802D8618's integer delay.
            spawn(
                rng,
                ShyGuySpawn {
                    group_index: index,
                    position,
                    speed_variant,
                    delay: gekko_math::msl::fctiwz(SHY_GUY_STAGGER * index as f32),
                },
            );
            // Retail 801E3614 fsubs, 801E3624 fmuls, 801E362C fmadds.
            position.y = fmadds(HEIGHT_JITTER, 2.0 * (rng.randf() - 0.5), height);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn story() -> Story {
        let parameters = Parameters {
            timer_minimum: 600.0,
            timer_range: 600,
            group_rarity: 4,
            heights: [60.0, 50.0, 40.0, 75.0, 65.0, 45.0],
        };
        Story::initialize(parameters, &mut HsdRng::new(7))
    }

    #[test]
    fn a_live_shy_guy_holds_the_timer_without_drawing() {
        let mut stage = story();
        let mut rng = HsdRng::new(1);
        stage.tick_shy_guys(&mut rng, true, |_, _| unreachable!());
        assert_eq!(stage.shy_timer, SHY_GUY_DELAY);
        assert_eq!(rng.seed, 1);
    }

    #[test]
    fn the_frozen_stage_never_runs_the_spawner() {
        let mut stage = story();
        stage.shy_guys_disabled = true;
        stage.shy_timer = 0;
        let mut rng = HsdRng::new(1);
        stage.tick_shy_guys(&mut rng, false, |_, _| unreachable!());
        assert_eq!((stage.shy_timer, rng.seed), (0, 1));
    }

    #[test]
    fn a_group_spawns_in_order_with_staggered_delays() {
        let mut stage = story();
        stage.shy_timer = 0;
        for seed in 0..64 {
            let mut rng = HsdRng::new(seed);
            let mut spawned = Vec::new();
            let mut probe = stage.clone();
            probe.tick_shy_guys(&mut rng, false, |_, spawn| spawned.push(spawn));
            assert_eq!(spawned.len(), probe.spawn_count as usize);
            for (index, spawn) in spawned.iter().enumerate() {
                assert_eq!(spawn.group_index, index as i32);
                assert_eq!(spawn.delay, 25 * index as i32);
                assert_eq!(spawn.speed_variant, spawned[0].speed_variant);
            }
            let first = spawned[0].position;
            assert!(first.x == LEFT_ENTRY_X || first.x == RIGHT_ENTRY_X);
            assert_eq!(probe.shy_timer, SHY_GUY_DELAY);
        }
    }
}
