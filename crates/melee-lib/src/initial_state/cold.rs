//! Versus setup from scene parameters and the pre-music boundary seed.
//! Captures are comparison inputs in cold_tests.rs only.
use super::InitialState;
use crate::{assets::Assets, scene_fighter::SceneFighter, scene_stage::SceneStage, setup::Setup};
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
use hsd_anim::load::load_joint_tree;
use hsd_particle::{
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Vec3;
use melee_ft::fighter::{PlayerSlot, RetailTrig, SpawnContext, SpawnCounter};
use melee_gr::{
    battle::Battlefield,
    last::{animation::BackgroundAnimation, FinalDestination},
};
use melee_types::{GrKind, PlayerKind};
use std::collections::BTreeMap;

/// ftCo_800A101C and ftCo_800B9704, including human slots and a
/// player's partner fighter (Nana).
const CPU_SETUP_DRAWS_PER_FIGHTER: usize = 2;
/// fn_8016D8AC increments Player's Entry delay by five per entering slot.
const ENTRY_STAGGER_FRAMES: i32 = 5;

impl InitialState {
    /// fn_8016E730 (gm_16AE.c): Ground creation, Player/Fighter creation,
    /// then the pre-music boundary. Only owned DAT resources are read.
    pub fn from_parameters(source: &dyn crate::diagnostics::ScenarioSource) -> Result<Self> {
        let assets = Self::load_assets(source)?;
        Self::from_parameters_with_assets(source, assets)
    }
    /// The archives a cold setup reads. They depend on the characters and
    /// the stage alone, so setups that differ in anything else (seed,
    /// costumes, codes) can share them.
    pub fn load_assets(
        source: &dyn crate::diagnostics::ScenarioSource,
    ) -> Result<std::sync::Arc<Assets>> {
        let setup = source.setup()?;
        Ok(std::sync::Arc::new(Assets::load(
            &source.assets_path(),
            &setup.roster_descriptors(),
            setup.stage_descriptor(),
        )?))
    }
    /// [`Self::from_parameters`] with archives already read
    /// ([`Self::load_assets`] of a setup with the same characters and stage).
    pub fn from_parameters_with_assets(
        source: &dyn crate::diagnostics::ScenarioSource,
        assets: std::sync::Arc<Assets>,
    ) -> Result<Self> {
        ensure!(
            source.is_cold(),
            "cold construction requires savestate omitted"
        );
        let setup = source.setup()?;
        let mut state = Self::from_assets(&setup, assets)?;
        if source.has_boundary_recording() {
            state.carry_stale_heap(&setup, source)?;
            state.carry_particle_families(source)?;
        }
        Ok(state)
    }
    /// The particle family counter (lbl_804D6368, psGenerator idnum) only
    /// counts up over the session, menus and earlier matches included, so no
    /// scene parameter decides it either: take it from the boundary's initial
    /// particle dump when the recording has one (a Sudden Death or timeout
    /// boundary saved after a match starts above the power-on 0x100). The
    /// families the stage's creation took are numbered from it as well.
    fn carry_particle_families(
        &mut self,
        source: &dyn crate::diagnostics::ScenarioSource,
    ) -> Result<()> {
        let path = source.trace_path("particles.jsonl.initial.jsonl");
        if source.open_trace(&path).is_err() {
            return Ok(());
        }
        let record = super::first_json(source, &path)?;
        let counter = record["state"]["particles.family_id_counter"]["v"]
            .as_u64()
            .context("missing particles.family_id_counter")?;
        // The families the stage's creation already took shift with it.
        let counter = u16::try_from(counter)?;
        let shift = counter.wrapping_sub(self.particles.family_counter);
        if shift == 0 {
            return Ok(());
        }
        for generator in &mut self.particles.generators {
            generator.family_id = generator.family_id.wrapping_add(shift);
        }
        for particle in self.particles.particles.iter_mut().flatten() {
            particle.family_id = particle.family_id.wrapping_add(shift);
        }
        self.particles.family_counter = counter;
        Ok(())
    }
    /// HSD_ObjAlloc (0x8037ABC8) returns a Fighter block uncleared and
    /// Fighter_Create (0x80068E98) writes only part of it, so some character
    /// variables start as whatever the heap held. No scene parameter decides
    /// them: take them from the boundary's retail recording, whose first
    /// record is each Fighter as created (fighter-list order).
    fn carry_stale_heap(
        &mut self,
        setup: &Setup,
        source: &dyn crate::diagnostics::ScenarioSource,
    ) -> Result<()> {
        let boundary = super::first_json(source, &source.trace_path("tick.raw.jsonl"))?;
        let rows = boundary["fighters"]
            .as_array()
            .context("missing fighters")?;
        let roster = setup.roster();
        ensure!(
            rows.len() == roster.len() && rows.len() == self.fighters.len(),
            "the boundary recording holds {} fighters, the scene {}",
            rows.len(),
            roster.len()
        );
        for ((row, entry), fighter) in rows.iter().zip(&roster).zip(&mut self.fighters) {
            let hex = row["bytes"].as_str().context("missing fighter bytes")?;
            ensure!(hex.len() == 0x23EC * 2, "invalid Fighter dump length");
            let raw = hex
                .as_bytes()
                .chunks_exact(2)
                .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
                .collect::<Result<Vec<u8>>>()?;
            ensure!(
                super::word(&raw, 4) == i32::from(entry.descriptor.kind) as u32
                    && raw[12] == setup.fighters[entry.player].slot,
                "the boundary recording's fighters are not this scene's"
            );
            fighter.0.character.restore_stale_heap(&raw);
            // The block's own address, which no parameter decides either.
            let base = row["base"].as_str().context("missing fighter base")?;
            fighter.0.cpu.fighter_address =
                Some(u32::from_str_radix(base.trim_start_matches("0x"), 16)?);
        }
        Ok(())
    }
    pub(crate) fn from_assets(scenario: &Setup, assets: std::sync::Arc<Assets>) -> Result<Self> {
        let mut map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        // The supplied seed is after creation, before music. The random item
        // spawner's first interval, grLast's four draws (Battle/Story: one;
        // Dream Land: two), then two CPU draws per slot.
        // Invert this fixed, audited interval; never search an oracle at runtime.
        let boundary_seed = scenario.seed.expect("validated cold seed");
        // it_8026D018 draws its first interval when it creates the spawner.
        let spawner_draws =
            usize::from(create_item_spawner(scenario, &assets, &mut HsdRng::default()).is_some());
        let fixed_draws = setup_draws(
            assets.stage_desc.kind,
            scenario.roster().iter().map(|entry| entry.descriptor.kind),
        )? + spawner_draws;
        let mut rng = HsdRng::new(before_setup(boundary_seed, fixed_draws));
        if assets.stage_desc.kind == GrKind::Pura {
            // The sky's draws depend on what it draws: the one count whose
            // creation, run from the seed it implies, draws that many.
            let start = (1..=MAX_VARIABLE_DRAWS)
                .map(|n| HsdRng::new(before_setup(boundary_seed, fixed_draws + n)))
                .find(|&start| {
                    let mut at_stage = start;
                    for _ in 0..spawner_draws {
                        at_stage.rand();
                    }
                    let n = variable_stage_draws(GrKind::Pura, at_stage);
                    HsdRng::new(before_setup(boundary_seed, fixed_draws + n)) == start
                });
            rng = start.context("Poké Floats: no sky draw count reaches the boundary seed")?;
        }
        // fn_8016E730: Item_80266FCC, then it_8026D018 (the random item
        // spawner), after the stage's files load (Stage_802251E8) and before
        // its maps are created (Stage_8022524C), where every stage draws.
        let item_spawner = create_item_spawner(scenario, &assets, &mut rng);
        let mut particles = ParticleSystem::default();
        let (mut stage, mut stage_animations) =
            initialize_stage(&assets, &scenario.slippi, item_spawner.is_some(), &mut rng, &mut particles, &mut map)?;
        if let SceneStage::Stadium(stadium) = &mut stage {
            if scenario.slippi.stadium_preload {
                // Init isLoaded Bool.asm (0x801D14C8), in map 2's init.
                stadium.transformation.preload = melee_gr::stadium::transform::Preload::Pending;
            }
            stadium.transformation.frozen = scenario.slippi.stadium_frozen;
        }
        stage.set_frozen_stages(scenario.slippi.frozen_stages);
        // grShrine_80201C64's lb_80011A50: the field outlives the match.
        let mut radial_forces = melee_lb::radial_force::RadialForces::default();
        if matches!(stage, SceneStage::Shrine(_)) {
            radial_forces.insert_directional(melee_gr::shrine::wind());
        }
        // grVenom_80203B18's two lb_80011A50 fields, after maps 4..7.
        if matches!(stage, SceneStage::Venom(_)) {
            for gust in crate::scene_stage::venom::winds() {
                radial_forces.insert_directional(gust);
            }
        }
        // Fountain of Dreams and the N64 stages bind their collision
        // joints during creation.
        let archive_bindings = !matches!(
            stage,
            SceneStage::Izumi(_)
                | SceneStage::OldYoshi(_)
                | SceneStage::OldKongo(_)
                | SceneStage::Kongo(_)
                | SceneStage::Fourside(_)
                | SceneStage::Kraid(_)
                | SceneStage::FlatZone(_)
                | SceneStage::Yorster(_)
                | SceneStage::Inishie1(_)
                | SceneStage::Onett(_)
                | SceneStage::Zebes(_)
                | SceneStage::Venom(_)
                | SceneStage::MuteCity(_)
                | SceneStage::Pura(_)
                | SceneStage::Inishie2(_)
                | SceneStage::GreatBay(_)
        );
        if matches!(stage, SceneStage::Stadium(_)) {
            crate::scene_stage::stadium::initialize_collision(&mut map, &mut stage_animations);
        }
        for (&id, animation) in stage_animations.iter_mut().filter(|_| archive_bindings) {
            let bindings = &assets.stage_desc.models[id as usize].joint_mappings;
            animation.update_collision(&mut map, bindings);
            for binding in bindings {
                map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
            }
        }
        let (mut fighters, spawn_counter) =
            create_players(scenario, &assets, &mut map, &mut stage_animations, &mut rng)?;
        for (index, fighter) in fighters.iter_mut().enumerate() {
            fighter.0.cpu.fighter_address =
                super::heap::fighter_address_after_boot(scenario, index);
        }
        // gm_16AE: Camera_80030730 (the stage's field of view, see
        // StageCamera::fov), then Camera_8002F3AC snaps to the players.
        let mut camera = melee_cm::GameCamera::new();
        camera.screen = scenario.slippi.screen;
        // Ground_EnableMatchCamera, then Camera_8002F3AC.
        camera.enable_match_camera(assets.stage_desc.parameters.fixed_camera);
        {
            // cm_804D6468 runs newest first: the reverse of creation.
            let live_camera = crate::scene_stage::live_camera(&stage, &assets.stage_camera);
            let stage_subjects: &mut [melee_cm::Subject] = match &mut stage {
                SceneStage::Pura(stage) => &mut stage.subjects[..],
                _ => &mut [],
            };
            let subjects = stage_subjects
                .iter_mut()
                .rev()
                .chain(fighters.iter_mut().rev().map(|f| &mut f.0.camera));
            crate::frame::with_subjects(subjects, |subjects| {
                camera.snap(subjects, &live_camera)
            });
        }
        let quakes =
            crate::quake::Quakes::load(&assets.stage, assets.stage_desc.quake_model.as_ref())?;
        ensure!(
            rng.seed == boundary_seed,
            "setup draw count differs from the boundary seed contract"
        );
        let pending_music = Some((
            melee_gr::desc::read_music(&assets.stage, assets.stage_descriptor.music_id)
                .map_err(|e| anyhow::anyhow!("{e}"))?,
            if scenario
                .all_characters_unlocked
                .expect("validated cold music rule")
            {
                melee_gr::music::ALL_UNLOCKABLE_CHARACTERS
            } else {
                0
            },
        ));
        let mut effects = Box::new(melee_ef::Effects::from_resources(&assets.effect_resources));
        effects.quake_models = quakes.has_models();
        if let Some(mode) = scenario.special {
            mode.require_ported();
        }
        if matches!(
            scenario.mode,
            melee_types::MatchKind::Bonus
        ) {
            unimplemented!(
                "gm_8016B0B4: a {:?} match (coins, bonus points) needs a retail witness",
                scenario.mode
            );
        }
        effects.score_popups = super::score_popups(
            crate::match_clock::Scoring {
                match_kind: scenario.mode,
                ..Default::default()
            },
            scenario.sudden_death,
            scenario.fighters.iter().map(|f| (f.slot, f.team)),
        );
        let mut items = Box::new(melee_it::ItemPool::new(assets.items.common.clone()));
        items.fighter_scale = super::fighter_scale(&fighters, &assets);
        items.set_heap_ammo_words(&scenario.heap_ammo_words);
        // grOnett_801E41C8: the cars' material items.
        crate::scene_stage::onett::spawn_items(
            &mut stage,
            &mut stage_animations,
            &mut items,
            &mut effects.gobjs,
            &assets.items,
            &mut map,
        );
        // grZebes_801D9100 / 801D8644: the plates' and bubbles' items.
        crate::scene_stage::zebes::spawn_items(
            &mut stage,
            &mut stage_animations,
            &mut items,
            &mut effects.gobjs,
            &assets.items,
            &mut map,
        );
        // it_802ECA70: Tingle, with the draws the stage's creation took.
        crate::scene_stage::greatbay::spawn_tingle(
            &mut stage,
            &mut items,
            &mut effects.gobjs,
            &assets.items,
            &mut map,
            assets.stage_desc.parameters.map_scale,
        );
        // grInishie1_801FAD84: the bricks' material items.
        crate::scene_stage::inishie1::spawn_items(
            &mut stage,
            &mut stage_animations,
            &mut items,
            &mut effects.gobjs,
            &assets.items,
            &mut map,
        );
        if matches!(stage, SceneStage::Garden(_)) {
            // grGarden_802031A4: the Klaptrap's material item on its jaw.
            let anchor = crate::scene_stage::garden::klaptrap_jaw(&mut stage_animations);
            crate::scene_items::spawn_yaku(
                &mut items,
                &mut effects.gobjs,
                &assets.items,
                &mut map,
                anchor,
            );
        }
        let stage_limits = crate::stage_limits::StageLimits::new(&assets);
        let rendered_camera =
            camera.render_camera(&crate::scene_stage::live_camera(&stage, &stage_limits.camera));
        // fn_801B9850, the Stamina scene's on_match_start.
        let stamina = (scenario.special == Some(crate::SpecialMelee::Stamina)).then(|| {
            crate::stamina::StaminaRules::starting(std::array::from_fn(|slot| {
                scenario
                    .fighters
                    .iter()
                    .any(|f| usize::from(f.slot) == slot)
            }))
        });
        let mut state = Self {
            items,
            article_poses: Box::new(crate::article_pose::ArticlePoses::new(
                &assets.items.article_skeletons,
            )),
            stock_displays: super::stock::create(
                &assets.interface,
                &scenario.fighters.iter().map(|f| f.stocks).collect::<Vec<_>>(),
                &scenario.fighters.iter().map(|f| f.slot).collect::<Vec<_>>(),
            )?,
            spawn_counter,
            banner: Some(crate::banner::Banner::from_archive(
                &assets.interface,
                if scenario.sudden_death {
                    crate::banner::BannerKind::SuddenDeathCountdown
                } else {
                    crate::banner::BannerKind::Countdown
                },
            )?),
            stage_gobjs: Default::default(),
            go_banner: Some(crate::banner::Banner::preload(
                &assets.interface,
                crate::banner::BannerKind::Go,
            )?),
            clock: crate::match_clock::MatchClock {
                sudden_death: scenario.sudden_death,
                timer: scenario
                    .time_limit
                    .map(crate::match_clock::CountdownTimer::start),
                scoring: crate::match_clock::Scoring {
                    match_kind: scenario.mode,
                    ..Default::default()
                },
                ..Default::default()
            },
            bomb_rain: Default::default(),
            item_spawner,
            item_rules: scenario.items,
            revival_offsets: Default::default(),
            assets,
            map,
            stage,
            particles,
            stage_animations,
            fighters,
            rng,
            pending_music,
            selected_music: None,
            resume: super::scheduler_resume::SchedulerResume::between_ticks(true),
            // A cold match has no emission interrupted by a save boundary.
            pending_emission: None,
            effects,
            camera,
            rendered_camera,
            quakes,
            controller_fixes: scenario.controller_fixes()?,
            pad_buffers: Default::default(),
            dween_previous_x: [0.0; 4],
            ko_counts: Default::default(),
            eliminated_at: [None; 6],
            stamina,
            match_terminated: false,
            coins: Default::default(),
            scene_frame: 0,
            frame_timer: crate::frame_timer::FrameTimer::entering(
                scenario.special.map_or(1.0, crate::SpecialMelee::game_speed),
            ),
            offscreen_damage_by_camera_limits: scenario
                .slippi
                .offscreen_damage_by_camera_limits,
            radial_forces,
            stage_limits,
        };
        crate::coins::start(&mut state, true);
        Ok(state)
    }
}

/// it_8026D018 for a Versus match of `scenario`'s players.
fn create_item_spawner(
    scenario: &Setup,
    assets: &Assets,
    rng: &mut HsdRng,
) -> Option<melee_it::random_spawn::RandomItemSpawner> {
    super::require_item_witness(
        scenario.items,
        scenario.sudden_death,
        assets.stage_desc.kind,
        scenario.fighters.len(),
    );
    let common = &assets.items.common.random_items;
    melee_it::random_spawn::RandomItemSpawner::create(
        scenario.items,
        scenario.sudden_death,
        &assets.item_rates,
        super::item_weight_scale(scenario.fighters.len()),
        &common.monster_weights,
        &common.intervals,
        rng,
    )
}

/// Inverse of the odd HSD LCG multiplier, modulo 2^32.
/// The setup's fixed RNG interval: grLast's four draws (Battle/Story/
/// Pokemon Stadium: one; Dream Land: two), then per fighter two CPU draws
/// and what its kind's OnDeath draws (Kirby's Stone form).
fn setup_draws(
    stage: GrKind,
    fighters: impl Iterator<Item = melee_types::FighterKind>,
) -> Result<usize> {
    let stage_draws = match stage {
        GrKind::Last => 4,
        GrKind::Battle | GrKind::Story | GrKind::PStadium => 1,
        GrKind::OldPupupu => 2,
        // grIzumi_801CC358's first step draws each platform's wait.
        GrKind::Izumi => 2,
        // grOldYoshi_8020EFCC: the guest's first wait.
        GrKind::OldYoshi => 1,
        // stageGObj1_OnInit: the barrel's turn wait and drift;
        // stageGObj2_OnInit: the bird's wait.
        GrKind::OldKongo => 3,
        // Map 4's six frame-zero keys, Cranky's loops (grGarden_80203004),
        // the Klaptrap's wait (grGarden_802031A4), then the main stage's
        // keys (a common-bank one draws).
        GrKind::Garden => 9,
        // The barrel's two waits (grKongo_801D55D8), then two keys.
        GrKind::Kongo => 4,
        // grShrine_80201C64 draws nothing (GrSh.dat has no particle keys).
        GrKind::Shrine => 0,
        // grFourside_801F3154 and 801F3C40: the crane's and the
        // helicopter's first waits.
        GrKind::Fourside => 2,
        // grKraid_801FE818: Kraid's first wait.
        GrKind::Kraid => 1,
        // The controller's event wait (grFlatzone_8021707C), then the
        // platforms' variant wait (grFlatzone_80218060(1)).
        GrKind::Flatzone => 2,
        // grYorster_80201FA4 draws nothing (GrYt.dat has no particle keys).
        GrKind::Yorster => 0,
        // grInishie1_801FAD84 draws only with items on (the `?` blocks).
        GrKind::Inishie1 => 0,
        // grOnett_801E3A34's smoke pause, grOnett_801E41C8's HSD_Randi(4).
        GrKind::Onett => 2,
        // grZebes_801D9508's HSD_Randi(600); grZebes_801D9798's first delay
        // (rand_range(800, 0)) and height (HSD_Randf).
        GrKind::Zebes => 3,
        // grVenom_80203B18 draws nothing: its first wait is 600 to 600.
        GrKind::Venom => 0,
        // grMuteCity_801EFC6C draws nothing.
        GrKind::MuteCity => 0,
        // stageGObj1_OnInit's sky draws until its colours differ
        // (variable_stage_draws); GrPu.dat has no particle keys.
        GrKind::Pura => 0,
        // grInishie2_801FCE04: lift A, lift B, Birdo, the carpet.
        GrKind::Inishie2 => 4,
        // grGreatBay_801F4694 (the turtle's wait), 801F5A00 (the moon's),
        // it_802ECA70 (the facing coin, it_802EB6DC twice).
        GrKind::GreatBay => 7,
        kind => anyhow::bail!("cold setup: {kind:?} has no audited setup draws"),
    };
    let fighter_draws: usize = fighters
        .map(|kind| {
            CPU_SETUP_DRAWS_PER_FIGHTER + crate::scene_fighter::SceneFighter::reset_draws_for(kind)
        })
        .sum();
    Ok(stage_draws + fighter_draws)
}

/// A scene built after another one ends: the RNG carries over unchanged
/// through the scenes between (the Sudden Death tie screen draws nothing),
/// so the new scene's boundary seed is the old seed after its setup draws.
pub(crate) fn boundary_seed_after(
    seed: u32,
    stage: GrKind,
    fighters: impl Iterator<Item = melee_types::FighterKind>,
) -> Result<u32> {
    let mut rng = HsdRng::new(seed);
    let draws = setup_draws(stage, fighters)? + variable_stage_draws(stage, rng);
    for _ in 0..draws {
        rng.rand();
    }
    Ok(rng.seed)
}

/// More draws than any variable stage creation takes.
const MAX_VARIABLE_DRAWS: usize = 64;

/// The draws a stage's creation takes from `rng` when their count depends
/// on what it draws: Poké Floats' sky (stageGObj1_OnInit).
fn variable_stage_draws(stage: GrKind, rng: HsdRng) -> usize {
    if stage != GrKind::Pura {
        return 0;
    }
    let mut probe = rng;
    melee_gr::pura::Sky::new(&mut probe);
    let mut walk = rng;
    let mut draws = 0;
    while walk != probe {
        walk.rand();
        draws += 1;
    }
    draws
}

fn before_setup(mut seed: u32, draws: usize) -> u32 {
    const INVERSE_MULTIPLIER: u32 = 0xB9B3_3155;
    for _ in 0..draws {
        seed = seed
            .wrapping_sub(HsdRng::INCREMENT)
            .wrapping_mul(INVERSE_MULTIPLIER);
    }
    seed
}

/// Stage_80224E64 -> Ground_801C2D24: map-0 spawn marker under map scale.
fn spawn_position(assets: &Assets, slot: i16) -> Result<Vec3> {
    let binding = assets
        .stage_desc
        .position_bindings
        .iter()
        .rfind(|b| b.stage_position == slot)
        .ok_or_else(|| anyhow::anyhow!("missing spawn marker {slot}"))?;
    let (mut tree, root) = load_joint_tree(
        &assets.stage,
        &assets.stage_desc.models[binding.model_id].joint,
    )?;
    let wrapper = tree.alloc();
    let scale = assets.stage_desc.parameters.map_scale;
    tree.set_scale(wrapper, &Vec3::new(scale, scale, scale));
    tree.add_child(wrapper, root);
    let joint = tree
        .bone(root, binding.joint_index as usize)
        .expect("validated marker joint");
    Ok(melee_ft::collision::ecb::world_position(&mut tree, joint))
}

/// Ground_801C2D24 reads the marker's live JObj (stage_info.x280). Kongo
/// Jungle's markers sit on the bridge and platform ends its creation
/// already moved; the other stages' markers rest at their bind pose.
fn live_spawn_position(
    assets: &Assets,
    stage_animations: &mut BTreeMap<u8, BackgroundAnimation>,
    slot: i16,
) -> Option<Vec3> {
    if assets.stage_desc.kind != GrKind::Kongo {
        return None;
    }
    let binding = assets
        .stage_desc
        .position_bindings
        .iter()
        .rfind(|b| b.stage_position == slot)?;
    let model = stage_animations.get_mut(&u8::try_from(binding.model_id).ok()?)?;
    let matrix = model.joint_matrix(binding.joint_index as usize);
    Some(Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]))
}

