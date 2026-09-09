//! TOML scenario loading. This slice deliberately accepts only the M3 idle setup.
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
    #[serde(default)]
    pub inputs: Vec<toml::Value>,
    #[serde(skip)]
    pub root: PathBuf,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FighterScenario {
    pub slot: u8,
    pub kind: String,
    pub controller: String,
    // The current TOML places inputs=[] under its second [[fighters]] table.
    #[serde(default)]
    pub inputs: Vec<toml::Value>,
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
            self.name == "idle_fd_fox" && self.stage == "FinalDestination",
            "only idle_fd_fox on FinalDestination is supported"
        );
        ensure!(
            (1..=600).contains(&self.frames),
            "imported M3 boundary supports 1..=600 ticks"
        );
        ensure!(
            self.fighters.len() == 2 && self.inputs.is_empty(),
            "requires two idle fighters"
        );
        for (slot, fighter) in self.fighters.iter().enumerate() {
            ensure!(
                usize::from(fighter.slot) == slot
                    && fighter.kind == "Fox"
                    && matches!(fighter.controller.as_str(), "scripted" | "idle")
                    && fighter.inputs.is_empty(),
                "requires ordered human Fox slots 0/1 with neutral input"
            );
        }
        Ok(())
    }
    pub fn trace_path(&self, suffix: &str) -> PathBuf {
        self.root
            .join("harness/traces")
            .join(format!("{}.{suffix}", self.name))
    }
    pub fn assets_path(&self) -> PathBuf {
        self.root.join("harness/roms/files")
    }
    pub fn savestate_path(&self) -> PathBuf {
        self.root.join(&self.savestate)
    }
    /// Local assets/captures whose absence lets integration tests skip.
    pub fn required_files(&self) -> Vec<PathBuf> {
        let mut paths = [
            "PlFxNr.dat",
            "PlFxOr.dat",
            "PlFx.dat",
            "PlFxAJ.dat",
            "PlCo.dat",
            "GrNLa.dat",
        ]
        .map(|n| self.assets_path().join(n))
        .to_vec();
        paths.push(self.savestate_path());
        paths.extend(
            [
                "ledger600.raw.jsonl",
                "tick.expected.jsonl",
                "particles.jsonl.initial.jsonl",
                "particles.jsonl.initial.jsonl.meta.json",
            ]
            .map(|s| self.trace_path(s)),
        );
        paths
    }
}
