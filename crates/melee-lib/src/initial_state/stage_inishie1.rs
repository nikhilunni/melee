//! Mushroom Kingdom at a saved boundary: the two maps and their clocks, the
//! bricks and lifts (only as creation leaves them: every brick solid, the
//! lifts level and still), the `?` blocks and their Grounds, and the
//! stage's collision joints.
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
        inishie1::{controller, load, load_hatena_models, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::inishie1::{
    hatena::{Hatena, HatenaObject, HATENA_KEY_BASE, HATENA_MAP},
    procs::{BRICKS, LIFT_JOINTS, MAP_ORDER, STAGE_BINDINGS, STAGE_MAP},
    BrickPhase,
};

/// `GroundVars_inishie1` in the Ground's `u` union.
const HATENA_WAITS: usize = 0xC6;
const HATENA_LAST: usize = 0xCA;
const ALL_HATENA_COOLDOWN: usize = 0xD8;
const BLOCKS: usize = 0xDC;
const LIFT_PHASE: usize = 0xEE;
const LIFT_OFFSET: usize = 0xF0;
const LIFT_SPEED: usize = 0xF4;
const LIFT_BASE: usize = 0xFC;
/// sizeof(grInishie1_Block).
const BLOCK_SIZE: u32 = 0x24;
/// grInishie1_Block.jobj2 and .hatena_gobj.
const BLOCK_JOBJ: usize = 0x14;
const BLOCK_HATENA: usize = 0x18;
/// `grInishie1_GroundVars2` in a `?` Ground: its brick's JObj, the blink
/// and the steady flag.
const HATENA_JOBJ: usize = 0xC4;
const HATENA_FLICKER: usize = 0xC8;
const HATENA_STEADY: usize = 0xCC;

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
    // grInishie1_801FA90C's maps, then the `?` Grounds map 3 made.
    ensure!(
        maps.len() >= MAP_ORDER.len()
            && maps[..MAP_ORDER.len()] == MAP_ORDER.map(u32::from)
            && maps[MAP_ORDER.len()..].iter().all(|&m| m == u32::from(HATENA_MAP)),
        "Mushroom Kingdom map order {maps:?}"
    );
    let mut animations = Animations::new();
    for ground in &grounds[..MAP_ORDER.len()] {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        animations.insert(key, animation);
    }
    let stage_ground = grounds.iter().find(|g| g.map == u32::from(STAGE_MAP)).unwrap();
    let raw = saved.bytes(stage_ground.ground, 0x110);
    ensure!(
        half(raw, LIFT_PHASE) == 0 && float(raw, LIFT_OFFSET) == 0.0 && float(raw, LIFT_SPEED) == 0.0,
        "Mushroom Kingdom lifts in motion at the boundary need their state imported"
    );
    let mut stage = controller(assets, &animations[&STAGE_MAP])?;
    load_hatena_models(assets, &mut animations)?;
    stage.hatena_waits = [half(raw, HATENA_WAITS), half(raw, HATENA_WAITS + 2)];
    stage.hatena_last = [half(raw, HATENA_LAST), half(raw, HATENA_LAST + 2)];
    stage.all_hatena_cooldown = word(raw, ALL_HATENA_COOLDOWN) as i32;
    // xFC and each brick's x8: the translations creation read.
    stage.lifts.base_y = float(raw, LIFT_BASE);
    let blocks = word(raw, BLOCKS);
    let mut jobjs = [0u32; 19];
    let mut hatena_grounds = [0u32; 19];
    for index in 0..19 {
        let block = saved.bytes(blocks + index * BLOCK_SIZE, BLOCK_SIZE as usize);
        let status = half(block, 0);
        ensure!(
            (status == 0 || Hatena::from_status(status).is_some())
                && BrickPhase::from_saved(half(block, 2)) == Some(BrickPhase::Solid)
                && half(block, 4) == 0,
            "Mushroom Kingdom brick {index} at the boundary needs its state imported"
        );
        let brick = &mut stage.bricks[index as usize];
        brick.base_y = float(block, 8);
        brick.hatena = status;
        jobjs[index as usize] = word(block, BLOCK_JOBJ);
        hatena_grounds[index as usize] = word(block, BLOCK_HATENA);
        ensure!(
            (status != 0) == (hatena_grounds[index as usize] != 0),
            "Mushroom Kingdom brick {index}: `?` status {status} against its Ground"
        );
    }
    // The `?` Grounds in creation order (the scheduler's).
    for ground in &grounds[MAP_ORDER.len()..] {
        let raw = saved.bytes(ground.ground, 0xD0);
        let block = jobjs
            .iter()
            .position(|&jobj| jobj == word(raw, HATENA_JOBJ))
            .ok_or_else(|| anyhow::anyhow!("Mushroom Kingdom `?` Ground over no brick"))?;
        ensure!(
            hatena_grounds[block] == ground.gobj,
            "Mushroom Kingdom brick {block}'s `?` Ground differs"
        );
        let hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
        stage.restore_hatena(
            block,
            HatenaObject {
                flicker: word(raw, HATENA_FLICKER) as i32,
                steady: word(raw, HATENA_STEADY) != 0,
                hidden,
                addressed: false,
            },
        );
        let matrix = animations.get_mut(&STAGE_MAP).unwrap().joint_matrix(BRICKS[block].1);
        let model = animations.get_mut(&(HATENA_KEY_BASE + block as u8)).unwrap();
        let mut animation = load(assets, HATENA_MAP)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        *model = animation;
        model.set_gobj_translate(hsd_types::Vec3::new(
            matrix.0[0][3],
            matrix.0[1][3],
            matrix.0[2][3],
        ));
        model.set_hidden(hidden);
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
