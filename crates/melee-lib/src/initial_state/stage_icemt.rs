//! Icicle Mountain at a saved boundary: maps 0, 8 and 9 and the two field
//! segments map 9 made, before the countdown releases the scroll. Their
//! construction is deterministic once the controller's draws are known,
//! so it is rebuilt from map 9's saved controller and checked against the
//! saved collision.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_animation, restore_vertices},
};
use crate::{
    assets::Assets,
    scene_stage::{
        icemt::{build, read_parameters, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::icemt::{
    procs::{BACKGROUND_MAP, BASE_MAP, CONTROLLER_MAP},
    Controller, Scroll,
};

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([raw[at], raw[at + 1]])
}

/// Map 9's `grIceMt_GObj9_GroundVars` (gp+0xC4): the segments, the
/// scroll's state, x14 (waiting), the speeds' and segments' cooldowns.
fn saved_controller(saved: &SavedPose) -> Result<Controller> {
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps.len() == 5
            && maps[..3] == [BASE_MAP, BACKGROUND_MAP, CONTROLLER_MAP].map(u32::from)
            && (1..=6).contains(&maps[3])
            && (1..=6).contains(&maps[4]),
        "Icicle Mountain map order {maps:?}"
    );
    let raw = saved.bytes(grounds[2].ground, 0x110);
    let controller = Controller {
        under: half(raw, 0xC4),
        upper: half(raw, 0xC6),
        scroll: Scroll {
            phase: half(raw, 0xC8),
            delay: half(raw, 0xCA),
            ease: half(raw, 0xCC),
            index: half(raw, 0xD0),
            speed: float(raw, 0xD4),
        },
        waiting: half(raw, 0xD8) != 0,
        speed_cooldowns: std::array::from_fn(|i| half(raw, 0xDC + 2 * i)),
        field_cooldowns: std::array::from_fn(|i| half(raw, 0xF4 + 2 * i)),
    };
    ensure!(
        controller.waiting && half(raw, 0xCE) == 0 && controller.scroll == Scroll::default(),
        "Icicle Mountain's scroll has started at the boundary"
    );
    ensure!(
        maps[3] == controller.under as u32 && maps[4] == controller.upper as u32,
        "Icicle Mountain's segments {maps:?} are not the controller's {controller:?}"
    );
    Ok(controller)
}

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
        "Icicle Mountain joint-attached particles need attachment restoration"
    );
    let controller = saved_controller(saved)?;
    let mut scratch = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let (stage, mut animations) = build(assets, read_parameters(assets)?, controller, &mut scratch)?;
    for ground in ground_list(saved)? {
        let joints = model_joints(saved, ground.jobj);
        let animation = animations.get_mut(&(ground.map as u8)).unwrap();
        restore_animation(saved, animation, &joints, ground.map)?;
    }
    Ok((stage, animations))
}

/// The collision as the stage's creation leaves it, rebuilt, then the
/// saved vertices checked.
pub(super) fn restore_collision(saved: &SavedPose, assets: &Assets, map: &mut melee_mp::CollMap) -> Result<()> {
    let controller = saved_controller(saved)?;
    build(assets, read_parameters(assets)?, controller, map)?;
    restore_vertices(saved, map, "a field segment")
}
