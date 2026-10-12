//! One replay's simulations, shared by its dry passes and its comparison.
//!
//! A replay does not record every code its console ran, so the runner tries
//! candidate setups (`replay::resolve_setup`): a dry pass steps two of them
//! side by side until the recording follows one, and the comparison then
//! runs the chosen setup. The same setup is asked for again and again (the
//! standard screen under the dated UCF version is one side of the stage
//! pass, of the screen pass and of the UCF pass, and usually the comparison
//! too), so a [`Game`] simulates each setup once: a [`Run`] advances only as
//! far as a reader asks and keeps what every tick showed
//! (`replay_observed`), and later readers start from its first tick without
//! simulating it again. What no setup changes is prepared once as well: the
//! scenario and its pads, the archives, the recorded frames.
//!
//! A new candidate dimension is a field in [`Codes`] and its own dry pass.
use crate::replay::{cold_scenario_base, set_candidate, unsupported_setup, Setup};
use crate::replay_cost::{self as cost, Part};
use crate::replay_observed::{member_frames, Expected, Observation, Target, MEMBERS};
use crate::replay_online::Online;
use crate::{
    frame::Simulation, initial_state::InitialState, inputs::PadScript, scenario::Scenario,
};
use anyhow::{ensure, Result};
use melee_diff::Record;
use melee_lib::diagnostics::{Assets, FighterValues};
use melee_types::snapshot::SnapValue;
use slp::Replay;
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

/// The first Slippi version whose Post Frame is sampled at the end of the
/// fighter procs rather than after the map proc.
const POST_FRAME_AT_CAMERA: slp::Version = slp::Version {
    major: 3,
    minor: 4,
    build: 0,
};

/// What a setup's simulation depends on: the codes a dry pass varies.
/// Everything else a [`Setup`] holds is the same for every run of a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Codes {
    frozen_stages: bool,
    screen: melee_lib::slippi::Screen,
    /// `Setup::controller_fix`: the UCF version named for the UCF ports.
    controller_fix: Option<melee_lib::ControllerFix>,
    /// `Setup::skipped_display_pass`, which only the comparison applies.
    skipped_display_pass: Option<u64>,
}

impl Codes {
    fn of(setup: Setup, skipped_display_pass: Option<u64>) -> Self {
        Self {
            frozen_stages: setup.frozen_stages.unwrap_or(false),
            screen: setup.screen.unwrap_or_default(),
            controller_fix: setup.controller_fix,
            skipped_display_pass,
        }
    }
}

/// A run of a [`Game`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RunId(usize);

/// Why a setup has no run.
pub(crate) enum StartError {
    /// The cold scenario could not be built (`Stop::NeedsSetup`).
    Scenario(anyhow::Error),
    /// The match could not be constructed or did not reach the recording's
    /// first seed.
    Start(anyhow::Error),
}

/// Why a run has no further tick.
pub(crate) enum Stopped {
    /// The tick returned an error (taken by the first reader that reports
    /// it; a poisoned simulation says no more).
    Error(Option<anyhow::Error>),
    /// The tick panicked with this message.
    Panic(String),
}

/// What every run of a replay starts from, whatever its setup.
struct Shared {
    pads: Arc<PadScript>,
    assets: Arc<Assets>,
}

/// One setup's simulation and every tick it has run.
struct Run {
    codes: Codes,
    simulation: Simulation,
    /// The seed at the end of the last tick run.
    previous_seed: u32,
    observed: Vec<Observation>,
    /// The fighters `Observation::members` name.
    values: Vec<FighterValues>,
    stopped: Option<Stopped>,
    /// The ticks (as reports number them) this run has no display pass
    /// before, beside `Codes::skipped_display_pass`: the lag frames a
    /// rewindable run was rewound to try (`replay_lag_frames`).
    lag_frames: Vec<u64>,
    /// Set for a rewindable run ([`Game::rewindable`]).
    checkpoints: Option<Checkpoints>,
}

/// A run as it was before one of its ticks, kept so that the run can go
/// back and take that tick again ([`Game::rewind`]).
pub(crate) struct Checkpoint {
    /// The tick the simulation runs next.
    pub(crate) index: usize,
    simulation: Simulation,
    previous_seed: u32,
    /// How many of the run's `values` precede the tick.
    values: usize,
}

