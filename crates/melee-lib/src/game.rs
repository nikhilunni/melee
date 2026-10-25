use crate::{assets::Assets, frame::Simulation, initial_state::InitialState, *};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    sync::Arc,
};

/// Immutable resources shared by matches with the same stage and character slots.
/// Costumes, ports, rules, and seeds may vary without loading another resource set.
#[derive(Clone)]
pub struct GameAssets {
    pub(crate) inner: Arc<Assets>,
    stage: Stage,
    characters: Vec<Character>,
}
impl GameAssets {
    pub fn load(directory: impl AsRef<Path>, config: &MatchConfig) -> Result<Self, StartError> {
        Self::load_from(&directory.as_ref(), config)
    }
    /// Load from any file source, such as a disc image or files fetched
    /// ahead of time (see [`GameAssets::files`]).
    pub fn load_from(source: &dyn FileSource, config: &MatchConfig) -> Result<Self, StartError> {
        let setup = config.setup()?;
        let inner = Assets::load(source, &setup.roster_descriptors(), setup.stage)
            .map_err(|e| StartError::Load(format!("{e:#}")))?;
        Ok(Self {
            inner: Arc::new(inner),
            stage: config.stage,
            characters: config.players.iter().map(|p| p.character).collect(),
        })
    }
    /// Every file [`GameAssets::load_from`] reads for this configuration.
    pub fn files(config: &MatchConfig) -> Result<Vec<&'static str>, StartError> {
        let setup = config.setup()?;
        Ok(Assets::files(&setup.roster_descriptors(), setup.stage))
    }
    fn compatible(&self, config: &MatchConfig) -> bool {
        self.stage == config.stage
            && self
                .characters
                .iter()
                .copied()
                .eq(config.players.iter().map(|p| p.character))
    }
}

/// fn_8016D634's hold after an outcome: StartMeleeRules xD, 110 frames in
/// Versus.
const VERSUS_RESULT_HOLD_FRAMES: usize = 110;
/// From the outcome's frozen tick to the scene exit: the outcome tick, the
/// hold (`unk_30++ <= xD`), the frame that sets unk_0 = 3 and the frame whose
/// gm_801A4B60 leaves the scene.
const SCENE_EXIT_TICKS: usize = 1 + (VERSUS_RESULT_HOLD_FRAMES + 1) + 1 + 1;

/// Successful public ticks since construction, or since reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchOutcome {
    Winner(Port),
    /// A team battle's last team with stocks (gm_GetTeamBattleOutcome).
    TeamWinner(u8),
    Draw,
    /// The timer ran out with equal stocks; retail continues with a Sudden
    /// Death match (one stock each at 300%).
    SuddenDeath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchStatus {
    Startup,
    Playing,
    Finished(MatchOutcome),
    Faulted,
}
impl MatchStatus {
    pub fn is_running(self) -> bool {
        matches!(self, Self::Startup | Self::Playing)
    }
}

/// One independently owned match. Clone copies all mutable continuation state
/// and shares only immutable assets. Cloning/reset may allocate; stepping does not.
/// No renderer, input history, worker threads, or wall clock is retained here.
#[derive(Clone)]
pub struct Match {
    pub(crate) engine: Simulation,
    pub(crate) assets: GameAssets,
    pub(crate) config: MatchConfig,
}
impl Match {
    pub(crate) fn from_import(engine: Simulation, setup: &crate::setup::Setup) -> Self {
        let stage = Stage::from_name(setup.stage.name).expect("validated imported stage");
        let players: Vec<PlayerConfig> = (0..setup.fighters.len())
            .map(|i| {
                let f = &engine.state().player_fighter(i).0;
                PlayerConfig {
                    port: Port::from_index(f.player.id),
                    character: Character::from_kind(f.kind),
                    costume: Costume(f.player.costume),
                    controller_fix: engine.state().controller_fixes[usize::from(f.player.id)],
                    team: f.player.team,
                    cpu_level: (f.player.control == melee_types::PlayerKind::Cpu)
                        .then_some(f.player.cpu_level as u8),
                    handicap: crate::setup::HANDICAP_RATIOS
                        .iter()
                        .position(|&(attack, defense)| {
                            (attack, defense) == (f.player.attack_ratio, f.player.defense_ratio)
                        })
                        .map(|i| i as u8 + 1),
                }
            })
            .collect();
        let mut config = MatchConfig::versus(stage, players)
            .with_stocks(setup.fighters[0].stocks)
            .with_seed(Seed(setup.seed.unwrap_or(0)));
        config.rules.friendly_fire = engine.state().player_fighter(0).0.player.friendly_fire;
        config.rules.items = engine.state().item_rules;
        config.rules.mode = engine.state().clock.scoring.match_kind;
        config.rules.special = setup.special;
        let assets = GameAssets {
            inner: Arc::clone(&engine.state().assets),
            stage,
            characters: config.players.iter().map(|p| p.character).collect(),
        };
        Self {
            engine,
            assets,
            config,
        }
    }
    pub fn new(assets: &GameAssets, config: MatchConfig) -> Result<Self, StartError> {
        let setup = config.setup()?;
        if !assets.compatible(&config) {
            return Err(StartError::IncompatibleAssets);
        }
        Self::from_setup(assets, config, setup)
    }

