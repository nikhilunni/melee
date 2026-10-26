//! Mute City at a saved boundary: the stage as its creation leaves it
//! (only a boundary from before the route's first tick is imported). The
//! creation is rebuilt and checked against the saved Grounds, clocks and
//! car tables.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_vertices, JOBJ_AOBJ},
    word,
};
use crate::{assets::Assets, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use melee_gr::mutecity::procs::MAP_ORDER;

type Animations = crate::scene_stage::mutecity::Animations;

/// grMc_8049F440 (the pack order), the word before it, and grMc_8049F4B8
/// (the cars, 0x2C bytes each).
const ORDER: u32 = 0x8049_F440;
const CARS: u32 = 0x8049_F4B8;
const CAR_SIZE: u32 = 0x2C;

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(maps == MAP_ORDER.map(u32::from), "Mute City map order {maps:?}");
    if std::env::var_os("MUTE_CITY_DEBUG").is_some() {
        for ground in grounds.iter().filter(|g| g.map != 2) {
            let joints = model_joints(saved, ground.jobj);
            for (index, &joint) in joints.iter().enumerate() {
                let aobj = word(saved.bytes(joint + JOBJ_AOBJ, 4), 0);
                let detail = if aobj != 0 {
                    let raw = saved.bytes(aobj, 0x18);
                    format!(
                        "flags {:#x} frame {} rewind {} end {} rate {} fobj {:#x}",
                        word(raw, 0),
                        float(raw, 4),
                        float(raw, 8),
                        float(raw, 12),
                        float(raw, 16),
                        word(raw, 0x14)
                    )
                } else {
                    String::new()
                };
                eprintln!("map {} joint {index} at {joint:#x} aobj {aobj:#x} {detail}", ground.map);
            }
        }
        eprintln!("low RAM {:02x?}", saved.bytes(0x8000_0000, 0x10));
    }
    // The creation, rebuilt from the assets.
    let mut rng = HsdRng::new(0);
    let mut scratch_particles = ParticleSystem::default();
    let mut collision = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let (stage, animations) = crate::scene_stage::mutecity::initialize(
        assets,
        &mut rng,
        &mut scratch_particles,
        &mut collision,
    )?;
    ensure!(
        rng.seed == 0 && scratch_particles.generators.len() == particles.generators.len(),
        "Mute City's creation drew or spawned what the boundary does not hold"
    );
    let SceneStage::MuteCity(mut stage) = stage else {
        unreachable!()
    };
    // The pack's word before the order table is Brinstar's memory.
    let before = word(saved.bytes(ORDER - 4, 4), 0) as i32;
    stage.cars.order[0] = before;
    for (index, car) in stage.cars.cars.iter().enumerate() {
        let raw = saved.bytes(CARS + index as u32 * CAR_SIZE, CAR_SIZE as usize);
        let place = [float(raw, 0), float(raw, 4), float(raw, 8), float(raw, 0xC), float(raw, 0x10)];
        let ours = [car.distance, car.place, car.speed, car.lane, car.damage];
        ensure!(
            place.map(f32::to_bits) == ours.map(f32::to_bits)
                && raw[0x22] & 0xC0 == 0
                && word(raw, 0x24) == 0
                && word(raw, 0x28) == 0,
            "Mute City car {index} at the boundary {place:?} is not its grid place"
        );
        let saved_position = [float(raw, 0x14), float(raw, 0x18), float(raw, 0x1C)];
        let position = [car.position.x, car.position.y, car.position.z];
        ensure!(
            saved_position.map(f32::to_bits) == position.map(f32::to_bits),
            "Mute City car {index} placed at {position:?}, retail {saved_position:?}"
        );
        let order = word(saved.bytes(ORDER + index as u32 * 4, 4), 0);
        ensure!(order == index as u32, "Mute City pack order {index}: {order}");
    }
    Ok((SceneStage::MuteCity(stage), animations))
}

/// The platform's collision joints as Ground_801C2ED0 bound them, from the
/// model's pose before its animation (the saved JObj matrices are the
/// animated ones), joint 4's flag and callback, then the saved vertices
/// checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    crate::scene_stage::mutecity::bind_creation_collision(assets, map)?;
    restore_vertices(saved, map, "the platform")
}
