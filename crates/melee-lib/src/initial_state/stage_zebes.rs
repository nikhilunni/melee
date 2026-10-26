//! Brinstar at a saved boundary: the maps and their controllers (`u` of
//! maps 1, 4, 6, 7 and 8), the bubbles and their points (grZe_8049F140,
//! grZe_8049F158, grZe_8049F170), the burst cooldown (grZe_804D6994), the
//! splash count (grZe_804D6998), the stage's floor (stage_info.x724 and
//! x728) and the acid's collision device (ft_80459A8C).
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
        zebes::{load, pulse_ticks, read_parameters, Animations, SPLASH_KEYS},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use hsd_types::Vec3;
use melee_gr::zebes::{
    acid::{Acid, AcidPhase},
    bubbles::{Bubble, BubbleObject, BubbleState, Bubbles, Points, BUBBLES},
    island::{Island, IslandPhase},
    plates::{Plate, PlatePhase},
    procs::*,
    scenery::{AcidReaction, Ornament},
};

/// grZe_8049F140 (the bounds), grZe_8049F158 (the anchors), grZe_8049F170
/// (the bubbles, 0x24 bytes each).
const BOUNDS: u32 = 0x8049_F140;
const ANCHORS: u32 = 0x8049_F158;
const BUBBLE_TABLE: u32 = 0x8049_F170;
const BUBBLE_SIZE: u32 = 0x24;
/// grZe_804D6994 (s32), grZe_804D6998 (s16).
const COOLDOWN: u32 = 0x804D_6994;
const SPLASH_COUNT: u32 = 0x804D_6998;
/// stage_info (0x8049E6C8) +0x724 and +0x728.
const FLOOR: u32 = 0x8049_E6C8 + 0x724;
const PREVIOUS_FLOOR: u32 = 0x8049_E6C8 + 0x728;
/// ft_80459A8C[0].ground: the stage's collision device.
const COLLISION_DEVICE: u32 = 0x8045_9A8C;

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([raw[at], raw[at + 1]])
}

fn vec3(saved: &SavedPose, at: u32) -> Vec3 {
    let raw = saved.bytes(at, 12);
    Vec3::new(float(raw, 0), float(raw, 4), float(raw, 8))
}

/// A `grZe_AcidState` at `at` in a Ground.
fn plate(raw: &[u8], at: usize) -> Result<Plate> {
    let state = raw[at];
    let phase = match state {
        0xFF => None,
        state => Some(
            PlatePhase::from_saved(state)
                .ok_or_else(|| anyhow::anyhow!("Brinstar plate phase {state}"))?,
        ),
    };
    Ok(Plate {
        phase,
        next: PlatePhase::from_saved(raw[at + 1])
            .ok_or_else(|| anyhow::anyhow!("Brinstar plate next phase {}", raw[at + 1]))?,
        timer: half(raw, at + 2),
        base_x: float(raw, at + 4),
        offset: float(raw, at + 8),
        velocity: float(raw, at + 0xC),
        damage: float(raw, at + 0x10),
    })
}

