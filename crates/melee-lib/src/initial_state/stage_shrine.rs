//! Hyrule Temple at a saved boundary: the three maps in creation order and
//! map 1's looping animation clock. The stage keeps no controller state.
use super::{
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_animation},
};
use crate::{
    assets::Assets,
    scene_stage::{
        shrine::{controller, load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::shrine::procs::MAP_ORDER;

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
        "Hyrule Temple joint-attached particles need attachment restoration"
    );
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Hyrule Temple map order {maps:?}"
    );
    let mut animations = Animations::new();
    for ground in &grounds {
        let mut animation = load(assets, ground.map as u8)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        animations.insert(ground.map as u8, animation);
    }
    Ok((SceneStage::Shrine(controller(assets)?), animations))
}
