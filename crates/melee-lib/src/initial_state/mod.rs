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
mod heap;
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
mod stage_fourside;
mod stage_garden;
mod stage_izumi;
mod stage_oldkongo;
mod stage_flatzone;
mod stage_oldyoshi;
mod stage_shrine;
mod stage_kongo;
mod stage_kraid;
mod stage_yorster;
mod stage_inishie1;
mod stage_onett;
mod stock;
use hsd_types::Vec3;
/// The match's StartMeleeRules (lbl_8046B6A0 +24C8) a boundary was saved with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SavedRules {
    /// +8 is_teams (gm_8016B168).
    pub teams: bool,
    /// +1 bit 0, friendly_fire (gm_8016B0D4: lbz 0x24C9; clrlwi 31).
    pub friendly_fire: bool,
    /// +0 bits 7-5, match_kind (gm_8016B094: lbz 0x24C8; rlwinm 27,29,31).
    pub match_kind: melee_types::MatchKind,
    /// +0xC, the self-destruct penalty (gm_8016AEC8): 0, -1 or -2.
    pub self_destruct_penalty: i8,
    /// +0x30, the Damage Ratio rule (gm_8016B248).
    pub knockback_ratio: f32,
    /// +2 bit 4, single_button (gm_8016B0FC: lbz 0x24CA; extrwi 1, 27).
    pub single_button: bool,
}
/// Player_GetMoreFlagsBit2 (StaticPlayer +AD, 0x20) and
/// Player_GetOtherStamina (+64): a Stamina player's hit points.
pub(crate) fn saved_stamina(saved: &saved_pose::SavedPose, slot: u8) -> Option<u16> {
    let player = saved.bytes(0x8045_3080 + u32::from(slot) * 0xE90, 0xAE);
    (player[0xAD] & 0x20 != 0).then(|| u16::from_be_bytes([player[0x64], player[0x65]]))
}
/// FighterMatchInfo[slot].x6 (lbl_8046B6A0 + 0x38 + slot * 0xE + 6).
pub(crate) fn saved_revival_damage(saved: &saved_pose::SavedPose, slot: u8) -> f32 {
    let raw = saved.bytes(0x8046_B6A0 + 0x38 + u32::from(slot) * 0xE + 6, 2);
    f32::from(u16::from_be_bytes([raw[0], raw[1]]))
}
/// Every slot's Player_GetAttackRatio (StaticPlayer +54, stride 0xE90).
pub(crate) fn saved_attack_ratios(saved: &saved_pose::SavedPose) -> [f32; 6] {
    std::array::from_fn(|slot| float(saved.bytes(0x8045_3080 + slot as u32 * 0xE90 + 0x54, 4), 0))
}
impl SavedRules {
    const ADDRESS: u32 = 0x8046_B6A0 + 0x24C8;
    pub(crate) fn read(saved: &saved_pose::SavedPose) -> Self {
        let rules = saved.bytes(Self::ADDRESS, 0x34);
        Self {
            teams: rules[8] == 1,
            friendly_fire: rules[1] & 1 != 0,
            match_kind: melee_types::MatchKind::try_from(i32::from(rules[0] >> 5))
                .unwrap_or_else(|_| unimplemented!("match_kind {}", rules[0] >> 5)),
            self_destruct_penalty: rules[0xC] as i8,
            knockback_ratio: float(rules, 0x30),
            single_button: rules[2] >> 4 & 1 != 0,
        }
    }
}
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
    /// Each player's stock icons, in player order.
    pub(crate) stock_displays: Vec<Option<melee_if::StockDisplay>>,
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
    pub(crate) pending_music: Option<(melee_gr::music::MusicParameters, u16)>,
    pub(crate) selected_music: Option<i32>,
    /// The HUD banner running now (countdown or GO).
    pub(crate) banner: Option<crate::banner::Banner>,
    /// GO, loaded up front and started when the countdown ends.
    pub(crate) go_banner: Option<crate::banner::Banner>,
    pub(crate) stage_gobjs: crate::scene_stage::StageGObjs,
    pub(crate) clock: crate::match_clock::MatchClock,
    pub(crate) bomb_rain: melee_gr::bomb_rain::BombRain,
    /// it_804A0E30: the random item spawner, when items are on.
    pub(crate) item_spawner: Option<melee_it::random_spawn::RandomItemSpawner>,
    /// Actual match rules, retained even when the pick table is empty.
    pub(crate) item_rules: melee_it::random_spawn::ItemRules,
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
    /// StaticPlayer.match_frame_count by slot: the match frame a stock
    /// match took a player's last stock (Player_UpdateMatchFrameCount);
    /// `None` for retail's -1.
    pub(crate) eliminated_at: [Option<u32>; 6],
    /// lb_804329F0's game timer: which loop passes tick.
    pub(crate) frame_timer: crate::frame_timer::FrameTimer,
    /// A Stamina match's scene object (gm_804975F8, fn_801B96E8).
    pub(crate) stamina: Option<crate::stamina::StaminaRules>,
    /// lbl_8046B6A0.terminate_match (gm_8016B328): the next scene frame
    /// decides OUTCOME_TERMINATED.
    pub(crate) match_terminated: bool,
    /// `SlippiCodes::offscreen_damage_by_camera_limits` (match setup, not
    /// saved state).
    pub(crate) offscreen_damage_by_camera_limits: bool,
    /// The dynamics field pool (lb_804D63B0) as the stage's creation or the
    /// saved boundary left it; the runtime owns it from the first tick.
    pub(crate) radial_forces: melee_lb::radial_force::RadialForces,
}
/// Giant and Tiny Melee scale every fighter alike: their scale and PlCo's
/// damage curve for fighter-owned items (it_80272460).
pub(crate) fn fighter_scale(
    fighters: &[SceneFighter],
    assets: &crate::assets::Assets,
) -> Option<(f32, f32)> {
    let scale = fighters.first()?.player.scale;
    assert!(
        fighters.iter().all(|f| f.player.scale == scale),
        "it_80272460: fighters of different model scales"
    );
    (scale != 1.0).then(|| (scale, assets.fighters[0].scale_curves.damage()))
}
/// The HUD's score popups (if_2F72.c) for a match with these rules: a
/// Versus match shows them unless it is a stock match (x2_5, which
/// gm_SetupSuddenDeath clears).
pub(crate) fn score_popups(
    scoring: crate::match_clock::Scoring,
    sudden_death: bool,
    players: impl Iterator<Item = (u8, Option<u8>)>,
) -> melee_ef::score_popups::ScorePopups {
    let mut popups = melee_ef::score_popups::ScorePopups::default();
    popups.enabled = scoring.match_kind != melee_types::MatchKind::Stock && !sudden_death;
    popups.self_destruct_penalty = scoring.self_destruct_penalty;
    for (slot, team) in players {
        popups.teams[usize::from(slot)] = team;
    }
    popups
}
/// gm_8016AE94 for a Versus match (lbl_8046B6A0.unk_34, gm_16AE.c
/// get_unk_float): the random items' weight scale by player count
/// (gm_8016B558).
pub(crate) fn item_weight_scale(players: usize) -> f32 {
    match players {
        3 => 1.1,
        4 => 1.2,
        _ => 1.0,
    }
}

