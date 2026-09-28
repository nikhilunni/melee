//! Fountain of Dreams at a saved boundary: Ground owners, the platform
//! controllers (`GroundVars_izumi3`, gr/types.h:209), their JObjs, and the
//! collision joints those JObjs move.
use super::{float, saved_pose::SavedPose, stage::saved_joints, word};
use crate::{
    assets::Assets,
    scene_stage::{
        izumi::{load, Animations, COLLISION_BINDINGS, COLLISION_MAP},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use hsd_types::{Mtx, Vec3};
use melee_ft::fighter::RetailTrig;
use melee_gr::izumi::{
    procs::{OBJECT_ORDER, PLATFORMS},
    Izumi, Platform, PlatformPhase,
};

/// `HSD_GObj_Entities`: the p_link list heads.
const GOBJ_ENTITIES: u32 = 0x804D_782C;
const GROUND_PLINK: u32 = 5;
/// `groundCollVtx` (mplib.c), an array of 0x18-byte CollVtx.
const COLL_VTX: u32 = 0x804D_64B8;
const COLL_VTX_SIZE: u32 = 0x18;
/// HSD_JObj fields (jobj.h).
const JOBJ_CHILD: u32 = 0x10;
const JOBJ_FLAGS: u32 = 0x14;
const JOBJ_SCALE: u32 = 0x2C;
const JOBJ_TRANSLATE: u32 = 0x38;
const JOBJ_MTX: u32 = 0x44;
const JOBJ_AOBJ: u32 = 0x7C;
/// HSD_AObj curr_frame.
const AOBJ_FRAME: u32 = 0x4;
const JOBJ_HIDDEN: u32 = 0x10;
/// Ground map id of the star model (Ground_801C1A20(joint, -1)).
const STAR_MAP: u32 = u32::MAX;

struct SavedGround {
    /// gobj->hsd_obj: the map-scale wrapper.
    jobj: u32,
    ground: u32,
    map: u32,
}

fn ground_list(saved: &SavedPose) -> Result<Vec<SavedGround>> {
    let entities = word(saved.bytes(GOBJ_ENTITIES, 4), 0);
    let mut gobj = word(saved.bytes(entities + GROUND_PLINK * 4, 4), 0);
    let mut result = Vec::new();
    while gobj != 0 {
        ensure!(
            result.len() <= OBJECT_ORDER.len(),
            "cyclic Fountain of Dreams Ground list"
        );
        let object = saved.bytes(gobj, 0x30);
        let ground = word(object, 0x2C);
        if ground != 0 {
            result.push(SavedGround {
                jobj: word(object, 0x28),
                ground,
                map: word(saved.bytes(ground + 0x14, 4), 0),
            });
        }
        gobj = word(object, 8);
    }
    Ok(result)
}

/// The model's JObjs in Ground_801C3FA4 (descendant) order.
fn model_joints(saved: &SavedPose, wrapper: u32) -> Vec<u32> {
    let mut joints = Vec::new();
    saved_joints(
        saved,
        word(saved.bytes(wrapper + JOBJ_CHILD, 4), 0),
        &mut joints,
    );
    joints
}

fn saved_vec3(saved: &SavedPose, address: u32) -> Vec3 {
    let raw = saved.bytes(address, 12);
    Vec3::new(float(raw, 0), float(raw, 4), float(raw, 8))
}

fn saved_mtx(saved: &SavedPose, jobj: u32) -> Mtx {
    let raw = saved.bytes(jobj + JOBJ_MTX, 48);
    Mtx(std::array::from_fn(|r| {
        std::array::from_fn(|c| float(raw, (r * 4 + c) * 4))
    }))
}

/// Collision joints as the last mpLib_80055E9C left them: transform each
/// from its saved JObj matrix, then require the saved vertices (current and
/// previous) to match. Runs before fighters read the map.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    let fountain = grounds
        .iter()
        .find(|g| g.map == u32::from(COLLISION_MAP))
        .ok_or_else(|| anyhow::anyhow!("Fountain of Dreams map 3 missing"))?;
    let joints = model_joints(saved, fountain.jobj);
    for binding in &COLLISION_BINDINGS {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    // The previous positions are the ones before the last transform, which
    // the saved matrices cannot rebuild: import them.
    let base = word(saved.bytes(COLL_VTX, 4), 0);
    for index in 0..map.vertices().len() {
        let raw = saved.bytes(base + index as u32 * COLL_VTX_SIZE, COLL_VTX_SIZE as usize);
        map.vtx_restore_prev_pos(index as i32, float(raw, 16), float(raw, 20));
    }
    for (index, vertex) in map.vertices().iter().enumerate() {
        let raw = saved.bytes(base + index as u32 * COLL_VTX_SIZE, COLL_VTX_SIZE as usize);
        let port = [vertex.pos.x, vertex.pos.y, vertex.x10, vertex.x14];
        let retail = [8, 12, 16, 20].map(|offset| float(raw, offset));
        ensure!(
            port.map(f32::to_bits) == retail.map(f32::to_bits),
            "Fountain of Dreams collision vertex {index} {port:?} differs from the saved \
             {retail:?} (a platform moving at the boundary is not imported)"
        );
    }
    Ok(())
}

/// Evaluate a restored model's animation through its saved current frame,
/// or leave it unanimated when retail removed the animation.
fn restore_animation(
    saved: &SavedPose,
    animation: &mut melee_gr::last::animation::BackgroundAnimation,
    joints: &[u32],
    map: u32,
) -> Result<()> {
    let mut clock = None;
    for &joint in joints {
        let aobj = word(saved.bytes(joint + JOBJ_AOBJ, 4), 0);
        if aobj != 0 {
            let raw = saved.bytes(aobj, 8);
            let current = (
                float(raw, AOBJ_FRAME as usize),
                word(raw, 0) & hsd_anim::aobj::AOBJ_FIRST_PLAY != 0,
            );
            ensure!(
                clock.is_none_or(|c| c == current),
                "Fountain of Dreams map {map} animation clocks disagree"
            );
            clock = Some(current);
        }
    }
    match clock {
        // Requested (grAnime_801C7FF8) but not yet evaluated: the platform
        // models created at setup wait for their first s_link 1 update.
        Some((frame, true)) => ensure!(
            frame == 0.0,
            "Fountain of Dreams map {map}: unevaluated animation requested at {frame}"
        ),
        Some((frame, false)) => animation.restore_frame::<RetailTrig>(frame),
        None => animation.clear_animation(),
    }
    Ok(())
}

fn restore_platform(
    saved: &SavedPose,
    assets: &Assets,
    ground: &SavedGround,
    rest_height: f32,
) -> Result<(
    usize,
    Platform,
    melee_gr::last::animation::BackgroundAnimation,
)> {
    let raw = saved.bytes(ground.ground, 0xE0);
    let half = |offset: usize| i16::from_be_bytes([raw[offset], raw[offset + 1]]);
    let side = half(0xC8);
    ensure!(matches!(side, 0 | 1), "platform collision joint {side}");
    let mut model = load(assets, 4)?;
    let origin = saved_vec3(saved, ground.jobj + JOBJ_TRANSLATE);
    model.set_gobj_translate(origin);
    let joints = model_joints(saved, ground.jobj);
    restore_animation(saved, &mut model, &joints, ground.map)?;
    let root = joints[0];
    let scale = saved_vec3(saved, root + JOBJ_SCALE);
    model.set_joint_scale(0, scale);
    let hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
    model.set_hidden(hidden);
    let platform = Platform {
        phase: PlatformPhase::from_saved(half(0xC4)),
        timer: half(0xC6),
        collision_joint: side,
        height: float(raw, 0xD0),
        target: float(raw, 0xD4),
        full_height: float(raw, 0xD8),
        rest_height: float(raw, 0xDC),
        origin_y: origin.y,
    };
    ensure!(
        platform.rest_height == rest_height,
        "platform rest height differs from yakumono_param"
    );
    Ok((side as usize, platform, model))
}

/// Restore the stage controller and every model at the saved boundary.
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
        "Fountain of Dreams joint-attached particles need attachment restoration"
    );
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    let parameters =
        melee_gr::desc::read_izumi_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == [0, 1, 3, STAR_MAP, 2, 4, 4],
        "Fountain of Dreams map order {maps:?}"
    );
    let mut animations = Animations::new();
    let mut platforms = [None, None];
    for ground in &grounds {
        match ground.map {
            STAR_MAP => {}
            4 => {
                let (side, platform, model) =
                    restore_platform(saved, assets, ground, parameters.rest_height)?;
                ensure!(platforms[side].is_none(), "duplicate platform {side}");
                platforms[side] = Some(platform);
                animations.insert(PLATFORMS[side], model);
            }
            map => {
                let key = map as u8;
                let mut animation = load(assets, key)?;
                let joints = model_joints(saved, ground.jobj);
                restore_animation(saved, &mut animation, &joints, map)?;
                // Controller-written SRT and visibility: map 3's collision
                // JObjs and map 2's platform tops.
                for (bone, &joint) in joints.iter().enumerate() {
                    let translate = saved_vec3(saved, joint + JOBJ_TRANSLATE);
                    if animation.joint_translation(bone) != translate {
                        animation.set_joint_translate(bone, translate);
                    }
                    let hidden = word(saved.bytes(joint + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
                    if animation.joint_hidden(bone) != hidden {
                        animation.set_joint_hidden(bone, hidden);
                    }
                }
                animations.insert(key, animation);
            }
        }
    }
    let [Some(left), Some(right)] = platforms else {
        anyhow::bail!("Fountain of Dreams platforms missing");
    };
    Ok((
        SceneStage::Izumi(Izumi {
            parameters,
            platforms: [left, right],
        }),
        animations,
    ))
}