/// Ground creation precedes every Fighter_Create call.
fn initialize_stage(
    assets: &Assets,
    codes: &crate::slippi::SlippiCodes,
    items_on: bool,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    map: &mut melee_mp::CollMap,
) -> Result<(SceneStage, BTreeMap<u8, BackgroundAnimation>)> {
    let mut stage_animations = BTreeMap::new();
    let stage = match assets.stage_desc.kind {
        GrKind::Last => {
            let mut stage = FinalDestination::initialize(&assets.stage_desc, rng);
            stage.actions.clear();
            stage_animations = crate::scene_stage::last::load_animations(assets)?;
            SceneStage::FinalDestination(Box::new(stage))
        }
        GrKind::Battle => {
            let mut stage = Battlefield::initialize(rng);
            stage.lights = melee_gr::battle::lights::load(&assets.stage, &assets.stage_desc)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            // grBattle_OnInit creates 0,3,1,6. Map 3 waits hidden without
            // animation; grAnime_801C8138 immediately evaluates the others.
            for id in [0, 1, 6] {
                let mut animation = BackgroundAnimation::load_model(
                    &assets.stage,
                    &assets.stage_desc.models[id as usize],
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                animation.set_map_scale(assets.stage_desc.parameters.map_scale);
                for event in animation.evaluate_initial_frame::<RetailTrig>() {
                    let mut request = SpawnRequest::new(event.bank, event.kind, 0);
                    request.joint = Some((super::stage::joint_id(id, event.joint), event.matrix));
                    particles.spawn::<RetailTrig>(
                        &assets.particle_bank,
                        request,
                        rng,
                        &mut DrawLog::default(),
                    )?;
                }
                stage_animations.insert(id, animation);
            }
            crate::scene_stage::battle::load_reserved(assets, &mut stage_animations)?;
            SceneStage::Battlefield(stage)
        }
        GrKind::Story => {
            let parameters = melee_gr::desc::read_story_parameters(&assets.stage)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut stage = melee_gr::story::Story::initialize(parameters, rng);
            stage.lights =
                melee_gr::battle::lights::load_model(&assets.stage, &assets.stage_desc, 3)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            // grStory_801E3030: creation order 0,1,3,2. Maps 1/2 use
            // grAnime_801C8138 (evaluate frame zero); map 3 only requests it.
            for id in [0, 1, 3, 2] {
                let model = &assets.stage_desc.models[id as usize];
                let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                if id == 3 {
                    let subtree = melee_gr::desc::animation_subtree(&assets.stage, model, 1, 5)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    animation
                        .attach_subtree(&assets.stage, 5, &subtree, model.animation_loops[1])
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                animation.set_map_scale(assets.stage_desc.parameters.map_scale);
                if matches!(id, 1 | 2) {
                    ensure!(
                        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
                        "unexpected Story setup particle event"
                    );
                }
                stage_animations.insert(id, animation);
            }
            SceneStage::Story(stage)
        }
        GrKind::OldPupupu => {
            let parameters = melee_gr::desc::read_pupupu_parameters(&assets.stage)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut stage = melee_gr::pupupu::Pupupu::initialize(parameters, rng);
            stage.lights =
                melee_gr::battle::lights::load_model(&assets.stage, &assets.stage_desc, 5)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            // The flyby model is created during the match
            // (grOldPupupu_80210D10); only its animation's length is
            // needed up front.
            stage.flyby_frames = BackgroundAnimation::load_model(
                &assets.stage,
                &assets.stage_desc.models[2],
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .first_animation_end_frame()
            .context("Dream Land flyby animation")? as i32;
            for id in melee_gr::pupupu::procs::MAP_ORDER {
                if id == 8 {
                    continue;
                }
                let mut animation = BackgroundAnimation::load_model(
                    &assets.stage,
                    &assets.stage_desc.models[id as usize],
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                animation.set_map_scale(assets.stage_desc.parameters.map_scale);
                if matches!(id, 0 | 3 | 7 | 4 | 1) {
                    ensure!(
                        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
                        "unexpected Dream Land setup particle event"
                    );
                }
                if id == 6 {
                    animation.clear_animation();
                }
                stage_animations.insert(id, animation);
            }
            SceneStage::Pupupu(stage)
        }
        GrKind::Izumi => {
            let (stage, animations) =
                crate::scene_stage::izumi::initialize(assets, codes, rng, particles, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::OldYoshi => {
            let (stage, animations) = crate::scene_stage::oldyoshi::initialize(assets, rng, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::OldKongo => {
            let (stage, animations) = crate::scene_stage::oldkongo::initialize(assets, rng, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Garden => {
            let (stage, animations) =
                crate::scene_stage::garden::initialize(assets, rng, particles)?;
            stage_animations = animations;
            stage
        }
        GrKind::Kongo => {
            // A fresh boot's bridge (grKg_803E188C as the DOL holds it).
            let (stage, animations) = crate::scene_stage::kongo::initialize(
                assets,
                melee_gr::kongo::Bridge::default(),
                rng,
                particles,
                map,
            )?;
            stage_animations = animations;
            stage
        }
        GrKind::Kraid => {
            let (stage, animations) =
                crate::scene_stage::kraid::initialize(assets, rng, particles, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Fourside => {
            let (stage, animations) = crate::scene_stage::fourside::initialize(assets, rng, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Shrine => {
            let (stage, animations) = crate::scene_stage::shrine::initialize(assets)?;
            stage_animations = animations;
            stage
        }
        GrKind::Flatzone => {
            let (stage, animations) = crate::scene_stage::flatzone::initialize(
                assets,
                rng,
                map,
                melee_gr::flatzone::BOOT_STALE_EVENT,
            )?;
            stage_animations = animations;
            stage
        }
        GrKind::Yorster => {
            let (stage, animations) = crate::scene_stage::yorster::initialize(assets, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Onett => {
            let (stage, animations) =
                crate::scene_stage::onett::initialize(assets, rng, particles, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Zebes => {
            let (stage, animations) = crate::scene_stage::zebes::initialize(assets, rng, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Venom => {
            let (stage, animations) = crate::scene_stage::venom::initialize(assets, rng, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::MuteCity => {
            let (stage, animations) =
                crate::scene_stage::mutecity::initialize(assets, rng, particles, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Pura => {
            let (stage, animations) =
                crate::scene_stage::pura::initialize(assets, rng, particles, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::GreatBay => {
            let (stage, animations) =
                crate::scene_stage::greatbay::initialize(assets, items_on, rng, particles, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Inishie2 => {
            let (stage, animations) = crate::scene_stage::inishie2::initialize(assets, rng, map)?;
            stage_animations = animations;
            stage
        }
        GrKind::Inishie1 => {
            let (stage, animations) =
                crate::scene_stage::inishie1::initialize(assets, map, items_on)?;
            stage_animations = animations;
            stage
        }
        GrKind::PStadium => {
            let parameters = melee_gr::desc::read_stadium_parameters(&assets.stage)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let stage = melee_gr::stadium::Stadium::initialize(parameters, rng);
            stage_animations = crate::scene_stage::stadium::load_models(assets)?;
            SceneStage::Stadium(Box::new(stage))
        }
        _ => unreachable!(),
    };
    Ok((stage, stage_animations))
}

/// fn_8016E2BC: populate Player slots and create fighters in port order.
/// player_standings_inline: PlayerInitData x12 = 300 for Sudden Death.
const SUDDEN_DEATH_DAMAGE: f32 = 300.0;

/// Within this distance a player faces away from its chosen opponent's
/// facing rather than toward its marker (fn_8016DEEC's +-5.0).
const FACE_DEADBAND: f32 = 5.0;

/// fn_8016DEEC (gm_16AE.c:1762): each player, in slot order, faces the
/// opponent it picks by marker X. The pick keeps the largest distance, but
/// retail's first candidate takes the distance register's stale value (f26,
/// 0x8016E054 after the `r23 == -1` branch skips 0x8016E02C-0x8016E044) and
/// only a later candidate strictly farther than that value replaces it. The
/// register carries over from player to player; on entry it is the caller's,
/// which the three- and four-player boundaries show lies below 38.8 (0.0
/// here). Within +-5 of the pick the player faces away from the pick's
/// facing (+1 while the pick has none yet). A team battle never picks a
/// teammate (0x8016DFFC: is_teams, then Player_GetTeam).
fn initial_facings(spawn_x: &[f32], teams: &[Option<u8>]) -> Vec<f32> {
    let mut facing = vec![0.0f32; spawn_x.len()];
    let mut distance = 0.0f32; // f26
    for i in 0..spawn_x.len() {
        let mut pick: Option<usize> = None;
        let mut farthest = 0.0f32; // f25
        let opponent = |j: usize| j != i && (teams[i].is_none() || teams[i] != teams[j]);
        for j in (0..spawn_x.len()).filter(|&j| opponent(j)) {
            if pick.is_some() {
                // 0x8016E034: fsubs, then fneg below zero.
                let dx = spawn_x[i] - spawn_x[j];
                distance = if dx < 0.0 { -dx } else { dx };
                // 0x8016E048: fcmpo, then `bge` (also taken when unordered).
                if farthest.partial_cmp(&distance) != Some(std::cmp::Ordering::Less) {
                    continue;
                }
            }
            pick = Some(j);
            farthest = distance;
        }
        facing[i] = match pick {
            None => 1.0,
            Some(j) => {
                let dx = spawn_x[j] - spawn_x[i];
                if dx < -FACE_DEADBAND {
                    -1.0
                } else if dx > FACE_DEADBAND {
                    1.0
                } else if facing[j] == 1.0 {
                    -1.0
                } else {
                    1.0
                }
            }
        };
    }
    facing
}

fn create_players(
    scenario: &Setup,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
    stage_animations: &mut BTreeMap<u8, BackgroundAnimation>,
    rng: &mut HsdRng,
) -> Result<(Vec<SceneFighter>, SpawnCounter)> {
    let mut positions: Vec<_> = scenario
        .fighters
        .iter()
        .map(|fighter| {
            let marker = if fighter.spawn_point == -1 {
                i16::from(fighter.slot)
            } else {
                i16::from(fighter.spawn_point)
            };
            live_spawn_position(assets, stage_animations, marker)
                .map_or_else(|| spawn_position(assets, marker), Ok)
        })
        .collect::<Result<_>>()?;
    let spawn_x: Vec<f32> = positions.iter().map(|p| p.x).collect();
    let teams: Vec<Option<u8>> = scenario.fighters.iter().map(|f| f.team).collect();
    let mut facing = initial_facings(&spawn_x, &teams);
    // fn_8016D8AC (0x8016DA3C): the delay grows by five for every Player slot
    // up to this one, entering or not (ports 2 and 4 enter at 10 and 20).
    let mut entry_delays: Vec<i32> = scenario
        .fighters
        .iter()
        .map(|fighter| ENTRY_STAGGER_FRAMES * (i32::from(fighter.slot) + 1))
        .collect();
    let team_order = teams
        .iter()
        .copied()
        .collect::<Option<Vec<u8>>>()
        .map(|teams| crate::slippi::teams_order(&teams));
    if let (crate::slippi::SpawnRule::NeutralTable(table), Some(order)) =
        (scenario.slippi.spawn, &team_order)
    {
        // A team battle: NeutralSpawn's teams rows by team order, when every
        // team has none or two players (0x8016E5C4); else retail's spawns.
        if let Some(order) = order {
            for (player, &row) in order.iter().enumerate() {
                let (position, face) = crate::slippi::neutral_spawn_teams(
                    assets.stage_descriptor.music_id,
                    row,
                )
                .context("NeutralSpawn: no teams row for this stage")?;
                positions[player] = position;
                facing[player] = face;
                if let Some(delay) = table.entry_delay(row) {
                    entry_delays[player] = delay;
                }
            }
        }
    } else if let crate::slippi::SpawnRule::NeutralTable(table) = scenario.slippi.spawn {
        // Slippi's NeutralSpawn (0x8016E510) then overwrites each Player's
        // initial coordinates and facing by spawn order (players ascend by
        // slot); one version also overwrites the entry delay.
        for (order, _) in scenario.fighters.iter().enumerate() {
            let (position, face) =
                crate::slippi::neutral_spawn(table, assets.stage_descriptor.music_id, order)
                    .context("NeutralSpawn: no row for this stage")?;
            positions[order] = position;
            facing[order] = face;
            if let Some(delay) = table.entry_delay(order) {
                entry_delays[order] = delay;
            }
        }
    }
    let mut counter = SpawnCounter(1);
    let mut fighters = Vec::new();
    // Player_80031AD0 per slot: the player's fighter, then its partner, which
    // reads the same Player position, facing and entry delay.
    for (index, entry) in scenario.roster().into_iter().enumerate() {
        let p = entry.player;
        let player = PlayerSlot {
            id: scenario.fighters[p].slot,
            secondary: entry.secondary,
            control: if scenario.fighters[p].cpu_level.is_some() {
                PlayerKind::Cpu
            } else {
                PlayerKind::Human
            },
            costume: scenario.fighters[p].costume,
            stocks: scenario.fighters[p].stocks,
            falls: 0,
            position: positions[p],
            facing: facing[p],
            scale: scenario.model_scale(),
            damage: if scenario.sudden_death {
                SUDDEN_DEATH_DAMAGE
            } else {
                scenario.special.map_or(0.0, crate::SpecialMelee::starting_damage)
            },
            cpu_mode: 4,
            // A human retains the CSS default level 1.
            cpu_level: i32::from(scenario.fighters[p].cpu_level.unwrap_or(1)),
            team: scenario.fighters[p].team,
            friendly_fire: scenario.friendly_fire,
            match_kind: scenario.mode,
            self_destructs: 0,
            attack_ratio: scenario.ratios(p).0,
            defense_ratio: scenario.ratios(p).1,
            knockback_ratio: scenario.damage_ratio,
            attack_ratios: {
                let mut ratios = [1.0; 6];
                for (i, f) in scenario.fighters.iter().enumerate() {
                    ratios[usize::from(f.slot)] = scenario.ratios(i).0;
                }
                ratios
            },
            // gmstamina.c:118: 150 hit points (Player_SetOtherStamina).
            stamina: (scenario.special == Some(crate::SpecialMelee::Stamina)).then_some(150),
            // gmsinglebutton.c:141 (rules.single_button) and gminvisible.c
            // (PlayerInitData vs_invisible -> Player_SetFlagsBit6).
            single_button: scenario.special == Some(crate::SpecialMelee::SingleButton),
            invisible: scenario.special == Some(crate::SpecialMelee::Invisible),
            revival_marker: None,
            revival_damage: if scenario.sudden_death {
                SUDDEN_DEATH_DAMAGE
            } else {
                scenario.special.map_or(0.0, crate::SpecialMelee::starting_damage)
            },
        };
        let context = SpawnContext {
            map: &mut *map,
            stage_camera: &assets.stage_camera,
            rng: &mut *rng,
            counter: &mut counter,
        };
        let transformation_partner =
            entry.secondary && crate::scene_fighter::transforms(entry.descriptor.kind);
        if transformation_partner {
            // A transforming character's other form sleeps (ftCo_800BFD04).
            fighters.push(SceneFighter::partner_from_parameters(
                &assets.characters[index],
                &assets.fighters[index],
                player,
                context,
            )?);
            continue;
        }
        fighters.push(SceneFighter::from_parameters(
            &assets.characters[index],
            &assets.fighters[index],
            player,
            entry_delays[p],
            context,
        )?);
        if entry.secondary {
            // ftCo_800A101C's FTKIND_NANA loop reads the player's own
            // fighter, created just before (ftCo_800A589C).
            let own = &fighters[index - 1].0;
            let (position, facing) = (own.physics.position, own.physics.facing);
            fighters[index].0.cpu.seed_follow_ring(position, facing);
        }
    }
    Ok((fighters, counter))
}
