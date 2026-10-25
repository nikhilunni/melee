//! Fourside at a saved boundary: the crane (`foursideCrane`), the
//! helicopter (`fourside2`) and the UFO (`foursideUfo`) from their Grounds'
//! `u` unions (gr/types.h:1112-1148), their models, and the three collision
//! joints with the roof's callback word.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS,
        JOBJ_HIDDEN,
    },
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        fourside::{count_roof_contact, load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::fourside::{
    crane::{Crane, CranePhase},
    procs::{bindings, COLLISION_MAP, CRANE_MAP, HELI_MAP, MAP_ORDER, ROOF_JOINT, UFO_MAP},
    ufo::{Ufo, UfoPhase, JOINT as UFO_JOINT},
    Fourside, Heli, HeliPhase,
};

/// The Ground's `u` union.
const VARS: usize = 0xC4;

fn saved_crane(raw: &[u8]) -> Result<Crane> {
    let flags = raw[VARS + 1];
    Ok(Crane {
        phase: CranePhase::from_saved(raw[VARS])
            .ok_or_else(|| anyhow::anyhow!("Fourside crane phase {}", raw[VARS]))?,
        // x1.b0 and x1.b1, most significant bit first.
        out: flags & 0x80 != 0,
        lowered: flags & 0x40 != 0,
        timer: word(raw, VARS + 4) as i32,
        rest_y: float(raw, VARS + 8),
        y: float(raw, VARS + 0xC),
        target_y: float(raw, VARS + 0x10),
        settle_limit: float(raw, VARS + 0x14),
        speed: float(raw, VARS + 0x18),
        acceleration: float(raw, VARS + 0x1C),
    })
}

fn saved_heli(raw: &[u8]) -> Result<Heli> {
    Ok(Heli {
        phase: HeliPhase::from_saved(raw[VARS])
            .ok_or_else(|| anyhow::anyhow!("Fourside helicopter phase {}", raw[VARS]))?,
        descended: raw[VARS + 1] != 0,
        timer: word(raw, VARS + 4) as i32,
    })
}

fn saved_ufo(raw: &[u8]) -> Result<Ufo> {
    ensure!(
        word(raw, VARS + 0xC) == 0,
        "Fourside UFO camera subject at the boundary"
    );
    Ok(Ufo {
        phase: UfoPhase::from_saved(raw[VARS])
            .ok_or_else(|| anyhow::anyhow!("Fourside UFO phase {}", raw[VARS]))?,
        building: raw[VARS + 1],
        tries: raw[VARS + 2],
        sound_step: raw[VARS + 3],
        timer: word(raw, VARS + 4) as i32,
        roof_contacts: word(raw, VARS + 8) as i32,
        subject: None,
    })
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
        "Fourside joint-attached particles need attachment restoration"
    );
    let parameters = melee_gr::desc::read_fourside_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Fourside map order {maps:?}"
    );
    let mut animations = Animations::new();
    let (mut crane, mut heli, mut ufo) = (None, None, None);
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x108);
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        let hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
        match key {
            CRANE_MAP => {
                let saved_crane = saved_crane(raw)?;
                // The arm's clock stays stopped until the first swing; a
                // swinging arm's clock and rate are not imported.
                ensure!(
                    saved_crane.phase == CranePhase::Waiting && !saved_crane.out,
                    "Fourside crane {:?} at the boundary needs its arm's clock",
                    saved_crane.phase
                );
                animation.set_rate(0.0);
                animation.set_joint_translate_y(
                    melee_gr::fourside::crane::IRON_BONE,
                    saved_crane.y,
                );
                crane = Some(saved_crane);
            }
            HELI_MAP => {
                let saved_heli = saved_heli(raw)?;
                ensure!(
                    saved_heli.phase == HeliPhase::Waiting && hidden,
                    "Fourside helicopter {:?} at the boundary needs its clock",
                    saved_heli.phase
                );
                animation.set_hidden(true);
                heli = Some(saved_heli);
            }
            UFO_MAP => {
                let saved_ufo = saved_ufo(raw)?;
                ensure!(
                    saved_ufo.phase == UfoPhase::Away && hidden,
                    "Fourside UFO {:?} at the boundary needs its clock",
                    saved_ufo.phase
                );
                animation.set_hidden(true);
                ufo = Some(saved_ufo);
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    let stage = Fourside {
        parameters,
        crane: crane.unwrap(),
        heli: heli.unwrap(),
        ufo: ufo.unwrap(),
    };
    Ok((SceneStage::Fourside(Box::new(stage)), animations))
}

/// The collision joints as the last Ground_801C2FE0 left them, from the
/// saved JObj matrices; the UFO's joint out of the list while it is away;
/// the roof's callback with the UFO's saved count. Runs before fighters
/// read the map.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    for key in [COLLISION_MAP, CRANE_MAP, UFO_MAP] {
        let ground = grounds
            .iter()
            .find(|g| g.map == u32::from(key))
            .ok_or_else(|| anyhow::anyhow!("Fourside map {key} missing"))?;
        let joints = model_joints(saved, ground.jobj);
        for binding in bindings(key) {
            let jobj = joints[binding.extra as usize];
            let hidden = word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
            let mtx = saved_mtx(saved, jobj);
            let joint = i32::from(binding.joint_index);
            // grFourside_801F37FC binds the UFO before hiding it: its lines
            // took the bone's place once, and a hidden JObj never moves them
            // again (mpLib_80055E9C only hides the joint).
            map.update_joint_transform(joint, Some(melee_mp::JobjState { mtx, hidden: false }));
            if hidden {
                map.update_joint_transform(joint, Some(melee_mp::JobjState { mtx, hidden }));
            }
        }
    }
    restore_vertices(saved, map, "the crane's beam")?;
    let ufo = grounds
        .iter()
        .find(|g| g.map == u32::from(UFO_MAP))
        .map(|g| saved_ufo(saved.bytes(g.ground, 0x108)))
        .transpose()?
        .unwrap();
    ensure!(
        ufo.phase == UfoPhase::Away,
        "Fourside UFO {:?} at the boundary",
        ufo.phase
    );
    map.joint_list_remove(UFO_JOINT);
    map.joint_set_cb1(ROOF_JOINT, ufo.roof_contacts as u32, count_roof_contact);
    Ok(())
}
