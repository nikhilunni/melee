//! Mushroom Kingdom at a saved boundary: the two maps and their clocks, the
//! bricks and lifts (only as creation leaves them: every brick solid, the
//! lifts level and still) and the stage's collision joints.
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
        inishie1::{controller, load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::inishie1::{
    procs::{BRICKS, LIFT_JOINTS, MAP_ORDER, STAGE_BINDINGS, STAGE_MAP},
    BrickPhase,
};

/// `GroundVars_inishie1` in the Ground's `u` union.
const HATENA_WAITS: usize = 0xC6;
const ALL_HATENA_COOLDOWN: usize = 0xD8;
const BLOCKS: usize = 0xDC;
const LIFT_PHASE: usize = 0xEE;
const LIFT_OFFSET: usize = 0xF0;
const LIFT_SPEED: usize = 0xF4;
const LIFT_BASE: usize = 0xFC;
/// sizeof(grInishie1_Block).
const BLOCK_SIZE: u32 = 0x24;

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
        "Mushroom Kingdom joint-attached particles need attachment restoration"
    );
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Mushroom Kingdom map order {maps:?}"
    );
    let mut animations = Animations::new();
    for ground in &grounds {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        animations.insert(key, animation);
    }
    let stage_ground = grounds.iter().find(|g| g.map == u32::from(STAGE_MAP)).unwrap();
    let raw = saved.bytes(stage_ground.ground, 0x110);
    ensure!(
        half(raw, HATENA_WAITS) == 0
            && half(raw, HATENA_WAITS + 2) == 0
            && word(raw, ALL_HATENA_COOLDOWN) == 0,
        "Mushroom Kingdom `?` blocks at the boundary are not ported"
    );
    ensure!(
        half(raw, LIFT_PHASE) == 0 && float(raw, LIFT_OFFSET) == 0.0 && float(raw, LIFT_SPEED) == 0.0,
        "Mushroom Kingdom lifts in motion at the boundary need their state imported"
    );
    let mut stage = controller(assets, &animations[&STAGE_MAP])?;
    // xFC and each brick's x8: the translations creation read.
    stage.lifts.base_y = float(raw, LIFT_BASE);
    let blocks = word(raw, BLOCKS);
    for index in 0..19 {
        let block = saved.bytes(blocks + index * BLOCK_SIZE, BLOCK_SIZE as usize);
        ensure!(
            half(block, 0) == 0
                && BrickPhase::from_saved(half(block, 2)) == Some(BrickPhase::Solid)
                && half(block, 4) == 0,
            "Mushroom Kingdom brick {index} at the boundary needs its state imported"
        );
        stage.bricks[index as usize].base_y = float(block, 8);
    }
    Ok((SceneStage::Inishie1(Box::new(stage)), animations))
}

/// Map 3's collision joints from the saved matrices (the archive's and
/// grI1_803E48C8's), then the saved vertices checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    let stage = grounds
        .iter()
        .find(|g| g.map == u32::from(STAGE_MAP))
        .ok_or_else(|| anyhow::anyhow!("Mushroom Kingdom map 3 missing"))?;
    let joints = model_joints(saved, stage.jobj);
    let bindings = &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings;
    for binding in bindings.iter().chain(&STAGE_BINDINGS) {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    // mpJointSetCb2 on the bricks, mpJointSetCb1 on the lifts.
    for (joint, _) in BRICKS {
        map.record_ceiling_events(joint, true);
    }
    for joint in LIFT_JOINTS {
        map.record_joint_events(joint, true);
    }
    restore_vertices(saved, map, "a brick or lift")
}