fn bubbles(saved: &SavedPose, cooldown: i32) -> Result<Bubbles> {
    let mut entries = [Bubble::FREE; BUBBLES];
    for (index, bubble) in entries.iter_mut().enumerate() {
        let raw = saved.bytes(
            BUBBLE_TABLE + index as u32 * BUBBLE_SIZE,
            BUBBLE_SIZE as usize,
        );
        let state = BubbleState::from_saved(raw[0])
            .ok_or_else(|| anyhow::anyhow!("Brinstar bubble {index} state {}", raw[0]))?;
        let pulse = word(raw, 4);
        let gobj = word(raw, 0x20);
        ensure!(
            state == BubbleState::Free || pulse == 0,
            "Brinstar bubble {index}'s pulse at the boundary needs its clock"
        );
        *bubble = Bubble {
            state,
            timer: half(raw, 2),
            pulse: None,
            x: float(raw, 8),
            y: float(raw, 0xC),
            vx: float(raw, 0x10),
            vy: float(raw, 0x14),
            size: float(raw, 0x18),
            target_size: float(raw, 0x1C),
            object: (state != BubbleState::Free && gobj != 0).then_some(BubbleObject::Initial),
        };
    }
    Ok(Bubbles {
        entries,
        points: Points {
            bounds: [vec3(saved, BOUNDS), vec3(saved, BOUNDS + 0xC)],
            anchors: [vec3(saved, ANCHORS), vec3(saved, ANCHORS + 0xC)],
        },
        cooldown,
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
        "Brinstar joint-attached particles need attachment restoration"
    );
    let parameters = read_parameters(assets)?;
    let grounds = ground_list(saved)?;
    let mut animations = Animations::new();
    let (mut reaction, mut island, mut ornament, mut acid) = (None, None, None, None);
    let (mut left_plate, mut right_plate) = (None, None);
    let (mut left_seen, mut right_seen) = (0, 0);
    let mut bubble_models = 0;
    let mut maps = Vec::new();
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x108);
        match key {
            BUBBLE_MAP => {
                bubble_models += 1;
                continue;
            }
            SPLASH_MAP => anyhow::bail!("Brinstar's splashes at the boundary need importing"),
            _ => maps.push(key),
        }
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        match key {
            REACTION_MAP => {
                reaction = Some(AcidReaction {
                    shaking: word(raw, 0xC4) != 0,
                });
            }
            LEFT_MAP => {
                let phase = half(raw, 0xC4);
                island = Some(Island {
                    phase: IslandPhase::from_saved(phase)
                        .ok_or_else(|| anyhow::anyhow!("Brinstar island phase {phase}"))?,
                    timer: half(raw, 0xC6),
                    risen: half(raw, 0xF6),
                    pulse_timer: half(raw, 0xF8),
                    loose_timer: half(raw, 0xF4),
                });
                left_plate = Some(plate(raw, 0xC8)?);
                left_seen = word(raw, 0xEC) as i32;
            }
            RIGHT_MAP => {
                right_plate = Some(plate(raw, 0xC4)?);
                right_seen = word(raw, 0xE8) as i32;
            }
            ORNAMENT_MAP => {
                ornament = Some(Ornament {
                    waiting: raw[0xC4] & 0x80 != 0,
                    next: half(raw, 0xCC),
                    timer: half(raw, 0xCE),
                });
                if word(saved.bytes(joints[0] + super::stage_izumi::JOBJ_AOBJ, 4), 0) == 0 {
                    animation.clear_animation();
                }
            }
            ACID_MAP => {
                let phase = word(raw, 0xC8) as i32;
                acid = Some(Acid {
                    row: half(raw, 0xC4),
                    timer: half(raw, 0xC6),
                    phase: AcidPhase::from_saved(phase)
                        .ok_or_else(|| anyhow::anyhow!("Brinstar acid phase {phase}"))?,
                    max_speed: float(raw, 0xCC),
                    speed: float(raw, 0xD0),
                    target: float(raw, 0xD4),
                    level: float(raw, 0xD8),
                });
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    ensure!(maps == MAP_ORDER.to_vec(), "Brinstar map order {maps:?}");
    let island = island.unwrap();
    ensure!(
        island.phase == IslandPhase::Standing,
        "Brinstar's top platform at the boundary needs its animation clock"
    );
    let left_plate = left_plate.unwrap();
    let right_plate = right_plate.unwrap();
    ensure!(
        [left_plate, right_plate]
            .iter()
            .all(|p| matches!(p.phase, None | Some(PlatePhase::Whole))),
        "Brinstar's plates at the boundary need their animation clocks"
    );
    for key in SPLASH_KEYS {
        let mut splash = load(assets, SPLASH_MAP)?;
        splash.set_hidden(true);
        animations.insert(key, splash);
    }
    let bubbles = bubbles(saved, word(saved.bytes(COOLDOWN, 4), 0) as i32)?;
    ensure!(
        bubbles
            .entries
            .iter()
            .filter(|b| b.object.is_some())
            .count()
            == bubble_models,
        "Brinstar's bubble models disagree with their table"
    );
    let splashes = half(saved.bytes(SPLASH_COUNT, 2), 0);
    ensure!(
        splashes == 0,
        "Brinstar's splashes at the boundary need importing"
    );
    let stage = ZebesStage {
        parameters,
        acid: acid.unwrap(),
        floor: float(saved.bytes(FLOOR, 4), 0),
        previous_floor: float(saved.bytes(PREVIOUS_FLOOR, 4), 0),
        left_plate,
        right_plate,
        left_plate_seen: left_seen,
        right_plate_seen: right_seen,
        island,
        bubbles,
        ornament: ornament.unwrap(),
        reaction: reaction.unwrap(),
        splashes,
        acid_device: word(saved.bytes(COLLISION_DEVICE, 4), 0) != 0,
        left_plate_item: None,
        right_plate_item: None,
        bubble_item: None,
        pulse_ticks: pulse_ticks(assets)?,
        live_splashes: Default::default(),
        splash_cursor: 0,
    };
    Ok((SceneStage::Zebes(Box::new(stage)), animations))
}

/// The parts' joints as the last Ground_801C2FE0 left them, from the saved
/// bone matrices; joint 0 (the chain's line) from the saved vertices.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    for (key, bindings) in [
        (LEFT_MAP, &LEFT_BINDINGS[..]),
        (RIGHT_MAP, &RIGHT_BINDINGS[..]),
    ] {
        let ground = grounds
            .iter()
            .find(|g| g.map == u32::from(key))
            .ok_or_else(|| anyhow::anyhow!("Brinstar map {key} missing"))?;
        let joints = model_joints(saved, ground.jobj);
        for binding in bindings {
            let jobj = joints[binding.extra as usize];
            let state = melee_mp::JobjState {
                mtx: saved_mtx(saved, jobj),
                hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
            };
            map.update_joint_transform(i32::from(binding.joint_index), Some(state));
        }
    }
    map.joint_set_b10(CHAIN_JOINT);
    restore_chain(saved, map)?;
    restore_vertices(saved, map, "the chain's line")
}

/// The chain's six vertices as grZebes_801D881C last set them.
fn restore_chain(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    const COLL_VTX: u32 = 0x804D_64B8;
    let base = word(saved.bytes(COLL_VTX, 4), 0);
    for index in 0..melee_gr::zebes::bubbles::columns::VERTICES as u32 {
        let raw = saved.bytes(base + index * 0x18, 0x18);
        map.vtx_set_pos(index as i32, float(raw, 8), float(raw, 12));
    }
    Ok(())
}
