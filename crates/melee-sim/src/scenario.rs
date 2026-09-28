//! TOML scenario loading: registered characters and stages, with a
//! scripted input schedule for one or more ports.
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub name: String,
    pub savestate: Option<PathBuf>,
    /// Comparison trace name, independent of construction and scenario name.
    pub expected: Option<String>,
    /// Ground music rule 6; required explicitly for a cold Versus scene.
    pub all_characters_unlocked: Option<bool>,
    pub frames: u64,
    /// Cold-start seed; a restored match uses its saved seed instead.
    #[serde(default)]
    pub seed: Option<u32>,
    /// A cold Sudden Death scene (gm_SetupSuddenDeath): one stock at 300%.
    #[serde(default)]
    pub sudden_death: bool,
    /// A cold match's counting-down timer, in seconds.
    #[serde(default)]
    pub time_limit: Option<u32>,
    pub stage: String,
    pub fighters: Vec<FighterScenario>,
    /// The VI-frame schedule that drove Dolphin. The port itself replays the
    /// per-tick pads recorded beside the expected trace (`inputs.rs`); this
    /// list only says whether the scenario is scripted.
    #[serde(default)]
    pub inputs: Vec<InputStep>,
    /// Harness clock of `inputs` frames: `vi` (default) or `tick`, where the
    /// tracer injects each step's `raw` pad into the tick it names.
    #[serde(default)]
    pub input_clock: Option<String>,
    /// Per-tick Slippi inputs, independent of comparison traces.
    #[serde(default)]
    pub replay_inputs: Vec<slp::cold::ControllerFrame>,
    pub replay_rules: Option<slp::cold::ReplayRules>,
    #[serde(skip)]
    pub root: PathBuf,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FighterScenario {
    pub slot: u8,
    pub kind: String,
    pub controller: String,
    #[serde(default)]
    pub costume: u8,
    #[serde(default = "default_spawn_point")]
    pub spawn_point: i8,
    #[serde(default = "one_stock")]
    pub stocks: u8,
}
fn one_stock() -> u8 {
    1
}
fn default_spawn_point() -> i8 {
    -1
}
/// One step of the Dolphin-side schedule: `buttons` holds on `port` from VI
/// frame `frame` until that port's next step. Keys are the GC pad names the
/// harness accepts (`remote_proto.GC_KEYS`); an empty table is neutral.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputStep {
    pub frame: u64,
    #[serde(default)]
    pub port: u8,
    #[serde(default)]
    pub buttons: toml::Table,
    /// Raw PADStatus values for the harness's tick input clock.
    #[serde(default)]
    pub raw: toml::Table,
}
impl Scenario {
    pub fn load(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .with_context(|| format!("scenario {}", path.display()))?;
        let mut scenario: Self = toml::from_str(&fs::read_to_string(&path)?)?;
        scenario.root = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .context("scenario must be under harness/scenarios")?
            .to_path_buf();
        // The decomp and new recordings use Mars for Marth's retail kind.
        for fighter in &mut scenario.fighters {
            if fighter.kind == "Mars" {
                fighter.kind = "Marth".into();
            }
        }
        scenario.validate()?;
        Ok(scenario)
    }
    pub fn stage_descriptor(&self) -> &'static crate::scene_stage::StageDescriptor {
        crate::scene_stage::descriptor(&self.stage).expect("validated stage")
    }
    pub fn validate(&self) -> Result<()> {
        if self.is_cold() {
            ensure!(
                self.seed.is_some(),
                "cold setup requires an explicit boundary seed"
            );
            ensure!(
                matches!(
                    self.stage.as_str(),
                    "FinalDestination"
                        | "Battlefield"
                        | "YoshisStory"
                        | "DreamLand"
                        | "FountainOfDreams"
                ),
                "cold setup supports FD, Battlefield, Yoshi's Story, Dream Land and Fountain of Dreams"
            );
            ensure!(
                self.all_characters_unlocked.is_some(),
                "cold setup requires all_characters_unlocked"
            );
            ensure!(
                self.inputs.is_empty(),
                "cold setup accepts replay_inputs, not a Dolphin VI input schedule"
            );
        }
        ensure!(
            crate::scene_stage::descriptor(&self.stage).is_some(),
            "unsupported stage"
        );
        // Recordings are bounded by the tracer's run, not by the importer; the
        // eight-minute acceptance match (S11) is 28,800 ticks.
        ensure!(
            self.frames > 0 && self.frames <= 30_000,
            "scenario frames must be in 1..=30000"
        );
        ensure!(self.fighters.len() == 2, "requires two fighters");
        let mut previous_port = None;
        for (slot, fighter) in self.fighters.iter().enumerate() {
            ensure!(
                ((self.is_cold()
                    && fighter.slot < 4
                    && previous_port.is_none_or(|p| p < fighter.slot))
                    || (!self.is_cold() && usize::from(fighter.slot) == slot))
                    && melee_lib::diagnostics::CHARACTERS.contains(&fighter.kind.as_str())
                    && matches!(fighter.controller.as_str(), "scripted" | "idle" | "human"),
                "requires ascending distinct human ports with a registered character kind"
            );
            previous_port = Some(fighter.slot);
            if self.is_cold() {
                ensure!(
                    usize::from(fighter.costume) < fighter.descriptor().costumes.len()
                        && (1..=99).contains(&fighter.stocks),
                    "cold setup requires a valid costume and stock count"
                );
            }
        }
        if let Some(rules) = &self.replay_rules {
            ensure!(self.is_cold(), "replay rules require cold setup");
            ensure!(
                rules.game_mode == 1
                    && matches!(rules.timer_type, 0 | 2)
                    && !rules.teams
                    && rules.item_spawn_behavior == -1
                    && rules.damage_ratio.to_bits() == 1.0_f32.to_bits(),
                "replay requires stock mode, countdown/no timer, singles, items off, normal damage"
            );
        }
        for step in &self.inputs {
            ensure!(
                usize::from(step.port) < crate::inputs::PORTS,
                "input step port {} out of range",
                step.port
            );
        }
        Ok(())
    }
    /// Whether the Dolphin-side schedule ever leaves neutral.
    pub fn is_scripted(&self) -> bool {
        self.inputs.iter().any(|step| !step.buttons.is_empty())
    }
    pub fn is_cold(&self) -> bool {
        self.savestate.is_none()
    }
    pub fn expected_path(&self) -> PathBuf {
        self.root.join("harness/traces").join(format!(
            "{}.tick.expected.jsonl",
            self.expected.as_deref().unwrap_or(&self.name)
        ))
    }
    /// Traces recorded for this scenario (its own tick trace).
    pub fn trace_path(&self, suffix: &str) -> PathBuf {
        self.root
            .join("harness/traces")
            .join(format!("{}.{suffix}", self.name))
    }
    /// The scenario whose captures describe this savestate's boundary: the
    /// savestate's file stem (`idle_fd_fox.sav` -> `idle_fd_fox`). Several
    /// scripted scenarios start from one savestate and share its particle
    /// dump and RNG ledger.
    pub fn boundary_name(&self) -> String {
        self.savestate
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.name.clone())
    }
    pub fn boundary_path(&self, suffix: &str) -> PathBuf {
        let shared = self
            .root
            .join("harness/traces")
            .join(format!("{}.{suffix}", self.boundary_name()));
        if melee_trace_io::exists(&shared) {
            return shared;
        }
        // A newly captured savestate may have its initial population alongside
        // the scripted scene. The importer still verifies the saved RNG seed.
        self.trace_path(if suffix == "ledger600.raw.jsonl" {
            "ledger.raw.jsonl"
        } else {
            suffix
        })
    }
    pub fn assets_path(&self) -> PathBuf {
        self.root.join("harness/roms/files")
    }
    pub fn savestate_path(&self) -> PathBuf {
        self.root
            .join(self.savestate.as_ref().expect("saved scenario"))
    }
    /// Local assets/captures required to run this scenario's oracle.
    pub fn required_files(&self) -> Vec<PathBuf> {
        let mut paths = [
            "PlCo.dat",
            self.stage_descriptor().file,
            "EfCoData.dat",
            "EfFxData.dat",
            "ItCo.dat",
            "PlFx.dat",
            "PlFc.dat",
        ]
        .map(|n| self.assets_path().join(n))
        .to_vec();
        for fighter in &self.fighters {
            let descriptor = fighter.descriptor();
            paths.extend(
                [descriptor.data_file, descriptor.animation_file]
                    .map(|n| self.assets_path().join(n)),
            );
            paths.extend(
                descriptor
                    .costumes
                    .iter()
                    .map(|c| self.assets_path().join(c.file)),
            );
        }
        if self.is_cold() {
            paths.push(self.assets_path().join("IfAll.usd"));
            paths.push(self.expected_path());
            return paths;
        }
        paths.push(self.savestate_path());
        paths.push(self.savestate_path().with_extension("sav.json"));
        paths.push(self.trace_path("tick.raw.jsonl"));
        paths.push(self.trace_path("tick.expected.jsonl"));
        paths.extend(
            [
                "ledger600.raw.jsonl",
                "particles.jsonl.initial.jsonl",
                "particles.jsonl.initial.jsonl.meta.json",
            ]
            .map(|s| self.boundary_path(s)),
        );
        paths
    }
}

