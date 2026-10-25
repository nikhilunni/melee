//! Flat Zone at a saved boundary: the controller (map 2), the platforms
//! (map 5), the oil man (map 3), the tools (map 8), their models, the
//! collision joints of maps 4, 5 and 6, and the spill pool.
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
        flatzone::{bindings, load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::system::ParticleSystem;
use hsd_types::Vec3;
use melee_gr::flatzone::{
    procs::MAP_ORDER, Controller, FlatZone, OilMan, OilPhase, Platforms, Tools, CONTROLLER, OIL,
    PLATFORMS, TOOLS,
};

/// grDynamicAttr_804D6960: the live spill list's head.
const LIVE_SPILLS: u32 = 0x804D_6960;

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([raw[at], raw[at + 1]])
}

fn saved_vec3(saved: &SavedPose, address: u32) -> Vec3 {
    let raw = saved.bytes(address, 12);
    Vec3::new(float(raw, 0), float(raw, 4), float(raw, 8))
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
        "Flat Zone joint-attached particles need attachment restoration"
    );
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    ensure!(
        word(saved.bytes(LIVE_SPILLS, 4), 0) == 0,
        "Flat Zone: an oil spill at the boundary is not imported"
    );
    let parameters = melee_gr::desc::read_flatzone_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Flat Zone map order {maps:?}"
    );
    let mut stage = FlatZone::create(parameters, 0, &mut HsdRng::new(0));
    let mut animations = Animations::new();
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0xE0);
        match key {
            CONTROLLER => {
                stage.controller = Controller {
                    waiting: raw[0xC4] != 0,
                    event: raw[0xC5],
                    last_event: raw[0xC6],
                    ready: raw[0xC7] != 0,
                    timer: half(raw, 0xC8),
                };
            }
            PLATFORMS => {
                stage.platforms = Platforms {
                    waiting: raw[0xC4] != 0,
                    animation: raw[0xC5],
                    stitched: raw[0xC6],
                    layout: raw[0xC7],
                    avoided: raw[0xC8],
                    variant: raw[0xC9],
                    previous_variant: raw[0xCA],
                    timer: half(raw, 0xCC),
                };
            }
            OIL => {
                let phase = word(raw, 0xD0) as i32;
                stage.oil = OilMan {
                    triggered: raw[0xC4] != 0,
                    facing: float(raw, 0xC8),
                    spill: None,
                    phase: OilPhase::from_saved(phase)
                        .ok_or_else(|| anyhow::anyhow!("Flat Zone oil phase {phase}"))?,
                    timer: word(raw, 0xD4) as i32,
                };
                ensure!(
                    stage.oil.phase == OilPhase::Idle && word(raw, 0xCC) == 0,
                    "Flat Zone: the oil man at the boundary is not imported"
                );
            }
            TOOLS => {
                stage.tools = Tools {
                    triggered: raw[0xC4] != 0,
                    frames: word(raw, 0xC8) as i32,
                };
            }
            4 | 6 => stage.unused_triggers[usize::from(key == 6)] = raw[0xC4] != 0,
            _ => {}
        }
        let mut model = load(assets, key)?;
        model.set_gobj_translate(saved_vec3(saved, ground.jobj + JOBJ_TRANSLATE));
        let wrapper_hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
        model.set_hidden(wrapper_hidden);
        let joints = model_joints(saved, ground.jobj);
        // The platforms play their layout's animation; map 2 plays
        // animation 1 only while a left-facing oil man walks in, and the oil
        // man is idle here.
        if key == PLATFORMS {
            model.reset_pose();
            model
                .select_animation(
                    &assets.stage,
                    &assets.stage_desc.models[usize::from(key)],
                    usize::from(stage.platforms.animation),
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        restore_animation(saved, &mut model, &joints, ground.map)?;
        // Controller-written SRT and visibility.
        for (bone, &joint) in joints.iter().enumerate() {
            let translate = saved_vec3(saved, joint + JOBJ_TRANSLATE);
            if model.joint_translation(bone) != translate {
                model.set_joint_translate(bone, translate);
            }
            let hidden = word(saved.bytes(joint + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
            if model.joint_hidden(bone) != hidden {
                model.set_joint_hidden(bone, hidden);
            }
        }
        animations.insert(key, model);
    }
    ensure!(
        !stage.tools.triggered && stage.tools.frames == 0,
        "Flat Zone: falling tools at the boundary are not imported"
    );
    Ok((SceneStage::FlatZone(Box::new(stage)), animations))
}

/// The collision joints of maps 4, 5 and 6 as the last Ground_801C2FE0 left
/// them, from the saved JObj matrices; the previous positions from the
/// saved vertices. Runs before fighters read the map.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    let platforms = grounds
        .iter()
        .find(|g| g.map == u32::from(PLATFORMS))
        .ok_or_else(|| anyhow::anyhow!("Flat Zone map 5 missing"))?;
    // grFlatzone_802174EC stitches on its first run (xC6 starts at 0xFF).
    let procs_ran = saved.bytes(platforms.ground + 0xC6, 1)[0] != 0xFF;
    for ground in &grounds {
        let key = ground.map as u8;
        if !procs_ran {
            // No Ground_801C2FE0 yet: the joints are as Ground_801C2ED0
            // bound them, from the visible, unanimated model.
            crate::scene_stage::flatzone::bind(&mut load(assets, key)?, assets, key, map);
            continue;
        }
        let joints = model_joints(saved, ground.jobj);
        for binding in bindings(assets, key) {
            let jobj = joints[binding.extra as usize];
            let state = melee_mp::JobjState {
                mtx: saved_mtx(saved, jobj),
                hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
            };
            map.update_joint_transform(i32::from(binding.joint_index), Some(state));
        }
        if key == PLATFORMS {
            // grFlatzone_802174EC's mpLib_80058560 for the animation it
            // stitched.
            map.stitch_all_joints();
        }
    }
    restore_vertices(saved, map, "a platform layout")
}
