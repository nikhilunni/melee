//! Venom at a saved boundary: the maps and their clocks, the spawner
//! (grVe_804D6A34..804D6A40, the slots at grVe_803E5348 +0x20 and
//! grVe_803E5380) and the flight's flags (map 7's `u.venom2.xE0`, `xE4`).
//! A boundary with a pass out is refused until one is witnessed.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_animation, saved_mtx, saved_vec3, JOBJ_FLAGS,
        JOBJ_HIDDEN, JOBJ_SCALE, JOBJ_TRANSLATE,
    },
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        venom::{load, read_parameters, reserve_pass_maps, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::venom::{
    arwings::Spawner,
    procs::{VenomStage, FLIGHT_MAP, MAP_ORDER, SHIP_MAP},
    sequence::{Sequence, TUNNEL_JOINT},
};

/// grVe_804D6A34 (the creating slot), 804D6A38 (the wait), 804D6A40 (the
/// 1P stage's three slots).
const SLOT: u32 = 0x804D_6A34;
const WAIT: u32 = 0x804D_6A38;
const MULTI: u32 = 0x804D_6A40;
/// grVe_803E5348.arwing: three GObjs, three types; grVe_803E5380: groups.
const PASSES: u32 = 0x803E_5348 + 0x20;
const GROUPS: u32 = 0x803E_5380;

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
        "Venom joint-attached particles need attachment restoration"
    );
    ensure!(
        word(saved.bytes(MULTI, 4), 0) == 0,
        "Venom's 1P slots (grVe_804D6A40) at a versus boundary"
    );
    let passes = saved.bytes(PASSES, 0x18);
    ensure!(
        (0..3).all(|slot| word(passes, slot * 4) == 0),
        "Venom's Arwings at the boundary need importing"
    );
    let groups = saved.bytes(GROUPS, 0xC);
    let spawner = Spawner {
        wait: word(saved.bytes(WAIT, 4), 0) as i32,
        kinds: std::array::from_fn(|slot| word(passes, 0xC + slot * 4) as i32),
        groups: std::array::from_fn(|slot| word(groups, slot * 4) as i32),
        slot: word(saved.bytes(SLOT, 4), 0) as usize,
    };
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Venom map order {maps:?}"
    );
    let mut animations = Animations::new();
    let mut sequence = Sequence::default();
    for ground in &grounds {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        // The GObj's wrapper as the maps' inits left it (map 9's x and y
        // scale 1, grVenom_80204DD4).
        animation.set_gobj_scale(saved_vec3(saved, ground.jobj + JOBJ_SCALE));
        animation.set_gobj_translate(saved_vec3(saved, ground.jobj + JOBJ_TRANSLATE));
        if key == SHIP_MAP {
            animation.clear_animation();
            crate::scene_stage::venom::play_ship_switches(&mut animation);
        }
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        if key == FLIGHT_MAP {
            let raw = saved.bytes(ground.ground, 0xE8);
            sequence = Sequence {
                flags: std::array::from_fn(|bit| raw[0xE0] & (0x80 >> bit) != 0),
                frame: float(raw, 0xE4),
            };
        }
        animations.insert(key, animation);
    }
    ensure!(
        !sequence.flags[2],
        "Venom's tunnel floor (joint {TUNNEL_JOINT}) at the boundary"
    );
    reserve_pass_maps(assets, &mut animations)?;
    let stage = VenomStage {
        parameters: read_parameters(assets)?,
        spawner,
        sequence,
        flight: None,
        pending_shot: None,
    };
    Ok((SceneStage::Venom(Box::new(stage)), animations))
}

/// The Great Fox's joints as the last Ground_801C2FE0 left them, from its
/// root's saved matrix; the tunnel floor's and platforms' joints off, as
/// the stage's creation left them.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    let ship = grounds
        .iter()
        .find(|g| g.map == u32::from(SHIP_MAP))
        .ok_or_else(|| anyhow::anyhow!("Venom map {SHIP_MAP} missing"))?;
    let joints = model_joints(saved, ship.jobj);
    for binding in melee_gr::venom::procs::bindings(SHIP_MAP) {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    for joint in [TUNNEL_JOINT, 3, 4] {
        map.joint_list_remove(joint);
    }
    Ok(())
}
