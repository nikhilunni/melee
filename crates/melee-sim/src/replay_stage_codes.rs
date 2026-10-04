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
use crate::replay::{
    cold_scenario, compared_tick, dated_controller_fix, unsupported_setup, ColdRun, Setup,
};
use anyhow::Result;
use slp::Replay;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};

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

/// Choose the stage code for a recording on a stage it changes. `None` on
/// the other stages, where the question does not arise.
pub fn resolve_stage_codes(
    replay: &Replay,
    root: &Path,
    setup: Setup,
) -> (Setup, Option<StageCodeChoice>) {
    if !FROZEN_STAGE_IDS.contains(&replay.start.stage) {
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
        let probe = catch_unwind(AssertUnwindSafe(|| {
            recorded_stage(replay, root, dated(false), dated(true))
        }));
        if let Ok(Ok(Some(recorded))) = probe {
            choice = recorded;
        }
    }
    (named(choice.frozen), Some(choice))
}

/// Run the replay's inputs without and with the code in step. Until the two
/// runs differ (a compared field or the tick's end seed) nothing is asked of
/// the recording. From then on each is compared with it as the comparison
/// run would be, Pre Frame seeds included: the first tick only one of them
/// matches names that one. `None` when they never differ, or both stop
/// matching on the same tick.
fn recorded_stage(
    replay: &Replay,
    root: &Path,
    plain: Setup,
    frozen: Setup,
) -> Result<Option<StageCodeChoice>> {
    let mut runs = [
        ColdRun::start(replay, &cold_scenario(replay, root, plain)?)?,
        ColdRun::start(replay, &cold_scenario(replay, root, frozen)?)?,
    ];
    let leaders: Vec<usize> = replay.leader_ports().collect();
    let last_tick = runs[0].unavailable.as_ref().map(|(tick, _)| *tick);
    let mut previous_seeds = runs
        .each_ref()
        .map(|run| crate::replay::seed_of(&run.initial));
    let mut apart = None;
    for expected in slp::to_trace(replay) {
        if last_tick == Some(expected.frame) {
            break;
        }
        let tick = expected.frame;
        let frame = &replay.frames[&(tick as i32 + slp::SLIPPI_FIRST_FRAME)];
        let mut compared = Vec::with_capacity(2);
        for (run, previous) in runs.iter_mut().zip(&mut previous_seeds) {
            let record = run.simulation.tick()?;
            let record = run.simulation.after_map_record(&record).unwrap_or(record);
            let tick = compared_tick(&record, &leaders, frame, expected.clone(), *previous);
            *previous = tick.end_seed;
            compared.push(tick);
        }
        let (plain, frozen) = (&compared[0], &compared[1]);
        if apart.is_none()
            && (plain.end_seed != frozen.end_seed || plain.actual.state != frozen.actual.state)
        {
            apart = Some(tick);
        }
        let Some(apart) = apart else { continue };
        let reason = StageCodeReason::Recorded { apart, tick };
        match (plain.matches(), frozen.matches()) {
            (true, true) => {}
            (false, false) => return Ok(None),
            (_, frozen) => return Ok(Some(StageCodeChoice { frozen, reason })),
        }
    }
    Ok(None)
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
