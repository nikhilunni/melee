//! Onett at a saved boundary: the five maps, their clocks, and the
//! controllers as creation leaves them (only a boundary from before the
//! stage starts is imported: the cars parked, the roof whole, no smoke).
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
        onett::{load, read_parameters, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::onett::{
    cars::{Cars, RIGHT_END},
    procs::{OnettStage, AWNING_BONES, CAR_BONES, CAR_MAP, MAP_ORDER, ROOF_MAP, STREET_MAP},
    Awning, Roof, Smoke,
};

/// Map 5: the awnings (stride 0x1C), the smoke timer and generator, the
/// subject.
const AWNINGS: usize = 0xC4;
const AWNING_SIZE: usize = 0x1C;
const SMOKE_TIMER: usize = 0xFC;
const SMOKE_GENERATOR: usize = 0x100;
const SUBJECT: usize = 0x104;
/// Map 4: the roof's state and next state (s16), hit count, frame.
const ROOF_STATE: usize = 0xC4;
/// Map 3: x0_b0, the lanes' cars and states.
const CAR_FLAGS: usize = 0xC4;
const CURRENT_CAR: usize = 0x104;
const STATE_A: usize = 0x105;
const NEXT_CAR: usize = 0x118;
const STATE_B: usize = 0x119;

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([raw[at], raw[at + 1]])
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
        "Onett joint-attached particles need attachment restoration"
    );
    let parameters = read_parameters(assets)?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(maps == MAP_ORDER.map(u32::from), "Onett map order {maps:?}");
    let mut animations = Animations::new();
    let mut stage = None;
    let (mut awnings, mut smoke, mut roof) = (None, None, None);
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x130);
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        match key {
            STREET_MAP => {
                ensure!(
                    word(raw, SMOKE_GENERATOR) == 0 && word(raw, SUBJECT) == 0,
                    "Onett's smoke or lane A's subject at the boundary needs importing"
                );
                let mut pair = [Awning::new(0.0); 2];
                for (index, awning) in pair.iter_mut().enumerate() {
                    let at = AWNINGS + index * AWNING_SIZE;
                    *awning = Awning::new(float(raw, at + 4));
                    ensure!(
                        float(raw, at + 8) == 0.0 && float(raw, at + 0xC) == 0.0 && float(raw, at + 0x10) == 0.0,
                        "Onett awning {index} moving at the boundary"
                    );
                }
                ensure!(
                    pair.iter().zip(AWNING_BONES).all(|(a, bone)| a.initial_y == animation.joint_translation(bone).y),
                    "Onett awning heights differ from the model"
                );
                awnings = Some(pair);
                smoke = Some(Smoke {
                    timer: word(raw, SMOKE_TIMER) as i32,
                    puffing: false,
                });
            }
            ROOF_MAP => {
                let saved_roof = Roof {
                    state: i32::from(half(raw, ROOF_STATE)),
                    next_state: i32::from(half(raw, ROOF_STATE + 2)),
                    hit_count: word(raw, ROOF_STATE + 4) as i32,
                    frame: word(raw, ROOF_STATE + 8) as i32,
                    timer: 0,
                };
                ensure!(
                    matches!(saved_roof.state, -1 | 0) && saved_roof.next_state == 0 && saved_roof.hit_count == 0,
                    "Onett's roof {saved_roof:?} at the boundary needs its clocks"
                );
                roof = Some(saved_roof);
            }
            CAR_MAP => {
                ensure!(
                    raw[CAR_FLAGS] & 0x80 != 0
                        && raw[CURRENT_CAR] as i8 == 4
                        && raw[NEXT_CAR] as i8 == 4
                        && raw[STATE_A] == 0
                        && raw[STATE_B] == 0,
                    "Onett's cars at the boundary need their state imported"
                );
                for bone in CAR_BONES {
                    animation.set_joint_translate_x(bone, RIGHT_END);
                    animation.set_joint_hidden(bone, true);
                }
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    let cars = Cars {
        waiting: true,
        current: 4,
        next: 4,
        state_a: 0,
        state_b: 0,
        timer_a: 0,
        approach: 0,
        speed_a: 0.0,
        timer_b: 0,
        speed_b: 0.0,
        sub_state_b: 0,
    };
    stage.get_or_insert(OnettStage {
        parameters,
        awnings: awnings.unwrap(),
        smoke: smoke.unwrap(),
        subject: None,
        camera_phase: 0,
        roof: roof.unwrap(),
        cars,
        car_items: [None; 4],
        smoke_generator: None,
        lights: melee_gr::battle::lights::load_model(
            &assets.stage,
            &assets.stage_desc,
            usize::from(STREET_MAP),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?,
    });
    Ok((SceneStage::Onett(Box::new(stage.unwrap())), animations))
}

/// Maps 5's and 4's collision joints from the saved matrices, the roof's
/// floor callbacks recorded, then the saved vertices checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    for key in [STREET_MAP, ROOF_MAP] {
        let ground = grounds
            .iter()
            .find(|g| g.map == u32::from(key))
            .ok_or_else(|| anyhow::anyhow!("Onett map {key} missing"))?;
        let joints = model_joints(saved, ground.jobj);
        for binding in &assets.stage_desc.models[usize::from(key)].joint_mappings {
            let jobj = joints[binding.extra as usize];
            let state = melee_mp::JobjState {
                mtx: saved_mtx(saved, jobj),
                hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
            };
            map.update_joint_transform(i32::from(binding.joint_index), Some(state));
        }
    }
    for joint in melee_gr::onett::procs::ROOF_JOINTS {
        map.record_joint_events(joint, true);
    }
    restore_vertices(saved, map, "the roof or an awning")
}
