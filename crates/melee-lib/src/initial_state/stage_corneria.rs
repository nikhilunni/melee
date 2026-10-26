//! Corneria at a saved boundary: the maps and their clocks, the Great
//! Fox's controller (`u.corneria` of map 3: the drift, the descent and the
//! cannon), the city's scroll (map 8) and the spawner (grCn_804D69A4..B0,
//! arwing_gobjs, arwing_types, arwing_groups). A boundary with a pass out,
//! a background other than the first city, a charging cannon or a broken
//! one is refused until one is witnessed.
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
        corneria::{load, read_parameters, reserve_later_maps, Animations, Corneria},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::corneria::{
    arwings::Spawner,
    backgrounds::{Background, CITY_MAP},
    great_fox::{Cannon, CannonPhase, GreatFox, BASE},
    procs::{CorneriaStage, MAP_ORDER, PLATFORM_JOINTS, SHIP_MAP},
};

/// grCn_804D69A4 (the creating slot), 804D69A8 (the wait), 804D69AC (the
/// stage still loading), 804D69B0 (the 1P stage's three slots).
const SLOT: u32 = 0x804D_69A4;
const WAIT: u32 = 0x804D_69A8;
const LOADING: u32 = 0x804D_69AC;
const MULTI: u32 = 0x804D_69B0;
/// arwing_gobjs, arwing_types, arwing_groups (three words each).
const PASSES: u32 = 0x803E_1D68;
const TYPES: u32 = 0x803E_1D74;
const GROUPS: u32 = 0x803E_1D80;

/// `u.corneria` of map 3, as grCorneria_801DD674 lays it out.
fn great_fox(raw: &[u8]) -> Result<(bool, GreatFox, Cannon)> {
    let flags = raw[0xC4];
    ensure!(
        float(raw, 0xD4) == BASE[0] && float(raw, 0xD8) == BASE[1],
        "Corneria's Great Fox rests elsewhere"
    );
    let ship = GreatFox {
        descent: float(raw, 0xD0),
        offset: [float(raw, 0xDC), float(raw, 0xE0)],
        previous_offset: [float(raw, 0xE4), float(raw, 0xE8)],
        velocity: [float(raw, 0xEC), float(raw, 0xF0)],
        acceleration: [float(raw, 0xF4), float(raw, 0xF8)],
        descent_speed: float(raw, 0xFC),
        descent_clock: word(raw, 0x100) as i32,
        drift_wait: word(raw, 0x104) as i32,
        descending: flags & 0x40 != 0,
        sea_seen: flags & 0x20 != 0,
    };
    let phase = match word(raw, 0x108) {
        0 => CannonPhase::Resting,
        1 => CannonPhase::Charging,
        2 => CannonPhase::Aiming,
        3 => CannonPhase::Firing,
        phase => anyhow::bail!("Corneria's cannon in phase {phase} at the boundary"),
    };
    ensure!(
        phase == CannonPhase::Resting,
        "Corneria's cannon charging or firing at the boundary needs its items' states"
    );
    let cannon = Cannon {
        phase,
        countdown: word(raw, 0x10C) as i32,
        wait: word(raw, 0x110) as i32,
        damage: float(raw, 0x114),
        shots: raw[0x118] as i8,
        laser: raw[0x119] as i8,
        side: raw[0x11A],
    };
    Ok((flags & 0x80 != 0, ship, cannon))
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
        "Corneria joint-attached particles need attachment restoration"
    );
    ensure!(
        word(saved.bytes(MULTI, 4), 0) == 0,
        "Corneria's 1P slots (grCn_804D69B0) at a versus boundary"
    );
    ensure!(
        word(saved.bytes(LOADING, 4), 0) == 0,
        "Corneria still loading (grCn_804D69AC) at the boundary"
    );
    let passes = saved.bytes(PASSES, 0xC);
    ensure!(
        (0..3).all(|slot| word(passes, slot * 4) == 0),
        "Corneria's Arwings at the boundary need importing"
    );
    let kinds = saved.bytes(TYPES, 0xC);
    let groups = saved.bytes(GROUPS, 0xC);
    let spawner = Spawner {
        wait: word(saved.bytes(WAIT, 4), 0) as i32,
        kinds: std::array::from_fn(|slot| word(kinds, slot * 4) as i32),
        groups: std::array::from_fn(|slot| word(groups, slot * 4) as i32),
        slot: word(saved.bytes(SLOT, 4), 0) as usize,
    };
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Corneria map order {maps:?}"
    );
    let mut animations = Animations::new();
    let mut state = None;
    let mut city = None;
    for ground in &grounds {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        animation.set_gobj_scale(saved_vec3(saved, ground.jobj + JOBJ_SCALE));
        let translation = saved_vec3(saved, ground.jobj + JOBJ_TRANSLATE);
        animation.set_gobj_translate(translation);
        if key == CITY_MAP {
            // grCorneria_801DFC2C plays no animation.
            animation.clear_animation();
            let raw = saved.bytes(ground.ground, 0xE4);
            ensure!(
                raw[0xC4] == 0 && raw[0xE0] & 0x80 == 0,
                "Corneria's city frozen at the boundary"
            );
            city = Some(Background {
                x: translation.x,
                passed: raw[0xC5] as i8 != 0,
            });
        }
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        if key == SHIP_MAP {
            state = Some(great_fox(saved.bytes(ground.ground, 0x130))?);
        }
        animations.insert(key, animation);
    }
    let (waiting, great_fox, cannon) = state.expect("map 3 in the order");
    let city = city.expect("map 8 in the order");
    ensure!(
        !city.passed,
        "Corneria's coast at the boundary needs importing"
    );
    reserve_later_maps(assets, &mut animations)?;
    let translation = great_fox.translation();
    let (arena, camera) =
        crate::scene_stage::corneria::carry_markers(assets, &mut animations, translation)?;
    let stage = CorneriaStage {
        parameters: read_parameters(assets)?,
        waiting,
        great_fox,
        cannon,
        cannon_items: [None; 2],
        spawner,
        backgrounds: [Some(city), None, None],
        flight: None,
        pending_shot: None,
    };
    Ok((
        SceneStage::Corneria(Box::new(Corneria {
            stage,
            arena,
            camera,
        })),
        animations,
    ))
}

/// The Great Fox's joints as the last Ground_801C2FE0 left them, from its
/// root's saved matrix; the platforms' joints off, as the stage's creation
/// left them.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    let ship = grounds
        .iter()
        .find(|g| g.map == u32::from(SHIP_MAP))
        .ok_or_else(|| anyhow::anyhow!("Corneria map {SHIP_MAP} missing"))?;
    let joints = model_joints(saved, ship.jobj);
    for binding in melee_gr::corneria::procs::bindings(SHIP_MAP) {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    for joint in PLATFORM_JOINTS {
        map.joint_list_remove(joint);
    }
    Ok(())
}
