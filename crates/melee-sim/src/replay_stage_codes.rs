//! Which stage code a recording's console ran.
//!
//! Tournament consoles ran 20XX TE's Frozen Mode writes beside Slippi
//! (`melee_lib::slippi::SlippiCodes::frozen_stages`: no Shy Guys, no Final
//! Destination background phases, no Stadium transformations, no wind on
//! Dream Land). A replay does not record them, and Game Start is the same
//! either way. The code changes what the stage draws from the random stream,
//! so the recording's seeds show it: run the port with and without the code
//! in step, from the recorded inputs alone, until the two differ; the run
//! the recording then follows names the setup, which runs the whole
//! comparison from its first frame.
use crate::replay::{dated_controller_fix, unsupported_setup, NoAnswer, Setup};
use crate::replay_cost::{self as cost, Part};
use crate::replay_game::Game;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Whether the frozen-stage code ran and what decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageCodeChoice {
    pub frozen: bool,
    pub reason: StageCodeReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageCodeReason {
    /// `Setup::frozen_stages`.
    Named,
    /// The runs with and without the code never differ over the recording,
    /// or the recording follows neither once they do: retail's stage.
    Undecided,
    /// The two runs first differ at tick `apart`; at tick `tick` the
    /// recording matches this one and not the other.
    Recorded { apart: u64, tick: u64 },
}

impl std::fmt::Display for StageCodeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", name(self.frozen))?;
        match self.reason {
            StageCodeReason::Named => write!(f, " (named)"),
            StageCodeReason::Undecided => {
                write!(f, " (nothing recorded separates it from {})", name(true))
            }
            StageCodeReason::Recorded { apart, tick } => write!(
                f,
                " (not {}: the stages differ from tick {apart} and the recording is this one's at tick {tick})",
                name(!self.frozen)
            ),
        }
    }
}

/// The names `--stage-codes` takes.
pub const NAMES: [(&str, bool); 2] = [("none", false), (FROZEN_STAGES, true)];
const FROZEN_STAGES: &str = "frozen-stages";

fn name(frozen: bool) -> &'static str {
    if frozen {
        FROZEN_STAGES
    } else {
        "none"
    }
}

/// Slippi stage ids the code changes: Pokémon Stadium, Yoshi's Story,
/// Dream Land and Final Destination. Battlefield and Fountain of Dreams
/// have no write.
const FROZEN_STAGE_IDS: [u16; 4] = [3, 8, 28, 32];

/// Whether the code changes the stage with this Slippi id.
pub fn changes_stage(stage: u16) -> bool {
    FROZEN_STAGE_IDS.contains(&stage)
}

/// Choose the stage code for a recording on a stage it changes. `None` on
/// the other stages, where the question does not arise.
pub(crate) fn resolve_stage_codes(
    game: &mut Game,
    setup: Setup,
) -> (Setup, Option<StageCodeChoice>) {
    let replay = game.replay;
    if !changes_stage(replay.start.stage) {
        return (setup, None);
    }
    let named = |frozen| Setup {
        frozen_stages: Some(frozen),
        ..setup
    };
    if let Some(frozen) = setup.frozen_stages {
        let reason = StageCodeReason::Named;
        return (setup, Some(StageCodeChoice { frozen, reason }));
    }
    let mut choice = StageCodeChoice {
        frozen: false,
        reason: StageCodeReason::Undecided,
    };
    // The probe runs under the dated UCF version, as the comparison will
    // unless a dashback then shows the other one (`resolve_setup`).
    let dated = |frozen| dated_controller_fix(replay, named(frozen));
    if unsupported_setup(replay, dated(false)).is_empty() {
        // An unported boundary or setup error before the recording decides
        // leaves retail's stage; the comparison run reports it.
        let _t = cost::time(Part::StageProbe);
        let probe = catch_unwind(AssertUnwindSafe(|| {
            recorded_between(game, dated(false), dated(true))
        }));
        if let Ok(Ok(Some(recorded))) = probe {
            choice = StageCodeChoice {
                frozen: recorded.coded,
                reason: StageCodeReason::Recorded {
                    apart: recorded.apart,
                    tick: recorded.tick,
                },
            };
        }
    }
    (named(choice.frozen), Some(choice))
}

/// Which of two setups a recording followed, read from its frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Recorded {
    /// The recording is the second setup's (the one with the code).
    pub coded: bool,
    /// The first tick the two runs differ.
    pub apart: u64,
    /// The tick only one of them matches the recording.
    pub tick: u64,
}

