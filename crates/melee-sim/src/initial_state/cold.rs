//! Versus setup from scene parameters and the pre-music boundary seed.
//! Captures are comparison inputs in cold_tests.rs only.
use super::InitialState;
use crate::{
    assets::Assets, scenario::Scenario, scene_fighter::SceneFighter, scene_stage::SceneStage,
};
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
    pub fn from_parameters(scenario: &Scenario) -> Result<Self> {
        scenario.validate()?;
        ensure!(
            scenario.is_cold(),
            "cold construction requires savestate omitted"
        );
        let assets = Assets::load(
            &scenario.assets_path(),
            std::array::from_fn(|p| scenario.fighters[p].descriptor()),
            scenario.stage_descriptor(),
        )?;
        let mut map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        // The supplied seed is after creation, before music. grLast's four
        // draws (Battle/Story: one; Dream Land: two), then two CPU draws per slot.
        // Invert this fixed, audited interval; never search an oracle at runtime.
        let stage_draws = match assets.stage_desc.kind {
            GrKind::Last => 4,
            GrKind::Battle | GrKind::Story => 1,
            GrKind::OldPupupu => 2,
            _ => anyhow::bail!("cold setup supports FD, Battlefield, Yoshi's Story and Dream Land"),
        };
        let boundary_seed = scenario.seed.expect("validated cold seed");
        let fighter_draws = CPU_SETUP_DRAWS_PER_PLAYER * scenario.fighters.len();
        let mut rng = HsdRng::new(before_setup(boundary_seed, stage_draws + fighter_draws));
        let mut particles = ParticleSystem::default();
        let (stage, mut stage_animations) = initialize_stage(&assets, &mut rng, &mut particles)?;
        for (&id, animation) in &mut stage_animations {
            let bindings = &assets.stage_desc.models[id as usize].joint_mappings;
            animation.update_collision(&mut map, bindings);
            for binding in bindings {
                map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
            }
        }
        let fighters = create_players(scenario, &assets, &mut map, &mut rng)?;
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
        Ok(Self {
            countdown: Some(crate::countdown::Countdown::load(&scenario.assets_path())?),
            assets,
            map,
            stage,
            particles,
            stage_animations,
            fighters,
            rng,
            pending_music,
            selected_music: None,
            resume_s_link: 24,
            // A cold match has no emission interrupted by a save boundary.
            pending_emission: None,
            effects: crate::effects::Effects::default(),
        })
    }
}

/// Inverse of the odd HSD LCG multiplier, modulo 2^32.
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
) -> Result<(SceneStage, BTreeMap<u8, BackgroundAnimation>)> {
    let mut stage_animations = BTreeMap::new();
    let stage = match assets.stage_desc.kind {
        GrKind::Last => {
            let mut stage = FinalDestination::initialize(&assets.stage_desc, rng);
            stage.actions.clear();
            // grLast_8021B920 requests this animation without evaluating
            // its first DPtcl key; Ground's first scheduler pass does that.
            stage_animations.insert(
                4,
                BackgroundAnimation::load(&assets.stage, &assets.stage_desc)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            );
            // grLast_804D4968: collision joint 0 belongs to map 3, root bone; the
            // scene applies its transform through the same animation table.
            stage_animations.insert(
                3,
                BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[3])
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            );
            SceneStage::FinalDestination(Box::new(stage))
        }
        GrKind::Battle => {
            let mut stage = Battlefield::initialize(rng);
            stage.lights = melee_gr::battle::lights::load(&assets.stage, &assets.stage_desc)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            // grBattle_OnInit creates 0,3,1,6. Map 3 has no animated
            // model; grAnime_801C8138 immediately evaluates the others.
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
        _ => unreachable!(),
    };
    Ok((stage, stage_animations))
}

/// fn_8016E2BC: populate Player slots and create fighters in port order.
fn create_players(
    scenario: &Scenario,
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
            damage: 0.0,
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
                rng: &mut *rng,
                counter: &mut counter,
            },
        )?);
    }
    Ok(fighters.try_into().ok().expect("two players"))
}
