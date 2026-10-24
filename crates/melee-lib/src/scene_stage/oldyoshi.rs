//! Yoshi's Island N64 composition: the clouds' engine operations (material
//! animations, collision joints, the puff) and the guest's model, in
//! grOldYoshi_8020E79C's creation order.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::{rng_sites::DrawLog, system::SpawnRequest};
use hsd_types::Mtx;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    oldyoshi::{
        procs::{CLOUD_MAP, GUEST_MAP, MAP_ORDER},
        CloudEngine, GuestStep, OldYoshi, CLOUDS,
    },
};
use melee_types::mp::CollData;
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grLib_801C97DC(30000, 30, jobj): the cloud puff, stage bank 30.
const STAGE_PARTICLE_BANK: u8 = 30;
const PUFF_PARTICLE: u32 = 30000;
/// `CollData.x34_flags.b1234` of a fighter (ft_081B.c:50).
const FIGHTER_COLLISION: u8 = 1;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Yoshi's Island N64 map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// fn_8020F2A8 (0x8020F2A8), each cloud joint's `cb_0`: a fighter on the
/// cloud sets the cloud's xC4_4 (the joint's callback word here).
pub(crate) fn carry(
    carried: &mut u32,
    _joint: i32,
    coll: &mut CollData,
    _coll_x50: i32,
    _ground_kind: i32,
    _delta_y: f32,
) {
    if coll.x34_flags.b1234 == FIGHTER_COLLISION {
        *carried = 1;
    }
}

/// Ground_801C2ED0 for map 2 (0x801C2ED0): bind each cloud's collision
/// joint to its JObj from the pose before any animation, transform it and
/// take that as the previous position too.
fn bind_clouds(clouds: &mut BackgroundAnimation, assets: &Assets, map: &mut melee_mp::CollMap) {
    let bindings = &assets.stage_desc.models[usize::from(CLOUD_MAP)].joint_mappings;
    clouds.update_collision(map, bindings);
    for binding in bindings {
        map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// grOldYoshi_8020E79C (0x8020E79C): the cold setup. Maps 0, 1, 4, 5 and
/// 2 evaluate animation 0's frame zero (grAnime_801C8138); map 2 binds its
/// collision first and registers each cloud's callback; the guest (map 3)
/// hides and draws its first wait.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    map: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_old_yoshi_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut animations = Animations::new();
    let mut rest_y = [0.0; 3];
    for key in MAP_ORDER {
        let mut animation = load(assets, key)?;
        if key == GUEST_MAP {
            // grOldYoshi_8020EFCC: hidden, and no animation until the
            // guest's proc first restarts it.
            animation.clear_animation();
            animation.set_hidden(true);
            animations.insert(key, animation);
            continue;
        }
        if key == CLOUD_MAP {
            bind_clouds(&mut animation, assets, map);
        }
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "Yoshi's Island N64 map {key}: unexpected setup particle key"
        );
        if key == CLOUD_MAP {
            for (index, cloud) in CLOUDS.iter().enumerate() {
                rest_y[index] = animation.joint_translation(cloud.bone).y;
                map.joint_set_cb1(cloud.joint, 0, carry);
            }
        }
        animations.insert(key, animation);
    }
    let stage = OldYoshi::create(parameters, rest_y, rng);
    Ok((SceneStage::OldYoshi(stage), animations))
}

/// The cloud steps' engine half over map 2's model and the collision map.
/// Puffs are spawned after the step with the matrix their cloud had when
/// asked: nothing else in the step draws the RNG.
struct Clouds<'a> {
    animation: &'a mut BackgroundAnimation,
    map: &'a mut melee_mp::CollMap,
    puffs: Vec<(usize, Mtx)>,
}
impl CloudEngine for Clouds<'_> {
    fn materials_ended(&mut self, bone: usize) -> bool {
        self.animation.material_subtree_ended(bone)
    }
    fn play_materials(&mut self, bone: usize, animation: usize) {
        self.animation.play_material_subtree(bone, animation);
    }
    fn spawn_puff(&mut self, bone: usize) {
        let matrix = self.animation.joint_matrix(bone);
        self.puffs.push((bone, matrix));
    }
    fn remove_joint(&mut self, joint: i32) {
        self.map.joint_list_remove(joint);
    }
    fn add_joint(&mut self, joint: i32) {
        self.map.joint_list_add(joint);
    }
    fn set_hidden(&mut self, bone: usize, hidden: bool) {
        self.animation.set_joint_hidden(bone, hidden);
    }
    fn set_translate_y(&mut self, bone: usize, y: f32) {
        self.animation.set_joint_translate_y(bone, y);
    }
}

/// A Yoshi's Island N64 gobj proc (s_link 4). Map 4's lb_800115F4 runs in
/// the scene dispatch.
pub(crate) fn run_proc(state: &mut InitialState, key: u8, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::OldYoshi(stage) = &mut state.stage else {
        unreachable!()
    };
    match key {
        CLOUD_MAP => {
            let carried = CLOUDS.map(|cloud| {
                let carried = state.map.joint_cb1_data(cloud.joint) != 0;
                state.map.set_joint_cb1_data(cloud.joint, 0);
                carried
            });
            let animation = state.stage_animations.get_mut(&CLOUD_MAP).unwrap();
            let mut engine = Clouds {
                animation,
                map: &mut state.map,
                puffs: Vec::new(),
            };
            stage.tick_clouds(carried, &mut engine);
            let puffs = engine.puffs;
            // Ground_801C2FE0 ends the proc.
            let bindings = &state.assets.stage_desc.models[usize::from(CLOUD_MAP)].joint_mappings;
            state
                .stage_animations
                .get_mut(&CLOUD_MAP)
                .unwrap()
                .update_collision(&mut state.map, bindings);
            for (bone, matrix) in puffs {
                // grLib_801C97DC (0x801C97DC) -> hsd_8039F6CC: a detached
                // generator at the cloud JObj's world position.
                let mut request = SpawnRequest::new(STAGE_PARTICLE_BANK, PUFF_PARTICLE, 0);
                request.joint = Some((joint_id(CLOUD_MAP, bone), matrix));
                state.effects.events.spawn(&request, true, false);
                let id = state.particles.spawn::<RetailTrig>(
                    &state.assets.particle_bank,
                    request,
                    &mut state.rng,
                    draws,
                )?;
                state.particles.pending_generators.push(id);
            }
            let animation = state.stage_animations.get_mut(&CLOUD_MAP).unwrap();
            super::publish_joint_matrices(
                animation,
                CLOUD_MAP,
                &mut state.particles,
                &mut state.effects.events,
            );
        }
        GUEST_MAP => {
            let animation = state.stage_animations.get_mut(&GUEST_MAP).unwrap();
            let ended = animation.first_animation_ended();
            match stage.tick_guest(ended, &mut state.rng) {
                GuestStep::Restart { height } => {
                    animation
                        .select_animation(
                            &state.assets.stage,
                            &state.assets.stage_desc.models[usize::from(GUEST_MAP)],
                            0,
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    ensure!(
                        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
                        "Yoshi's Island N64 guest: unexpected particle key"
                    );
                    animation.set_wrapper_translate_y(height);
                }
                GuestStep::Shown { bone, ended } => {
                    if animation.joint_hidden(bone) {
                        animation.set_joint_hidden(bone, false);
                    }
                    if ended {
                        animation.set_joint_hidden(bone, true);
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
