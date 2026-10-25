//! Kongo Jungle at a saved boundary: the bridge (grKg_803E188C, static
//! data), map 10's swings and event timer, the barrel, each model's clock
//! and the JObj rotations the controllers wrote, and the collision the
//! bridge and platform ends moved.
use super::{float, saved_pose::SavedPose, stage::saved_joints, word};
use crate::{
    assets::Assets,
    initial_state::stage::joint_id,
    scene_stage::{
        kongo::{bindings, load, COLLISION_BINDINGS},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use hsd_types::{Mtx, Vec3};
use melee_ft::fighter::RetailTrig;
use melee_gr::kongo::{
    barrel::{Hold, Spin},
    planks::PLANK_COUNT,
    Barrel, Bridge, Kongo, Swing, BARREL, MAIN, MAP_ORDER,
};
use std::collections::BTreeMap;

const GOBJ_ENTITIES: u32 = 0x804D_782C;
const GROUND_PLINK: u32 = 5;
/// `groundCollVtx` (mplib.c), an array of 0x18-byte CollVtx.
const COLL_VTX: u32 = 0x804D_64B8;
const COLL_VTX_SIZE: u32 = 0x18;
/// grKg_803E188C: the bridge's planks, 0x18 bytes each.
const PLANKS: u32 = 0x803E_188C;
/// HSD_JObj fields (jobj.h) and HSD_AObj curr_frame.
const JOBJ_FLAGS: u32 = 0x14;
const JOBJ_ROTATE: u32 = 0x1C;
const JOBJ_TRANSLATE: u32 = 0x38;
const JOBJ_MTX: u32 = 0x44;
const JOBJ_AOBJ: u32 = 0x7C;
const AOBJ_FRAME: usize = 4;
const JOBJ_HIDDEN: u32 = 0x10;

type Animations = BTreeMap<u8, melee_gr::last::animation::BackgroundAnimation>;

struct SavedGround {
    jobj: u32,
    ground: u32,
    map: u8,
}

fn ground_list(saved: &SavedPose) -> Result<Vec<SavedGround>> {
    let entities = word(saved.bytes(GOBJ_ENTITIES, 4), 0);
    let mut gobj = word(saved.bytes(entities + GROUND_PLINK * 4, 4), 0);
    let mut result = Vec::new();
    while gobj != 0 {
        ensure!(result.len() < 16, "cyclic Kongo Jungle Ground list");
        let object = saved.bytes(gobj, 0x30);
        let ground = word(object, 0x2C);
        if ground != 0 {
            result.push(SavedGround {
                jobj: word(object, 0x28),
                ground,
                map: word(saved.bytes(ground + 0x14, 4), 0) as u8,
            });
        }
        gobj = word(object, 8);
    }
    Ok(result)
}

fn model_joints(saved: &SavedPose, wrapper: u32) -> Vec<u32> {
    let mut joints = Vec::new();
    saved_joints(saved, word(saved.bytes(wrapper + 0x10, 4), 0), &mut joints);
    joints
}

fn saved_vec3(saved: &SavedPose, address: u32) -> Vec3 {
    let raw = saved.bytes(address, 12);
    Vec3::new(float(raw, 0), float(raw, 4), float(raw, 8))
}

fn half(raw: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes([raw[offset], raw[offset + 1]])
}

/// The platform ends' joints from their saved matrices, then every
/// vertex (current and previous) as the save holds it: the bridge's
/// lines are offsets no matrix rebuilds. Runs before fighters read the map.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    for ground in &grounds {
        let joints = model_joints(saved, ground.jobj);
        for binding in bindings(ground.map) {
            let jobj = joints[binding.extra as usize];
            let raw = saved.bytes(jobj + JOBJ_MTX, 48);
            let state = melee_mp::JobjState {
                mtx: Mtx(std::array::from_fn(|r| {
                    std::array::from_fn(|c| float(raw, (r * 4 + c) * 4))
                })),
                hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
            };
            map.update_joint_transform(i32::from(binding.joint_index), Some(state));
        }
    }
    // The logs' joints start disabled (grKongo_801D523C); the bridge's
    // collision joint takes B10 (grKongo_801D69B0).
    ensure!(
        !grounds.iter().any(|g| matches!(g.map, 1 | 2)),
        "a log at the boundary is not imported"
    );
    map.joint_list_remove(0);
    map.joint_list_remove(1);
    map.joint_set_b10(melee_gr::kongo::planks::BRIDGE_COLLISION_JOINT);
    let base = word(saved.bytes(COLL_VTX, 4), 0);
    for index in 0..map.vertices().len() {
        let raw = saved.bytes(base + index as u32 * COLL_VTX_SIZE, COLL_VTX_SIZE as usize);
        map.vtx_set_pos(index as i32, float(raw, 8), float(raw, 12));
        map.vtx_restore_prev_pos(index as i32, float(raw, 16), float(raw, 20));
    }
    let _ = COLLISION_BINDINGS;
    Ok(())
}

fn restore_bridge(saved: &SavedPose) -> Bridge {
    let mut bridge = Bridge::default();
    for (i, plank) in bridge.planks.iter_mut().enumerate().take(PLANK_COUNT) {
        let raw = saved.bytes(PLANKS + i as u32 * 0x18, 0x18);
        plank.pinned = half(raw, 2) != 0;
        plank.rest = float(raw, 0x8);
        plank.angle = float(raw, 0xC);
        plank.velocity = float(raw, 0x10);
        plank.sag = float(raw, 0x14);
    }
    bridge
}