/// Item Switch witnesses cover [`crate::Stage::items_supported`]. Check the
/// actual saved rules on warm import as well as the requested cold setup.
fn require_item_witness(
    rules: melee_it::random_spawn::ItemRules,
    sudden_death: bool,
    stage: melee_types::GrKind,
    players: usize,
) {
    let witnessed = crate::Stage::VERSUS.into_iter().any(|versus| {
        versus.descriptor().is_some_and(|d| d.kind == stage) && versus.items_supported(players)
    });
    if rules.enabled() && !sudden_death && !witnessed {
        unimplemented!(
            "it_8026D018: random items on {stage:?} with {players} players need a retail witness"
        );
    }
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
        // A partner saved before its entry's motion change completed (x3EC
        // still stale heap bytes, not a script; its callbacks and mv words
        // likewise) takes the tick-zero row, which the rest of its creation
        // finished: start_fd_kirby_iceclimbers4's Nana. The other fighters'
        // saved bytes equal that row there.
        let saved_bytes = bases
            .iter()
            .zip(&bytes)
            .zip(&roster)
            .map(|((&address, tick_zero), entry)| {
                let raw = saved.bytes(address, 0x23EC);
                let script = word(raw, 0x3EC);
                let unfinished_partner =
                    entry.secondary && script != 0 && !(0x8000_0000..0x8180_0000).contains(&script);
                if unfinished_partner {
                    tick_zero.clone()
                } else {
                    raw.to_vec()
                }
            })
            .collect::<Vec<_>>();
        let bytes = saved_bytes;
        for (index, raw) in bytes.iter().enumerate() {
            // A primary may already name Entry while Fighter_Create has not
            // initialized its command state or callbacks. No later trace row
            // may complete that saved construction on the port's behalf.
            let script = word(raw, 0x3EC);
            ensure!(
                !(match_start
                    && word(raw, 0x10) == 322
                    && word(raw, 0x14) == u32::MAX
                    && script != 0
                    && !(0x8000_0000..0x8180_0000).contains(&script)),
                "unfinished saved fighter creation at slot {} (script {script:#010X}); use the cold scenario",
                ports[index]
            );
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
        if assets.stage_desc.kind == melee_types::GrKind::Kraid {
            stage_kraid::restore_collision(&saved, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::Onett {
            stage_onett::restore_collision(&saved, &assets, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::Inishie1 {
            stage_inishie1::restore_collision(&saved, &assets, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::Yorster {
            stage_yorster::restore_collision(&saved, &assets, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::Fourside {
            stage_fourside::restore_collision(&saved, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::OldYoshi {
            stage_oldyoshi::restore_collision(&saved, &assets, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::OldKongo {
            stage_oldkongo::restore_collision(&saved, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::Kongo {
            stage_kongo::restore_collision(&saved, &mut map)?;
        }
        if assets.stage_desc.kind == melee_types::GrKind::Flatzone {
            stage_flatzone::restore_collision(&saved, &assets, &mut map)?;
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
        // Every live Item (the item GObjs' user data, p_link 9).
        let items = {
            let entities = word(saved.bytes(0x804D_782C, 4), 0);
            let mut gobj = word(saved.bytes(entities + 9 * 4, 4), 0);
            let mut items = Vec::new();
            while gobj != 0 {
                ensure!(items.len() < 256, "cyclic item GObj list");
                let object = saved.bytes(gobj, 0x30);
                items.push(word(object, 0x2C));
                gobj = word(object, 8);
            }
            items
        };
        for (index, fighter) in fighters.iter_mut().enumerate() {
            let unfinished =
                match_start && word(&bytes[index], 0x10) == 0 && word(&bytes[index], 0x2C) == 0;
            if !unfinished {
                // StaticPlayer (stride 0xE90): slot_type +8, cpu_level +49,
                // cpu_type +4A, as Fighter_Create read them.
                let player = saved.bytes(0x8045_3080 + u32::from(fighter.0.player.id) * 0xE90, 0x4C);
                fighter.0.player.control = match word(player, 8) {
                    0 => melee_types::PlayerKind::Human,
                    1 => melee_types::PlayerKind::Cpu,
                    kind => anyhow::bail!("slot type {kind} is unsupported"),
                };
                fighter.0.player.cpu_level = i32::from(player[0x49]);
                fighter.0.player.cpu_mode = i32::from(player[0x4A]);
            }
            // Only a CPU-driven fighter reads the rest of CpuFighter; a
            // human's keeps stale words (Peach's savestates hold an old x4C).
            let cpu_driven = fighter.0.input_source() == melee_ft::input::human::InputSource::Cpu;
            if !unfinished && cpu_driven {
                fighter.0.cpu = cpu::restore(&bytes[index], bases[index], &bases, &items);
            }
            fighter.0.cpu.fighter_address = Some(bases[index]);
        }
        for fighter in &mut fighters {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                // Player_GetHandicap, StaticPlayer +4B, stride E90.
                f.grab_handicap =
                    saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0x4B, 1)[0];
                // Player_GetTeam (StaticPlayer +47) and the team rules.
                let rules = SavedRules::read(&saved);
                let team = saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0x47, 1)[0];
                f.player.team = rules.teams.then_some(team);
                f.player.friendly_fire = rules.friendly_fire;
                f.player.match_kind = rules.match_kind;
                // Player_GetAttackRatio / Player_GetDefenseRatio: +54, +58.
                let ratios = saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0x54, 8);
                f.player.attack_ratio = float(ratios, 0);
                f.player.defense_ratio = float(ratios, 4);
                f.player.knockback_ratio = rules.knockback_ratio;
                f.player.attack_ratios = saved_attack_ratios(&saved);
                f.player.revival_damage = saved_revival_damage(&saved, f.player.id);
                f.player.stamina = saved_stamina(&saved, f.player.id);
                f.player.single_button = rules.single_button;
                // Player_GetFlagsBit6: StaticPlayer +AC, extrwi 1, 30.
                f.player.invisible =
                    saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0xAC, 1)[0] & 2 != 0;
                // Player_GetSuicideCount: StaticPlayer +8C.
                let sd = saved.bytes(0x8045_3080 + u32::from(f.player.id) * 0xE90 + 0x8C, 2);
                f.player.self_destructs = u32::from(u16::from_be_bytes([sd[0], sd[1]]));
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
        ensure!(
            clock.scoring.match_kind == setup.mode,
            "the scenario names a {:?} match, the savestate runs {:?}",
            setup.mode,
            clock.scoring.match_kind
        );
        // gm_1A3F.c state_machine (0x80479D30) curr_mode: the Special Melee
        // modes (Stamina, Giant, Tiny, Invisible, Super Sudden Death,
        // Lightning, Slo-Mo, Camera, Single Button, Fixed Camera) are major
        // scenes of their own, with rules the VS scene never sets.
        const GM_VS: u8 = 2;
        let major = saved.bytes(0x8047_9D30, 1)[0];
        let special = match major {
            GM_VS => None,
            other => Some(crate::SpecialMelee::from_game_mode(other).unwrap_or_else(|| {
                unimplemented!("GameModeKind {other:#x}: a match outside VS. Mode")
            })),
        };
        ensure!(
            special == setup.special,
            "the scenario names Special Melee {:?}, the savestate runs {special:?}",
            setup.special
        );
        if let Some(mode) = special {
            mode.require_ported();
        }
        let rules = SavedRules::read(&saved);
        ensure!(
            rules.knockback_ratio == setup.damage_ratio,
            "the scenario names damage ratio {}, the savestate runs {}",
            setup.damage_ratio,
            rules.knockback_ratio
        );
        for (p, player) in setup.fighters.iter().enumerate() {
            let saved_ratios = saved.bytes(0x8045_3080 + u32::from(player.slot) * 0xE90 + 0x54, 8);
            let saved_ratios = (float(saved_ratios, 0), float(saved_ratios, 4));
            ensure!(
                saved_ratios == setup.ratios(p),
                "port {}: the scenario's handicap gives ratios {:?}, the savestate runs {saved_ratios:?}",
                player.slot,
                setup.ratios(p)
            );
        }
        if matches!(
            clock.scoring.match_kind,
            melee_types::MatchKind::Coin | melee_types::MatchKind::Bonus
        ) {
            unimplemented!(
                "gm_8016B0B4: a {:?} match (coins, bonus points) needs a retail witness",
                clock.scoring.match_kind
            );
        }
        let (mut camera, subjects) = camera::restore(&saved)?;
        camera.screen = setup.slippi.screen;
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
        let radial_forces = saved_radial_forces(&saved)?;
        stage.set_frozen_stages(setup.slippi.frozen_stages);
        crate::frame::validate_saved_resume(
            &resume,
            &stage,
            fighters.iter().filter(|f| !f.0.player.secondary).count(),
            fighters.len(),
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
            Some((
                melee_gr::desc::read_music(&assets.stage, assets.stage_descriptor.music_id)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                unlocks,
            ))
        } else {
            None
        };
        // ifStock_804A1378: import only the saved HUD boundary, never later trace rows.
        let players: Vec<(u8, u8)> = (0..fighters.iter().filter(|f| !f.0.player.secondary).count())
            .map(|player| {
                let own = crate::scene_fighter::player_fighter_index(&fighters, player);
                crate::scene_fighter::with_fighter!(&fighters[own], |f| (
                    f.player.stocks,
                    f.player.id
                ))
            })
            .collect();
        let stock_displays = if match_start {
            stock::create(
                &assets.interface,
                &players.iter().map(|p| p.0).collect::<Vec<_>>(),
                &players.iter().map(|p| p.1).collect::<Vec<_>>(),
            )?
        } else {
            // The icons' frame-0 poses a stolen stock's path reads, from the
            // HUD as the match created it.
            let created = stock::create(
                &assets.interface,
                &players.iter().map(|_| 0).collect::<Vec<_>>(),
                &players.iter().map(|&(_, port)| port).collect::<Vec<_>>(),
            )?;
            // ifStock_804A1378 keeps one record per port.
            players
                .iter()
                .zip(created)
                .map(|(&(_, port), created)| {
                    let created = created.expect("a created stock display");
                    let slot = u32::from(port);
                    let player = saved.bytes(0x804A_1378 + 8 + slot * 0x50, 0x50);
                    let state = saved.bytes(0x804A_1378 + 0x204 + slot * 0x54, 12);
                    let root = vector(saved.bytes(word(player, 4) + 0x38, 12), 0);
                    let icon_positions = std::array::from_fn(|i| {
                        let local = vector(saved.bytes(word(player, 8 + i * 4) + 0x38, 12), 0);
                        Vec3::new(local.x + root.x, local.y + root.y, local.z + root.z)
                    });
                    // anim[5..7]: a stolen stock in flight.
                    if state[10] != 0 || state[11] != 0 {
                        unimplemented!("ifStock_802F8298: a stolen stock in flight at a boundary");
                    }
                    Some(melee_if::StockDisplay {
                        icon_positions,
                        animation_frames: state[5..10].try_into().unwrap(),
                        animate_losses: state[2] != 0,
                        root: created.root,
                        icon_locals: created.icon_locals,
                        steals: [None; 2],
                    })
                })
                .collect()
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
        let mut effects = Box::new(melee_ef::Effects::from_resources(&assets.effect_resources));
        effects.quake_models = quakes.has_models();
        effects.score_popups = score_popups(
            clock.scoring,
            clock.sudden_death,
            fighters.iter().map(|f| (f.player.id, f.player.team)),
        );
        let mut items = Box::new(melee_it::ItemPool::new(assets.items.common.clone()));
        items.fighter_scale = fighter_scale(&fighters, &assets);
        // Onett's cars' material items, rebuilt on their bones.
        crate::scene_stage::onett::spawn_items(
            &mut stage,
            &mut stage_animations,
            &mut items,
            &mut effects.gobjs,
            &assets.items,
            &mut map,
        );
        // Mushroom Kingdom's bricks' material items, rebuilt on their bones.
        crate::scene_stage::inishie1::spawn_items(
            &mut stage,
            &mut stage_animations,
            &mut items,
            &mut effects.gobjs,
            &assets.items,
            &mut map,
        );
        if matches!(stage, crate::scene_stage::SceneStage::Garden(_)) {
            // The Klaptrap's idle material item, rebuilt on its jaw.
            let anchor = crate::scene_stage::garden::klaptrap_jaw(&mut stage_animations);
            crate::scene_items::spawn_yaku(
                &mut items,
                &mut effects.gobjs,
                &assets.items,
                &mut map,
                anchor,
            );
        }
        Ok(Self {
            items,
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
            stage_gobjs: Default::default(),
            go_banner: Some(crate::banner::Banner::preload(
                &assets.interface,
                crate::banner::BannerKind::Go,
            )?),
            clock,
            bomb_rain,
            item_spawner: scene_flow::restore_item_spawner(&saved, &setup, &assets)?,
            item_rules: scene_flow::restore_item_rules(&saved),
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
            // Player_GetMatchFrameCount: StaticPlayer +88, -1 unset.
            frame_timer: {
                // lb_804329F0: x0[0].x0 (+0), x0[0].x8 (+8), x38, x40.
                let timer = saved.bytes(0x8043_29F0, 0x48);
                let wide = |at: usize| u64::from(word(timer, at)) << 32 | u64::from(word(timer, at + 4));
                let saved_timer = crate::frame_timer::FrameTimer {
                    period: wide(0),
                    accumulated: wide(8),
                    requested: wide(0x38),
                    step: wide(0x40),
                    requested_in_tick: false,
                };
                let speed = setup.special.map_or(1.0, crate::SpecialMelee::game_speed);
                ensure!(
                    saved_timer.requested == crate::frame_timer::FrameTimer::period_for(speed),
                    "the savestate's frame timer {saved_timer:?} is not game speed {speed}'s"
                );
                saved_timer
            },
            eliminated_at: std::array::from_fn(|slot| {
                let frame = word(saved.bytes(0x8045_3080 + slot as u32 * 0xE90 + 0x88, 4), 0);
                (frame != u32::MAX).then_some(frame)
            }),
            match_terminated: false,
            stamina: (setup.special == Some(crate::SpecialMelee::Stamina)).then(|| {
                let (address, len) = crate::stamina::StaminaRules::SAVED_RANGE;
                let rules = crate::stamina::StaminaRules::saved(saved.bytes(address, len));
                // A match-start boundary predates fn_801B9850 (its empty
                // slots are not yet marked out).
                if match_start {
                    assert!(
                        rules == Default::default(),
                        "a match-start Stamina boundary after fn_801B9850"
                    );
                    crate::stamina::StaminaRules::starting(std::array::from_fn(|slot| {
                        // StaticPlayer +8, slot_type: Gm_PKind_NA is 3.
                        word(saved.bytes(0x8045_3080 + slot as u32 * 0xE90 + 8, 4), 0) != 3
                    }))
                } else {
                    rules
                }
            }),
            offscreen_damage_by_camera_limits: setup.slippi.offscreen_damage_by_camera_limits,
            radial_forces,
        })
    }
}

/// lb_804D63B0 (the dynamics field pool's active list) and lb_804D63B4 (its
/// wind state) at a saved boundary; `lb_80011A50_t` (lb/types.h:399).
fn saved_radial_forces(saved: &saved_pose::SavedPose) -> Result<melee_lb::radial_force::RadialForces> {
    use melee_lb::{dynamics::ForceField, radial_force::SavedField};
    /// `x0` of a directional field (lb_80011A50); lb_800119DC's are 0.
    const DIRECTIONAL: u8 = 1;
    let mut entries = Vec::new();
    let mut entry = word(saved.bytes(0x804D_63B0, 4), 0);
    while entry != 0 {
        ensure!(entries.len() < 8, "cyclic dynamics field list");
        let raw = saved.bytes(entry, 0x38);
        let vec = |at: usize| Vec3::new(float(raw, at), float(raw, at + 4), float(raw, at + 8));
        entries.push(SavedField {
            field: ForceField {
                direction_or_center: vec(0x4),
                rectangle: (raw[0] == DIRECTIONAL).then(|| {
                    [float(raw, 0x10), float(raw, 0x14), float(raw, 0x18), float(raw, 0x1C)]
                }),
                strength: float(raw, 0x20),
                phase: word(raw, 0x30) as i32,
                phase_step: float(raw, 0x2C),
            },
            frames: word(raw, 0x28) as i32,
            decay: float(raw, 0x24),
            priority: i32::from(raw[1]),
        });
        entry = word(raw, 0x34);
    }
    let wind = word(saved.bytes(0x804D_63B4, 4), 0) as i32;
    melee_lb::radial_force::RadialForces::from_saved(&entries, wind)
        .ok_or_else(|| anyhow::anyhow!("dynamics field pool: wind state {wind}"))
}
