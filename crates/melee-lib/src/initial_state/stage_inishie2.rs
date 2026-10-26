//! Mushroom Kingdom II at a saved boundary: maps 0 and 2 and map 2's four
//! timers (only a boundary before any event map is made is imported).
use super::{
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_animation, restore_vertices},
};
use crate::{
    assets::Assets,
    scene_stage::{
        inishie2::{load, load_reserved, read_parameters, stage, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::inishie2::{
    procs::{DISABLED_AT_SETUP, MAP_ORDER, STAGE_MAP},
    Controller, Timer,
};

/// Map 2's `GroundVars_inishie2`: the flags byte and the four counts.
const FLAGS: usize = 0xC4;
const LIFT_A: usize = 0xC6;
const LIFT_B: usize = 0xC8;
const BIRDO: usize = 0xCA;
const CARPET: usize = 0xCC;

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([raw[at], raw[at + 1]])
}

/// The flag bit `b` of a big-endian bitfield byte (b0 is 0x80).
fn bit(byte: u8, b: u8) -> bool {
    byte & (0x80 >> b) != 0
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
        "Mushroom Kingdom II joint-attached particles need attachment restoration"
    );
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Mushroom Kingdom II map order {maps:?} (an event map at the boundary is not imported)"
    );
    let mut animations = Animations::new();
    let mut controller = None;
    for ground in &grounds {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        if key == STAGE_MAP {
            animation.clear_animation();
            let raw = saved.bytes(ground.ground, 0xE4);
            let flags = raw[FLAGS];
            let timer = |b, at| Timer {
                armed: bit(flags, b),
                ticks: half(raw, at),
            };
            controller = Some(Controller {
                lifts: [timer(1, LIFT_A), timer(2, LIFT_B)],
                birdo: timer(3, BIRDO),
                carpet: timer(4, CARPET),
                music_decided: bit(flags, 0),
            });
        }
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        animations.insert(key, animation);
    }
    load_reserved(assets, &mut animations)?;
    let stage = stage(
        assets,
        read_parameters(assets)?,
        controller.expect("map 2's Ground"),
    )?;
    Ok((SceneStage::Inishie2(Box::new(stage)), animations))
}

/// grInishie2_801FCBC4's disabled joints, then the saved vertices checked.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    for joint in DISABLED_AT_SETUP {
        map.joint_list_remove(joint);
    }
    restore_vertices(saved, map, "a lift, the carpet or Birdo's platform")
}
