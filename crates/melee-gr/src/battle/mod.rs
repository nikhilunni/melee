//! Battlefield's background controller (`gr/grbattle.c`).
pub mod lights;
pub mod procs;
use gekko_math::HsdRng;

/// Map IDs used by the background-selection retry loop.
const BACKGROUND_MAPS: [u8; 3] = [1, 2, 4];
/// Base delay and random extension, in scheduler ticks (grbattle.c:333).
const BACKGROUND_MINIMUM_DELAY: i32 = 2400;
const BACKGROUND_DELAY_RANGE: i32 = 1200;
/// Map whose model plays the background swap animation (grNBa_StageCallbacks[3]).
pub const TRANSITION_MAP: u8 = 3;
/// `yakumono_param` color-overlay scripts (grBattle_YakumonoParam): the
/// incoming background uses +0, the outgoing one +4.
pub const CURRENT_BACKGROUND_OVERLAY: usize = 0;
pub const PREVIOUS_BACKGROUND_OVERLAY: usize = 1;

/// `grNBa_BG_Vars::state` (gp+C4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundPhase {
    Waiting,
    Transitioning,
    Done,
}
#[derive(Clone, Debug)]
pub struct Battlefield {
    pub phase: BackgroundPhase,
    /// gp+D0.
    pub timer: i32,
    /// gp+C8; `None` is retail's -1 before the first swap.
    pub current: Option<u8>,
    /// gp+CC.
    pub previous: Option<u8>,
    pub lights: Vec<lights::Light>,
}

/// Engine operation issued by one `grBattle_BG_Callback2` step, in C order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundEvent {
    None,
    /// grAnime_801C8138(map 3, animation 0): attach and evaluate frame zero.
    AttachTransition,
    /// HSD_JObjClearFlagsAll(hidden) on the transition model when its root
    /// is hidden; the swap animation is still running.
    ShowTransition,
    /// The transition model is shown (as for `ShowTransition`), then:
    /// grMaterial_801C9604 on `previous` with the outgoing overlay,
    /// grBattle_80219D84(`current`) creates that map, and grMaterial_801C9604
    /// on it with the incoming overlay.
    Swap { previous: u8, current: u8 },
    /// Ground_801C4A08(`previous`), then HSD_JObjSetFlagsAll(hidden) on the
    /// transition model. The timer was redrawn after both.
    Retire { previous: u8 },
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
    ///
    /// `transition_ended` is grAnime_801C83D0(map 3, 0, 7); `map_live` is
    /// Ground_GetMapGObj; `fade_complete` is grLib_801C96E8 (Ground x10 b4).
    /// The caller applies the returned event before the next proc runs.
    pub fn tick(
        &mut self,
        rng: &mut HsdRng,
        transition_ended: impl FnOnce() -> bool,
        map_live: impl Fn(u8) -> bool,
        fade_complete: impl FnOnce(u8) -> bool,
    ) -> BackgroundEvent {
        match self.phase {
            BackgroundPhase::Waiting => {
                // Signed post-decrement: a timer of zero still waits one more tick.
                let old = self.timer;
                self.timer -= 1;
                if old < 0 {
                    self.phase = BackgroundPhase::Transitioning;
                    return BackgroundEvent::AttachTransition;
                }
                BackgroundEvent::None
            }
            BackgroundPhase::Transitioning => {
                if !transition_ended() {
                    return BackgroundEvent::ShowTransition;
                }
                if self.current.is_none() {
                    // grbattle.c:390-399: adopt the first live background.
                    let live = BACKGROUND_MAPS.into_iter().find(|&map| map_live(map));
                    self.current = Some(live.expect("grbattle.c:527: live background"));
                }
                let current = self.choose_next(rng);
                let previous = self.previous.unwrap();
                assert!(map_live(previous), "grbattle.c:535: previous background");
                self.phase = BackgroundPhase::Done;
                BackgroundEvent::Swap { previous, current }
            }
            BackgroundPhase::Done => {
                let previous = self.previous.unwrap();
                assert!(map_live(previous), "grbattle.c:546: previous background");
                if !fade_complete(previous) {
                    return BackgroundEvent::None;
                }
                self.phase = BackgroundPhase::Waiting;
                // reset_bg_timer, retail 0x8021A5DC: drawn after the retirement.
                self.timer = rng.randi(BACKGROUND_DELAY_RANGE) + BACKGROUND_MINIMUM_DELAY;
                BackgroundEvent::Retire { previous }
            }
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
/// Scheduler callback address of a Battlefield map's gobj proc (s_link 4),
/// from grNBa_StageCallbacks.
pub fn map_callback(map: u8) -> u32 {
    match map {
        0 => 0x8021A114,
        1 => 0x8021A26C,
        2 => 0x8021A2D4,
        3 => 0x8021A3BC,
        4 => 0x8021A33C,
        5 => 0x8021A204,
        6 => 0x8021A174,
        _ => unreachable!("Battlefield map {map}"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waiting_timer_does_not_draw_and_zero_waits() {
        let mut stage = Battlefield::initialize(&mut HsdRng::new(1));
        stage.timer = 600;
        let mut rng = HsdRng::new(1);
        let mut step = |stage: &mut Battlefield| {
            stage.tick(
                &mut rng,
                || unreachable!(),
                |_| unreachable!(),
                |_| unreachable!(),
            )
        };
        for _ in 0..600 {
            assert_eq!(step(&mut stage), BackgroundEvent::None);
        }
        assert_eq!(stage.timer, 0);
        step(&mut stage);
        assert_eq!(stage.phase, BackgroundPhase::Waiting);
        assert_eq!(stage.timer, -1);
        assert_eq!(step(&mut stage), BackgroundEvent::AttachTransition);
        assert_eq!(stage.phase, BackgroundPhase::Transitioning);
        assert_eq!(rng.seed, 1);
    }
    #[test]
    fn cycle_swaps_from_the_first_live_background_and_retires_after_the_fade() {
        let mut rng = HsdRng::new(1);
        let mut stage = Battlefield::initialize(&mut HsdRng::new(0));
        stage.phase = BackgroundPhase::Transitioning;
        let live = |map| [0, 1, 3, 6].contains(&map);
        let event = stage.tick(&mut rng, || false, live, |_| unreachable!());
        assert_eq!(event, BackgroundEvent::ShowTransition);
        let event = stage.tick(&mut rng, || true, live, |_| unreachable!());
        let BackgroundEvent::Swap { previous, current } = event else {
            panic!("{event:?}")
        };
        assert_eq!(previous, 1);
        assert!(current == 2 || current == 4);
        assert_eq!(stage.phase, BackgroundPhase::Done);
        let live = |map| [0, 1, 3, 6, current].contains(&map);
        let event = stage.tick(&mut rng, || unreachable!(), live, |_| false);
        assert_eq!(event, BackgroundEvent::None);
        let seed = rng.seed;
        let event = stage.tick(&mut rng, || unreachable!(), live, |map| map == 1);
        assert_eq!(event, BackgroundEvent::Retire { previous: 1 });
        assert_eq!(stage.phase, BackgroundPhase::Waiting);
        assert_ne!(rng.seed, seed);
        assert!((2400..3600).contains(&stage.timer));
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