impl Checkpoint {
    /// The run's simulation before tick `index`.
    pub(crate) fn simulation(&self) -> &Simulation {
        &self.simulation
    }
}

/// A rewindable run's recent checkpoints, oldest first.
struct Checkpoints {
    /// No tick before this one is kept.
    from: usize,
    recent: VecDeque<Rc<Checkpoint>>,
}

/// A rewindable run copies its simulation before every tick whose index is
/// a multiple of this.
const CHECKPOINT_INTERVAL: usize = 16;
/// How many ticks back a rewindable run can always go: the checkpoints
/// older than this (and one interval) are dropped.
pub(crate) const REWIND_REACH: usize = 64;

pub(crate) struct Game<'a> {
    pub(crate) replay: &'a Replay,
    root: &'a Path,
    /// The leaders' ports, ascending: `pN` is the Nth.
    leaders: Vec<usize>,
    /// The recorded frames from the match's first, one per tick.
    frames: Vec<&'a slp::Frame>,
    /// The first tick whose recorded controller cannot be installed. A bad
    /// late controller field must not discard an already comparable prefix,
    /// and no guessed pads stand in for it.
    unavailable: Option<(u64, String)>,
    /// The cold scenario; its candidate fields are rewritten for each run.
    scenario: Option<Scenario>,
    shared: Option<Shared>,
    expected: Option<Expected>,
    runs: Vec<Run>,
    /// Set for a Slippi Online game: each tick's seed and which ticks a
    /// rollback ran (`replay_online`).
    online: Option<Online>,
    /// Whether the comparison takes zeros of either sign as equal in
    /// `replay_online::ZERO_SIGN_FREE`: an online game's, unless the caller
    /// asked for strict zeros.
    zero_sign_free: bool,
}

impl<'a> Game<'a> {
    pub(crate) fn new(replay: &'a Replay, root: &'a Path) -> Self {
        Self {
            replay,
            root,
            leaders: replay.leader_ports().collect(),
            frames: replay
                .frames
                .values()
                .filter(|frame| slp::harness_frame(frame.number).is_some())
                .collect(),
            unavailable: None,
            scenario: None,
            shared: None,
            expected: None,
            runs: Vec::new(),
            online: Online::of(replay),
            zero_sign_free: false,
        }
    }

    /// Compare an online game's knockback zeros by sign too
    /// (`Setup::strict_zeros`), or leave their sign free.
    pub(crate) fn set_strict_zeros(&mut self, strict: bool) {
        self.zero_sign_free = self.online.is_some() && !strict;
    }

    /// The game's online facts, for a Slippi Online game.
    pub(crate) fn online(&self) -> Option<&Online> {
        self.online.as_ref()
    }

    /// How many ticks the recording holds.
    pub(crate) fn ticks(&self) -> usize {
        self.frames.len()
    }

    /// Tick `index` as reports number it.
    pub(crate) fn tick_number(&self, index: usize) -> u64 {
        slp::harness_frame(self.frames[index].number).expect("a frame of the match")
    }