/// Run the replay's inputs without and with a code in step: `recorded_among`
/// for two setups. `None` when they never differ, or both stop matching on
/// the same tick.
pub(crate) fn recorded_between(
    game: &mut Game,
    plain: Setup,
    coded: Setup,
) -> Result<Option<Recorded>, NoAnswer> {
    let Some(followed) = recorded_among(game, [plain, coded])? else {
        return Ok(None);
    };
    let other = 1 - followed.chosen;
    let left = followed.left[other].expect("one of two setups was left");
    Ok(Some(Recorded {
        coded: followed.chosen == 1,
        apart: left.apart,
        tick: left.tick,
    }))
}

/// Which of several setups a recording followed, read from its frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Followed<const N: usize> {
    /// The first setup, in the order given, the recording never left alone.
    pub chosen: usize,
    /// For each setup the recording left while another still matched it:
    /// where.
    pub left: [Option<Left>; N],
}

/// Where a recording left one setup's run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Left {
    /// The first tick this run differs from the chosen one.
    pub apart: u64,
    /// The tick the recording matches another run and not this one.
    pub tick: u64,
}

/// Run the replay's inputs under each setup in step. Until two runs differ
/// (a compared field or the tick's end seed) nothing is asked of the
/// recording. From then on each is compared with it as the comparison run
/// would be, Pre Frame seeds included: a run that stops matching on a tick
/// another still matches is left behind. `None` when no run is left behind:
/// they never differ, or all stop matching on the same tick. Runs that
/// remain together to the end, or stop matching together, are not told
/// apart, and the first of them in the order given is chosen.
pub(crate) fn recorded_among<const N: usize>(
    game: &mut Game,
    setups: [Setup; N],
) -> Result<Option<Followed<N>>, NoAnswer> {
    let mut runs = Vec::with_capacity(N);
    for setup in setups {
        runs.push(game.run(setup).map_err(|_| NoAnswer)?);
    }
    let last_tick = game.unavailable().map(|(tick, _)| *tick);
    // Runs the recording has not left, and each pair's first differing tick.
    let mut together = [true; N];
    let mut apart = [[None; N]; N];
    let mut left = [None; N];
    let followed = |together: &[bool; N], left: [Option<u64>; N], apart: &[[Option<u64>; N]; N]| {
        let chosen = together.iter().position(|t| *t).expect("a run remains");
        let left = std::array::from_fn(|run| {
            left[run].map(|tick| Left {
                apart: apart[chosen.min(run)][chosen.max(run)].expect("left runs differ"),
                tick,
            })
        });
        Followed { chosen, left }
    };
    for index in 0..game.ticks() {
        let tick = game.tick_number(index);
        if last_tick == Some(tick) {
            break;
        }
        for run in (0..N).filter(|run| together[*run]) {
            if !game.observe(runs[run], index) {
                return Err(NoAnswer);
            }
        }
        let mut asked = false;
        for a in 0..N {
            for b in a + 1..N {
                if !(together[a] && together[b]) {
                    continue;
                }
                if apart[a][b].is_none() && !game.same(runs[a], runs[b], index) {
                    apart[a][b] = Some(tick);
                }
                asked |= apart[a][b].is_some();
            }
        }
        if !asked {
            continue;
        }
        let matches: [bool; N] =
            std::array::from_fn(|run| together[run] && game.matches(runs[run], index));
        if !matches.contains(&true) {
            // Every remaining run stops here: nothing tells them apart.
            return Ok(left
                .iter()
                .any(Option::is_some)
                .then(|| followed(&together, left, &apart)));
        }
        for run in 0..N {
            if together[run] && !matches[run] {
                together[run] = false;
                left[run] = Some(tick);
            }
        }
        if together.iter().filter(|t| **t).count() == 1 {
            break;
        }
    }
    Ok(left
        .iter()
        .any(Option::is_some)
        .then(|| followed(&together, left, &apart)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_line_says_what_decided() {
        let recorded = StageCodeChoice {
            frozen: true,
            reason: StageCodeReason::Recorded {
                apart: 120,
                tick: 121,
            },
        };
        assert_eq!(
            recorded.to_string(),
            "frozen-stages (not none: the stages differ from tick 120 and the recording is this one's at tick 121)"
        );
        let undecided = StageCodeChoice {
            frozen: false,
            reason: StageCodeReason::Undecided,
        };
        assert_eq!(
            undecided.to_string(),
            "none (nothing recorded separates it from frozen-stages)"
        );
    }
}