    /// The Sudden Death match retail plays after this one timed out on a
    /// stock tie. The finished scene first runs frozen to its exit (TIME!
    /// held for StartMeleeRules xD frames, fn_8016D634), then the tie screen
    /// leaves the RNG untouched and the new scene's setup draws carry it on.
    pub fn sudden_death(&self) -> Result<Self, StartError> {
        if self.status() != MatchStatus::Finished(MatchOutcome::SuddenDeath) {
            return Err(StartError::InvalidConfig(
                "Sudden Death follows a timed-out stock tie",
            ));
        }
        let mut engine = self.engine.clone();
        let exited = catch_unwind(AssertUnwindSafe(|| {
            for _ in 0..SCENE_EXIT_TICKS {
                engine.tick_without_snapshot()?;
            }
            crate::initial_state::boundary_seed_after(
                engine.state().rng.seed,
                engine.state().assets.stage_desc.kind,
                engine
                    .state()
                    .fighters
                    .iter()
                    .map(|fighter| fighter.0.core.kind),
            )
        }))
        .map_err(|p| StartError::Initialization(panic_message(p)))?
        .map_err(|e| StartError::Initialization(format!("{e:#}")))?;
        // gm_SetupSuddenDeath: only the winners play on (the others' slots
        // go to Gm_PKind_NA).
        {
            let state = self.engine.state();
            let ranks = crate::standings::current_ranks(state);
            if (0..self.config.players.len())
                .any(|p| ranks[usize::from(state.player_fighter(p).0.player.id)] != 0)
            {
                return Err(StartError::InvalidConfig(
                    "a Sudden Death without every player needs a retail witness",
                ));
            }
        }
        let mut config = self.config.clone();
        config.rules.mode = crate::MatchKind::Stock;
        config.rules.stocks = 1;
        config.rules.time_limit_seconds = None;
        config.rules.sudden_death = true;
        config.seed = Seed(exited);
        let setup = config.setup()?;
        Self::from_setup(&self.assets, config, setup)
    }

    fn from_setup(
        assets: &GameAssets,
        config: MatchConfig,
        setup: crate::setup::Setup,
    ) -> Result<Self, StartError> {
        let engine = catch_unwind(AssertUnwindSafe(|| {
            let state = InitialState::from_assets(&setup, Arc::clone(&assets.inner))?;
            let mut engine = Simulation::new(state);
            // The oracle's tick zero completes the pre-music/reset boundary;
            // no scheduled gameplay callback runs there. Keep that boundary in
            // diagnostics, but expose full scheduler ticks to application callers.
            engine.complete_setup()?;
            Ok::<_, anyhow::Error>(engine)
        }))
        .map_err(|p| StartError::Initialization(panic_message(p)))?
        .map_err(|e| StartError::Initialization(format!("{e:#}")))?;
        Ok(Self {
            engine,
            assets: assets.clone(),
            config,
        })
    }

