//! Kongo Jungle N64 at a saved boundary: the barrel (`gp->u.taru`), the
//! bird, their models, the cannon registry (stage_info x730/x73C), the
//! catch device, and the platforms' collision joints.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS,
        JOBJ_HIDDEN,
    },
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        oldkongo::{load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use hsd_types::Vec3;
use melee_gr::oldkongo::{
    procs::{BARREL_MAP, BIRD_MAP, COLLISION_BINDINGS, MAP_ORDER, PLATFORM_MAP},
    Barrel, Bird, Catch, Drift, OldKongo, Turn,
};

/// stage_info (0x8049E6C8) +0x730: Ground_801C4D70's position, +0x73C its aim.
const CANNON_POSITION: u32 = 0x8049_E6C8 + 0x730;
const CANNON_AIM: u32 = 0x8049_E6C8 + 0x73C;
/// ftDevice_BuryThingCount: catch devices registered (grOldKongo_8020F4E8).
const CATCH_DEVICES: u32 = 0x804D_6574;

fn saved_barrel(raw: &[u8]) -> Result<Barrel> {
    let half = |at: usize| i16::from_be_bytes([raw[at], raw[at + 1]]);
    let bad = |what: &str, value: i16| anyhow::anyhow!("Kongo Jungle N64 barrel {what} {value}");
    let barrel = Barrel {
        turn: Turn::from_saved(half(0xC4)).ok_or_else(|| bad("turn", half(0xC4)))?,
        catch: Catch::from_saved(half(0xC6)).ok_or_else(|| bad("catch", half(0xC6)))?,
        drift: Drift::from_saved(half(0xC8)).ok_or_else(|| bad("drift", half(0xC8)))?,
        shot_delay: half(0xCA),
        turn_timer: half(0xCC),
        drift_timer: half(0xCE),
        windup: half(0xD0),
        held: None,
        target_aim: float(raw, 0xD8),
        aim: float(raw, 0xDC),
        spin_acceleration: float(raw, 0xE0),
        spin: float(raw, 0xE4),
        speed: float(raw, 0xEC),
    };
    ensure!(
        barrel.catch == Catch::Empty && word(raw, 0xD4) == 0,
        "Kongo Jungle N64 barrel holding a fighter at the boundary is not imported"
    );
    Ok(barrel)
}

/// The stage controller and every model at the saved boundary.
pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    ensure!(
        particles
            .generators
            .iter()
            .all(|g| g.attachment_id.is_none()),
        "Kongo Jungle N64 joint-attached particles need attachment restoration"
    );
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    let parameters = melee_gr::desc::read_old_kongo_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Kongo Jungle N64 map order {maps:?}"
    );
    // The creation draws are already in the saved seed; the saved state
    // replaces what they chose.
    let mut stage = OldKongo::create(parameters, &mut HsdRng::new(0));
    let mut animations = Animations::new();
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x108);
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        match key {
            BARREL_MAP => {
                stage.barrel = saved_barrel(raw)?;
                // The rate grAnime_801C7A04 set and the cannon's aim.
                animation.set_rate(stage.barrel.speed);
                animation.set_joint_rotation_z(1, stage.barrel.aim);
            }
            BIRD_MAP => {
                stage.bird = Bird {
                    timer: word(raw, 0xC4) as i32,
                };
                ensure!(
                    stage.bird.timer > 0,
                    "Kongo Jungle N64 bird flying at the boundary is not imported"
                );
                let hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
                animation.set_hidden(hidden);
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    let position = saved.bytes(CANNON_POSITION, 12);
    stage.cannon = Vec3::new(float(position, 0), float(position, 4), float(position, 8));
    stage.cannon_aim = float(saved.bytes(CANNON_AIM, 4), 0);
    stage.catch_enabled = word(saved.bytes(CATCH_DEVICES, 4), 0) != 0;
    Ok((SceneStage::OldKongo(stage), animations))
}

/// The platforms' joints as the last Ground_801C2FE0 left them, from the
/// saved JObj matrices. Runs before fighters read the map.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    let platforms = grounds
        .iter()
        .find(|g| g.map == u32::from(PLATFORM_MAP))
        .ok_or_else(|| anyhow::anyhow!("Kongo Jungle N64 map 3 missing"))?;
    let joints = model_joints(saved, platforms.jobj);
    for binding in &COLLISION_BINDINGS {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    restore_vertices(saved, map, "a moving platform")
}
