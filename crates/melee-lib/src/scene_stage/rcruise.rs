//! Rainbow Cruise composition (grrcruise.c): the two map-3 GObjs (the
//! camera path and the scroll controller, keyed 3 and 7), maps 0, 2, 5, 1,
//! 6, 4 in grRCruise_801FF168's order, the scroll that moves them, and the
//! ship's parts' contacts.
use super::kraid::spawn_keys;
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::Result;
use gekko_math::HsdRng;
use hsd_particle::{rng_sites::DrawLog, system::ParticleSystem};
use hsd_types::Vec3;
use melee_gr::{
    last::animation::BackgroundAnimation,
    rcruise::{
        parts::{
            step_vanish, Cloud, CloudEngine, Swing, SwingEngine, Tilt, VanishEngine, CLOUD_BONES,
            DECK_JOINT, SWING_ANIMATION, SWING_BONES, TILT_BONE, VANISH, VANISH_MARGIN,
        },
        procs::{
            model, project, ship_bindings, ship_joint_of, RCruiseStage, BACKDROP_DEPTH, DISABLED_JOINT, EYE_BONE,
            FLAG_MAP, INTEREST_BONE, KEY_ORDER, PATH_KEY, SCROLLED, SCROLL_KEY, SHIP_COPY_MAP,
            SHIP_MAP, SPAWN_BONES, SWING_MAP,
        },
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grRCruise_801FF73C: map 5's one bone left shown.
const SHIP_COPY_SHOWN: usize = 31;
/// The appearing blocks' material animations (grAnime_801C7FF8(gobj,
/// bone, 2, index)): 1 away, 2 coming in, 3 going.
const VANISH_AWAY: usize = 1;

pub(crate) fn load(assets: &Assets, key: u8) -> Result<BackgroundAnimation> {
    let map = model(key);
    let desc = &assets.stage_desc.models[usize::from(map)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, desc)
        .map_err(|e| anyhow::anyhow!("Rainbow Cruise map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    if map == SWING_MAP {
        for bone in SWING_BONES {
            animation
                .prepare_subtree(&assets.stage, desc, bone, SWING_ANIMATION)
                .map_err(|e| anyhow::anyhow!("Rainbow Cruise swing {bone}: {e}"))?;
        }
    }
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<melee_gr::rcruise::Parameters> {
    melee_gr::desc::read_rcruise_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

/// The collision joints `key`'s model moves: map 1's from the stage
/// table and the archive (`ship`), the others' from the archive.
fn bindings<'a>(assets: &'a Assets, ship: &'a [melee_gr::desc::JointMapping], key: u8) -> &'a [melee_gr::desc::JointMapping] {
    if key == SHIP_MAP {
        ship
    } else {
        archive_bindings(assets, key)
    }
}

fn archive_bindings(assets: &Assets, key: u8) -> &[melee_gr::desc::JointMapping] {
    &assets.stage_desc.models[usize::from(model(key))].joint_mappings
}

/// The joint bound to `bone` of `key`'s model (Ground_801C32D4).
fn joint_of(assets: &Assets, key: u8, bone: usize) -> i32 {
    let archive = archive_bindings(assets, key);
    let joint = if key == SHIP_MAP {
        ship_joint_of(archive, bone)
    } else {
        archive
            .iter()
            .rev()
            .find(|b| b.extra as usize == bone)
            .map(|b| i32::from(b.joint_index))
    };
    joint.expect("a bound bone")
}

/// Ground_801C2ED0: the archive joints from the pose before any
/// animation, that pose the previous one too.
fn bind(
    animation: &mut BackgroundAnimation,
    bindings: &[melee_gr::desc::JointMapping],
    collision: &mut melee_mp::CollMap,
) {
    animation.update_collision(collision, bindings);
    for binding in bindings {
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

fn world(model: &mut BackgroundAnimation, bone: usize) -> Vec3 {
    let matrix = model.joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// mpLib_80055E9C then mpLib_80057424 on the joint bound to `bone`.
fn snap(model: &mut BackgroundAnimation, collision: &mut melee_mp::CollMap, bone: usize, joint: i32) {
    let state = melee_mp::JobjState {
        mtx: model.joint_matrix(bone),
        hidden: model.joint_hidden(bone),
    };
    collision.update_joint_transform(joint, Some(state));
    collision.joint_snapshot_prev_pos(joint);
}

/// grRCruise_801FF168 (0x801FF168): the maps and their draws, the first
/// scroll, the moving maps' joints taken as they now stand, joint 11 off.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let draws = &mut DrawLog::default();
    let mut animations = Animations::new();
    let mut stage = RCruiseStage {
        parameters,
        scroll: Vec3::ZERO,
        drift: Vec3::ZERO,
        tilt: Tilt::default(),
        clouds: [Cloud::new(0, 0.0); 17],
        swings: [Swing::new(0); 3],
        vanish: [0; 20],
        waiting: true,
        gusts: Default::default(),
        ship_bindings: ship_bindings(archive_bindings(assets, SHIP_MAP)),
    };
    for key in KEY_ORDER {
        let mut animation = load(assets, key)?;
        match key {
            PATH_KEY | SCROLL_KEY => {
                // grRCruise_801FF924: animation 0, every clock looping.
                spawn_keys(&mut animation, key, assets, particles, None, rng, draws)?;
                animation.loop_all();
                if key == PATH_KEY {
                    animation.set_hidden(true);
                }
            }
            SHIP_COPY_MAP => {
                // grRCruise_801FF73C: animation 0, hidden but for bone 31.
                spawn_keys(&mut animation, key, assets, particles, None, rng, draws)?;
                animation.set_hidden(true);
                animation.set_joint_hidden(SHIP_COPY_SHOWN, false);
            }
            SHIP_MAP => ship(&mut animation, &mut stage, assets, particles, rng, draws, collision)?,
            FLAG_MAP => {
                // grRCruise_801FF7A4: animation 0 looping; the flag's
                // dynamics are drawing only; bones 10..13 the spawn points.
                bind(&mut animation, archive_bindings(assets, key), collision);
                spawn_keys(&mut animation, key, assets, particles, None, rng, draws)?;
                animation.loop_all();
            }
            SWING_MAP => {
                // grRCruise_80200074: animation 0, then each swing's
                // animation 1 at frame 0, stopped.
                bind(&mut animation, archive_bindings(assets, key), collision);
                spawn_keys(&mut animation, key, assets, particles, None, rng, draws)?;
                for (index, bone) in SWING_BONES.into_iter().enumerate() {
                    animation.play_prepared_subtree_at(bone, SWING_ANIMATION, 0.0, 0.0);
                    let joint = joint_of(assets, key, bone);
                    collision.record_joint_events(joint, true);
                    stage.swings[index] = Swing::new(joint);
                }
            }
            _ => spawn_keys(&mut animation, key, assets, particles, None, rng, draws)?,
        }
        animations.insert(key, animation);
    }
    scroll(&mut stage, &mut animations, assets);
    for key in [SHIP_MAP, FLAG_MAP, SWING_MAP] {
        animations.get_mut(&key).unwrap().update_collision(collision, bindings(assets, &stage.ship_bindings, key));
        // Ground_801C32AC: mpLib_80057424 on each.
        for binding in bindings(assets, &stage.ship_bindings, key) {
            collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
        }
    }
    collision.joint_list_remove(DISABLED_JOINT);
    Ok((SceneStage::RCruise(Box::new(stage)), animations))
}

/// grRCruise_801FF5B4 (0x801FF5B4): map 1's creation.
fn ship(
    animation: &mut BackgroundAnimation,
    stage: &mut RCruiseStage,
    assets: &Assets,
    particles: &mut ParticleSystem,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
    collision: &mut melee_mp::CollMap,
) -> Result<()> {
    bind(animation, &stage.ship_bindings, collision);
    spawn_keys(animation, SHIP_MAP, assets, particles, None, rng, draws)?;
    // grRCruise_80201410: the appearing blocks.
    for (index, &(bone, joint, always)) in VANISH.iter().enumerate() {
        if always {
            stage.vanish[index] = 2;
            collision.joint_list_add(joint);
        } else {
            stage.vanish[index] = 0;
            animation.play_material_subtree(bone, VANISH_AWAY);
            collision.joint_list_remove(joint);
        }
    }
    // grRCruise_80200540 (the tilt), grRCruise_80200B48 (the clouds).
    stage.tilt = Tilt::default();
    for (index, bone) in CLOUD_BONES.into_iter().enumerate() {
        let base = animation.joint_translation(bone).y;
        stage.clouds[index] = Cloud::new(joint_of(assets, SHIP_MAP, bone), base);
    }
    animation.update_collision(collision, &stage.ship_bindings);
    // mpLib_80058560.
    collision.stitch_all_joints();
    Ok(())
}

/// grRCruise_801FFADC (0x801FFADC): the path's eye and interest projected,
/// the drift its change, the scroll controller and maps 2, 1, 5, 6, 4
/// translated to the negated projection plus the camera offset.
fn scroll(stage: &mut RCruiseStage, animations: &mut Animations, assets: &Assets) {
    let camera = &assets.stage_camera;
    let offset = Vec3::new(camera.offset_x, camera.offset_y, 0.0);
    let path = animations.get_mut(&PATH_KEY).unwrap();
    let eye = world(path, EYE_BONE);
    let interest = world(path, INTEREST_BONE);
    let point = project(eye, interest, offset);
    stage.drift = Vec3::new(
        stage.scroll.x - point.x,
        stage.scroll.y - point.y,
        stage.scroll.z - point.z,
    );
    stage.scroll = point;
    // fmuls by -1.0 (a sign flip), then lbVector_Add.
    let translate = Vec3::new(-point.x + offset.x, -point.y + offset.y, -point.z + offset.z);
    let backdrop = animations.get_mut(&SCROLL_KEY).unwrap();
    backdrop.set_gobj_translate(translate);
    let mut moved = backdrop.gobj_translation();
    moved.z += BACKDROP_DEPTH * assets.stage_desc.parameters.map_scale;
    backdrop.set_gobj_translate(moved);
    for map in SCROLLED {
        animations.get_mut(&map).unwrap().set_gobj_translate(translate);
    }
}

/// The players' spawn points (Ground_801C2D0C(0..3, map 6's bones 10..13)).
pub(crate) fn spawn_point(animations: &mut Animations, slot: i16) -> Option<Vec3> {
    let bone = *SPAWN_BONES.get(usize::try_from(slot).ok()?)?;
    Some(world(animations.get_mut(&FLAG_MAP)?, bone))
}

/// The deferred grRCruise_801FF444: the deck's and the clouds' floor
/// callbacks; the parts start.
pub(crate) fn start(state: &mut InitialState) {
    let SceneStage::RCruise(stage) = &mut state.stage else {
        unreachable!()
    };
    state.map.record_joint_events(DECK_JOINT, true);
    for cloud in &stage.clouds {
        state.map.record_joint_events(cloud.joint, true);
    }
    stage.waiting = false;
}

/// The floor callbacks a collider's proc made: the deck's weights
/// (grRCruise_80200578, against the pivot as it stands now), the clouds'
/// (grRCruise_802010A4) and the swings' (fn_80200460, which start at once).
pub(crate) fn apply_contacts(state: &mut InitialState) {
    let SceneStage::RCruise(stage) = &mut state.stage else {
        unreachable!()
    };
    if !stage.waiting {
        let events = state.map.take_joint_events(DECK_JOINT);
        if !events.is_empty() {
            let pivot = world(state.stage_animations.get_mut(&SHIP_MAP).unwrap(), TILT_BONE);
            for event in events.iter() {
                stage
                    .tilt
                    .stand(event.collision_type, event.x50, (event.x, event.y), (pivot.x, pivot.y));
            }
        }
        for cloud in &mut stage.clouds {
            for event in state.map.take_joint_events(cloud.joint).iter() {
                cloud.stand(event.collision_type);
            }
        }
    }
    for (index, &bone) in SWING_BONES.iter().enumerate() {
        let joint = stage.swings[index].joint;
        let events = state.map.take_joint_events(joint);
        let model = state.stage_animations.get_mut(&SWING_MAP).unwrap();
        let mut engine = SwingStep {
            model,
            collision: &mut state.map,
            bone,
        };
        for event in events.iter() {
            stage.swings[index].stand(index, event.collision_type, &mut engine);
        }
    }
}

/// The ship's engine for its clouds and appearing blocks.
struct ShipModel<'a> {
    model: &'a mut BackgroundAnimation,
    collision: &'a mut melee_mp::CollMap,
    camera: &'a melee_cm::StageCamera,
    joints: [i32; 17],
}

impl CloudEngine for ShipModel<'_> {
    fn translate_y(&self, cloud: usize) -> f32 {
        self.model.joint_translation(CLOUD_BONES[cloud]).y
    }
    fn set_translate_y(&mut self, cloud: usize, y: f32) {
        self.model.set_joint_translate_y(CLOUD_BONES[cloud], y);
    }
    fn show(&mut self, _cloud: usize, _shown: bool) {}
    fn enable_joint(&mut self, joint: i32) {
        self.collision.joint_list_add(joint);
    }
    fn disable_joint(&mut self, joint: i32) {
        self.collision.joint_list_remove(joint);
    }
    fn snap_joint(&mut self, joint: i32) {
        let cloud = self.joints.iter().position(|&j| j == joint).expect("a cloud's joint");
        snap(self.model, self.collision, CLOUD_BONES[cloud], joint);
    }
    fn camera_bottom(&self) -> f32 {
        self.camera.bottom()
    }
}

struct Blocks<'a> {
    model: &'a mut BackgroundAnimation,
    collision: &'a mut melee_mp::CollMap,
    camera: &'a melee_cm::StageCamera,
}

impl VanishEngine for Blocks<'_> {
    fn on_screen(&mut self, block: usize) -> bool {
        let position = world(self.model, VANISH[block].0);
        self.camera.contains_within(position, VANISH_MARGIN)
    }
    fn play(&mut self, block: usize, index: usize) {
        self.model.play_material_subtree(VANISH[block].0, index);
    }
    fn ended(&self, block: usize) -> bool {
        self.model.material_subtree_ended(VANISH[block].0)
    }
    fn enable_joint(&mut self, joint: i32) {
        self.collision.joint_list_add(joint);
    }
    fn disable_joint(&mut self, joint: i32) {
        self.collision.joint_list_remove(joint);
    }
    fn snap_joint(&mut self, joint: i32) {
        let bone = VANISH.iter().find(|v| v.1 == joint).expect("a block's joint").0;
        snap(self.model, self.collision, bone, joint);
    }
}

/// A Rainbow Cruise gobj proc (s_link 4). Map 1's lb_800115F4 runs in the
/// scene dispatch (only the wind after it reads the field pool).
pub(crate) fn run_proc(
    state: &mut InitialState,
    key: u8,
    forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::RCruise(stage) = &mut state.stage else {
        unreachable!()
    };
    match key {
        SCROLL_KEY => scroll(stage, &mut state.stage_animations, &assets),
        SHIP_MAP => {
            let parameters = stage.parameters;
            if !stage.waiting {
                let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
                let mut blocks = Blocks {
                    model,
                    collision: &mut state.map,
                    camera: &assets.stage_camera,
                };
                step_vanish(&mut stage.vanish, &mut blocks);
                // grRCruise_8020071C: bone 8 of maps 1 and 5.
                let rotation = stage.tilt.step(&parameters);
                for map in [SHIP_MAP, SHIP_COPY_MAP] {
                    state
                        .stage_animations
                        .get_mut(&map)
                        .unwrap()
                        .set_joint_rotation_z(TILT_BONE, rotation);
                }
                let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
                let mut ship = ShipModel {
                    model,
                    collision: &mut state.map,
                    camera: &assets.stage_camera,
                    joints: stage.clouds.map(|c| c.joint),
                };
                for (index, cloud) in stage.clouds.iter_mut().enumerate() {
                    cloud.step(index, &parameters, &mut state.rng, &mut ship);
                }
            }
            // grRCruise_80201110.
            let field = forces
                .directional_mut()
                .ok_or_else(|| anyhow::anyhow!("grRCruise_80201110: the ship's wind field is gone"))?;
            stage.gusts.steer_with(field, &mut state.rng, |r| gekko_math::fma::fmadd(0.2, r, 0.1));
            let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
            model.update_collision(&mut state.map, &stage.ship_bindings);
        }
        SWING_MAP => {
            let parameters = stage.parameters;
            for (index, &bone) in SWING_BONES.iter().enumerate() {
                let model = state.stage_animations.get_mut(&SWING_MAP).unwrap();
                let mut engine = SwingStep {
                    model,
                    collision: &mut state.map,
                    bone,
                };
                stage.swings[index].step(index, &parameters, &mut engine);
            }
            let model = state.stage_animations.get_mut(&SWING_MAP).unwrap();
            model.update_collision(&mut state.map, archive_bindings(&assets, SWING_MAP));
        }
        FLAG_MAP => {
            let model = state.stage_animations.get_mut(&FLAG_MAP).unwrap();
            model.update_collision(&mut state.map, archive_bindings(&assets, FLAG_MAP));
        }
        _ => {}
    }
    let animation = state.stage_animations.get_mut(&key).unwrap();
    super::publish_joint_matrices(animation, key, &mut state.particles, &mut state.effects.events);
    Ok(())
}

/// A swing's engine with its bone known.
struct SwingStep<'a> {
    model: &'a mut BackgroundAnimation,
    collision: &'a mut melee_mp::CollMap,
    bone: usize,
}

impl SwingEngine for SwingStep<'_> {
    fn set_rate(&mut self, _swing: usize, rate: f32) {
        self.model.set_joint_rate(self.bone, rate);
    }
    fn request_frame(&mut self, _swing: usize, frame: f32) {
        self.model.request_joint_frame(self.bone, frame);
    }
    fn ended(&self, _swing: usize) -> bool {
        self.model.subtree_animation_ended(self.bone)
    }
    fn show(&mut self, _swing: usize, _shown: bool) {}
    fn enable_joint(&mut self, joint: i32) {
        self.collision.joint_list_add(joint);
    }
    fn disable_joint(&mut self, joint: i32) {
        self.collision.joint_list_remove(joint);
    }
    fn snap_joint(&mut self, joint: i32) {
        snap(self.model, self.collision, self.bone, joint);
    }
}
