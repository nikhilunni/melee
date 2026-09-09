//! Restore the owned, local savestate boundary. See ../M3.md.
mod collision;
mod fighter;
pub(crate) use fighter::import as import_fighter;
pub(crate) use melee_mp::CollMap;
pub(crate) use saved_pose::SavedPose;
pub(crate) mod particles;
mod saved_pose;
pub(crate) mod stage;
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

use crate::scene_fighter::SceneFighter;
use crate::{assets::Assets, scenario::Scenario};
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use hsd_types::Mtx;
use melee_diff::{first_divergence, Record, RecordSink};
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
    pub(crate) fighters: [SceneFighter; 2],
    pub(crate) map: melee_mp::CollMap,
    pub(crate) stage: crate::scene_stage::SceneStage,
    pub(crate) particles: ParticleSystem,
    pub(crate) stage_animations:
        std::collections::BTreeMap<u8, melee_gr::last::animation::BackgroundAnimation>,
    pub(crate) effects: crate::effects::Effects,
    pub(crate) rng: HsdRng,
    /// Match setup still owes Stage_80225074 before the first observation.
    pub(crate) pending_music: Option<(melee_gr::music::MusicParameters, bool)>,
    pub(crate) selected_music: Option<i32>,
    /// First unfinished phase: 0 between idle ticks, 14 inside the older idle
    /// capture, or 24 before the first match-start scheduler pass.
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
        let assets = Assets::load(
            &scenario.assets_path(),
            std::array::from_fn(|p| scenario.fighters[p].descriptor()),
            scenario.stage_descriptor(),
        )?;
        let mut map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
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
                    && usize::from(raw[0x619]) < assets.characters[slot].descriptor.costumes.len()
                    && word(raw, 4) == i32::from(assets.characters[slot].descriptor.kind) as u32
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
        let saved_link = word(saved.bytes(0x804D_7834, 4), 0);
        let resume_s_link = if current_proc == 0 && saved_link == 24 {
            // Entry is observed before this match's scheduler starts. An idle
            // savestate between ticks instead needs one complete scheduler pass.
            if match_start {
                24
            } else {
                0
            }
        } else {
            ensure!(
                current_proc != 0 && !match_start,
                "unsupported scheduler boundary"
            );
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
        // The raw tick-zero row locates MEM1; it may be one full tick after
        // the saved idle boundary. Runtime imports always use saved memory.
        let saved_bytes = rows
            .iter()
            .map(|row| -> Result<Vec<u8>> {
                let address = u32::from_str_radix(
                    row["base"]
                        .as_str()
                        .context("fighter address")?
                        .trim_start_matches("0x"),
                    16,
                )?;
                Ok(saved.bytes(address, 0x23EC).to_vec())
            })
            .collect::<Result<Vec<_>>>()?;
        let bytes = saved_bytes;
        let fighters = std::array::from_fn(|p| {
            SceneFighter::from_saved(
                &assets.characters[p],
                &assets.fighters[p],
                &map,
                &bytes[p],
                &saved,
            )
        });
        let mut sink = RecordSink::new(0, "frame_end");
        for (p, fighter) in fighters.iter().enumerate() {
            fighter.snapshot(&mut PrefixSink::new(&mut sink, &format!("p{p}")));
        }
        let mut fighter_expected = expected;
        fighter_expected.state.remove("rng.seed");
        if resume_s_link != 0 {
            if let Some(diff) = first_divergence([&fighter_expected], [&sink.finish()]) {
                anyhow::bail!("imported fighter boundary: {diff}");
            }
        }
        // A full first tick is compared by the ordinary gate, including tick zero.
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
        let mut particles = particles::restore(
            &initial,
            &std::collections::BTreeMap::from([
                (0, assets.common_particle_bank.clone()),
                (30, assets.particle_bank.clone()),
            ]),
        );
        let meta = &metadata["particles"];
        let captured_generators = meta["generators"]
            .as_array()
            .context("generator metadata")?;
        ensure!(
            particles.generators.len() == captured_generators.len(),
            "generator metadata count"
        );
        for (runtime_generator, captured) in
            particles.generators.iter_mut().zip(captured_generators)
        {
            let generator = &captured["fields"];
            ensure!(
                generator["callback"] == 0 && generator["user_functions"] == 0,
                "particle callbacks unsupported"
            );
            if generator["jobj"] == 0 {
                continue;
            }
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
            runtime_generator.joint_matrix = Some(Mtx(std::array::from_fn(|r| {
                std::array::from_fn(|c| f32::from_bits(words[r * 4 + c].as_u64().unwrap() as u32))
            })));
        }
        let (stage, mut stage_animations) = stage::restore_scene(
            &saved,
            &assets,
            match_start,
            scenario.frames,
            &mut particles,
            &metadata,
        )?;
        if matches!(stage, crate::scene_stage::SceneStage::Story(_)) {
            for (&id, animation) in &mut stage_animations {
                let bindings = &assets.stage_desc.models[id as usize].joint_mappings;
                animation.update_collision(&mut map, bindings);
                for binding in bindings {
                    map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
                }
            }
        }
        let pending_music = if match_start {
            // gmMainLib_GetUnlockedCharactersBitmaskPtr (8015ED8C): lwz the
            // global, add 0x1868. gm/types.h's +1898 comment is stale.
            let main = word(saved.bytes(0x804D_3EE0, 4), 0);
            let unlocks = u16::from_be_bytes(saved.bytes(main + 0x1868, 2).try_into().unwrap());
            const ALL_UNLOCKABLE_CHARACTERS: u16 = (1 << 11) - 1;
            Some((
                melee_gr::desc::read_music(&assets.stage, assets.stage_descriptor.music_id)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                unlocks & ALL_UNLOCKABLE_CHARACTERS == ALL_UNLOCKABLE_CHARACTERS,
            ))
        } else {
            None
        };
        Ok(Self {
            pending_music,
            selected_music: None,
            assets,
            fighters,
            map,
            stage,
            particles,
            rng: HsdRng::new(seed),
            resume_s_link,
            stage_animations,
            effects: crate::effects::Effects::default(),
        })
    }
}
