//! The match's own clock and rules state (gm_16AE.c, lbl_8046B6A0): the
//! frame count that runs once the HUD is enabled, and the Sudden Death rule.

/// lbl_8046B6A0 (0x8046B6A0), the fields gameplay reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MatchClock {
    /// +0x05: set by fn_8016B784 when the GO banner ends.
    pub hud_enabled: bool,
    /// +0x24: gm_GetFrameCount.
    pub frame_count: u32,
    /// StartMeleeRules x6 (+0x24CE), gm_8016B238: set by
    /// gm_Scene_SuddenDeath_OnEnter.
    pub sudden_death: bool,
    /// StartMeleeRules timer_enabled with a counting-down time limit.
    pub timer: Option<CountdownTimer>,
}

/// The match timer (lbl_8046B6A0 +0x28 timer_seconds, +0x2C unk_2C).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CountdownTimer {
    pub seconds: u32,
    /// Frames into the current second, wrapping at 60.
    pub frames: u16,
}

impl CountdownTimer {
    /// fn_8016E730's countdown setup: the full limit, one frame short of
    /// the first second (unk_2C = 0x3B, as StartMeleeRules x14 is zero).
    pub(crate) fn start(limit_seconds: u32) -> Self {
        Self {
            seconds: limit_seconds,
            frames: FRAMES_PER_SECOND - 1,
        }
    }
}

const FRAMES_PER_SECOND: u16 = 60;

impl MatchClock {
    /// gm_Scene_Vs_OnFrame -> fn_8016CD98 (8016CD98), while no outcome is
    /// decided: the frame count advances once the HUD is enabled. Its guard
    /// `frame_count < -1` is unsigned and only stops at 0xFFFFFFFF.
    pub(crate) fn advance(&mut self) {
        if !self.hud_enabled {
            return;
        }
        // `frame_count < -1` in unsigned: stop at 0xFFFFFFFF.
        self.frame_count = self.frame_count.saturating_add(1);
        if let Some(timer) = &mut self.timer {
            timer.frames += 1;
            if timer.frames >= FRAMES_PER_SECOND {
                timer.frames = 0;
                // The last five seconds also play a countdown voice.
                timer.seconds = timer.seconds.saturating_sub(1);
            }
        }
    }

    /// gm_GetMatchOutcome's OUTCOME_TIMEOUT for a counting-down timer: zero
    /// seconds and 59 frames, one frame after the clock reached zero.
    pub(crate) fn timed_out(&self) -> bool {
        self.timer
            .is_some_and(|t| t.seconds == 0 && t.frames == FRAMES_PER_SECOND - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-minute timer times out exactly when the clock reads 3600 frames
    /// (timeout_tie_fd_marth's recorded frame_count at TIME!).
    #[test]
    fn one_minute_timer_times_out_after_3600_frames() {
        let mut clock = MatchClock {
            hud_enabled: true,
            timer: Some(CountdownTimer::start(60)),
            ..Default::default()
        };
        while !clock.timed_out() {
            clock.advance();
        }
        assert_eq!(clock.frame_count, 3600);
    }

    #[test]
    fn the_clock_waits_for_the_hud() {
        let mut clock = MatchClock {
            timer: Some(CountdownTimer::start(60)),
            ..Default::default()
        };
        clock.advance();
        assert_eq!(clock.frame_count, 0);
        assert_eq!(clock.timer, Some(CountdownTimer::start(60)));
    }
}
