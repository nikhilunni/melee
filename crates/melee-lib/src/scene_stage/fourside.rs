//! Fourside composition (grfourside.c): the crane's arm and beam, the
//! helicopter and the UFO over their models and collision joints, in
//! grFourside_801F2D10's creation order.
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    fourside::{
        crane::{Crane, CraneEngine, IRON_BONE},
        procs::{
            bindings, COLLISION_MAP, CRANE_MAP, HELI_MAP, MAP_ORDER, ROOF_JOINT, UFO_MAP,
        },
        ufo::{Ufo, UfoEngine, BONE as UFO_BONE, JOINT as UFO_JOINT},
        Fourside, Heli, HeliStep,
    },
    last::animation::BackgroundAnimation,
};
use melee_types::mp::CollData;
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// The helicopter's bone whose height is tested (Ground_801C3FA4(gobj, 2)).
const HELI_BONE: usize = 2;
/// grFourside_801F3CC8: the arrival counts as descended this far above the
/// camera's top.
const HELI_DESCENT: f32 = 50.0;
/// The helicopter's animations: arrive, (unused), leave.
const HELI_ARRIVE: usize = 0;
const HELI_LEAVE: usize = 2;
/// `CollData.x34_flags.b1234` of a fighter (ft_081B.c:50).
const FIGHTER_COLLISION: u8 = 1;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Fourside map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// `lb_8000B1CC(jobj, NULL, &out)` (0x8000B1CC): a bone's world translation.
fn world_position(animation: &mut BackgroundAnimation, bone: usize) -> Vec3 {
    let matrix = animation.joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// grFourside_801F30A0 (0x801F30A0), the roof joint's `cb_0`: a fighter on
/// it counts towards the UFO's odds (map 5's x8).
pub(crate) fn count_roof_contact(
    contacts: &mut u32,
    _joint: i32,
    coll: &mut CollData,
    _coll_x50: i32,
    _ground_kind: i32,
    _delta_y: f32,
) {
    if coll.x34_flags.b1234 == FIGHTER_COLLISION {
        *contacts = contacts.wrapping_add(1);
    }
}

/// Ground_801C2ED0 (0x801C2ED0): bind `map`'s collision joints from the
/// pose before any animation, transform them and take that as the previous
/// position too.
fn bind(animation: &mut BackgroundAnimation, map: u8, collision: &mut melee_mp::CollMap) {
    let bindings = bindings(map);
    animation.update_collision(collision, bindings);
    for binding in bindings {
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// The models and controllers as grFourside_801F2D10 leaves them, without
/// the draws (`rng` draws the crane's and the helicopter's waits).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_fourside_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut animations = Animations::new();
    let mut crane = None;
    let mut heli = None;
    for key in MAP_ORDER {
        let mut animation = load(assets, key)?;
        match key {
            CRANE_MAP => {
                // grFourside_801F3154: bound, then grAnime_801C7FF8(gobj, 0,
                // 7, 0, 0.0, 0.0): animation 0 requested and stopped, not
                // evaluated.
                bind(&mut animation, key, collision);
                animation.set_rate(0.0);
                let iron_y = animation.joint_translation(IRON_BONE).y;
                crane = Some(Crane::create(&parameters, iron_y, rng));
            }
            HELI_MAP => {
                // grFourside_801F3C40: hidden, no animation.
                animation.clear_animation();
                animation.set_hidden(true);
                heli = Some(Heli::create(&parameters, rng));
            }
            UFO_MAP => {
                // grFourside_801F37FC: bound, hidden, joint 4 disabled.
                animation.clear_animation();
                bind(&mut animation, key, collision);
                animation.set_hidden(true);
                collision.joint_list_remove(UFO_JOINT);
            }
            _ => {
                if key == COLLISION_MAP {
                    bind(&mut animation, key, collision);
                }
                // grAnime_801C8138(gobj, map, 0).
                ensure!(
                    animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
                    "Fourside map {key}: unexpected setup particle key"
                );
                if key == COLLISION_MAP {
                    collision.joint_set_cb1(ROOF_JOINT, 0, count_roof_contact);
                }
            }
        }
        animations.insert(key, animation);
    }
    let stage = Fourside {
        parameters,
        crane: crane.unwrap(),
        heli: heli.unwrap(),
        ufo: Ufo::create(&parameters),
    };
    Ok((SceneStage::Fourside(Box::new(stage)), animations))
}

struct Arm<'a>(&'a mut BackgroundAnimation);
impl CraneEngine for Arm<'_> {
    fn request_frame(&mut self, frame: f32) {
        self.0.request_frame(frame);
    }
    fn set_rate(&mut self, rate: f32) {
        self.0.set_rate(rate);
    }
}