    /// Advance exactly one tick. A terminal tick succeeds; subsequent calls
    /// reject the request. Invalid samples are checked before any mutation.
    /// External events take the port's default policies
    /// ([`ExternalEvents::default`]).
    pub fn step(&mut self, inputs: &Inputs) -> Result<(), StepError> {
        self.step_with_events(inputs, &ExternalEvents::default())
    }
    /// [`Self::step`] with this tick's external events, as a recording
    /// observed them (see [`ExternalEvents`]).
    pub fn step_with_events(
        &mut self,
        inputs: &Inputs,
        events: &ExternalEvents,
    ) -> Result<(), StepError> {
        match self.status() {
            MatchStatus::Faulted => return Err(StepError::Faulted),
            MatchStatus::Finished(_) => return Err(StepError::Finished),
            _ => {}
        }
        inputs.validate()?;
        // HSD_PadRenewMasterStatus derives the stick direction bits on every
        // read; a caller's samples carry only the physical state.
        let mut pads = inputs.0;
        for pad in &mut pads {
            *pad = pad.with_stick_directions();
        }
        self.engine.set_inputs(pads);
        self.engine.set_external_events(*events);
        let result = catch_unwind(AssertUnwindSafe(|| self.engine.tick_without_snapshot()));
        let error = match result {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(error)) => format!("{error:#}"),
            Err(panic) => panic_message(panic),
        };
        self.engine.poison(&error);
        Err(StepError::Simulation(error))
    }
    /// What the last step consumed from its external events (the outcome
    /// of each poll, whichever policy decided it). A recorder stores these
    /// to replay the match exactly.
    pub fn consumed_events(&self) -> ConsumedEvents {
        self.engine.consumed_events()
    }
    pub fn tick(&self) -> Tick {
        Tick(self.engine.frame())
    }
    /// The configuration this match was built from (Sudden Death's is derived).
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }
    pub fn status(&self) -> MatchStatus {
        if self.engine.is_faulted() {
            return MatchStatus::Faulted;
        }
        let state = self.engine.state();
        let fighters = || (0..self.config.players.len())
            .map(|player| &**state.player_fighter(player).0);
        let port = |f: &melee_ft::fighter::Fighter| Port::from_index(f.player.id);
        if self.engine.state().clock.timed_out() {
            // The results rank the standings (gm_80166378: stocks, or a
            // Time match's score); more than one winner, or winning team
            // (fn_80165FA4), goes to Sudden Death (gmVsMelee_ExitVs).
            let ranks = crate::standings::current_ranks(state);
            let mut top = fighters().filter(|f| ranks[usize::from(f.player.id)] == 0);
            if fighters().next().is_some_and(|f| f.player.team.is_some()) {
                let mut teams = top.filter_map(|f| f.player.team);
                let first = teams.next();
                return MatchStatus::Finished(match first {
                    Some(team) if teams.all(|t| t == team) => MatchOutcome::TeamWinner(team),
                    _ => MatchOutcome::SuddenDeath,
                });
            }
            return MatchStatus::Finished(match (top.next(), top.next()) {
                (Some(winner), None) => MatchOutcome::Winner(port(winner)),
                _ => MatchOutcome::SuddenDeath,
            });
        }
        if fighters().next().is_some_and(|f| f.player.team.is_some()) {
            let decided = crate::frame::stock_outcome_decided(
                fighters().map(|f| (f.player.team, f.player.stocks)),
            );
            let survivor = fighters().find(|f| f.player.stocks > 0);
            return match (decided, survivor) {
                (true, Some(f)) => MatchStatus::Finished(MatchOutcome::TeamWinner(
                    f.player.team.expect("a team battle's player"),
                )),
                (true, None) => MatchStatus::Finished(MatchOutcome::Draw),
                _ if state.fighters.iter().any(|f| f.0.status.input_frozen) => {
                    MatchStatus::Startup
                }
                _ => MatchStatus::Playing,
            };
        }
        let mut alive = fighters().filter(|f| f.player.stocks > 0);
        match (alive.next(), alive.next()) {
            (Some(winner), None) => MatchStatus::Finished(MatchOutcome::Winner(port(winner))),
            (None, _) => MatchStatus::Finished(MatchOutcome::Draw),
            _ if state.fighters.iter().any(|f| f.0.status.input_frozen) => MatchStatus::Startup,
            _ => MatchStatus::Playing,
        }
    }
    pub fn observe(&self) -> Result<Observation<'_>, StateError> {
        if self.status() == MatchStatus::Faulted {
            return Err(StateError::Faulted);
        }
        Ok(Observation::new(
            self.engine.state(),
            self.tick(),
            self.status(),
            self.config.stage,
        ))
    }
    pub fn reset(&mut self, seed: Seed) -> Result<(), StartError> {
        let mut config = self.config.clone();
        config.seed = seed;
        let replacement = Self::new(&self.assets, config)?;
        *self = replacement;
        Ok(())
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "simulation panicked with a non-string payload".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timed_team_setups_are_accepted() {
        let mut config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox).with_team(0),
                PlayerConfig::new(Port::P2, Character::Marth).with_team(1),
            ],
        );
        config.rules.time_limit_seconds = Some(60);
        assert!(config.setup().is_ok());
        config.rules.mode = crate::MatchKind::Time;
        assert!(config.setup().is_ok());
    }

    /// A timed-out team battle ranks teams by fn_80165E7C's scores, not one
    /// player's stocks.
    #[test]
    fn a_team_timeout_ranks_teams() {
        let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            std::path::PathBuf::from,
        );
        let files = root.join("harness/roms/files");
        if !melee_test_support::require_files([files.join("PlCo.dat")]) {
            return;
        }
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox).with_team(0),
                PlayerConfig::new(Port::P2, Character::Marth).with_team(1),
            ],
        );
        let assets = GameAssets::load(files, &config).unwrap();
        let setup = config.setup().unwrap();
        let mut state = InitialState::from_assets(&setup, Arc::clone(&assets.inner)).unwrap();
        state.clock.timer = Some(crate::match_clock::CountdownTimer { seconds: 0, frames: 59 });
        state.fighters[0].0.player.stocks = 1;
        state.fighters[1].0.player.stocks = 4;
        let game = Match::from_import(Simulation::new(state.clone()), &setup);
        assert_eq!(
            game.status(),
            MatchStatus::Finished(MatchOutcome::TeamWinner(1))
        );
        let mut simulation = Simulation::new(state);
        simulation.tick_without_snapshot().unwrap();
    }

    #[test]
    fn faults_reject_access_and_clone_until_reset_or_replacement() {
        let files = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
        if !melee_test_support::require_files([files.join("PlCo.dat")]) {
            return;
        }
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox),
                PlayerConfig::new(Port::P2, Character::Marth),
            ],
        )
        .with_seed(Seed(42));
        let assets = GameAssets::load(files, &config).unwrap();
        let healthy = Match::new(&assets, config).unwrap();
        let mut faulted = healthy.clone();
        faulted.engine.poison("test simulation failure");
        assert_eq!(faulted.status(), MatchStatus::Faulted);
        assert!(matches!(faulted.observe(), Err(StateError::Faulted)));
        assert_eq!(faulted.step(&Inputs::default()), Err(StepError::Faulted));
        assert_eq!(faulted.tick(), Tick(0));
        let mut clone = faulted.clone();
        assert_eq!(clone.status(), MatchStatus::Faulted);
        clone.reset(Seed(42)).unwrap();
        assert_eq!(
            crate::diagnostics::inspect(&clone).unwrap(),
            crate::diagnostics::inspect(&healthy).unwrap()
        );
        faulted.clone_from(&healthy);
        assert_eq!(
            crate::diagnostics::inspect(&faulted).unwrap(),
            crate::diagnostics::inspect(&healthy).unwrap()
        );
        // A rejected reset must leave the old match intact. Invalid retained
        // configuration is injectable here only because this is an internal test.
        faulted.config.rules.stocks = 0;
        let before = crate::diagnostics::inspect(&faulted).unwrap();
        assert!(faulted.reset(Seed(99)).is_err());
        assert_eq!(before, crate::diagnostics::inspect(&faulted).unwrap());
    }
}
