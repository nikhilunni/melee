//! Rainbow Cruise at a saved boundary. Only a boundary from before the
//! stage's first update is imported: grRCruise_801FF168 draws nothing, so
//! the scene is creation's, checked against the saved clocks, matrices and
//! vertices.
use super::{
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_vertices, saved_mtx, JOBJ_AOBJ, JOBJ_FLAGS, JOBJ_HIDDEN},
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        rcruise::{initialize, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use melee_gr::rcruise::procs::{FLAG_MAP, KEY_ORDER, SHIP_MAP, SWING_MAP};

/// The saved Ground list in creation order, map 3 twice (the path, then
/// the scroll controller).
fn check_maps(saved: &SavedPose) -> Result<Vec<super::stage_izumi::SavedGround>> {
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    let expected: Vec<u32> = KEY_ORDER
        .iter()
        .map(|&k| u32::from(melee_gr::rcruise::procs::model(k)))
        .collect();
    ensure!(maps == expected, "Rainbow Cruise maps {maps:?}");
    Ok(grounds)
}

fn scratch_initialize(
    assets: &Assets,
    particles: &ParticleSystem,
    map: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    // Creation draws nothing and its particles are already in the saved
    // system: both are scratch.
    let mut rng = HsdRng::new(0);
    let mut scratch_particles = particles.clone();
    initialize(assets, &mut rng, &mut scratch_particles, map)
}

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    ensure!(
        particles.generators.iter().all(|g| g.attachment_id.is_none()),
        "Rainbow Cruise joint-attached particles need attachment restoration"
    );
    let grounds = check_maps(saved)?;
    for (ground, key) in grounds.iter().zip(KEY_ORDER) {
        for jobj in model_joints(saved, ground.jobj) {
            let aobj = word(saved.bytes(jobj + JOBJ_AOBJ, 4), 0);
            if aobj != 0 {
                let raw = saved.bytes(aobj, 8);
                ensure!(
                    word(raw, 4) == 0,
                    "Rainbow Cruise key {key} has run (aobj flags {:#x}, frame {}): only a boundary before the stage's first update is imported",
                    word(raw, 0),
                    super::float(raw, 4)
                );
            }
        }
    }
    let mut scratch = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    scratch_initialize(assets, particles, &mut scratch)
}

/// Creation's joints, flags and lists on the real map, then the moving
/// maps' joints from the saved matrices and the saved vertices checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = check_maps(saved)?;
    scratch_initialize(assets, &ParticleSystem::default(), map)?;
    for (ground, key) in grounds.iter().zip(KEY_ORDER) {
        if ![SHIP_MAP, FLAG_MAP, SWING_MAP].contains(&key) {
            continue;
        }
        let joints = model_joints(saved, ground.jobj);
        let archive = &assets.stage_desc.models[usize::from(key)].joint_mappings;
        let ship = melee_gr::rcruise::procs::ship_bindings(archive);
        let bindings = if key == SHIP_MAP { &ship } else { archive };
        for binding in bindings {
            let jobj = joints[binding.extra as usize];
            let state = melee_mp::JobjState {
                mtx: saved_mtx(saved, jobj),
                hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
            };
            map.update_joint_transform(i32::from(binding.joint_index), Some(state));
        }
    }
    restore_vertices(saved, map, "a ship part")
}
