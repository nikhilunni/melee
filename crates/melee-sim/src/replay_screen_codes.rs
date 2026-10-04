//! Which screen code a recording's console ran.
//!
//! Tournament consoles on 16:9 monitors ran the Widescreen code
//! (`melee_lib::slippi::SlippiCodes::widescreen`), which a replay does not
//! record. It changes which ticks a fighter counts as off screen, so the
//! magnifier's damage and Pokémon Stadium's close-up draws show it: run the
//! port on the standard screen and the wide one in step, from the recorded
//! inputs alone, until the two differ; the run the recording then follows
//! names the screen, which runs the whole comparison from its first frame.
use crate::replay::{dated_controller_fix, unsupported_setup, Setup};
use crate::replay_stage_codes::recorded_between;
use slp::Replay;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};

/// Whether the Widescreen code ran and what decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenChoice {
    pub widescreen: bool,
    pub reason: ScreenReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenReason {
    /// `Setup::widescreen`.
    Named,
    /// The runs on either screen never differ over the recording, or the
    /// recording follows neither once they do: retail's screen.
    Undecided,
    /// The two runs first differ at tick `apart`; at tick `tick` the
    /// recording matches this one and not the other.
    Recorded { apart: u64, tick: u64 },
}

impl std::fmt::Display for ScreenChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", name(self.widescreen))?;
        match self.reason {
            ScreenReason::Named => write!(f, " (named)"),
            ScreenReason::Undecided => {
                write!(f, " (nothing recorded separates it from {})", name(true))
            }
            ScreenReason::Recorded { apart, tick } => write!(
                f,
                " (not {}: the screens differ from tick {apart} and the recording is this one's at tick {tick})",
                name(!self.widescreen)
            ),
        }
    }
}

/// The names `--screen` takes.
pub const NAMES: [(&str, bool); 2] = [("standard", false), (WIDESCREEN, true)];
const WIDESCREEN: &str = "widescreen";

fn name(widescreen: bool) -> &'static str {
    if widescreen {
        WIDESCREEN
    } else {
        "standard"
    }
}

/// Choose the screen code for a recording, under the setup's stage code.
pub fn resolve_screen(replay: &Replay, root: &Path, setup: Setup) -> (Setup, ScreenChoice) {
    let named = |widescreen| Setup {
        widescreen: Some(widescreen),
        ..setup
    };
    if let Some(widescreen) = setup.widescreen {
        let reason = ScreenReason::Named;
        return (setup, ScreenChoice { widescreen, reason });
    }
    let mut choice = ScreenChoice {
        widescreen: false,
        reason: ScreenReason::Undecided,
    };
    // The probe runs under the dated UCF version, as the stage code's does.
    let dated = |widescreen| dated_controller_fix(replay, named(widescreen));
    if unsupported_setup(replay, dated(false)).is_empty() {
        // An unported boundary or setup error before the recording decides
        // leaves retail's screen; the comparison run reports it.
        let probe = catch_unwind(AssertUnwindSafe(|| {
            recorded_between(replay, root, dated(false), dated(true))
        }));
        if let Ok(Ok(Some(recorded))) = probe {
            choice = ScreenChoice {
                widescreen: recorded.coded,
                reason: ScreenReason::Recorded {
                    apart: recorded.apart,
                    tick: recorded.tick,
                },
            };
        }
    }
    (named(choice.widescreen), choice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_line_says_what_decided() {
        let recorded = ScreenChoice {
            widescreen: true,
            reason: ScreenReason::Recorded {
                apart: 1565,
                tick: 1565,
            },
        };
        assert_eq!(
            recorded.to_string(),
            "widescreen (not standard: the screens differ from tick 1565 and the recording is this one's at tick 1565)"
        );
        let undecided = ScreenChoice {
            widescreen: false,
            reason: ScreenReason::Undecided,
        };
        assert_eq!(
            undecided.to_string(),
            "standard (nothing recorded separates it from widescreen)"
        );
    }
}
