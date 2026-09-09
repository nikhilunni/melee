//! Restore the owned, local savestate boundary. See ../M3.md.
mod collision;
mod fighter;
pub(crate) mod particles;
mod saved_pose;
mod stage;
use hsd_types::Vec3;
fn word(raw: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap())
}
fn float(raw: &[u8], offset: usize) -> f32 {
    f32::from_bits(word(raw, offset))
}
fn vector(raw: &[u8], offset: usize) -> Vec3 {
    Vec3::new(
        float(raw, offset),
        float(raw, offset + 4),
        float(raw, offset + 8),
    )
}

use crate::{assets::Assets, scenario::Scenario};
use anyhow::{ensure, Context, Result};
use ft_fox::init::Fox;
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use hsd_types::Mtx;
use melee_diff::{first_divergence, Record, RecordSink};
use melee_ft::fighter::Fighter;
use melee_types::snapshot::{PrefixSink, Snapshot};
use serde_json::Value as Json;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

/// The sole trace-to-runtime boundary. No expected records or ledger draws are
/// retained by Simulation. Imports vs archive-derived state are listed in M3.md.
pub struct InitialState {
    pub(crate) assets: Assets,
    pub(crate) fighters: [Fighter<Fox>; 2],
    pub(crate) map: melee_mp::CollMap,
    pub(crate) stage: melee_gr::last::FinalDestination,
    pub(crate) particles: ParticleSystem,
    pub(crate) stage_animation: Option<melee_gr::last::animation::BackgroundAnimation>,
    pub(crate) effects: crate::effects::Effects,
    pub(crate) rng: HsdRng,
    /// First unfinished phase: 14 for the partial idle tick, 24 (past the
    /// scheduler) for the completed match-start boundary.
    pub(crate) resume_s_link: u8,
}
fn first_json(path: &Path) -> Result<Json> {
    let line =
        BufReader::new(File::open(path).with_context(|| format!("opening {}", path.display()))?)
            .lines()
            .next()
            .context("empty boundary trace")??;
    Ok(serde_json::from_str(&line)?)
}
impl InitialState {
    pub fn from_savestate_traces(scenario: &Scenario) -> Result<Self> {
        scenario.validate()?;
        let assets = Assets::load(&scenario.assets_path())?;
        let map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        // Only the first line is read. Later rows (including all rng_draws)
        // never enter simulation, even transiently.
        let boundary = first_json(&scenario.trace_path("tick.raw.jsonl"))?;
        let expected: Record =
            serde_json::from_value(first_json(&scenario.trace_path("tick.expected.jsonl"))?)?;
        ensure!(
            expected.frame == 0 && expected.phase == "frame_end",
            "expected frame-zero boundary"
        );
        let rows = boundary["fighters"]
            .as_array()
            .context("missing fighters")?;
        ensure!(rows.len() == 2, "requires two fighters");
        let bytes = rows
            .iter()
            .map(|row| -> Result<Vec<u8>> {
                let hex = row["bytes"].as_str().context("missing fighter bytes")?;
                ensure!(hex.len() == 0x23EC * 2, "invalid Fighter dump length");
                hex.as_bytes()
                    .chunks_exact(2)
                    .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
                    .collect::<Result<_>>()
            })
            .collect::<Result<Vec<_>>>()?;
        for (slot, raw) in bytes.iter().enumerate() {
            ensure!(
                usize::from(raw[12]) == slot
                    && raw[0x619] == slot as u8
                    && word(raw, 4) == 1
                    && matches!(word(raw, 0x10), 14 | 322),
                "unsupported fighter boundary"
            );
        }
        let address = u32::from_str_radix(
            rows[0]["base"]
                .as_str()
                .context("missing Fighter address")?
                .trim_start_matches("0x"),
            16,
        )?;
        let saved = saved_pose::SavedPose::load(&scenario.savestate_path(), &bytes[0], address);
        let match_start = word(&bytes[0], 0x10) == 322;
        ensure!(
            word(&bytes[1], 0x10) == word(&bytes[0], 0x10),
            "fighters must share the imported Wait/Entry boundary"
        );
        let current_proc = word(saved.bytes(0x804D_7838, 4), 0);
        let resume_s_link = if match_start {
            // Match setup has not entered this match's scheduler yet.
            // The first completed observation is the unchanged Entry boundary.
            ensure!(
                current_proc == 0 && word(saved.bytes(0x804D_7834, 4), 0) == 24,
                "unsupported match-start scheduler boundary"
            );
            24
        } else {
            let proc = saved.bytes(current_proc, 0x18);
            ensure!(
                word(saved.bytes(0x804D_7834, 4), 0) == 14
                    && proc[12] == 14
                    && word(proc, 0x14) == 0x8006_D1EC
                    && word(proc, 0x10) == word(&bytes[0], 0),
                "unsupported scheduler resume boundary"
            );
            proc[12]
        };
        let mut fighters = std::array::from_fn(|p| fighter::import(&assets, &map, &bytes[p]));
        for (p, fighter) in fighters.iter_mut().enumerate() {
            saved.restore(fighter, &bytes[p]);
        }
        let mut sink = RecordSink::new(0, "frame_end");
        for (p, fighter) in fighters.iter().enumerate() {
            fighter.snapshot(&mut PrefixSink::new(&mut sink, &format!("p{p}")));
        }
        let mut fighter_expected = expected;
        fighter_expected.state.remove("rng.seed");
        if let Some(diff) = first_divergence([&fighter_expected], [&sink.finish()]) {
            anyhow::bail!("imported fighter boundary: {diff}");
        }
        // The particle population belongs to the savestate, so scripted
        // scenarios recorded from the same savestate share this capture.
        let initial: Record = serde_json::from_value(first_json(
            &scenario.boundary_path("particles.jsonl.initial.jsonl"),
        )?)?;
        let metadata: Json = serde_json::from_reader(File::open(
            scenario.boundary_path("particles.jsonl.initial.jsonl.meta.json"),
        )?)?;
        ensure!(
            metadata["sampling"] == "savestate_loaded_before_first_tick",
            "unsupported particle boundary"
        );
        let seed = u32::try_from(particles::uint(&initial, "rng.seed"))?;
        let sidecar: Json = serde_json::from_reader(File::open(
            scenario.savestate_path().with_extension("sav.json"),
        )?)?;
        ensure!(
            sidecar["seed"].as_u64() == Some(u64::from(seed))
                && metadata["particles"]["seed"].as_u64() == Some(u64::from(seed))
                && word(saved.bytes(0x804D_5F90, 4), 0) == seed,
            "inconsistent saved RNG seed"
        );
        let mut particles = particles::restore(&initial, &assets.particle_bank);
        ensure!(
            particles.generators.len() == usize::from(!match_start),
            "unexpected initial FD generator population"
        );
        if !match_start {
            let meta = &metadata["particles"];
            let generator = &meta["generators"][0]["fields"];
            ensure!(
                generator["callback"] == 0 && generator["user_functions"] == 0,
                "particle callbacks unsupported"
            );
            let joint = meta["joints"]
                .as_array()
                .context("missing joint metadata")?
                .iter()
                .find(|j| j["pointer"] == generator["jobj"])
                .context("unresolved particle attachment")?;
            let words = joint["fields"]["matrix"]
                .as_array()
                .context("missing attachment matrix")?;
            ensure!(
                words.len() == 12
                    && words
                        .iter()
                        .all(|w| w.as_u64().is_some_and(|v| v <= u64::from(u32::MAX))),
                "invalid attachment matrix"
            );
            particles.generators[0].joint_matrix = Some(Mtx(std::array::from_fn(|r| {
                std::array::from_fn(|c| f32::from_bits(words[r * 4 + c].as_u64().unwrap() as u32))
            })));
        }
        let stage = stage::restore(&saved, &assets)?;
        ensure!(
            stage.ground.waiting_for_start == match_start,
            "FD start flag disagrees with fighter boundary"
        );
        let stage_animation = match_start
            .then(|| {
                melee_gr::last::animation::BackgroundAnimation::load(
                    &assets.stage,
                    &assets.stage_desc,
                )
            })
            .transpose()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        ensure!(
            stage.ground.elapsed + scenario.frames as f32 <= 1800.0,
            "FD transition exceeds implemented stationary attachment interval"
        );
        Ok(Self {
            assets,
            fighters,
            map,
            stage,
            particles,
            rng: HsdRng::new(seed),
            resume_s_link,
            stage_animation,
            effects: crate::effects::Effects::default(),
        })
    }
}
