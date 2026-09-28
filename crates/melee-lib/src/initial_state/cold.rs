//! Versus setup from scene parameters and the pre-music boundary seed.
//! Captures are comparison inputs in cold_tests.rs only.
use super::InitialState;
use crate::{assets::Assets, scene_fighter::SceneFighter, scene_stage::SceneStage, setup::Setup};
use anyhow::{ensure, Result};
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

/// ftCo_800A101C and ftCo_800B9704, including human slots.
const CPU_SETUP_DRAWS_PER_PLAYER: usize = 2;
/// fn_8016D8AC increments Player's Entry delay by five per entering slot.
const ENTRY_STAGGER_FRAMES: i32 = 5;

impl InitialState {
    /// fn_8016E730 (gm_16AE.c): Ground creation, Player/Fighter creation,
    /// then the pre-music boundary. Only owned DAT resources are read.
    pub fn from_parameters(source: &impl crate::diagnostics::ScenarioSource) -> Result<Self> {
        ensure!(
            source.is_cold(),
            "cold construction requires savestate omitted"
        );
        let setup = source.setup()?;
        let assets = std::sync::Arc::new(Assets::load(
            &source.assets_path(),
            std::array::from_fn(|p| setup.fighters[p].descriptor()),
            setup.stage_descriptor(),
        )?);
        Self::from_assets(&setup, assets)
    }
    pub(crate) fn from_assets(scenario: &Setup, assets: std::sync::Arc<Assets>) -> Result<Self> {
        let mut map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        // The supplied seed is after creation, before music. grLast's four
        // draws (Battle/Story: one; Dream Land: two), then two CPU draws per slot.
        // Invert this fixed, audited interval; never search an oracle at runtime.
        let boundary_seed = scenario.seed.expect("validated cold seed");
        let mut rng = HsdRng::new(before_setup(
            boundary_seed,
            setup_draws(assets.stage_desc.kind, scenario.fighters.len())?,
        ));
        let mut particles = ParticleSystem::default();
        let (stage, mut stage_animations) =
            initialize_stage(&assets, &mut rng, &mut particles, &mut map)?;
        // Fountain of Dreams binds its collision joints during creation.
        let archive_bindings = !matches!(stage, SceneStage::Izumi(_));
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
        let mut fighters = create_players(scenario, &assets, &mut map, &mut rng)?;
        // gm_16AE: Camera_80030730 (the stage's field of view, see
        // StageCamera::fov), then Camera_8002F3AC snaps to the players.
        let mut camera = melee_cm::GameCamera::new();
        {
            let [a, b] = &mut fighters;
            let mut subjects = [&mut b.0.camera, &mut a.0.camera];
            camera.snap_standard(&mut subjects, &assets.stage_camera);
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
            scenario
                .all_characters_unlocked
                .expect("validated cold music rule"),
        ));
        let effects = Box::new(melee_ef::Effects::from_resources(&assets.effect_resources));
        Ok(Self {
            items: Box::new(melee_it::ItemPool::new(assets.items.common.clone())),
            stock_displays: super::stock::create(
                &assets.interface,
                std::array::from_fn(|slot| {
                    crate::scene_fighter::with_fighter!(&fighters[slot], |f| f.player.stocks)
                }),
            )?,
            spawn_counter: melee_ft::fighter::SpawnCounter(3),
            banner: Some(crate::banner::Banner::from_archive(
                &assets.interface,
                if scenario.sudden_death {
                    crate::banner::BannerKind::SuddenDeathCountdown
                } else {
                    crate::banner::BannerKind::Countdown
                },
            )?),
            go_banner: Some(crate::banner::Banner::preload(
                &assets.interface,
                crate::banner::BannerKind::Go,
            )?),
            clock: crate::match_clock::MatchClock {
                sudden_death: scenario.sudden_death,
                timer: scenario
                    .time_limit
                    .map(crate::match_clock::CountdownTimer::start),
                ..Default::default()
            },
            bomb_rain: Default::default(),
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
            quakes,
        })
    }
}

/// Inverse of the odd HSD LCG multiplier, modulo 2^32.
/// The setup's fixed RNG interval: grLast's four draws (Battle/Story/
/// Pokemon Stadium: one; Dream Land: two), then two CPU draws per slot.
fn setup_draws(stage: GrKind, players: usize) -> Result<usize> {
    let stage_draws = match stage {
        GrKind::Last => 4,
        GrKind::Battle | GrKind::Story | GrKind::PStadium => 1,
        GrKind::OldPupupu => 2,
        // grIzumi_801CC358's first step draws each platform's wait.
        GrKind::Izumi => 2,
        _ => anyhow::bail!(
            "cold setup supports FD, Battlefield, Yoshi's Story, Dream Land, Fountain of Dreams \
             and Pokemon Stadium"
        ),
    };
    Ok(stage_draws + CPU_SETUP_DRAWS_PER_PLAYER * players)
}

/// A scene built after another one ends: the RNG carries over unchanged
/// through the scenes between (the Sudden Death tie screen draws nothing),
/// so the new scene's boundary seed is the old seed after its setup draws.
pub(crate) fn boundary_seed_after(seed: u32, stage: GrKind, players: usize) -> Result<u32> {
    let mut rng = HsdRng::new(seed);
    for _ in 0..setup_draws(stage, players)? {
        rng.rand();
    }
    Ok(rng.seed)
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
        .find(|b| b.stage_position == slot)
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

/// Ground creation precedes every Fighter_Create call.
fn initialize_stage(
    assets: &Assets,
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
                crate::scene_stage::izumi::initialize(assets, rng, particles, map)?;
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

fn create_players(
    scenario: &Setup,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
    rng: &mut HsdRng,
) -> Result<[SceneFighter; 2]> {
    let positions: Vec<_> = scenario
        .fighters
        .iter()
        .map(|fighter| {
            let marker = if fighter.spawn_point == -1 {
                i16::from(fighter.slot)
            } else {
                i16::from(fighter.spawn_point)
            };
            spawn_position(assets, marker)
        })
        .collect::<Result<_>>()?;
    // fn_8016DEEC: face the other player; nearby/equal-X markers
    // resolve +1 for the first player, then -1 for the second.
    let facing = if positions[1].x - positions[0].x < -5.0 {
        [-1.0, 1.0]
    } else {
        [1.0, -1.0]
    };
    let mut counter = SpawnCounter(1);
    let mut fighters = Vec::new();
    for p in 0..2 {
        let player = PlayerSlot {
            id: scenario.fighters[p].slot,
            control: PlayerKind::Human,
            costume: scenario.fighters[p].costume,
            stocks: scenario.fighters[p].stocks,
            position: positions[p],
            facing: facing[p],
            scale: 1.0,
            damage: if scenario.sudden_death {
                SUDDEN_DEATH_DAMAGE
            } else {
                0.0
            },
            cpu_mode: 4,
            cpu_level: 1,
        };
        // fn_8016D8AC: increment by five for each entering Player slot.
        fighters.push(SceneFighter::from_parameters(
            &assets.characters[p],
            &assets.fighters[p],
            player,
            ENTRY_STAGGER_FRAMES * (p as i32 + 1),
            SpawnContext {
                map: &mut *map,
                stage_camera: &assets.stage_camera,
                rng: &mut *rng,
                counter: &mut counter,
            },
        )?);
    }
    Ok(fighters.try_into().ok().expect("two players"))
}
