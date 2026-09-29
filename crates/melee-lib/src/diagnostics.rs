//! Unstable oracle/import adapter. Not part of the supported application API.
//! Scenario/replay parsing and trace output belong to the consuming tool.
pub use crate::assets::Assets;
pub use crate::frame::rendered_pose::RenderedPose;
pub use crate::frame::{LocalSrt, Simulation};
pub use crate::initial_state::InitialState;
pub use crate::scene_stage::{
    descriptor as stage_descriptor, StageDescriptor, BATTLEFIELD, DREAM_LAND, FINAL_DESTINATION,
    FOUNTAIN_OF_DREAMS, YOSHIS_STORY,
};
pub use crate::setup::{PlayerSetup, Setup};
use std::path::{Path, PathBuf};
pub fn character_descriptor(
    name: &str,
) -> Option<&'static melee_ft::fighter::assets::CharacterDescriptor> {
    crate::scene_fighter::SceneFighter::descriptor_for(name)
}
/// The cold pre-music boundary seed from the seed before fn_8016E730's
/// Ground and Player creation (Slippi's Game Start seed, injected at
/// 0x8016E74C): that seed advanced by the setup's fixed draws.
pub fn boundary_seed_from_creation(setup: &Setup, seed: u32) -> anyhow::Result<u32> {
    crate::initial_state::boundary_seed_after(seed, setup.stage.kind, setup.roster().len())
}
pub const CHARACTERS: &[&str] = crate::scene_fighter::SceneFighter::NAMES;
/// Content fingerprint of the source bytes actually loaded, including articles.
/// Stable for the same Rust toolchain, loader and assets, independent of directory.
/// Non-cryptographic: detects accidental data drift, not malicious substitution.
pub fn asset_fingerprint(assets: &crate::GameAssets) -> u64 {
    assets.inner.fingerprint
}
/// Import a captured oracle boundary. Unlike Match::new, the first step may
/// complete a partial tick. Tick numbering intentionally matches the fixture.
pub fn import_match(source: &dyn ScenarioSource) -> anyhow::Result<crate::Match> {
    let setup = source.setup()?;
    let state = if source.is_cold() {
        InitialState::from_parameters(source)?
    } else {
        InitialState::from_savestate_traces(source)?
    };
    Ok(crate::Match::from_import(Simulation::new(state), &setup))
}

/// Allocating diagnostic output; never constructed by Match::step or observe.
#[derive(Debug, PartialEq)]
pub struct Inspection {
    pub fighters: melee_diff::Record,
    pub items: melee_diff::Record,
    pub particles: melee_diff::Record,
    pub particle_rng_sites: Vec<u32>,
    pub effect_rng_sites: Vec<u32>,
    pub rng_writers: Vec<(String, u32)>,
    pub bones: [Vec<[u32; 12]>; 2],
}
pub fn inspect(game: &crate::Match) -> Result<Inspection, crate::StateError> {
    game.observe()?;
    let engine = &game.engine;
    let state = engine.state();
    let frame = engine.frame().saturating_sub(1);
    let mut banks = std::collections::BTreeMap::from([
        (0, state.assets.common_particle_bank.clone()),
        (30, state.assets.particle_bank.clone()),
    ]);
    for id in 0..65 {
        if let Some(bank) = state.particles.bank(id) {
            banks.insert(id, bank.clone());
        }
    }
    Ok(Inspection {
        fighters: snapshot(state, frame),
        items: item_snapshot(&state.items, frame),
        particles: crate::initial_state::particles::snapshot(
            &state.particles,
            state.rng.seed,
            frame,
            &banks,
        ),
        particle_rng_sites: engine.particle_rng_sites(),
        effect_rng_sites: engine.effect_rng_sites(),
        rng_writers: engine.rng_writers(),
        bones: std::array::from_fn(|p| engine.rendered_fighter_pose(p).matrices),
    })
}
pub trait ScenarioSource {
    fn setup(&self) -> anyhow::Result<Setup>;
    fn is_cold(&self) -> bool;
    fn frames(&self) -> u64;
    fn assets_path(&self) -> PathBuf;
    fn trace_path(&self, suffix: &str) -> PathBuf;
    fn boundary_path(&self, suffix: &str) -> PathBuf;
    fn savestate_path(&self) -> PathBuf;
    /// Read a captured trace named by its plain `.jsonl` path. The consuming
    /// tool owns trace storage (plain or zstd-compressed), so melee-lib never
    /// links a decompressor.
    fn open_trace(&self, path: &Path) -> anyhow::Result<Box<dyn std::io::BufRead>>;
}

pub use crate::initial_state::{decode_camera, decode_subject, magnified};
mod snapshot;
pub use snapshot::snapshot;
mod items;
pub use items::actual as item_snapshot;

#[cfg(test)]
impl ScenarioSource for melee_sim::scenario::Scenario {
    fn setup(&self) -> anyhow::Result<Setup> {
        self.validate()?;
        Ok(Setup {
            fighters: std::array::from_fn(|p| {
                let f = &self.fighters[p];
                PlayerSetup {
                    slot: f.slot,
                    descriptor: f.descriptor(),
                    costume: f.costume,
                    stocks: f.stocks,
                    spawn_point: f.spawn_point,
                    controller_fix: f.controller_fix().expect("validated controller fix"),
                }
            }),
            stage: stage_descriptor(&self.stage).unwrap(),
            seed: self.seed,
            all_characters_unlocked: self.all_characters_unlocked,
            time_limit: self.time_limit,
            sudden_death: self.sudden_death,
            slippi: Default::default(),
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
        Ok(Box::new(melee_test_support::trace::open(path)?))
    }
}
