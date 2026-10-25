//! The scene's frame timer (lb_804329F0, lb_0195.c): with the game slowed
//! (StartMeleeRules game_speed below 1), some passes of gm_801A4D34's loop
//! run no tick.
//!
//! Pads are polled at the shorter of the game's period (1/60 s / speed) and
//! 1/60 s (lb_80019628), and the loop makes one scheduler pass per poll.
//! Each pass adds the poll period to the game timer (lb_80019900); only a
//! pass that completes a game period runs the scene's frame callback and
//! unpauses the objects (gm_803DA8C8, every p_link): the others run nothing.

/// OS_TIMER_CLOCK: the bus clock (162 MHz) over 4.
const TIMER_CLOCK: f32 = 40_500_000.0;

/// OSSecondsToTicks as Melee spells it for a float: the f32 product,
/// truncated (__cvt_fp2unsigned).
fn seconds_to_ticks(seconds: f32) -> u64 {
    (seconds * TIMER_CLOCK) as u64
}

/// lb_804329F0's game timer (x0[0]) and poll period (x40).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameTimer {
    /// x0[0].x0: the game's period.
    pub period: u64,
    /// x40: the poll period, which each pass adds.
    pub step: u64,
    /// x0[0].x8: time accumulated toward the next game period.
    pub accumulated: u64,
    /// x38: the period gm_SetGameSpeed asked for, which lb_80019628 takes
    /// up at the next loop iteration's poll count (lb_80019894).
    pub requested: u64,
    /// `requested` came from a proc of the last tick (gm_SetGameSpeed
    /// during the scene's pass), after this pass's poll was modelled.
    pub requested_in_tick: bool,
}

impl FrameTimer {
    /// A match's creation pass: the menus' 1/60 s timer at rest (every
    /// full-speed pass leaves x8 at 0), with fn_8016E730's
    /// gm_SetGameSpeed(game_speed) still pending.
    pub(crate) fn entering(speed: f32) -> Self {
        let frame = seconds_to_ticks(1.0 / 60.0);
        Self {
            period: frame,
            step: frame,
            accumulated: 0,
            requested: Self::period_for(speed),
            requested_in_tick: false,
        }
    }

    /// gm_SetGameSpeed from a proc of the tick this pass ran: the next
    /// iteration's lb_80019894 takes it up before its lb_80019900.
    pub(crate) fn request(&mut self, speed: f32) {
        self.requested = Self::period_for(speed);
        self.requested_in_tick = true;
    }

    /// lb_80019880(OSSecondsToTicks(1 / 60.0F / speed)).
    pub(crate) fn period_for(speed: f32) -> u64 {
        seconds_to_ticks(1.0 / 60.0 / speed)
    }

    /// lb_80019900 for x0[0]: whether this pass runs the game's tick; then
    /// the next iteration's lb_80019628 takes up a requested period.
    pub(crate) fn pass(&mut self) -> bool {
        if std::mem::take(&mut self.requested_in_tick) {
            self.take_request();
        }
        self.accumulated += self.step;
        let due = if self.accumulated >= self.period {
            self.accumulated -= self.period;
            true
        } else {
            false
        };
        self.take_request();
        due
    }

    /// lb_80019628 (80019628): a new game period resets a timer already past
    /// it, and the polls follow the shorter of it and 1/60 s.
    fn take_request(&mut self) {
        if self.requested == self.period {
            return;
        }
        self.period = self.requested;
        if self.accumulated >= self.period {
            self.accumulated = 0;
        }
        self.step = self.period.min(seconds_to_ticks(1.0 / 60.0));
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::entering(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_speed_and_lightning_tick_every_pass() {
        for speed in [1.0, 1.25] {
            let mut timer = FrameTimer::entering(speed);
            assert!((0..1000).all(|_| timer.pass()), "speed {speed}");
        }
    }

    #[test]
    fn slow_motion_skips_every_other_pass_after_the_creation_pass() {
        let mut timer = FrameTimer::entering(0.5);
        let passes: Vec<bool> = (0..7).map(|_| timer.pass()).collect();
        assert_eq!(passes, [true, false, true, false, true, false, true]);
    }
}
