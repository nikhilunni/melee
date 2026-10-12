//! Which screen code a recording's console ran.
//!
//! Tournament consoles on 16:9 monitors ran the Widescreen code
//! (`melee_lib::slippi::SlippiCodes::screen`), which a replay does not
//! record, with one of two aspects (320:219, or the 92:63 it had until late
//! 2019). It changes which ticks a fighter counts as off screen, so the
//! magnifier's damage and Pokémon Stadium's close-up draws show it: run the
//! port on each screen in step, from the recorded inputs alone, until they
//! differ; the run the recording then follows names the screen, which runs
//! the whole comparison from its first frame.
use crate::replay::{dated_controller_fix, unsupported_setup, Setup};
use crate::replay_cost::{self as cost, Part};
use crate::replay_game::Game;
use crate::replay_stage_codes::{recorded_among, Left};
use melee_lib::slippi::Screen;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// The screens a recording is read against, retail's first: runs the
/// recording does not tell apart are taken for the earliest here.
const CANDIDATES: [Screen; 3] = [
    Screen::Standard,
    Screen::Widescreen,
    Screen::Widescreen92By63,
];

/// The screen code that ran and what decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenChoice {
    pub screen: Screen,
    pub reason: ScreenReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenReason {
    /// `Setup::screen`.
    Named,
    /// The runs on every screen never differ over the recording, or the
    /// recording follows none once they do: retail's screen.
    Undecided,
    /// The recording left the other screens' runs, in `CANDIDATES` order
    /// (`None` for this screen and for one it was never told apart from):
    /// that run first differs from this one at tick `apart`, and at tick
    /// `tick` the recording matches another run and not that one.
    Recorded([Option<Rejected>; 3]),
}

/// Where the recording left another screen's run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected {
    pub apart: u64,
    pub tick: u64,
}

impl std::fmt::Display for ScreenChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", name(self.screen))?;
        match self.reason {
            ScreenReason::Named => write!(f, " (named)"),
            ScreenReason::Undecided => write!(
                f,
                " (nothing recorded separates it from {})",
                name(Screen::Widescreen)
            ),
            ScreenReason::Recorded(rejected) => {
                let mut opening = " (";
                for (screen, rejected) in CANDIDATES.into_iter().zip(rejected) {
                    if screen == self.screen {
                        continue;
                    }
                    write!(f, "{opening}")?;
                    opening = "; ";
                    match rejected {
                        Some(Rejected { apart, tick }) => write!(
                            f,
                            "not {}: the screens differ from tick {apart} and the recording is this one's at tick {tick}",
                            name(screen)
                        )?,
                        None => write!(f, "nothing recorded separates it from {}", name(screen))?,
                    }
                }
                write!(f, ")")
            }
        }
    }
}

/// The names `--screen` takes.
pub const NAMES: [(&str, Screen); 3] = [
    ("standard", Screen::Standard),
    ("widescreen", Screen::Widescreen),
    ("widescreen-92-63", Screen::Widescreen92By63),
];

fn name(screen: Screen) -> &'static str {
    NAMES
        .iter()
        .find(|(_, named)| *named == screen)
        .map_or("?", |(name, _)| name)
}

/// Choose the screen code for a recording, under the setup's stage code.
pub(crate) fn resolve_screen(game: &mut Game, setup: Setup) -> (Setup, ScreenChoice) {
    let replay = game.replay;
    let named = |screen| Setup {
        screen: Some(screen),
        ..setup
    };
    if let Some(screen) = setup.screen {
        let reason = ScreenReason::Named;
        return (setup, ScreenChoice { screen, reason });
    }
    let mut choice = ScreenChoice {
        screen: Screen::Standard,
        reason: ScreenReason::Undecided,
    };
    // The probe runs under the dated UCF version, as the stage code's does.
    let dated = |screen| dated_controller_fix(replay, named(screen));
    if unsupported_setup(replay, dated(Screen::Standard)).is_empty() {
        // An unported boundary or setup error before the recording decides
        // leaves retail's screen; the comparison run reports it.
        let _t = cost::time(Part::ScreenProbe);
        let probe = catch_unwind(AssertUnwindSafe(|| {
            recorded_among(game, CANDIDATES.map(dated))
        }));
        if let Ok(Ok(Some(followed))) = probe {
            let rejected = followed
                .left
                .map(|left| left.map(|Left { apart, tick }| Rejected { apart, tick }));
            choice = ScreenChoice {
                screen: CANDIDATES[followed.chosen],
                reason: ScreenReason::Recorded(rejected),
            };
        }
    }
    (named(choice.screen), choice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_line_says_what_decided() {
        let left_at = |apart, tick| Some(Rejected { apart, tick });
        let cases = [
            (
                Screen::Widescreen,
                ScreenReason::Recorded([left_at(1565, 1565), None, None]),
                "widescreen (not standard: the screens differ from tick 1565 and the recording is this one's at tick 1565; nothing recorded separates it from widescreen-92-63)",
            ),
            (
                Screen::Widescreen92By63,
                ScreenReason::Recorded([left_at(1268, 1269), left_at(4456, 4456), None]),
                "widescreen-92-63 (not standard: the screens differ from tick 1268 and the recording is this one's at tick 1269; not widescreen: the screens differ from tick 4456 and the recording is this one's at tick 4456)",
            ),
            (
                Screen::Standard,
                ScreenReason::Undecided,
                "standard (nothing recorded separates it from widescreen)",
            ),
            (Screen::Widescreen, ScreenReason::Named, "widescreen (named)"),
        ];
        for (screen, reason, line) in cases {
            assert_eq!(ScreenChoice { screen, reason }.to_string(), line);
        }
    }
}
