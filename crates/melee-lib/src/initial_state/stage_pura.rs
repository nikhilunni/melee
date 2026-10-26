//! Poké Floats at a saved boundary: the stage as its creation leaves it
//! (only a boundary from before the sequence's first tick is imported).
//! The creation is rebuilt and checked against the saved Grounds; the
//! sky's colours, which the creation drew, are the saved ones.
use super::{
    saved_pose::SavedPose,
    stage_izumi::{ground_list, restore_vertices},
};
use crate::{assets::Assets, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use melee_gr::pura::procs::{model, OBJECT_ORDER, SKY_MAP};

type Animations = crate::scene_stage::pura::Animations;

/// grPura_GroundVars at gp+C4: the sky's colours and fade tick (s16 each).
const SKY_VARS: u32 = 0xC4;

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    let expected: Vec<u32> = OBJECT_ORDER.iter().map(|&key| u32::from(model(key))).collect();
    ensure!(maps == expected, "Poké Floats map order {maps:?}");
    let mut rng = HsdRng::new(0);
    let mut scratch_particles = ParticleSystem::default();
    let mut collision = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let (stage, animations) = crate::scene_stage::pura::initialize(
        assets,
        &mut rng,
        &mut scratch_particles,
        &mut collision,
    )?;
    ensure!(
        scratch_particles.generators.len() == particles.generators.len(),
        "Poké Floats' creation spawned what the boundary does not hold"
    );
    let SceneStage::Pura(mut stage) = stage else {
        unreachable!()
    };
    let sky = grounds
        .iter()
        .find(|g| g.map == u32::from(SKY_MAP))
        .expect("the sky's Ground");
    let raw = saved.bytes(sky.ground + SKY_VARS, 6);
    let half = |at: usize| i16::from_be_bytes([raw[at], raw[at + 1]]);
    stage.sky.from = half(0);
    stage.sky.to = half(2);
    stage.sky.tick = half(4);
    stage.sky.color = melee_gr::pura::SKY_COLORS[stage.sky.from.clamp(0, 3) as usize];
    ensure!(
        stage.sky.tick == 0 && (0..4).contains(&stage.sky.from) && (0..4).contains(&stage.sky.to),
        "Poké Floats' sky at the boundary {:?} is not its creation's",
        stage.sky
    );
    Ok((SceneStage::Pura(stage), animations))
}

/// The sequence's collision joints as Ground_801C2ED0 bound them, from the
/// model's pose before its animation, then the saved vertices checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let mut sequence = crate::scene_stage::pura::load(assets, melee_gr::pura::procs::SEQUENCE_MAP)?;
    crate::scene_stage::pura::bind_sequence(&mut sequence, assets, map);
    restore_vertices(saved, map, "the sequence")
}