    pub(crate) fn frame(&self, index: usize) -> &'a slp::Frame {
        self.frames[index]
    }

    pub(crate) fn leaders(&self) -> &[usize] {
        &self.leaders
    }

    /// The first tick whose recorded controller cannot be installed, known
    /// once a run has started.
    pub(crate) fn unavailable(&self) -> Option<&(u64, String)> {
        self.unavailable.as_ref()
    }

    /// The run of a setup as a dry pass steps it: started now unless an
    /// earlier reader already did.
    pub(crate) fn run(&mut self, setup: Setup) -> Result<RunId, StartError> {
        self.run_of(setup, Codes::of(setup, None))
    }

    /// The run the comparison reads: the dry passes' run of the setup,
    /// unless the comparison skips a display pass they did not.
    pub(crate) fn comparison_run(&mut self, setup: Setup) -> Result<RunId, StartError> {
        self.run_of(setup, Codes::of(setup, setup.skipped_display_pass))
    }

    fn run_of(&mut self, setup: Setup, codes: Codes) -> Result<RunId, StartError> {
        // A rewindable run is its reader's own: it may skip display passes.
        let shared = |run: &Run| run.codes == codes && run.checkpoints.is_none();
        if let Some(index) = self.runs.iter().position(shared) {
            return Ok(RunId(index));
        }
        let run = self.start(setup, codes)?;
        self.runs.push(run);
        Ok(RunId(self.runs.len() - 1))
    }

    /// A second simulation of the comparison's setup that can be rewound:
    /// it runs the same ticks as [`Self::comparison_run`]'s and keeps a
    /// copy of itself every few ticks from `from` on, [`REWIND_REACH`]
    /// ticks back. The shared runs keep no copies, so only a replay whose
    /// comparison stopped pays for one, by simulating its matched ticks
    /// once more.
    /// `lag_frames` are ticks it takes with no display pass before them
    /// from its start, as a run rewound to try them would
    /// ([`Self::rewind`]).
    pub(crate) fn rewindable(
        &mut self,
        setup: Setup,
        from: usize,
        lag_frames: Vec<u64>,
    ) -> Result<RunId, StartError> {
        let mut run = self.start(setup, Codes::of(setup, setup.skipped_display_pass))?;
        run.lag_frames = lag_frames;
        run.checkpoints = Some(Checkpoints {
            from,
            recent: VecDeque::new(),
        });
        self.runs.push(run);
        Ok(RunId(self.runs.len() - 1))
    }

    /// The rewindable run's latest checkpoint at or before tick `index`.
    pub(crate) fn checkpoint(&self, run: RunId, index: usize) -> Option<Rc<Checkpoint>> {
        let checkpoints = self.runs[run.0].checkpoints.as_ref()?;
        checkpoints
            .recent
            .iter()
            .rev()
            .find(|checkpoint| checkpoint.index <= index)
            .cloned()
    }

    /// Take the rewindable run back to a checkpoint: its ticks from there
    /// on are forgotten and run again, the ticks in `lag_frames` with no
    /// display pass before them.
    pub(crate) fn rewind(&mut self, run: RunId, to: &Rc<Checkpoint>, lag_frames: Vec<u64>) {
        let run = &mut self.runs[run.0];
        run.simulation = to.simulation.clone();
        run.previous_seed = to.previous_seed;
        run.observed.truncate(to.index);
        run.values.truncate(to.values);
        run.stopped = None;
        run.lag_frames = lag_frames;
        let recent = &mut run.checkpoints.as_mut().expect("a rewindable run").recent;
        recent.retain(|checkpoint| checkpoint.index < to.index);
        recent.push_back(Rc::clone(to));
    }

    /// The run's simulation as its last observed tick left it.
    pub(crate) fn simulation(&self, run: RunId) -> &Simulation {
        &self.runs[run.0].simulation
    }

    /// The ticks the run was rewound to take with no display pass before.
    pub(crate) fn lag_frames(&self, run: RunId) -> &[u64] {
        &self.runs[run.0].lag_frames
    }

    /// A replay's cold simulation, before its first compared tick.
    fn start(&mut self, setup: Setup, codes: Codes) -> Result<Run, StartError> {
        self.set_scenario(setup).map_err(StartError::Scenario)?;
        self.start_run(codes).map_err(StartError::Start)
    }

    /// The replay's cold scenario with the setup's codes.
    fn set_scenario(&mut self, setup: Setup) -> Result<()> {
        let _t = cost::time(Part::Scenario);
        ensure!(
            unsupported_setup(self.replay, setup).is_empty(),
            "unsupported replay setup"
        );
        match &mut self.scenario {
            Some(scenario) => set_candidate(self.replay, scenario, setup),
            None => self.scenario = Some(cold_scenario_base(self.replay, self.root, setup)?),
        }
        self.scenario.as_ref().expect("built").validate()
    }

    fn start_run(&mut self, codes: Codes) -> Result<Run> {
        let _t = cost::time(Part::Start);
        if self.shared.is_none() {
            self.shared = Some(self.share()?);
        }
        let (scenario, shared) = (
            self.scenario.as_ref().expect("built"),
            self.shared.as_ref().expect("shared"),
        );
        let state =
            InitialState::from_parameters_with_assets(scenario, Arc::clone(&shared.assets))?;
        let mut simulation = Simulation::with_shared_inputs(state, Arc::clone(&shared.pads));
        simulation.tick_without_snapshot()?;
        let seed = simulation.seed();
        // An online game's first Frame Start is the sync's write, not the
        // seed the setup reached (`advance` supplies it).
        let first_frame_start = match self.online {
            Some(_) => None,
            None => self
                .replay
                .frames
                .values()
                .next()
                .and_then(slp::Frame::scheduler_start_seed),
        };
        if let Some(first) = first_frame_start {
            ensure!(
                seed == first,
                "boundary seed does not reach first Frame Start"
            );
        }
        // Slippi before 3.4.0 sampled Post Frame at 0x8006C5D8, the end of
        // each fighter's map proc (Fighter_8006C27C, s_link 6), before hit
        // detection and damage; 3.4.0 moved it to 0x8006DA34 in the camera
        // proc (s_link 18), after the fighter procs the trace reads
        // (slippi-ssbm-asm 9398d52).
        simulation.capture_after_map_values(self.replay.version() < POST_FRAME_AT_CAMERA);
        Ok(Run {
            codes,
            simulation,
            previous_seed: seed,
            observed: Vec::with_capacity(self.frames.len()),
            values: Vec::new(),
            stopped: None,
            lag_frames: Vec::new(),
            checkpoints: None,
        })
    }

    /// The pads and archives every run of the replay starts from.
    fn share(&mut self) -> Result<Shared> {
        let scenario = self.scenario.as_ref().expect("built");
        self.unavailable = scenario.replay_inputs.iter().find_map(|input| {
            crate::inputs::replay_pad(input)
                .err()
                .map(|error| (input.frame, format!("port {}: {error}", input.port + 1)))
        });
        let pad_frames = self
            .unavailable
            .as_ref()
            .map_or(scenario.frames, |(frame, _)| *frame);
        let prefix: Vec<_> = scenario
            .replay_inputs
            .iter()
            .filter(|input| input.frame < pad_frames)
            .cloned()
            .collect();
        let pads = PadScript::from_replay_inputs(
            &prefix,
            pad_frames as usize,
            &scenario.fighters.iter().map(|f| f.slot).collect::<Vec<_>>(),
        )?;
        let assets = InitialState::load_assets(scenario)?;
        Ok(Shared {
            pads: Arc::new(pads),
            assets,
        })
    }

    /// Simulate the run through tick `index` unless it already has.
    /// `false` when the run stopped at or before that tick ([`Self::stop`]).
    pub(crate) fn observe(&mut self, run: RunId, index: usize) -> bool {
        while self.runs[run.0].observed.len() <= index {
            if self.runs[run.0].stopped.is_some() {
                return false;
            }
            self.advance(run);
        }
        true
    }

    /// Why the run has no tick `index`.
    pub(crate) fn stop(&mut self, run: RunId, _index: usize) -> Option<&mut Stopped> {
        self.runs[run.0].stopped.as_mut()
    }

    /// Run the run's next tick and keep what it showed.
    fn advance(&mut self, id: RunId) {
        let run = &mut self.runs[id.0];
        let index = run.observed.len();
        let tick = slp::harness_frame(self.frames[index].number).expect("a frame of the match");
        let frame = self.frames[index];
        if let Some(checkpoints) = &mut run.checkpoints {
            let due = index >= checkpoints.from && index.is_multiple_of(CHECKPOINT_INTERVAL);
            if due && checkpoints.recent.back().is_none_or(|last| last.index < index) {
                let oldest = index.saturating_sub(REWIND_REACH + CHECKPOINT_INTERVAL);
                checkpoints.recent.retain(|kept| kept.index >= oldest);
                checkpoints.recent.push_back(Rc::new(Checkpoint {
                    index,
                    simulation: run.simulation.clone(),
                    previous_seed: run.previous_seed,
                    values: run.values.len(),
                }));
            }
        }
        if let Some(online) = &self.online {
            // The netplay code's seed write (FN_SyncRNG, before the fighter
            // procs), as Frame Start recorded it; and for a tick a rollback
            // ran, the rollback loop in place of the display pass.
            run.previous_seed = online.seed(index);
            run.simulation.set_rng_seed(run.previous_seed);
            if online.resimulated(index) {
                run.simulation.rollback_loop_pass();
            }
        }
        if run.codes.skipped_display_pass == Some(tick) || run.lag_frames.contains(&tick) {
            run.simulation.skip_display_pass();
        }
        cost::ticks(1);
        let result = catch_unwind(AssertUnwindSafe(|| run.simulation.tick_without_snapshot()));
        match result {
            Ok(Ok(())) => {
                if self.expected.is_none() {
                    self.expected = Some(Expected::compile(
                        self.replay,
                        &self.frames,
                        run.simulation.fighter_paths(),
                        self.online.is_none(),
                    ));
                }
                let observation = self.expected.as_ref().expect("compiled").observe(
                    &run.simulation,
                    run.previous_seed,
                    &self.leaders,
                    frame,
                    &mut run.values,
                );
                run.previous_seed = observation.end_seed;
                run.observed.push(observation);
            }
            Ok(Err(error)) => run.stopped = Some(Stopped::Error(Some(error))),
            Err(payload) => {
                let message = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown panic");
                run.stopped = Some(Stopped::Panic(message.into()));
            }
        }
    }

    fn observation(&self, run: RunId, index: usize) -> (&Run, &Observation) {
        let run = &self.runs[run.0];
        (run, &run.observed[index])
    }

    fn member(&self, run: RunId, index: usize, member: usize) -> Option<&FighterValues> {
        let (run, observation) = self.observation(run, index);
        observation.members[member].map(|at| &run.values[at as usize])
    }

    /// The value an observed tick holds for a recorded key.
    fn value(&self, run: RunId, index: usize, target: Target) -> Option<SnapValue> {
        match target {
            Target::Seed => Some(SnapValue::U64(u64::from(
                self.observation(run, index).1.end_seed,
            ))),
            Target::Field { member, index: at } => self
                .member(run, index, usize::from(member))?
                .get(usize::from(at)),
            Target::Absent => None,
        }
    }

    fn expected(&self) -> &Expected {
        self.expected.as_ref().expect("a tick was observed")
    }

    /// Whether the observed tick holds every recorded value, the Pre Frame
    /// seeds included: no divergence from the recording on this tick.
    pub(crate) fn matches(&self, run: RunId, index: usize) -> bool {
        let (_, observation) = self.observation(run, index);
        self.expected()
            .entries(index)
            .iter()
            .all(|(target, value)| {
                let seen = self.value(run, index, *target);
                seen.as_ref() == Some(value) || self.zero_of_the_other_sign(*target, value, &seen)
            })
            && member_frames(&self.leaders, self.frames[index]).all(|(member, frame)| {
                match (observation.input_seeds[member], frame.pre.as_ref()) {
                    (Some(seen), Some(pre)) => seen == pre.random_seed,
                    _ => true,
                }
            })
    }

    /// Whether a recorded value and the port's are zeros of opposite sign
    /// in a field whose zero sign this comparison leaves free: an online
    /// game's knockback velocity (`replay_online::ZERO_SIGN_FREE`).
    fn zero_of_the_other_sign(
        &self,
        target: Target,
        recorded: &SnapValue,
        seen: &Option<SnapValue>,
    ) -> bool {
        let (Target::Field { index, .. }, true) = (target, self.zero_sign_free) else {
            return false;
        };
        let (SnapValue::F32(recorded), Some(SnapValue::F32(seen))) = (recorded, seen) else {
            return false;
        };
        self.expected().zero_sign_free(index)
            && crate::replay_online::zeros_of_opposite_sign(*recorded, *seen)
    }

    /// How many values of the run's first `ticks` ticks match the recording
    /// only by [`Self::zero_of_the_other_sign`].
    pub(crate) fn zeros_of_the_other_sign(&self, run: RunId, ticks: usize) -> usize {
        let observed = self.runs[run.0].observed.len();
        (0..ticks.min(observed))
            .flat_map(|index| {
                self.expected()
                    .entries(index)
                    .iter()
                    .map(move |(target, value)| (index, *target, value))
            })
            .filter(|&(index, target, value)| {
                self.zero_of_the_other_sign(target, value, &self.value(run, index, target))
            })
            .count()
    }

    /// Whether two runs' ticks are the same to the recording's eyes: the
    /// end seed, every value of each recorded fighter and the Pre Frame
    /// seeds the tick passed through.
    pub(crate) fn same(&self, a: RunId, b: RunId, index: usize) -> bool {
        let (first, second) = (self.observation(a, index).1, self.observation(b, index).1);
        first.end_seed == second.end_seed
            && first.input_seeds == second.input_seeds
            && (0..MEMBERS)
                .all(|member| self.member(a, index, member) == self.member(b, index, member))
    }

    /// Whether two runs' ticks agree on every key the recording holds for
    /// the tick (the Pre Frame seeds aside).
    pub(crate) fn same_recorded(&self, a: RunId, b: RunId, index: usize) -> bool {
        self.expected()
            .entries(index)
            .iter()
            .all(|(target, _)| self.value(a, index, *target) == self.value(b, index, *target))
    }

    /// A leader's `facing_dir` on an observed tick.
    pub(crate) fn facing(&self, run: RunId, index: usize, leader: usize) -> Option<SnapValue> {
        self.value(run, index, self.expected().facing(leader)?)
    }

    /// A leader's recorded `facing_dir` on a tick.
    pub(crate) fn recorded_facing(&self, index: usize, leader: usize) -> Option<&SnapValue> {
        let target = self.expected().facing(leader)?;
        self.expected()
            .entries(index)
            .iter()
            .find(|(recorded, _)| *recorded == target)
            .map(|(_, value)| value)
    }

    /// The recorded and the simulated record of an observed tick, as a
    /// report prints them (`Expected::records`).
    pub(crate) fn records(&self, run: RunId, index: usize) -> (Record, Record) {
        let (recorded, mut simulated) = self.records_as_observed(run, index);
        // A zero the comparison took as equal is not what stopped it: show
        // the recorded one, so the first divergence named is a real one.
        for (target, value) in self.expected().entries(index) {
            if !self.zero_of_the_other_sign(*target, value, &self.value(run, index, *target)) {
                continue;
            }
            let Some(key) = self.expected().key(*target) else {
                continue;
            };
            if let Some(recorded) = recorded.state.get(&key) {
                simulated.state.insert(key, recorded.clone());
            }
        }
        (recorded, simulated)
    }

    fn records_as_observed(&self, run: RunId, index: usize) -> (Record, Record) {
        self.expected().records(
            self.replay,
            &self.leaders,
            self.frames[index],
            self.frames
                .get(index + 1)
                .copied()
                .filter(|_| self.online.is_none()),
            self.observation(run, index).1,
            |member| self.member(run, index, member),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Replay {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../slp/tests/data")
            .join(name);
        Replay::parse(&std::fs::read(path).unwrap()).unwrap()
    }

    /// The knockback zeros' sign is left free only where the recording was
    /// computed by Slippi's Dolphin: a game the runner identified as online.
    /// In a console's game -0 against +0 stays a divergence, whatever the
    /// caller asks.
    #[test]
    fn only_an_online_game_leaves_the_sign_of_a_knockback_zero_free() {
        let root = Path::new(".");
        // (fixture, online, strict zeros asked for, sign left free)
        for (name, online, strict, free) in [
            ("v3.16.slp", true, false, true),
            ("v3.16.slp", true, true, false),
            ("netplay.slp", true, false, true),
            ("v3.18.slp", false, false, false),
            ("v3.18.slp", false, true, false),
            ("v3.13.slp", false, false, false),
            ("v0.1.slp", false, false, false),
        ] {
            let replay = fixture(name);
            let mut game = Game::new(&replay, root);
            assert_eq!(game.online().is_some(), online, "{name}");
            // Before the comparison says otherwise, nothing is free.
            assert!(!game.zero_sign_free, "{name}");
            game.set_strict_zeros(strict);
            assert_eq!(game.zero_sign_free, free, "{name} strict {strict}");
        }
    }
}
