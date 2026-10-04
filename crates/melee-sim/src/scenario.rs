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
    /// The scenario the port gates in this one's place: the cold twin of a
    /// recording whose savestate the importer does not cover (fighters on
    /// slots other than 0..n, a replaced boundary seed). `load` follows it.
    #[serde(default)]
    pub gate: Option<String>,
    /// The seed the recorder wrote over the savestate's before the first
    /// tick (another match's draws from the same boundary). Such a recording
    /// gates through its cold twin: `seed` builds the boundary as the
    /// savestate was made, then this replaces the RNG seed.
    #[serde(default)]
    pub boundary_seed: Option<u32>,
    /// The seed the boundary's match was created from: written over the RNG
    /// seed at 0x8016E74C's point, before fn_8016E730 creates the Ground
    /// and Players (`make_boundary.py --game-start-seed`, a Slippi replay's
    /// Game Start seed). The stage's creation draws (Final Destination's
    /// background accelerations, Fountain of Dreams' platform waits) are
    /// then the replay's. A cold scenario's `seed` must be this one after
    /// the setup draws.
    #[serde(default)]
    pub game_start_seed: Option<u32>,
    /// The recording also samples each fighter at the end of its map proc
    /// (Fighter_8006C27C, where Slippi before 3.4.0 reads Post Frame:
    /// 0x8006C5D8), and the gate compares the port's after-map record with
    /// it (`trace_after_map`). A state the same tick's later procs overwrite
    /// (a captured fighter riding a moving floor) shows only there.
    #[serde(default)]
    pub after_map: bool,
    /// A cold Sudden Death scene (gm_SetupSuddenDeath): one stock at 300%.
    #[serde(default)]
    pub sudden_death: bool,
    /// A cold match's counting-down timer, in seconds.
    #[serde(default)]
    pub time_limit: Option<u32>,
    /// How players spawn (melee_lib::slippi::SpawnRule).
    #[serde(default, deserialize_with = "spawn_rule::deserialize")]
    pub spawn: melee_lib::slippi::SpawnRule,
    /// Slippi's Stadium transformation preload code
    /// (`melee_lib::slippi::SlippiCodes::stadium_preload`).
    #[serde(default)]
    pub stadium_preload: bool,
    /// Slippi's Frozen Stadium code
    /// (`melee_lib::slippi::SlippiCodes::stadium_frozen`).
    #[serde(default)]
    pub stadium_frozen: bool,
    /// 20XX TE's Frozen Mode stage writes
    /// (`melee_lib::slippi::SlippiCodes::frozen_stages`).
    #[serde(default)]
    pub frozen_stages: bool,
    /// The Widescreen 16:9 code
    /// (`melee_lib::slippi::SlippiCodes::widescreen`).
    #[serde(default)]
    pub widescreen: bool,
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
    /// Gecko codes the recording's Dolphin ran (`harness/gecko.py`), such as
    /// `["ucf-0.8"]`. Every fighter then names the matching
    /// `controller_fix`, and `"ps-preload"` and `"ps-frozen"` go with
    /// `stadium_preload` and `stadium_frozen`, `"frozen-stages"` with
    /// `frozen_stages`, `"widescreen"` with `widescreen`; the port reads
    /// only those.
    #[serde(default)]
    pub gecko: Vec<String>,
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
    /// The port's controller-fix Gecko code (`"ucf-0.74"`, `"ucf-0.8"`,
    /// ...; `ControllerFix::ALL`). Absent: retail.
    #[serde(default)]
    pub controller_fix: Option<String>,
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
        if let Some(twin) = &scenario.gate {
            ensure!(
                scenario.savestate.is_some() && *twin != scenario.name,
                "`gate` names a recording's cold twin"
            );
            return Self::load(&path.with_file_name(format!("{twin}.toml")));
        }
        let checkout = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .context("scenario must be under harness/scenarios")?
            .to_path_buf();
        // MELEE_DATA_ROOT: a checkout whose harness data to read (a worktree
        // gates its own scenario file against the main checkout's
        // recordings), as harness/data_root.py.
        scenario.root = std::env::var_os("MELEE_DATA_ROOT").map_or(checkout, PathBuf::from);
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
                        | "PokemonStadium"
                ),
                "cold setup supports FD, Battlefield, Yoshi's Story, Dream Land, Fountain of \
                 Dreams and Pokemon Stadium"
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
            self.boundary_seed.is_none() || self.is_cold(),
            "a recording with a replaced boundary seed gates through its cold twin"
        );
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
        let stage_codes = [
            (STADIUM_PRELOAD_GECKO, self.stadium_preload),
            (STADIUM_FROZEN_GECKO, self.stadium_frozen),
            (FROZEN_STAGES_GECKO, self.frozen_stages),
            (WIDESCREEN_GECKO, self.widescreen),
        ];
        // Codes that are not controller fixes (`neutral-spawn`) have no
        // per-fighter setting.
        let fixes: Vec<&String> = self
            .gecko
            .iter()
            .filter(|code| melee_lib::ControllerFix::from_name(code).is_some())
            .collect();
        for fighter in &self.fighters {
            let fix = fighter.controller_fix()?;
            if !fixes.is_empty() {
                ensure!(
                    fixes.iter().any(|code| *code == fix.name()),
                    "gecko {:?} requires each fighter's controller_fix to name its code",
                    self.gecko
                );
            }
        }
        for (name, enabled) in stage_codes {
            ensure!(
                self.gecko.is_empty() || self.gecko.iter().any(|code| code == name) == enabled,
                "gecko {name:?} and its scenario flag go together"
            );
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
    /// Each port's controller fix (Off for an empty port).
    pub fn controller_fixes(&self) -> Result<[melee_lib::ControllerFix; 4]> {
        use melee_lib::diagnostics::ScenarioSource;
        self.setup()?.controller_fixes()
    }
    /// Whether the Dolphin-side schedule ever leaves neutral.
    pub fn is_scripted(&self) -> bool {
        self.inputs.iter().any(|step| !step.buttons.is_empty())
    }
    pub fn is_cold(&self) -> bool {
        self.savestate.is_none()
    }
    /// `spawn` as scenario TOML spells it.
    pub fn spawn_name(&self) -> &'static str {
        spawn_rule::name(self.spawn)
    }
    /// A cold twin of a scripted retail recording (`expected` names it and a
    /// fighter is `scripted`): built from parameters, driven by the pads
    /// that recording consumed. This is how a match on ports a savestate
    /// import does not cover (slots 1 and 3) gates against retail.
    pub fn replays_recorded_pads(&self) -> bool {
        self.is_cold()
            && self.expected.is_some()
            && self.replay_inputs.is_empty()
            && self.fighters.iter().any(|f| f.controller == "scripted")
    }
    pub fn expected_path(&self) -> PathBuf {
        self.root.join("harness/traces").join(format!(
            "{}.tick.expected.jsonl",
            self.expected.as_deref().unwrap_or(&self.name)
        ))
    }
    /// Traces recorded for this scenario, or for the recording a cold twin
    /// names as `expected`.
    pub fn trace_path(&self, suffix: &str) -> PathBuf {
        self.root.join("harness/traces").join(format!(
            "{}.{suffix}",
            self.expected.as_deref().unwrap_or(&self.name)
        ))
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
    /// The port's controller fix; an unknown name is an error.
    pub fn controller_fix(&self) -> Result<melee_lib::ControllerFix> {
        match &self.controller_fix {
            None => Ok(melee_lib::ControllerFix::Off),
            Some(name) => melee_lib::ControllerFix::from_name(name)
                .with_context(|| format!("unknown controller_fix {name:?}")),
        }
    }
    /// Composition root selects a character crate; gameplay uses its callbacks.
    pub fn descriptor(&self) -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        melee_lib::diagnostics::character_descriptor(&self.kind)
            .unwrap_or_else(|| unreachable!("validated scenario kind {}", self.kind))
    }
}

