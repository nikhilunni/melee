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
    characters: [Character; 2],
}
impl GameAssets {
    pub fn load(directory: impl AsRef<Path>, config: &MatchConfig) -> Result<Self, StartError> {
        let setup = config.setup()?;
        let inner = Assets::load(directory.as_ref(), &setup.roster_descriptors(), setup.stage)
            .map_err(|e| StartError::Load(format!("{e:#}")))?;
        Ok(Self {
            inner: Arc::new(inner),
            stage: config.stage,
            characters: config.players.each_ref().map(|p| p.character),
        })
    }
    fn compatible(&self, config: &MatchConfig) -> bool {
        self.stage == config.stage
            && self.characters == config.players.each_ref().map(|p| p.character)
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
        let stage = match setup.stage.name {
            "FinalDestination" => Stage::FinalDestination,
            "Battlefield" => Stage::Battlefield,
            "YoshisStory" => Stage::YoshisStory,
            "DreamLand" => Stage::DreamLand,
            "FountainOfDreams" => Stage::FountainOfDreams,
            "PokemonStadium" => Stage::PokemonStadium,
            _ => unreachable!("validated imported stage"),
        };
        let players = std::array::from_fn(|i| {
            let f = &engine.state().player_fighter(i).0;
            PlayerConfig {
                port: Port::from_index(f.player.id),
                character: Character::from_kind(f.kind),
                costume: Costume(f.player.costume),
            }
        });
        let config = MatchConfig::versus(stage, players)
            .with_stocks(setup.fighters[0].stocks)
            .with_seed(Seed(setup.seed.unwrap_or(0)));
        let assets = GameAssets {
            inner: Arc::clone(&engine.state().assets),
            stage,
            characters: config.players.each_ref().map(|p| p.character),
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
                engine.state().fighters.len(),
            )
        }))
        .map_err(|p| StartError::Initialization(panic_message(p)))?
        .map_err(|e| StartError::Initialization(format!("{e:#}")))?;
        let mut config = self.config.clone();
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
        let fighters = [state.player_fighter(0), state.player_fighter(1)];
        if self.engine.state().clock.timed_out() {
            // The results screen ranks a timed-out stock match by stocks.
            let stocks = fighters.each_ref().map(|f| f.0.player.stocks);
            return MatchStatus::Finished(match stocks[0].cmp(&stocks[1]) {
                std::cmp::Ordering::Greater => {
                    MatchOutcome::Winner(Port::from_index(fighters[0].0.player.id))
                }
                std::cmp::Ordering::Less => {
                    MatchOutcome::Winner(Port::from_index(fighters[1].0.player.id))
                }
                std::cmp::Ordering::Equal => MatchOutcome::SuddenDeath,
            });
        }
        let alive = fighters.each_ref().map(|f| f.0.player.stocks > 0);
        match alive {
            [true, false] => MatchStatus::Finished(MatchOutcome::Winner(Port::from_index(
                fighters[0].0.player.id,
            ))),
            [false, true] => MatchStatus::Finished(MatchOutcome::Winner(Port::from_index(
                fighters[1].0.player.id,
            ))),
            [false, false] => MatchStatus::Finished(MatchOutcome::Draw),
            _ if state.fighters.iter().any(|f| f.0.status.input_frozen) => MatchStatus::Startup,
            _ => MatchStatus::Playing,
        }
    }
    pub fn observe(&self) -> Result<Observation<'_>, StateError> {
        if self.engine.is_faulted() {
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
