//! Stage boundary adapters for gr/types.h Ground views and map animations.
use super::{float, saved_pose::SavedPose, word};
use crate::assets::Assets;
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use melee_gr::{
    ground::Phase,
    last::{background::BackgroundMotion, FinalDestination},
};

pub(super) fn restore(saved: &SavedPose, assets: &Assets) -> Result<FinalDestination> {
    // Cold initialization builds owned defaults only. Its four draws are private
    // and never enter the restored match's shared stream.
    let mut stage = FinalDestination::initialize(&assets.stage_desc, &mut HsdRng::new(0));
    stage.actions.clear();
    stage.ground.live_maps.fill(false);
    let entities = word(saved.bytes(0x804D_782C, 4), 0);
    let mut gobj = word(saved.bytes(entities + 5 * 4, 4), 0);
    let mut maps = Vec::new();
    while gobj != 0 {
        ensure!(maps.len() < 10, "cyclic or unsupported Ground list");
        let object = saved.bytes(gobj, 0x30);
        let address = word(object, 0x2C);
        // The stage-query GObj has no Ground user data.
        if address != 0 {
            let raw = saved.bytes(address, 0x108);
            let map = word(raw, 0x14) as usize;
            ensure!(
                map < 10 && !stage.ground.live_maps[map],
                "invalid/duplicate map {map}"
            );
            maps.push(map);
            stage.ground.live_maps[map] = true;
            match map {
                3 => {
                    let flags = word(raw, 0xC4);
                    stage.ground.waiting_for_start = flags & (1 << 31) != 0;
                    stage.ground.demo_frozen = flags & (1 << 30) != 0;
                    stage.ground.phase = Phase::try_from(((flags >> 14) & 0xFFFF) as u16)
                        .map_err(anyhow::Error::msg)?;
                    stage.ground.transition_enabled = flags & (1 << 13) != 0;
                    stage.ground.fade.complete = flags & (1 << 12) != 0;
                    stage.ground.elapsed = float(raw, 0xC8);
                }
                7 => {
                    stage.ground.background = Some(BackgroundMotion {
                        pitch: float(raw, 0xC4),
                        yaw: float(raw, 0xC8),
                        pitch_speed: float(raw, 0xCC),
                        yaw_speed: float(raw, 0xD0),
                        pitch_acceleration: float(raw, 0xD4),
                        yaw_acceleration: float(raw, 0xD8),
                        amplitude: float(raw, 0xDC),
                        generator_present: word(raw, 0xE0) != 0,
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }
        gobj = word(object, 8);
    }
    ensure!(
        maps == (0..9).collect::<Vec<_>>(),
        "unsupported FD creation order: {maps:?}"
    );
    ensure!(
        stage.ground.phase == Phase::LayeredStart
            && !stage.ground.transition_enabled
            && stage.ground.fade.complete
            && !stage.ground.demo_frozen,
        "unsupported FD controller boundary: {:?}",
        stage.ground
    );
    let bg = stage.ground.background.as_ref().unwrap();
    ensure!(
        bg.amplitude == 0.0 && !bg.generator_present,
        "tilt generator unsupported"
    );
    Ok(stage)
}

use crate::scene_stage::SceneStage;
use hsd_particle::system::ParticleSystem;
use melee_ft::fighter::RetailTrig;
use melee_gr::last::animation::BackgroundAnimation;
use std::collections::BTreeMap;

/// Stage attachment namespace below Effects::FIRST_EFFECT_JOINT. Preserve FD's
/// existing map-4 identities; other maps occupy disjoint ranges.
pub(crate) fn joint_id(map: u8, joint: usize) -> usize {
    if map == 4 {
        joint
    } else {
        usize::from(map) * 100 + joint
    }
}
type Animations = BTreeMap<u8, BackgroundAnimation>;

pub(super) fn restore_scene(
    saved: &SavedPose,
    assets: &Assets,
    match_start: bool,
    frames: u64,
    particles: &mut ParticleSystem,
    metadata: &serde_json::Value,
) -> Result<(SceneStage, Animations)> {
    match assets.stage_desc.kind {
        melee_types::GrKind::Last => {
            let stage = restore(saved, assets)?;
            ensure!(
                stage.ground.waiting_for_start == match_start,
                "FD start flag disagrees with fighter boundary"
            );
            ensure!(
                stage.ground.elapsed + frames as f32 <= 1800.0,
                "FD transition exceeds stationary attachment interval"
            );
            ensure!(
                particles.generators.len() == usize::from(!match_start),
                "unexpected initial FD generator population"
            );
            let mut animations = BTreeMap::new();
            if match_start {
                animations.insert(
                    4,
                    BackgroundAnimation::load(&assets.stage, &assets.stage_desc)
                        .map_err(|e| anyhow::anyhow!("{e}"))?,
                );
            }
            Ok((SceneStage::FinalDestination(Box::new(stage)), animations))
        }
        melee_types::GrKind::Battle => {
            restore_battlefield(saved, assets, frames, particles, metadata)
        }
        _ => unreachable!("registered stage descriptor"),
    }
}

/// `grNBa_BG_Vars` (gr/types.h:1680) and Ground/JObj saved state. No later
/// oracle row is read; archive animations rebuild their keyframe cursors.
fn restore_battlefield(
    saved: &SavedPose,
    assets: &Assets,
    frames: u64,
    particles: &mut ParticleSystem,
    metadata: &serde_json::Value,
) -> Result<(SceneStage, Animations)> {
    use melee_gr::battle::{BackgroundPhase, Battlefield};
    ensure!(
        assets
            .stage_desc
            .models
            .iter()
            .all(|m| m.joint_mappings.is_empty()),
        "Battlefield animated collision bindings unsupported"
    );
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    let mut controller = None;
    let mut animations = BTreeMap::new();
    let mut maps = Vec::new();
    let mut attachments = BTreeMap::new();
    let entities = word(saved.bytes(0x804D_782C, 4), 0);
    let mut gobj = word(saved.bytes(entities + 5 * 4, 4), 0);
    while gobj != 0 {
        ensure!(maps.len() <= 4, "cyclic Battlefield map list");
        let object = saved.bytes(gobj, 0x30);
        let ground = word(object, 0x2C);
        if ground != 0 {
            let raw = saved.bytes(ground, 0x108);
            let map = word(raw, 0x14) as u8;
            maps.push(map);
            if map == 3 {
                ensure!(
                    word(raw, 0xC4) == 0 && word(raw, 0xC8) == u32::MAX,
                    "Battlefield transition boundary unsupported"
                );
                let timer = word(raw, 0xD0) as i32;
                ensure!(
                    i64::from(timer) >= frames as i64,
                    "Battlefield transition outside imported interval"
                );
                controller = Some(Battlefield {
                    phase: BackgroundPhase::Waiting,
                    timer,
                    current: None,
                    previous: None,
                    lights: melee_gr::battle::lights::load(&assets.stage, &assets.stage_desc)
                        .map_err(|e| anyhow::anyhow!("{e}"))?,
                });
            } else {
                let mut joints = Vec::new();
                let root = word(object, 0x28);
                saved_joints(saved, word(saved.bytes(root + 0x10, 4), 0), &mut joints);
                let mut frame = None;
                for (index, &joint) in joints.iter().enumerate() {
                    attachments.insert(joint, joint_id(map, index));
                    let aobj = word(saved.bytes(joint + 0x7C, 4), 0);
                    if aobj != 0 {
                        let current = float(saved.bytes(aobj, 0x18), 4);
                        ensure!(
                            frame.is_none_or(|f| f == current),
                            "stage animation frames disagree"
                        );
                        frame = Some(current);
                    }
                }
                let mut animation = BackgroundAnimation::load_model(
                    &assets.stage,
                    &assets.stage_desc.models[usize::from(map)],
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                animation.set_map_scale(assets.stage_desc.parameters.map_scale);
                if let Some(frame) = frame {
                    animation.restore_frame::<RetailTrig>(frame);
                }
                animations.insert(map, animation);
            }
        }
        gobj = word(object, 8);
    }
    ensure!(
        maps == [0, 3, 1, 6],
        "unsupported Battlefield map order: {maps:?}"
    );
    for (runtime, captured) in particles
        .generators
        .iter_mut()
        .zip(metadata["particles"]["generators"].as_array().unwrap())
    {
        let pointer = captured["fields"]["jobj"].as_u64().unwrap() as u32;
        runtime.attachment_id = Some(
            *attachments
                .get(&pointer)
                .ok_or_else(|| anyhow::anyhow!("unknown Battlefield particle attachment"))?,
        );
    }
    Ok((SceneStage::Battlefield(controller.unwrap()), animations))
}
fn saved_joints(saved: &SavedPose, pointer: u32, joints: &mut Vec<u32>) {
    if pointer == 0 {
        return;
    }
    assert!(joints.len() < 100, "stage joint tree bound");
    joints.push(pointer);
    let raw = saved.bytes(pointer, 0x18);
    saved_joints(saved, word(raw, 0x10), joints);
    saved_joints(saved, word(raw, 8), joints);
}
