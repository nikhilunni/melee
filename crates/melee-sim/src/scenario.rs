//! TOML scenario loading: supported characters on Final Destination, with a
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
    pub savestate: PathBuf,
    pub frames: u64,
    /// Cold-start seed; a restored match uses its saved seed instead.
    pub seed: u32,
    pub stage: String,
    pub fighters: Vec<FighterScenario>,
    /// The VI-frame schedule that drove Dolphin. The port itself replays the
    /// per-tick pads recorded beside the expected trace (`inputs.rs`); this
    /// list only says whether the scenario is scripted.
    #[serde(default)]
    pub inputs: Vec<InputStep>,
    #[serde(skip)]
    pub root: PathBuf,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FighterScenario {
    pub slot: u8,
    pub kind: String,
    pub controller: String,
}
/// One step of the Dolphin-side schedule: `buttons` holds on `port` from VI
/// frame `frame` until that port's next step. Keys are the GC pad names the
/// harness accepts (`remote_proto.GC_KEYS`); an empty table is neutral.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputStep {
    pub frame: u64,
    #[serde(default)]
    pub port: u8,
    #[serde(default)]
    pub buttons: toml::Table,
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
        scenario.validate()?;
        Ok(scenario)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.stage == "FinalDestination",
            "only FinalDestination is supported"
        );
        ensure!(
            (1..=600).contains(&self.frames),
            "imported boundary supports 1..=600 ticks"
        );
        ensure!(self.fighters.len() == 2, "requires two fighters");
        for (slot, fighter) in self.fighters.iter().enumerate() {
            ensure!(
                usize::from(fighter.slot) == slot
                    && matches!(fighter.kind.as_str(), "Fox" | "Marth")
                    && matches!(fighter.controller.as_str(), "scripted" | "idle"),
                "requires ordered human Fox/Marth slots 0/1"
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
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.name.clone())
    }
    pub fn boundary_path(&self, suffix: &str) -> PathBuf {
        self.root
            .join("harness/traces")
            .join(format!("{}.{suffix}", self.boundary_name()))
    }
    pub fn assets_path(&self) -> PathBuf {
        self.root.join("harness/roms/files")
    }
    pub fn savestate_path(&self) -> PathBuf {
        self.root.join(&self.savestate)
    }
    /// Local assets/captures whose absence lets integration tests skip.
    pub fn required_files(&self) -> Vec<PathBuf> {
        let mut paths = ["PlCo.dat", "GrNLa.dat", "EfCoData.dat"]
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
        match self.kind.as_str() {
            "Fox" => &ft_fox::init::DESCRIPTOR,
            "Marth" => &ft_mars::init::DESCRIPTOR,
            _ => unreachable!("validated scenario kind"),
        }
    }
}
