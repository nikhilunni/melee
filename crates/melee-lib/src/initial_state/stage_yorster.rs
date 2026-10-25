//! Yoshi's Island at a saved boundary: the two maps, map 1's clocks and
//! its collision joints. Only a boundary from before the stage's start (the
//! blocks not yet created, `xC4` still 1) is imported.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_vertices, saved_mtx, AOBJ_FRAME, JOBJ_AOBJ,
        JOBJ_FLAGS, JOBJ_HIDDEN,
    },
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        yorster::{controller, initialize, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_ft::fighter::RetailTrig;
use melee_gr::yorster::procs::{LOOP_BONES, MAP_ORDER, STAGE_MAP};

/// `GroundVars_yorster.xC4` in the Ground's `u` union.
const WAITING: usize = 0xC4;

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
        "Yoshi's Island joint-attached particles need attachment restoration"
    );
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Yoshi's Island map order {maps:?}"
    );
    let stage_ground = grounds.iter().find(|g| g.map == u32::from(STAGE_MAP)).unwrap();
    let raw = saved.bytes(stage_ground.ground, 0x108);
    ensure!(
        word(raw, WAITING) == 1,
        "Yoshi's Island blocks at the boundary need their state imported"
    );
    // The models as creation left them, then Ground_801C1CD0's updates
    // since: map 1's model animation 4 counts them.
    // A scratch map: restore_collision binds the real one from the saved
    // matrices.
    let mut scratch = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let (_, mut animations) = initialize(assets, &mut scratch)?;
    let joints = model_joints(saved, stage_ground.jobj);
    let saved_frame = |joint: u32| {
        let aobj = word(saved.bytes(joint + JOBJ_AOBJ, 4), 0);
        (aobj != 0).then(|| float(saved.bytes(aobj + AOBJ_FRAME, 4), 0))
    };
    let model = animations.get_mut(&STAGE_MAP).unwrap();
    // The first joint animation 4 drives (bones 23 and 26 play 3, a tick
    // behind).
    let looped: Vec<usize> = LOOP_BONES.iter().flat_map(|&b| model.subtree(b)).collect();
    let ticks = joints
        .iter()
        .enumerate()
        .filter(|(bone, _)| !looped.contains(bone))
        .find_map(|(_, &joint)| saved_frame(joint))
        .ok_or_else(|| anyhow::anyhow!("Yoshi's Island map 1 has no animation"))?;
    ensure!(
        ticks >= 0.0 && ticks.fract() == 0.0 && ticks < 60.0,
        "Yoshi's Island map 1 at frame {ticks}: only a start boundary is imported"
    );
    for _ in 0..ticks as usize {
        model.tick::<RetailTrig>();
    }
    for (bone, &joint) in joints.iter().enumerate() {
        let port = model.joint_frame(bone);
        ensure!(
            port.map(f32::to_bits) == saved_frame(joint).map(f32::to_bits),
            "Yoshi's Island bone {bone} clock {port:?} differs from the saved {:?}",
            saved_frame(joint)
        );
    }
    Ok((SceneStage::Yorster(Box::new(controller(assets)?)), animations))
}

/// Map 1's collision joints from the saved block matrices, then the saved
/// vertices checked. Runs before fighters read the map.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    let stage = grounds
        .iter()
        .find(|g| g.map == u32::from(STAGE_MAP))
        .ok_or_else(|| anyhow::anyhow!("Yoshi's Island map 1 missing"))?;
    let joints = model_joints(saved, stage.jobj);
    for binding in &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    restore_vertices(saved, map, "a block")
}
