//! Yoshi's Island N64 at a saved boundary: the clouds
//! (`grOldYoshi_Cloud_GroundVars`, gr/types.h:1790), the guest, their
//! models, and the cloud collision joints with their callback words.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS,
        JOBJ_HIDDEN, JOBJ_TRANSLATE,
    },
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        oldyoshi::{carry, load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use melee_gr::oldyoshi::{
    procs::{CLOUD_MAP, GUEST_MAP, MAP_ORDER},
    CloudPhase, CloudState, Guest, OldYoshi, CLOUDS,
};

/// `grOldYoshi_Cloud` is 0x14 bytes from Ground +0xC4.
const CLOUD_BASE: usize = 0xC4;
const CLOUD_SIZE: usize = 0x14;

/// One saved cloud and its xC4_4 bit.
fn saved_cloud(raw: &[u8], index: usize) -> Result<(CloudState, bool)> {
    let at = CLOUD_BASE + index * CLOUD_SIZE;
    // xC4_0123 is the byte's top nibble, xC4_4 the next bit, xC4_567 the
    // eight bits after it (0x8020EC4C extrwi 4,24; 0x8020ECD0 extrwi 8,21).
    let phase = CloudPhase::from_saved(raw[at] >> 4)
        .ok_or_else(|| anyhow::anyhow!("Yoshi's Island N64 cloud phase {}", raw[at] >> 4))?;
    let carried = raw[at] & 0x08 != 0;
    let counter = ((u16::from_be_bytes([raw[at], raw[at + 1]]) >> 3) & 0xFF) as u8;
    Ok((
        CloudState {
            phase,
            counter,
            rest_y: float(raw, at + 8),
            depth: float(raw, at + 0xC),
            speed: float(raw, at + 0x10),
        },
        carried,
    ))
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
        "Yoshi's Island N64 joint-attached particles need attachment restoration"
    );
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    let parameters = melee_gr::desc::read_old_yoshi_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Yoshi's Island N64 map order {maps:?}"
    );
    // The guest's first wait was drawn at creation; the saved one replaces it.
    let mut stage = OldYoshi::create(parameters, [0.0; 3], &mut HsdRng::new(0));
    let mut animations = Animations::new();
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x108);
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        match key {
            CLOUD_MAP => {
                for (index, cloud) in CLOUDS.iter().enumerate() {
                    let (state, _) = saved_cloud(raw, index)?;
                    ensure!(
                        state.phase == CloudPhase::Floating,
                        "Yoshi's Island N64 cloud {index} {:?} at the boundary needs its \
                         material clocks restored",
                        state.phase
                    );
                    stage.clouds[index] = state;
                    let jobj = joints[cloud.bone];
                    let translate = float(saved.bytes(jobj + JOBJ_TRANSLATE, 12), 4);
                    animation.set_joint_translate_y(cloud.bone, translate);
                    let hidden = word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
                    ensure!(
                        !hidden,
                        "Yoshi's Island N64 floating cloud {index} is hidden"
                    );
                }
            }
            GUEST_MAP => {
                let half = |at: usize| i16::from_be_bytes([raw[at], raw[at + 1]]);
                stage.guest = Guest {
                    timer: half(0xC4),
                    shown: half(0xC6),
                };
                ensure!(
                    stage.guest.shown == -1,
                    "Yoshi's Island N64 guest {} showing at the boundary",
                    stage.guest.shown
                );
                let hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
                animation.set_hidden(hidden);
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    Ok((SceneStage::OldYoshi(stage), animations))
}

/// The cloud joints as the last Ground_801C2FE0 left them, from the saved
/// JObj matrices, and each joint's callback with the saved xC4_4 bit. Runs
/// before fighters read the map.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    let clouds = grounds
        .iter()
        .find(|g| g.map == u32::from(CLOUD_MAP))
        .ok_or_else(|| anyhow::anyhow!("Yoshi's Island N64 map 2 missing"))?;
    let joints = model_joints(saved, clouds.jobj);
    for binding in &assets.stage_desc.models[usize::from(CLOUD_MAP)].joint_mappings {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    restore_vertices(saved, map, "a sinking cloud")?;
    let raw = saved.bytes(clouds.ground, 0x108);
    for (index, cloud) in CLOUDS.iter().enumerate() {
        let (_, carried) = saved_cloud(raw, index)?;
        map.joint_set_cb1(cloud.joint, u32::from(carried), carry);
    }
    Ok(())
}
