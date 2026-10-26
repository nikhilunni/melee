//! Big Blue at a saved boundary: the stage as its creation leaves it (only
//! a boundary from before the procs' first tick is imported). The creation
//! is rebuilt, the Falcon Flyer at its saved height (grBigBlue_801E8D64
//! searches under an uninitialised vector), and the cars' drawn tables
//! (ranks, lanes, arrival timer) and the flyers' uninitialised words are
//! taken from the saved Grounds; the rest is checked against them.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{ground_list, restore_vertices, SavedGround, JOBJ_TRANSLATE},
    word,
};
use crate::{assets::Assets, scene_stage::SceneStage};
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use hsd_types::Vec3;
use melee_gr::bigblue::{
    cars::{Lane, Manoeuvre, SLOTS},
    procs::{CARS_MAP, FALCON_MAP, FLYERS_MAP, MAP_ORDER, ROAD_MAP},
};

type Animations = crate::scene_stage::bigblue::Animations;

/// grBigBlue_CarVars: the ranks pointer, the arrival timer, the lanes.
const CAR_RANKS: u32 = 0xCC;
const CAR_TIMER: u32 = 0xD0;
const CAR_LANES: u32 = 0xD4;
const LANE_SIZE: u32 = 0x40;
/// grBigBlue_ManagerVars: the flyers (0x54 bytes each).
const FLYER_DATA: u32 = 0xE4;
const FLYER_SIZE: u32 = 0x54;

fn ground(grounds: &[SavedGround], map: u8) -> Result<&SavedGround> {
    grounds
        .iter()
        .find(|g| g.map == u32::from(map))
        .with_context(|| format!("Big Blue's map {map:#x} Ground"))
}

/// The Falcon Flyer's height at the boundary (its GObj JObj's y).
pub(super) fn falcon_y(saved: &SavedPose) -> Result<f32> {
    let grounds = ground_list(saved)?;
    let falcon = ground(&grounds, FALCON_MAP)?;
    Ok(float(saved.bytes(falcon.jobj + JOBJ_TRANSLATE + 4, 4), 0))
}

fn lane(raw: &[u8]) -> Result<Lane> {
    let status = u16::from_be_bytes([raw[0], raw[1]]);
    Ok(Lane {
        manoeuvre: Manoeuvre::from_state(raw[0] >> 2),
        airborne: raw[0] & 2 != 0,
        slot: usize::from((status >> 4) & 0x1F),
        place: float(raw, 0x04),
        offset: float(raw, 0x08),
        position: Vec3::new(float(raw, 0x0C), float(raw, 0x10), float(raw, 0x14)),
        alpha: float(raw, 0x18),
        wait: word(raw, 0x1C) as i32,
        sink_speed: float(raw, 0x20),
        sink: float(raw, 0x24),
        velocity: float(raw, 0x28),
        acceleration: float(raw, 0x2C),
        phase: float(raw, 0x30),
        bob: float(raw, 0x34),
        vertical_speed: float(raw, 0x38),
    })
}

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(maps == MAP_ORDER.map(u32::from), "Big Blue map order {maps:?}");
    let mut rng = HsdRng::new(0);
    let mut scratch_particles = ParticleSystem::default();
    let mut collision = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let (stage, mut animations) = crate::scene_stage::bigblue::initialize(
        assets,
        &mut rng,
        &mut scratch_particles,
        &mut collision,
    )?;
    ensure!(
        scratch_particles.generators.len() == particles.generators.len(),
        "Big Blue's creation spawned what the boundary does not hold"
    );
    let SceneStage::BigBlue(mut stage) = stage else {
        unreachable!()
    };
    // The cars as the creation drew them.
    let cars = ground(&grounds, CARS_MAP)?;
    let ranks = word(saved.bytes(cars.ground + CAR_RANKS, 4), 0);
    stage.cars.ranks.copy_from_slice(saved.bytes(ranks, SLOTS));
    let timer = saved.bytes(cars.ground + CAR_TIMER, 2);
    stage.cars.arrival_timer = i16::from_be_bytes([timer[0], timer[1]]);
    for (index, ours) in stage.cars.lanes.iter_mut().enumerate() {
        let at = cars.ground + CAR_LANES + index as u32 * LANE_SIZE;
        *ours = lane(saved.bytes(at, LANE_SIZE as usize))?;
    }
    let cars_model = animations.get_mut(&CARS_MAP).unwrap();
    for slot in 0..SLOTS {
        cars_model.set_joint_hidden(slot + 1, true);
    }
    for lane in stage.cars.lanes.iter().filter(|l| l.manoeuvre != Manoeuvre::Off) {
        cars_model.set_joint_hidden(lane.slot + 1, false);
        cars_model.set_joint_translate(lane.slot + 1, lane.position);
    }
    ensure!(
        stage.cars.ranks.iter().filter(|&&r| r == 1).count()
            == stage.cars.lanes.iter().filter(|l| l.manoeuvre != Manoeuvre::Off).count(),
        "Big Blue's saved car ranks disagree with its lanes"
    );
    // The road: its first two segments, and `b0` until the countdown ends.
    let road = ground(&grounds, ROAD_MAP)?;
    let bits = word(saved.bytes(road.ground + 0xC4, 4), 0);
    let current = (bits >> 15) & 0x7F;
    let next = (bits >> 8) & 0x7F;
    stage.road.starting = bits >> 31 != 0;
    ensure!(
        current == u32::from(stage.road.current)
            && next == u32::from(stage.road.next),
        "Big Blue's road at the boundary {bits:#010x} is not its creation's"
    );
    // The flyers' words their on_init leaves (stale heap) and the calls.
    let flyers = ground(&grounds, FLYERS_MAP)?;
    let calls = saved.bytes(flyers.ground + 0xC4, 0x10);
    ensure!(
        word(calls, 0) == 0 && word(calls, 4) == 0 && word(calls, 8) != 0 && word(calls, 0xC) == 0,
        "Big Blue's flyer calls at the boundary {calls:02x?} are not the creation's"
    );
    for (index, flyer) in stage.flyers.flyers.iter_mut().enumerate() {
        let raw = saved.bytes(flyers.ground + FLYER_DATA + index as u32 * FLYER_SIZE, FLYER_SIZE as usize);
        ensure!(raw[0] as usize == index && raw[1] == 0, "Big Blue flyer {index} at the boundary");
        flyer.weight_left = float(raw, 0x24);
        flyer.weight_right = float(raw, 0x28);
        flyer.previous_riders = word(raw, 0x30) as i32;
    }
    Ok((SceneStage::BigBlue(stage), animations))
}

/// The collision as the creation bound it (Ground_801C2ED0 on every map
/// with joints, from their poses before any tick), checked against the
/// saved vertices.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let mut rng = HsdRng::new(0);
    let mut particles = ParticleSystem::default();
    let (stage, mut animations) =
        crate::scene_stage::bigblue::initialize(assets, &mut rng, &mut particles, map)?;
    let _ = stage;
    let y = animations.get_mut(&FALCON_MAP).unwrap().gobj_translation().y;
    let saved_y = falcon_y(saved)?;
    ensure!(
        y.to_bits() == saved_y.to_bits(),
        "the Falcon Flyer at the boundary stands at {saved_y}, the origin's floor gives {y}"
    );
    restore_vertices(saved, map, "the road, the cars or the flyers")
}
