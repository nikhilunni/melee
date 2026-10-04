//! Restore the owned, local savestate boundary. See ../M3.md.
mod camera;
pub use camera::{decode_camera, decode_subject, magnified};
mod cold;
pub(crate) use cold::boundary_seed_after;
#[cfg(test)]
mod cold_tests;
mod collision;
mod cpu;
mod fighter;
mod particle_resume;
pub(crate) use fighter::import as import_fighter;
pub(crate) use melee_mp::CollMap;
pub(crate) use saved_pose::SavedPose;
pub(crate) mod particles;
mod saved_pose;
mod scene_flow;
pub(crate) mod scheduler_resume;
mod setup_resume;
pub(crate) mod stage;
mod stage_izumi;
mod stock;
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

use crate::assets::Assets;
use crate::scene_fighter::SceneFighter;
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use hsd_types::Mtx;
use melee_diff::{first_divergence, Record, RecordSink};
#[cfg(test)]
use melee_sim::scenario::Scenario;
use melee_types::snapshot::{PrefixSink, Snapshot};
use serde_json::Value as Json;
use std::{fs::File, io::BufRead, path::Path};

/// The sole trace-to-runtime boundary. No expected records or ledger draws are
/// retained by Simulation. Imports vs archive-derived state are listed in M3.md.
#[derive(Clone)]
pub struct InitialState {
    pub(crate) items: Box<melee_it::ItemPool>,
    /// Posed articles whose bones effects follow (crate::article_pose).
    pub(crate) article_poses: Box<crate::article_pose::ArticlePoses>,
    pub(crate) stock_displays: [Option<melee_if::StockDisplay>; 2],
    pub(crate) spawn_counter: melee_ft::fighter::SpawnCounter,
    pub(crate) assets: std::sync::Arc<Assets>,
    /// Every fighter GObj in fighter-list order (`Setup::roster`); a
    /// player's partner (Nana) follows its main fighter.
    pub(crate) fighters: Vec<SceneFighter>,
    pub(crate) map: melee_mp::CollMap,
    pub(crate) stage: crate::scene_stage::SceneStage,
    pub(crate) particles: ParticleSystem,
    pub(crate) pending_emission: Option<particle_resume::PendingEmission>,
    pub(crate) stage_animations:
        std::collections::BTreeMap<u8, melee_gr::last::animation::BackgroundAnimation>,
    // Unique storage allocated at setup keeps cold-start state moves off the stack.
    pub(crate) effects: Box<melee_ef::Effects>,
    pub(crate) rng: HsdRng,
    /// Match setup still owes Stage_80225074 before the first observation.
    pub(crate) pending_music: Option<(melee_gr::music::MusicParameters, bool)>,
    pub(crate) selected_music: Option<i32>,
    /// The HUD banner running now (countdown or GO).
    pub(crate) banner: Option<crate::banner::Banner>,
    /// GO, loaded up front and started when the countdown ends.
    pub(crate) go_banner: Option<crate::banner::Banner>,
    pub(crate) clock: crate::match_clock::MatchClock,
    pub(crate) bomb_rain: melee_gr::bomb_rain::BombRain,
    /// FighterMatchInfo[].x8, fn_80167638 / fn_8016758C.
    pub(crate) revival_offsets: melee_ft::fighter::life::RevivalOffsets,
    pub(crate) resume: scheduler_resume::SchedulerResume,
    /// game_camera and the screen-shake models it drives.
    pub(crate) camera: melee_cm::GameCamera,
    /// The main CObj as the last display pass left it (fn_800301D0 ->
    /// Camera_8002A4AC). Retail skips passes, so gameplay reads of the CObj
    /// (grStadium_801D32D0) can see the camera of an earlier tick.
    pub(crate) rendered_camera: hsd_anim::cobj::PerspectiveCamera,
    pub(crate) quakes: crate::quake::Quakes,
    /// Each port's controller-fix Gecko code (match setup, not saved state).
    pub(crate) controller_fixes: [melee_ft::input::ControllerFix; 4],
    /// UCF 0.84's per-port pad buffer (the Gecko code's own data), zero when
    /// the code is installed.
    pub(crate) pad_buffers: [melee_ft::input::controller_fix::PadBuffer; 4],
    /// Dween's per-port slot (the Gecko code's own data): the pad's stick x
    /// on the last tick the code ran for the port, zero when installed.
    pub(crate) dween_previous_x: [f32; 4],
    /// StaticPlayer.kos_by_player: no KOs at a match's start.
    pub(crate) ko_counts: crate::ko_counts::KoCounts,
}
/// ftCo_MS_Sleep: a transformation partner's motion at a boundary.
const SLEEP_MOTION: u32 = melee_types::CommonMotionState::Sleep as u32;
impl InitialState {
    /// The fighter-list index of player `player`'s own fighter (x221F_b4
    /// clear), `player` counting players in port order.
    pub(crate) fn player_fighter_index(&self, player: usize) -> usize {
        crate::scene_fighter::player_fighter_index(&self.fighters, player)
    }
    /// Player_GetEntity: player `player`'s own fighter.
    pub(crate) fn player_fighter(&self, player: usize) -> &SceneFighter {
        &self.fighters[self.player_fighter_index(player)]
    }
}
/// The first record of a captured trace, plain or compressed as the source reads it.
fn first_json(scenario: &dyn crate::diagnostics::ScenarioSource, path: &Path) -> Result<Json> {
    let line = scenario
        .open_trace(path)
        .with_context(|| format!("opening {}", path.display()))?
        .lines()
        .next()
        .context("empty boundary trace")??;
    Ok(serde_json::from_str(&line)?)
}
impl InitialState {
    pub fn from_savestate_traces(
        scenario: &dyn crate::diagnostics::ScenarioSource,
    ) -> Result<Self> {
        let setup = scenario.setup()?;
        ensure!(
            !scenario.is_cold(),
            "saved construction requires a savestate"
        );
        let assets = std::sync::Arc::new(Assets::load(
            &scenario.assets_path(),
            &setup.roster_descriptors(),
            setup.stage_descriptor(),
        )?);
        let mut map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        if assets.stage_desc.kind == melee_types::GrKind::PStadium {
            // grStadium_OnInit's collision edits precede fighter creation;
            // restore_scene validates that the boundary is in the base form.
            let mut models = crate::scene_stage::stadium::load_models(&assets)?;
            crate::scene_stage::stadium::initialize_collision(&mut map, &mut models);
        }
        // Only the first line is read. Later rows (including all rng_draws)
        // never enter simulation, even transiently.
        let boundary = first_json(scenario, &scenario.trace_path("tick.raw.jsonl"))?;
        let expected: Record = serde_json::from_value(first_json(
            scenario,
            &scenario.trace_path("tick.expected.jsonl"),
        )?)?;
        ensure!(
            expected.frame == 0 && expected.phase == "frame_end",
            "expected frame-zero boundary"
        );
        let rows = boundary["fighters"]
            .as_array()
            .context("missing fighters")?;
        let roster = setup.roster();
        ensure!(
            rows.len() == roster.len(),
            "requires {} fighters, found {}",
            roster.len(),
            rows.len()
        );
        // The player slot (port) each fighter-list entry belongs to.
        let ports: Vec<usize> = roster
            .iter()
            .map(|entry| usize::from(setup.fighters[entry.player].slot))
            .collect();
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
        for (index, raw) in bytes.iter().enumerate() {
            ensure!(
                usize::from(raw[12]) == ports[index]
                    && usize::from(raw[0x619]) < assets.characters[index].descriptor.costumes.len()
                    && word(raw, 4) == i32::from(assets.characters[index].descriptor.kind) as u32
                    // Wait, Entry, or a transformation partner's Sleep.
                    && matches!(word(raw, 0x10), SLEEP_MOTION | 14 | 322),
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
            bytes
                .iter()
                .filter(|raw| word(raw, 0x10) != SLEEP_MOTION)
                .all(|raw| word(raw, 0x10) == word(&bytes[0], 0x10)),
            "fighters must share the imported Wait/Entry boundary"
        );
        let resume = scheduler_resume::SchedulerResume::restore(
            &saved,
            match_start,
            assets.stage_desc.kind,
        )?;
        let partial_emission = resume
            .current
            .is_some_and(|(_, action)| action == scheduler_resume::Continuation::ParticleEmission);
        // The raw tick-zero row locates MEM1; it may be one full tick after
        // the saved idle boundary. Runtime imports always use saved memory.
        let bases = rows
            .iter()
            .map(|row| -> Result<u32> {
                Ok(u32::from_str_radix(
                    row["base"]
                        .as_str()
                        .context("fighter address")?
                        .trim_start_matches("0x"),
                    16,
                )?)
            })
            .collect::<Result<Vec<_>>>()?;
        let saved_bytes = bases
            .iter()
            .map(|&address| saved.bytes(address, 0x23EC).to_vec())
            .collect::<Vec<_>>();
        let bytes = saved_bytes;
        for (index, raw) in bytes.iter().enumerate() {
            let unfinished = match_start && word(raw, 0x10) == 0 && word(raw, 0x2C) == 0;
            ensure!(
                unfinished
                    || (usize::from(raw[12]) == ports[index]
                        && word(raw, 4)
                            == i32::from(assets.characters[index].descriptor.kind) as u32
                        && matches!(word(raw, 0x10), SLEEP_MOTION | 14 | 322)),
                "unsupported saved fighter boundary"
            );
        }
        if assets.stage_desc.kind == melee_types::GrKind::Izumi {
            stage_izumi::restore_collision(&saved, &mut map)?;
        }
        let mut rng = HsdRng::new(word(saved.bytes(0x804D_5F90, 4), 0));
        let mut fighters: Vec<SceneFighter> = (0..roster.len())
            .map(|p| {
                if match_start && word(&bytes[p], 0x10) == 0 && word(&bytes[p], 0x2C) == 0 {
                    ensure!(
                        !roster[p].secondary && ports[p] == p,
                        "an unfinished partner or port-shifted fighter creation is not imported"
                    );
                    setup_resume::fighter(&saved, &assets, p, &mut map, &mut rng)
                } else {
                    Ok(SceneFighter::from_saved(
                        &assets.characters[p],
                        &assets.fighters[p],
                        &map,
                        &bytes[p],
                        &saved,
                    ))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        for (index, fighter) in fighters.iter_mut().enumerate() {
            let unfinished =
                match_start && word(&bytes[index], 0x10) == 0 && word(&bytes[index], 0x2C) == 0;
            // Only a CPU-driven fighter reads the rest of CpuFighter; a
            // human's keeps stale words (Peach's savestates hold an old x4C).
            let cpu_driven = fighter.0.input_source() == melee_ft::input::human::InputSource::Cpu;
            if !unfinished && cpu_driven {
                fighter.0.cpu = cpu::restore(&bytes[index], bases[index], &bases);
            }
        }
        for fighter in &mut fighters {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                // Player_GetHandicap, StaticPlayer +4B, stride E90.
                f.grab_handicap =
                    saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0x4B, 1)[0];
                // Player_GetStocks, StaticPlayer stride 0xE90.
                f.player.stocks =
                    saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0x8E, 1)[0];
                // Player_GetFallsByIndex: falls[x221F_b4] at +68.
                let falls = 0x8045_3080
                    + u32::from(f.player.id) * 0xE90
                    + 0x68
                    + 4 * u32::from(f.player.secondary);
                f.player.falls = word(saved.bytes(falls, 4), 0);
            });
        }
        // The camera and its subjects: the list runs newest first, and each
        // fighter linked its subject at creation (fighter.c:893). A fighter the
        // setup resume creates keeps the subject its spawn reset.
        let (clock, bomb_rain) = scene_flow::restore_clock(&saved)?;
        let (camera, subjects) = camera::restore(&saved)?;
        ensure!(
            subjects.len() <= fighters.len(),
            "unsupported camera subjects: {} for {} fighters",
            subjects.len(),
            fighters.len()
        );
        for (fighter, subject) in fighters.iter_mut().zip(subjects.into_iter().rev()) {
            crate::scene_fighter::with_fighter!(fighter, |f| f.camera = subject);
        }
        for (slot, fighter) in fighters.iter_mut().enumerate() {
            let raw = &bytes[slot];
            crate::scene_fighter::with_fighter!(fighter, |f| {
                // x221F bit 0 (MSB-first) and dmg.x1910.
                f.offscreen.outside_camera = raw[0x221F] & 0x80 != 0;
                f.offscreen.magnified_ticks = word(raw, 0x1910) as i32;
                f.offscreen.magnified = camera::restore_magnified(&saved, ports[slot]);
            });
        }
        let quakes =
            crate::quake::Quakes::load(&assets.stage, assets.stage_desc.quake_model.as_ref())?;
        ensure!(
            camera.quake.frames_left.iter().all(|&f| f == 0),
            "a quake playing at the savestate boundary is not imported"
        );
        let mut sink = RecordSink::new(0, "frame_end");
        for (p, fighter) in fighters.iter().enumerate() {
            fighter.snapshot(&mut PrefixSink::new(&mut sink, &format!("p{p}")));
        }
        let mut fighter_expected = expected;
        fighter_expected.state.remove("rng.seed");
        // Earlier cursors can still owe animation and collision. Their end-of-tick
        // comparison belongs to the ordinary frame-zero gate after continuation;
        // comparing saved partial state to a completed tick would be invalid.
        if resume.s_link >= 14 {
            if let Some(diff) = first_divergence([&fighter_expected], [&sink.finish()]) {
                anyhow::bail!("imported fighter boundary: {diff}");
            }
        }
        // A full first tick is compared by the ordinary gate, including tick zero.
        // The particle population belongs to the savestate, so scripted
        // scenarios recorded from the same savestate share this capture.
        let initial: Record = serde_json::from_value(first_json(
            scenario,
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
        let pending_emission = partial_emission
            .then(|| particle_resume::PendingEmission::restore(&saved, &particles, &metadata))
            .transpose()?;
        let (mut stage, mut stage_animations) = stage::restore_scene(
            &saved,
            &assets,
            match_start,
            &mut particles,
            &metadata,
            &setup.slippi,
        )?;
        stage.set_frozen_stages(setup.slippi.frozen_stages);
        crate::frame::validate_saved_resume(&resume, &stage, fighters.len())?;
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
        // ifStock_804A1378: import only the saved HUD boundary, never later trace rows.
        let stock_displays = if match_start {
            stock::create(
                &assets.interface,
                std::array::from_fn(|slot| {
                    let own = crate::scene_fighter::player_fighter_index(&fighters, slot);
                    crate::scene_fighter::with_fighter!(&fighters[own], |f| f.player.stocks)
                }),
            )?
        } else {
            std::array::from_fn(|slot| {
                let player = saved.bytes(0x804A_1378 + 8 + slot as u32 * 0x50, 0x50);
                let state = saved.bytes(0x804A_1378 + 0x204 + slot as u32 * 0x54, 12);
                let root = vector(saved.bytes(word(player, 4) + 0x38, 12), 0);
                let icon_positions = std::array::from_fn(|i| {
                    let local = vector(saved.bytes(word(player, 8 + i * 4) + 0x38, 12), 0);
                    Vec3::new(local.x + root.x, local.y + root.y, local.z + root.z)
                });
                Some(melee_if::StockDisplay {
                    icon_positions,
                    animation_frames: state[5..10].try_into().unwrap(),
                    animate_losses: state[2] != 0,
                })
            })
        };
        let spawn_counter = melee_ft::fighter::SpawnCounter(
            fighters
                .iter()
                .map(|f| crate::scene_fighter::with_fighter!(f, |f| f.spawn_number))
                .max()
                .unwrap()
                + 1,
        );
        // The boundary's CObj is taken as its camera's latest render.
        let rendered_camera = camera.render_camera(&assets.stage_camera);
        let effects = Box::new(melee_ef::Effects::from_resources(&assets.effect_resources));
        Ok(Self {
            items: Box::new(melee_it::ItemPool::new(assets.items.common.clone())),
            article_poses: Box::new(crate::article_pose::ArticlePoses::new(
                &assets.items.article_skeletons,
            )),
            stock_displays,
            spawn_counter,
            // A match-start savestate is taken before the first countdown tick, so the
            // Versus countdown (and its input release, ftLib_800868A4 at tick 85) starts
            // from frame 0 exactly as in a cold start. Later boundaries resume the
            // banner running in saved MEM1, if any.
            banner: if match_start {
                Some(scene_flow::match_start_countdown(
                    &clock,
                    &assets.interface,
                )?)
            } else {
                scene_flow::restore_banner(&saved, &assets.interface)?
            },
            go_banner: Some(crate::banner::Banner::preload(
                &assets.interface,
                crate::banner::BannerKind::Go,
            )?),
            clock,
            bomb_rain,
            revival_offsets: scene_flow::restore_revival_offsets(&saved),
            pending_music,
            selected_music: None,
            assets,
            fighters,
            map,
            stage,
            particles,
            pending_emission,
            rng,
            resume,
            stage_animations,
            effects,
            camera,
            rendered_camera,
            quakes,
            controller_fixes: setup.controller_fixes()?,
            pad_buffers: Default::default(),
            dween_previous_x: [0.0; 4],
            ko_counts: Default::default(),
        })
    }
}
