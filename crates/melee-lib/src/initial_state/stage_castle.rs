//! Princess Peach's Castle at a saved boundary: maps 0, 4, 3 and 6 as
//! creation leaves them (only a boundary from before the stage starts is
//! imported: no switch, no Bill, no moat yet), the platforms' state and
//! grCastle_801D0BBC's two statics, which outlive matches.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS, JOBJ_HIDDEN},
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        castle::{load, load_reserved, read_parameters, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::castle::{
    platforms::{Platform, BONES as PLATFORM_BONES},
    procs::{CastleStage, DISABLED_JOINTS, MAP_ORDER, PLATFORM_MAP},
    wind::Gusts,
};

/// grCs_804D45E0 (float) and grCs_804D45E4 (byte).
const GUST_TURN: u32 = 0x804D_45E0;
const GUST_STRONG: u32 = 0x804D_45E4;
/// Map 6: the platforms (`grCastle_Platform`, stride 0x10) from +0xC4.
const PLATFORMS: usize = 0xC4;
const PLATFORM_SIZE: usize = 0x10;

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([raw[at], raw[at + 1]])
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
        "Peach's Castle joint-attached particles need attachment restoration"
    );
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Peach's Castle maps {maps:?}: only a boundary before the stage starts is imported"
    );
    let mut stage = CastleStage::new(read_parameters(assets)?);
    let turn = float(saved.bytes(GUST_TURN, 4), 0);
    ensure!(
        turn == melee_gr::castle::wind::TURN || turn == -melee_gr::castle::wind::TURN,
        "grCs_804D45E0 holds {turn}"
    );
    stage.gusts = Gusts {
        turn,
        strong: saved.bytes(GUST_STRONG, 1)[0] != 0,
    };
    let mut animations = Animations::new();
    for ground in &grounds {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        if key == PLATFORM_MAP {
            let raw = saved.bytes(ground.ground, 0x130);
            for (index, platform) in stage.platforms.iter_mut().enumerate() {
                let at = PLATFORMS + index * PLATFORM_SIZE;
                *platform = Platform {
                    position: float(raw, at + 4),
                    state: half(raw, at + 8),
                    timer: half(raw, at + 0xA),
                    weight: float(raw, at + 0xC),
                };
                animation.set_joint_translate_y(PLATFORM_BONES[index], platform.position);
            }
        }
        animations.insert(key, animation);
    }
    load_reserved(assets, &mut animations)?;
    Ok((SceneStage::Castle(Box::new(stage)), animations))
}

/// Map 6's collision joints from the saved matrices, the block sets'
/// joints disabled (grCastle_801CD37C), then the saved vertices checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    let ground = grounds
        .iter()
        .find(|g| g.map == u32::from(PLATFORM_MAP))
        .ok_or_else(|| anyhow::anyhow!("Peach's Castle map 6 missing"))?;
    let joints = model_joints(saved, ground.jobj);
    for binding in &assets.stage_desc.models[usize::from(PLATFORM_MAP)].joint_mappings {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    for joint in DISABLED_JOINTS {
        map.joint_list_remove(joint);
    }
    restore_vertices(saved, map, "a platform")
}
