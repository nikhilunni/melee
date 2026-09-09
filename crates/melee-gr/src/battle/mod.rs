//! Battlefield's background controller (`gr/grbattle.c`).
pub mod lights;
pub mod procs;
use gekko_math::HsdRng;

/// Map IDs used by the background-selection retry loop.
const BACKGROUND_MAPS: [u8; 3] = [1, 2, 4];
/// Base delay and random extension, in scheduler ticks (grbattle.c:333).
const BACKGROUND_MINIMUM_DELAY: i32 = 2400;
const BACKGROUND_DELAY_RANGE: i32 = 1200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundPhase {
    Waiting,
    Transitioning,
    Done,
}
#[derive(Clone, Debug)]
pub struct Battlefield {
    pub phase: BackgroundPhase,
    pub timer: i32,
    pub current: Option<u8>,
    pub previous: Option<u8>,
    pub lights: Vec<lights::Light>,
}
impl Battlefield {
    /// `grBattle_BG_Callback0`, retail 0x8021A344 (grbattle.c:343-355).
    pub fn initialize(rng: &mut HsdRng) -> Self {
        Self {
            phase: BackgroundPhase::Waiting,
            timer: rng.randi(BACKGROUND_DELAY_RANGE) + BACKGROUND_MINIMUM_DELAY,
            current: None,
            previous: None,
            lights: Vec::new(),
        }
    }
    /// `grBattle_BG_Callback2`, retail 0x8021A3BC (grbattle.c:362-424).
    /// Signed post-decrement: a timer of zero still waits one more tick.
    pub fn tick(&mut self) {
        match self.phase {
            BackgroundPhase::Waiting => {
                let old = self.timer;
                self.timer -= 1;
                if old < 0 {
                    self.phase = BackgroundPhase::Transitioning;
                    unimplemented!("grbattle.c:379-380: attach transition animation");
                }
            }
            BackgroundPhase::Transitioning => unimplemented!("grbattle.c:385-410: animation completion, background selection and material overlays"),
            BackgroundPhase::Done => unimplemented!("grbattle.c:415-423: retire faded background and reset timer"),
        }
    }
    /// Background choice in `grBattle_BG_Callback2`, 0x8021A3BC, lines 393-397.
    /// Rejected candidates consume a draw too.
    pub fn choose_next(&mut self, rng: &mut HsdRng) -> u8 {
        self.previous = self.current;
        loop {
            let map = BACKGROUND_MAPS[rng.randi(BACKGROUND_MAPS.len() as i32) as usize];
            self.current = Some(map);
            if self.current != self.previous {
                return map;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waiting_timer_does_not_draw_and_zero_waits() {
        let mut stage = Battlefield::initialize(&mut HsdRng::new(1));
        stage.timer = 600;
        for _ in 0..600 {
            stage.tick();
        }
        assert_eq!(stage.timer, 0);
        stage.tick();
        assert_eq!(stage.phase, BackgroundPhase::Waiting);
        assert_eq!(stage.timer, -1);
    }
    #[test]
    fn selection_retries_the_current_background() {
        let mut rng = HsdRng::new(1);
        let mut stage = Battlefield::initialize(&mut HsdRng::new(0));
        stage.current = Some(1);
        assert_eq!(stage.choose_next(&mut rng), 4);
        assert_eq!(rng.seed, 3_357_800_067);
    }
}