fn restore_barrel(raw: &[u8]) -> Result<Barrel> {
    ensure!(word(raw, 0xD0) == 0, "a barrel holding at the boundary is not imported");
    Ok(Barrel {
        spin: Spin::from_saved(half(raw, 0xC4)),
        hold: match half(raw, 0xC6) {
            0 => Hold::Empty,
            other => anyhow::bail!("barrel hold phase {other} at the boundary"),
        },
        rate_phase: half(raw, 0xC8),
        hold_timer: half(raw, 0xCA),
        spin_timer: half(raw, 0xCC),
        rate_timer: half(raw, 0xCE),
        target: float(raw, 0xD4),
        angle: float(raw, 0xD8),
        acceleration: float(raw, 0xDC),
        speed: float(raw, 0xE0),
        rate: float(raw, 0xE8),
        kept: None,
    })
}

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    match_start: bool,
    particles: &mut ParticleSystem,
    metadata: &serde_json::Value,
) -> Result<(SceneStage, Animations)> {
    ensure!(match_start, "Kongo Jungle boundaries are match starts");
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    let parameters = melee_gr::desc::read_kongo_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u8> = grounds.iter().map(|g| g.map).collect();
    ensure!(maps == MAP_ORDER, "Kongo Jungle map order {maps:?}");
    let mut animations = Animations::new();
    let mut attachments = BTreeMap::new();
    let mut swings = [Swing::default(); 2];
    let mut timers = (-1, -1);
    let mut barrel = None;
    for ground in &grounds {
        let raw = saved.bytes(ground.ground, 0xF0);
        let mut animation = load(assets, ground.map)?;
        let joints = model_joints(saved, ground.jobj);
        let mut frame = None;
        for (index, &joint) in joints.iter().enumerate() {
            attachments.insert(joint, joint_id(ground.map, index));
            let aobj = word(saved.bytes(joint + JOBJ_AOBJ, 4), 0);
            if aobj != 0 {
                let current = float(saved.bytes(aobj, 8), AOBJ_FRAME);
                ensure!(
                    frame.is_none_or(|f| f == current),
                    "Kongo Jungle map {} animation frames disagree",
                    ground.map
                );
                frame = Some(current);
            }
        }
        match frame {
            Some(frame) => animation.restore_frame::<RetailTrig>(frame),
            None => animation.clear_animation(),
        }
        // The controllers' JObj writes: rotations and translations.
        for (bone, &joint) in joints.iter().enumerate() {
            let rotate = saved_vec3(saved, joint + JOBJ_ROTATE);
            let current = animation.joint_rotation(bone);
            if current != rotate {
                animation.set_joint_rotation_x(bone, rotate.x);
                animation.set_joint_rotation_z(bone, rotate.z);
                ensure!(
                    current.y == rotate.y,
                    "Kongo Jungle map {} joint {bone}: rotation y differs",
                    ground.map
                );
            }
            let translate = saved_vec3(saved, joint + JOBJ_TRANSLATE);
            if animation.joint_translation(bone) != translate {
                animation.set_joint_translate(bone, translate);
            }
        }
        match ground.map {
            MAIN => {
                swings = [
                    Swing {
                        angle: float(raw, 0xC4),
                        velocity: float(raw, 0xC8),
                    },
                    Swing {
                        angle: float(raw, 0xD4),
                        velocity: float(raw, 0xD8),
                    },
                ];
                timers = (half(raw, 0xE4), half(raw, 0xE6));
            }
            BARREL => {
                let restored = restore_barrel(raw)?;
                // Retail 0x801D5608 installs animation at its own rate.
                // gp+E8 is applied only by link 4 (0x801D5D6C), after
                // the first link-1 animation step (0x801C1CF0).
                for (bone, &joint) in joints.iter().enumerate() {
                    animation.restore_joint_animation(bone, saved.joint_animation(joint));
                }
                barrel = Some(restored);
            }
            _ => {}
        }
        animations.insert(ground.map, animation);
    }
    for (runtime, captured) in particles
        .generators
        .iter_mut()
        .zip(metadata["particles"]["generators"].as_array().unwrap())
    {
        let pointer = captured["fields"]["jobj"].as_u64().unwrap_or(0) as u32;
        if pointer == 0 {
            continue;
        }
        runtime.attachment_id = Some(
            *attachments
                .get(&pointer)
                .ok_or_else(|| anyhow::anyhow!("unknown Kongo Jungle particle attachment"))?,
        );
    }
    let barrel = barrel.ok_or_else(|| anyhow::anyhow!("the barrel's map"))?;
    let model = animations.get_mut(&BARREL).unwrap();
    let matrix = model.joint_matrix(1);
    let barrel_position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
    let barrel_angle = barrel.angle;
    Ok((
        SceneStage::Kongo(Box::new(Kongo {
            parameters,
            bridge: restore_bridge(saved),
            swings,
            event_timer: timers.0,
            last_event: timers.1,
            barrel,
            barrel_position,
            barrel_angle,
            catch_active: false,
            logs: [None; 2],
            passersby: [false; 3],
        })),
        {
            crate::scene_stage::kongo::load_reserved(assets, &mut animations)?;
            animations
        },
    ))
}
