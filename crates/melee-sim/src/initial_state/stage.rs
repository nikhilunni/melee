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
                particles.generators.len() == usize::from(!match_start),
                "unexpected initial FD generator population"
            );
            let mut animations = BTreeMap::new();
            animations.insert(
                3,
                BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[3])
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            );
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
        melee_types::GrKind::Story => restore_story(saved, assets),
        melee_types::GrKind::OldPupupu => restore_pupupu(saved, assets, particles),
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

fn restore_story(saved: &SavedPose, assets: &Assets) -> Result<(SceneStage, Animations)> {
    use melee_gr::story::Story;
    let entities = word(saved.bytes(0x804D_782C, 4), 0);
    ensure!(
        word(saved.bytes(entities + 9 * 4, 4), 0) == 0,
        "Story boundary has live items; Heiho state restoration required"
    );
    let mut controller = Story {
        parameters: melee_gr::desc::read_story_parameters(&assets.stage)
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        puff_timer: 0,
        shy_timer: 0,
        previous_pattern: 0,
        spawn_count: 0,
        lights: melee_gr::battle::lights::load_model(&assets.stage, &assets.stage_desc, 3)
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        shy_guys: Vec::new(),
        occupied_frames: 0,
    };
    let mut animations = BTreeMap::new();
    let mut maps = Vec::new();
    let mut gobj = word(saved.bytes(entities + 5 * 4, 4), 0);
    while gobj != 0 {
        ensure!(maps.len() <= 4, "cyclic Story map list");
        let object = saved.bytes(gobj, 0x30);
        let ground = word(object, 0x2C);
        if ground != 0 {
            let raw = saved.bytes(ground, 0x108);
            let map = word(raw, 0x14) as u8;
            maps.push(map);
            if map == 3 {
                controller.spawn_count = raw[0xC4] as i8;
                controller.previous_pattern = raw[0xC5] as i8;
                controller.shy_timer = word(raw, 0xC8) as i32;
            }
            if map == 2 {
                controller.puff_timer = i16::from_be_bytes(raw[0xC4..0xC6].try_into().unwrap());
            }
            let root = word(object, 0x28);
            let mut joints = Vec::new();
            saved_joints(saved, word(saved.bytes(root + 0x10, 4), 0), &mut joints);
            let mut frame = None;
            for &joint in &joints {
                let aobj = word(saved.bytes(joint + 0x7C, 4), 0);
                if aobj != 0 {
                    let current = float(saved.bytes(aobj, 0x18), 4);
                    ensure!(
                        frame.is_none_or(|f| f == current),
                        "Story model animation frames disagree"
                    );
                    frame = Some(current);
                }
            }
            let mut animation = BackgroundAnimation::load_model(
                &assets.stage,
                &assets.stage_desc.models[map as usize],
            )
            .map_err(|e| anyhow::anyhow!("Story map {map}: {e}"))?;
            if map == 3 {
                let model = &assets.stage_desc.models[3];
                let subtree = melee_gr::desc::animation_subtree(&assets.stage, model, 1, 5)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                animation
                    .attach_subtree(&assets.stage, 5, &subtree, model.animation_loops[1])
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            animation.set_map_scale(assets.stage_desc.parameters.map_scale);
            if let Some(frame) = frame {
                animation.restore_frame::<RetailTrig>(frame);
            }
            animations.insert(map, animation);
        }
        gobj = word(object, 8);
    }
    ensure!(maps == [0, 1, 3, 2], "Story map order {maps:?}");
    Ok((SceneStage::Story(controller), animations))
}

/// Dream Land Ground owners and already-evaluated JObj frames at the saved boundary.
fn restore_pupupu(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    use melee_gr::pupupu::{Phase, Pupupu};
    ensure!(
        particles.generators.is_empty() && particles.particles.iter().all(Vec::is_empty),
        "Dream Land initial particle population requires attachment restoration"
    );
    ensure!(
        word(saved.bytes(0x804D6A9C, 4), 0) == 0,
        "Dream Land demo freeze unsupported"
    );
    ensure!(
        word(saved.bytes(0x804D63B0, 4), 0) == 0,
        "Dream Land saved gust list unsupported"
    );
    let mut stage = Pupupu::initialize(
        melee_gr::desc::read_pupupu_parameters(&assets.stage)
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        &mut HsdRng::new(0),
    );
    stage.lights = melee_gr::battle::lights::load_model(&assets.stage, &assets.stage_desc, 5)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut animations = BTreeMap::new();
    let mut maps = Vec::new();
    let entities = word(saved.bytes(0x804D782C, 4), 0);
    let mut gobj = word(saved.bytes(entities + 20, 4), 0);
    while gobj != 0 {
        ensure!(maps.len() <= 8, "cyclic Dream Land map list");
        let object = saved.bytes(gobj, 0x30);
        let ground = word(object, 0x2C);
        if ground != 0 {
            let raw = saved.bytes(ground, 0x108);
            let map = word(raw, 0x14) as u8;
            maps.push(map);
            if map == 7 {
                stage.cycle = word(raw, 0xC4) as i32;
                stage.phase = Phase::from_saved(word(raw, 0xC8) as i32);
                stage.blink_timer = word(raw, 0xCC) as i32;
                stage.timer = word(raw, 0xD0) as i32;
                stage.entering = word(raw, 0xD4) != 0;
                stage.facing_right = word(raw, 0xD8) != 0;
                stage.wind = word(raw, 0xDC) as i32;
                stage.elapsed = word(raw, 0xE0) as i32;
                ensure!(
                    stage.phase == Phase::Waiting && stage.facing_right,
                    "Dream Land saved active wind phase requires animation selection"
                );
            }
            if map == 1 {
                stage.secondary = (word(raw, 0xD0) == 1).then_some((
                    Phase::from_saved(word(raw, 0xC8) as i32),
                    stage.facing_right,
                ));
            }
            if map == 8 {
                stage.background_timer = i16::from_be_bytes(raw[0xC4..0xC6].try_into().unwrap());
                gobj = word(object, 8);
                continue;
            }
            let root = word(object, 0x28);
            let mut joints = Vec::new();
            if root != 0 {
                saved_joints(saved, word(saved.bytes(root + 16, 4), 0), &mut joints);
            }
            let mut frame = None;
            for joint in joints {
                let aobj = word(saved.bytes(joint + 0x7C, 4), 0);
                if aobj != 0 {
                    let current = float(saved.bytes(aobj, 0x18), 4);
                    ensure!(
                        frame.is_none_or(|f| f == current),
                        "Dream Land map {map} animation frames disagree"
                    );
                    frame = Some(current);
                }
            }
            let mut animation = BackgroundAnimation::load_model(
                &assets.stage,
                &assets.stage_desc.models[map as usize],
            )
            .map_err(|e| anyhow::anyhow!("Dream Land map {map}: {e}"))?;
            animation.set_map_scale(assets.stage_desc.parameters.map_scale);
            if let Some(frame) = frame {
                animation.restore_frame::<RetailTrig>(frame);
            } else {
                animation.clear_animation();
            }
            animations.insert(map, animation);
        }
        gobj = word(object, 8);
    }
    ensure!(
        maps == melee_gr::pupupu::procs::MAP_ORDER,
        "Dream Land map order {maps:?}"
    );
    Ok((SceneStage::Pupupu(stage), animations))
}