impl FighterScenario {
    /// Composition root selects a character crate; gameplay uses its callbacks.
    pub fn descriptor(&self) -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        melee_lib::diagnostics::character_descriptor(&self.kind)
            .unwrap_or_else(|| unreachable!("validated scenario kind {}", self.kind))
    }
}

impl melee_lib::diagnostics::ScenarioSource for Scenario {
    fn setup(&self) -> anyhow::Result<melee_lib::diagnostics::Setup> {
        self.validate()?;
        Ok(melee_lib::diagnostics::Setup {
            fighters: std::array::from_fn(|p| {
                let f = &self.fighters[p];
                melee_lib::diagnostics::PlayerSetup {
                    slot: f.slot,
                    descriptor: f.descriptor(),
                    costume: f.costume,
                    stocks: f.stocks,
                    spawn_point: f.spawn_point,
                }
            }),
            stage: self.stage_descriptor(),
            seed: self.seed,
            all_characters_unlocked: self.all_characters_unlocked,
            time_limit: self.time_limit,
            sudden_death: self.sudden_death,
        })
    }
    fn is_cold(&self) -> bool {
        self.is_cold()
    }
    fn frames(&self) -> u64 {
        self.frames
    }
    fn assets_path(&self) -> PathBuf {
        self.assets_path()
    }
    fn trace_path(&self, suffix: &str) -> PathBuf {
        self.trace_path(suffix)
    }
    fn boundary_path(&self, suffix: &str) -> PathBuf {
        self.boundary_path(suffix)
    }
    fn savestate_path(&self) -> PathBuf {
        self.savestate_path()
    }
    fn open_trace(&self, path: &Path) -> anyhow::Result<Box<dyn std::io::BufRead>> {
        Ok(Box::new(melee_trace_io::open(path)?))
    }
}