struct Saucer<'a> {
    animation: &'a mut BackgroundAnimation,
    collision: &'a mut melee_mp::CollMap,
    assets: &'a Assets,
}
impl UfoEngine for Saucer<'_> {
    fn select(&mut self, index: usize) {
        select(self.animation, self.assets, UFO_MAP, index);
    }
    fn bind_joint(&mut self) {
        self.animation
            .update_collision(self.collision, bindings(UFO_MAP));
        self.collision.joint_snapshot_prev_pos(UFO_JOINT);
    }
    fn set_hidden(&mut self, hidden: bool) {
        self.animation.set_hidden(hidden);
    }
    fn set_joint_enabled(&mut self, enabled: bool) {
        if enabled {
            self.collision.joint_list_add(UFO_JOINT);
        } else {
            self.collision.joint_list_remove(UFO_JOINT);
        }
    }
    fn ended(&self) -> bool {
        self.animation.first_animation_ended()
    }
    fn rewound(&self) -> bool {
        self.animation.first_animation_rewound()
    }
    fn bone_position(&mut self) -> Vec3 {
        world_position(self.animation, UFO_BONE)
    }
}

/// grAnime_801C8138(gobj, map, index): attach and evaluate frame zero.
fn select(animation: &mut BackgroundAnimation, assets: &Assets, map: u8, index: usize) {
    animation
        .select_animation(
            &assets.stage,
            &assets.stage_desc.models[usize::from(map)],
            index,
        )
        .expect("a Fourside animation");
    assert!(
        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
        "Fourside map {map}: unexpected particle key"
    );
}

/// A Fourside gobj proc (s_link 4). Map 6's lb_800115F4 runs in the scene
/// dispatch; maps 0, 2 and 4 have empty procs.
pub(crate) fn run_proc(state: &mut InitialState, key: u8) -> Result<()> {
    let SceneStage::Fourside(stage) = &mut state.stage else {
        unreachable!()
    };
    let assets = std::sync::Arc::clone(&state.assets);
    match key {
        COLLISION_MAP => {
            // Ground_801C2FE0: the roof never moves.
            let animation = state.stage_animations.get_mut(&key).unwrap();
            animation.update_collision(&mut state.map, bindings(key));
        }
        CRANE_MAP => {
            let animation = state.stage_animations.get_mut(&key).unwrap();
            stage
                .crane
                .tick(&stage.parameters, &mut state.rng, &mut Arm(animation));
            animation.set_joint_translate_y(IRON_BONE, stage.crane.y);
            animation.update_collision(&mut state.map, bindings(key));
        }
        HELI_MAP => {
            let animation = state.stage_animations.get_mut(&key).unwrap();
            let ended = animation.first_animation_ended();
            let camera_top = assets.arena.camera_top;
            let ufo_away = stage.ufo.away_or_far();
            let step = {
                let below_top = || {
                    // retail: fadds 50 + Stage_GetCamBoundsTopOffset.
                    world_position(animation, HELI_BONE).y <= HELI_DESCENT + camera_top
                };
                stage.heli.tick(&stage.parameters, ufo_away, below_top, ended, &mut state.rng)
            };
            let animation = state.stage_animations.get_mut(&key).unwrap();
            match step {
                HeliStep::None => {}
                HeliStep::Arrive => {
                    animation.set_hidden(false);
                    select(animation, &assets, key, HELI_ARRIVE);
                }
                HeliStep::Leave => select(animation, &assets, key, HELI_LEAVE),
                HeliStep::Hide => animation.set_hidden(true),
            }
        }
        UFO_MAP => {
            stage.ufo.roof_contacts = state.map.joint_cb1_data(ROOF_JOINT) as i32;
            let was_framed = stage.ufo.subject.is_some();
            let heli = stage.heli;
            let animation = state.stage_animations.get_mut(&key).unwrap();
            let mut engine = Saucer {
                animation,
                collision: &mut state.map,
                assets: &assets,
            };
            stage.ufo.tick(
                &stage.parameters,
                || heli.busy(),
                assets.arena.camera_top,
                &mut state.rng,
                &mut engine,
            );
            if !was_framed && stage.ufo.subject.is_some() {
                // Camera_80029020 links the subject after every item's.
                ensure!(
                    state.items.iter().all(|item| item.camera.is_none()),
                    "grFourside_801F3894: the UFO's camera subject would follow a framed item's"
                );
            }
            // Ground_801C2FE0, then x8 = 0.
            let animation = state.stage_animations.get_mut(&key).unwrap();
            animation.update_collision(&mut state.map, bindings(key));
            stage.ufo.end_step();
            state.map.set_joint_cb1_data(ROOF_JOINT, 0);
        }
        _ => {}
    }
    Ok(())
}
