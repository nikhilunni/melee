//! Great Bay at a saved boundary: the eleven maps and their clocks, the
//! controllers as creation leaves them (the platform at rest, the turtle
//! and moon waiting) and Tingle waiting high above (only a boundary from
//! before the stage starts is imported).
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS,
        JOBJ_HIDDEN,
    },
    word,
};

/// HSD_GObj's entity lists (`HSD_GObj_Entities`).
const GOBJ_ENTITIES: u32 = 0x804D_782C;
use crate::{
    assets::Assets,
    scene_stage::{
        greatbay::{load, read_parameters, stage, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::greatbay::{
    procs::{Moon, GOBJ_ORDER, MOON_MAP, STAGE_MAP, TURTLE_MAP},
    turtle::{Phase, Turtle},
    FloatFloor,
};

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
        "Great Bay joint-attached particles need attachment restoration"
    );
    let parameters = read_parameters(assets)?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(maps == GOBJ_ORDER.map(u32::from), "Great Bay map order {maps:?}");
    let mut animations = Animations::new();
    let (mut float_floor, mut turtle, mut moon) = (None, None, None);
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x100);
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        match key {
            STAGE_MAP => {
                // grGreatBay_GroundVars3: the rest translation, the targets
                // and the eased offsets.
                let floor = FloatFloor {
                    rest_x: float(raw, 0xC4),
                    rest_y: float(raw, 0xC8),
                    slant_target: float(raw, 0xCC),
                    slide_target: float(raw, 0xD0),
                    down_target: float(raw, 0xD4),
                    slant: float(raw, 0xD8),
                    slide: float(raw, 0xDC),
                    down: float(raw, 0xE0),
                };
                ensure!(
                    floor.slant == 0.0 && floor.slide == 0.0 && floor.down == 0.0,
                    "Great Bay's floating platform moving at the boundary"
                );
                float_floor = Some(floor);
            }
            TURTLE_MAP => {
                // grGreatBay_801F4694's placing of the model.
                let s = animation.gobj_scale();
                let k = parameters.turtle_scale;
                animation.set_gobj_scale(hsd_types::Vec3::new(s.x * k, s.y * k, s.z * k));
                let mut t = animation.gobj_translation();
                t.x = parameters.turtle_x + parameters.turtle_x_offset_init;
                t.y = parameters.turtle_y;
                animation.set_gobj_translate(t);
                ensure!(
                    raw[0xC4] == 0 && word(raw, 0xC8) == 0,
                    "Great Bay's turtle at the boundary has left its first wait"
                );
                turtle = Some(Turtle {
                    phase: Phase::Surface,
                    facing: raw[0xC5] >> 1,
                    item_checked: raw[0xC5] & 1 != 0,
                    timer: half(raw, 0xC6),
                    wake: false,
                    body_y: float(raw, 0xCC),
                });
            }
            MOON_MAP => {
                moon = Some(Moon {
                    timer: half(raw, 0xD4),
                    started: [false; 4],
                });
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    let mut stage = stage(
        assets,
        parameters,
        float_floor.expect("map 2"),
        turtle.expect("map 1"),
        moon.expect("map 9"),
    )?;
    stage.tingle_draws = Some(saved_tingle(saved, assets)?);
    Ok((SceneStage::GreatBay(Box::new(stage)), animations))
}

/// Tingle at the boundary: the one item, waiting (state 0); his place and
/// his wait stand in for it_802EB6DC's draws.
fn saved_tingle(saved: &SavedPose, assets: &Assets) -> Result<(f32, i32, i32)> {
    let entities = word(saved.bytes(GOBJ_ENTITIES, 4), 0);
    let gobj = word(saved.bytes(entities + 9 * 4, 4), 0);
    ensure!(gobj != 0, "Tingle's item is missing");
    let object = saved.bytes(gobj, 0x30);
    ensure!(word(object, 8) == 0, "Great Bay boundary has other items");
    let item = saved.bytes(word(object, 0x2C), 0xE50);
    ensure!(
        word(item, 0x10) == melee_types::ItemKind::Tincle as u32 && word(item, 0x24) == 0,
        "Tingle at the boundary is not waiting"
    );
    let tingle = assets.items.get(melee_types::ItemKind::Tincle);
    let left = tingle.special_attributes[3];
    let x = float(item, 0x4C) - left;
    ensure!(x == x.trunc(), "Tingle's place {x} is not it_802EB6DC's");
    Ok((1.0, x as i32, word(item, 0xDF4) as i32))
}

/// The collision joints from the saved matrices (grGb_803E3E60), the
/// turtle's head off as creation leaves it, the platform's and Tingle's
/// callbacks recorded; the saved vertices checked.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    for binding in melee_gr::greatbay::procs::STAGE_JOINTS {
        let ground = grounds
            .iter()
            .find(|g| g.map == binding.target_index as u32)
            .ok_or_else(|| anyhow::anyhow!("Great Bay map {} missing", binding.target_index))?;
        let joints = model_joints(saved, ground.jobj);
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    map.joint_list_remove(1);
    map.record_joint_events(melee_gr::greatbay::procs::FLOAT_JOINT, true);
    map.record_joint_events(melee_gr::greatbay::procs::TINGLE_JOINT, true);
    restore_vertices(saved, map, "the turtle, the platform or Tingle")
}
