//! Jungle Japes at a saved boundary: Cranky's and the Klaptrap's Ground
//! variables, each model's animation clock, and the particle generators the
//! models' keys attached.
use super::{float, saved_pose::SavedPose, stage::saved_joints, word};
use crate::{
    assets::Assets,
    initial_state::stage::joint_id,
    scene_stage::{garden::load, SceneStage},
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_ft::fighter::RetailTrig;
use melee_gr::garden::{
    procs::{CRANKY, KLAPTRAP, MAP_ORDER},
    Cranky, Garden, Klaptrap,
};
use std::collections::BTreeMap;

/// `HSD_GObj_Entities` and its Ground p_link.
const GOBJ_ENTITIES: u32 = 0x804D_782C;
const GROUND_PLINK: u32 = 5;
/// HSD_JObj aobj and HSD_AObj curr_frame.
const JOBJ_AOBJ: u32 = 0x7C;
const AOBJ_FRAME: usize = 4;

type Animations = BTreeMap<u8, melee_gr::last::animation::BackgroundAnimation>;

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    match_start: bool,
    particles: &mut ParticleSystem,
    metadata: &serde_json::Value,
) -> Result<(SceneStage, Animations)> {
    ensure!(match_start, "Jungle Japes boundaries are match starts");
    ensure!(
        word(saved.bytes(0x804D_63B0, 4), 0) == 0,
        "lb_800115F4: dynamic quake list unsupported"
    );
    let parameters = melee_gr::desc::read_garden_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut cranky = None;
    let mut klaptrap = None;
    let mut animations = Animations::new();
    let mut attachments = BTreeMap::new();
    let mut maps = Vec::new();
    let entities = word(saved.bytes(GOBJ_ENTITIES, 4), 0);
    let mut gobj = word(saved.bytes(entities + GROUND_PLINK * 4, 4), 0);
    while gobj != 0 {
        ensure!(maps.len() < MAP_ORDER.len(), "cyclic Jungle Japes map list");
        let object = saved.bytes(gobj, 0x30);
        let ground = word(object, 0x2C);
        gobj = word(object, 8);
        if ground == 0 {
            continue;
        }
        let raw = saved.bytes(ground, 0xD0);
        let map = word(raw, 0x14) as u8;
        maps.push(map);
        let mut animation = load(assets, map)?;
        match map {
            CRANKY => {
                let state = Cranky {
                    animation: word(raw, 0xC4) as i32,
                    loops_left: word(raw, 0xC8) as i32,
                };
                if state.animation != 0 {
                    animation
                        .select_animation(
                            &assets.stage,
                            &assets.stage_desc.models[usize::from(map)],
                            state.animation as usize,
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                cranky = Some(state);
            }
            KLAPTRAP => {
                let state = Klaptrap {
                    leaping: raw[0xC8] as i8 != 0,
                    timer: word(raw, 0xCC) as i32,
                };
                ensure!(
                    !state.leaping,
                    "a Klaptrap leaping at the boundary is not imported"
                );
                klaptrap = Some(state);
            }
            _ => {}
        }
        let root = word(object, 0x28);
        let mut joints = Vec::new();
        saved_joints(saved, word(saved.bytes(root + 0x10, 4), 0), &mut joints);
        let mut frame = None;
        for (index, &joint) in joints.iter().enumerate() {
            attachments.insert(joint, joint_id(map, index));
            let aobj = word(saved.bytes(joint + JOBJ_AOBJ, 4), 0);
            if aobj != 0 {
                let current = float(saved.bytes(aobj, 8), AOBJ_FRAME);
                ensure!(
                    frame.is_none_or(|f| f == current),
                    "Jungle Japes map {map} animation frames disagree"
                );
                frame = Some(current);
            }
        }
        match frame {
            Some(frame) => animation.restore_frame::<RetailTrig>(frame),
            None => animation.clear_animation(),
        }
        animations.insert(map, animation);
    }
    ensure!(maps == MAP_ORDER, "Jungle Japes map order {maps:?}");
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
                .ok_or_else(|| anyhow::anyhow!("unknown Jungle Japes particle attachment"))?,
        );
    }
    Ok((
        SceneStage::Garden(Garden {
            parameters,
            cranky: cranky.ok_or_else(|| anyhow::anyhow!("Cranky's map"))?,
            klaptrap: klaptrap.ok_or_else(|| anyhow::anyhow!("the Klaptrap's map"))?,
            water_active: false,
        }),
        animations,
    ))
}

/// The saved stage material item: one item, the Klaptrap's, idle.
pub(super) fn check_saved_items(saved: &SavedPose) -> Result<()> {
    let entities = word(saved.bytes(GOBJ_ENTITIES, 4), 0);
    let gobj = word(saved.bytes(entities + 9 * 4, 4), 0);
    ensure!(gobj != 0, "the Klaptrap's item is missing");
    let object = saved.bytes(gobj, 0x30);
    ensure!(word(object, 8) == 0, "Jungle Japes boundary has other items");
    let item = saved.bytes(word(object, 0x2C), 0x30);
    ensure!(
        word(item, 0x10) == melee_types::ItemKind::PokemonRandom as u32 && word(item, 0x24) == 0,
        "the Klaptrap's item at the boundary is not idle"
    );
    Ok(())
}