impl Scenario {
    /// The match parameters of a validated scenario.
    fn unchecked_setup(&self) -> melee_lib::diagnostics::Setup {
        melee_lib::diagnostics::Setup {
            fighters: std::array::from_fn(|p| {
                let f = &self.fighters[p];
                melee_lib::diagnostics::PlayerSetup {
                    slot: f.slot,
                    descriptor: f.descriptor(),
                    costume: f.costume,
                    stocks: f.stocks,
                    spawn_point: f.spawn_point,
                    controller_fix: f.controller_fix().expect("validated controller fix"),
                }
            }),
            stage: self.stage_descriptor(),
            seed: self.seed,
            all_characters_unlocked: self.all_characters_unlocked,
            time_limit: self.time_limit,
            sudden_death: self.sudden_death,
            slippi: melee_lib::slippi::SlippiCodes {
                spawn: self.spawn,
                stadium_preload: self.stadium_preload,
                stadium_frozen: self.stadium_frozen,
                frozen_stages: self.frozen_stages,
                widescreen: self.widescreen,
            },
        }
    }
}
impl melee_lib::diagnostics::ScenarioSource for Scenario {
    fn setup(&self) -> anyhow::Result<melee_lib::diagnostics::Setup> {
        self.validate()?;
        let setup = self.unchecked_setup();
        // A boundary made from a replay's Game Start seed
        // (`make_boundary.py --game-start-seed`): the port's setup draws
        // must take that seed to the one the savestate holds.
        if let (true, Some(start), Some(seed)) = (self.is_cold(), self.game_start_seed, self.seed) {
            let reached = melee_lib::diagnostics::boundary_seed_from_creation(&setup, start)?;
            ensure!(
                reached == seed,
                "game_start_seed {start} reaches boundary seed {reached} through the setup \
                 draws, not {seed}"
            );
        }
        Ok(setup)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn twin_text(extra: &str) -> String {
        format!(
            r#"name = "x_cold"
frames = 10
seed = 1
stage = "Battlefield"
all_characters_unlocked = true
spawn = "neutral-2020"
{extra}
[[fighters]]
slot = 1
kind = "Marth"
costume = 1
stocks = 4
controller = "scripted"
controller_fix = "ucf-0.8"

[[fighters]]
slot = 3
kind = "Peach"
costume = 1
stocks = 4
controller = "scripted"
controller_fix = "ucf-0.8"
"#
        )
    }
    fn twin(extra: &str) -> Scenario {
        toml::from_str(&twin_text(extra)).unwrap()
    }

    #[test]
    fn a_scripted_cold_twin_replays_the_pads_of_the_recording_it_names() {
        let scenario = twin("expected = \"x\"\nboundary_seed = 7");
        scenario.validate().unwrap();
        assert!(scenario.replays_recorded_pads());
        assert_eq!(scenario.boundary_seed, Some(7));
        assert!(scenario
            .trace_path("ledger.raw.jsonl")
            .ends_with("harness/traces/x.ledger.raw.jsonl"));
        // Without a recording to name, a cold scene's pads stay neutral.
        assert!(!twin("").replays_recorded_pads());
    }

    #[test]
    fn a_replaced_boundary_seed_needs_the_cold_twin() {
        let mut scenario = twin("boundary_seed = 7");
        scenario.savestate = Some("harness/roms/x.sav".into());
        let error = scenario.validate().unwrap_err().to_string();
        assert!(error.contains("cold twin"), "{error}");
    }

    #[test]
    fn a_game_start_seed_must_reach_the_boundary_seed_through_the_setup_draws() {
        use melee_lib::diagnostics::ScenarioSource;
        // MARTH/12_07_47 Marth + Marth (FD): Game Start's seed, and the seed
        // retail held after creating the match from it (eight draws).
        let mut scenario = twin("game_start_seed = 2534789673");
        scenario.stage = "FinalDestination".into();
        scenario.seed = Some(2_462_485_393);
        scenario.setup().unwrap();
        scenario.game_start_seed = Some(2_534_789_674);
        let error = scenario.setup().err().expect("another seed").to_string();
        assert!(error.contains("through the setup draws"), "{error}");
        // A recording names the seed without building anything from it.
        scenario.savestate = Some("harness/roms/x.sav".into());
        for (slot, fighter) in scenario.fighters.iter_mut().enumerate() {
            fighter.slot = slot as u8;
        }
        scenario.setup().unwrap();
    }

    #[test]
    fn a_code_that_is_no_controller_fix_asks_nothing_of_the_fighters() {
        let mut scenario = twin("");
        for fighter in &mut scenario.fighters {
            fighter.controller_fix = None;
        }
        scenario.gecko = vec!["neutral-spawn".into()];
        scenario.validate().unwrap();
        scenario.gecko.push("ucf-0.8".into());
        assert!(scenario.validate().is_err());
    }

    #[test]
    fn load_follows_gate_to_the_cold_twin() {
        let root = std::env::temp_dir().join(format!("melee-scenario-gate-{}", std::process::id()));
        let scenarios = root.join("harness/scenarios");
        fs::create_dir_all(&scenarios).unwrap();
        fs::write(
            scenarios.join("x.toml"),
            "name = \"x\"\ngate = \"x_cold\"\nsavestate = \"harness/roms/x.sav\"\nframes = 10\n\
             stage = \"Battlefield\"\nfighters = []\n",
        )
        .unwrap();
        fs::write(scenarios.join("x_cold.toml"), twin_text("expected = \"x\"")).unwrap();
        let loaded = Scenario::load(&scenarios.join("x.toml")).unwrap();
        assert_eq!(loaded.name, "x_cold");
        assert!(loaded.is_cold());
        fs::remove_dir_all(&root).unwrap();
    }
}

/// The `gecko` name of Slippi's Stadium transformation preload code.
const STADIUM_PRELOAD_GECKO: &str = "ps-preload";
/// The `gecko` name of Slippi's Frozen Stadium code.
const STADIUM_FROZEN_GECKO: &str = "ps-frozen";
/// The `gecko` name of 20XX TE's Frozen Mode stage writes.
const FROZEN_STAGES_GECKO: &str = "frozen-stages";
/// The `gecko` name of the Widescreen 16:9 code.
const WIDESCREEN_GECKO: &str = "widescreen";

/// `spawn = "retail" | "neutral-2019" | "neutral-2019-entry" | "neutral-2020"`
/// in scenario TOML.
mod spawn_rule {
    use melee_lib::slippi::{NeutralTable, SpawnRule};
    use serde::{Deserialize, Deserializer};
    const NAMES: [(&str, SpawnRule); 4] = [
        ("retail", SpawnRule::Retail),
        ("neutral-2019", SpawnRule::NeutralTable(NeutralTable::V2019)),
        (
            "neutral-2019-entry",
            SpawnRule::NeutralTable(NeutralTable::V2019EntryByOrder),
        ),
        ("neutral-2020", SpawnRule::NeutralTable(NeutralTable::V2020)),
    ];
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SpawnRule, D::Error> {
        let name = String::deserialize(d)?;
        NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, rule)| *rule)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown spawn rule {name}")))
    }
    /// The rule's scenario spelling (`make_boundary.py --spawn`).
    pub fn name(rule: SpawnRule) -> &'static str {
        let (name, _) = NAMES
            .iter()
            .find(|(_, r)| *r == rule)
            .expect("every rule is named");
        name
    }
}
