//! Big Blue composition (grbigblue.c): the maps in grBigBlue_801E57C0's
//! order, their on_inits, and each tick the road, the cars and the flyers
//! over the scene's collision, camera limits and random stream.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::Result;
use gekko_math::HsdRng;
use hsd_particle::{
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    bigblue::{
        cars::{slot_joint, Cars, RoadMotion, SLOTS},
        flyers::{FlyerItems, Flyers, FLYER_BONES},
        procs::{
            BigBlueStage, B11_JOINTS, CARS_MAP, FALCON_MAP, FLYERS_MAP, FLYER_JOINTS, MARKERS_MAP,
            ROAD_BINDINGS, ROAD_JOINTS, ROAD_MAP, ROUTE_MAP,
        },
        road::Road,
        Body, Limits, Parameters, Scene, NO_FLOOR,
    },
    desc::JointMapping,
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// Ground_GetStageGObj's model under the map-scale wrapper.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let model = &assets.stage_desc.models[usize::from(map)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
        .map_err(|e| anyhow::anyhow!("Big Blue map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_big_blue_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

fn archive_bindings(assets: &Assets, map: u8) -> &[JointMapping] {
    &assets.stage_desc.models[usize::from(map)].joint_mappings
}

/// Ground_801C2ED0 (0x801C2ED0): each of the map's collision joints bound
/// to its bone, transformed and taken as its previous position; GrBb.dat's
/// bindings, then grBb_803E2938's (which rebind joints 38 and 39).
fn bind(animation: &mut BackgroundAnimation, assets: &Assets, map: u8, collision: &mut melee_mp::CollMap) {
    let extras: &[JointMapping] = if map == ROAD_MAP { &ROAD_BINDINGS } else { &[] };
    for bindings in [archive_bindings(assets, map), extras] {
        for binding in bindings {
            animation.update_collision(collision, std::slice::from_ref(binding));
            collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
        }
    }
}

/// Ground_801C2FE0 (0x801C2FE0): grBb_803E2938's joints of the map, then
/// GrBb.dat's not already moved, each from the bone it was last bound to.
fn update_collision(animation: &mut BackgroundAnimation, assets: &Assets, map: u8, collision: &mut melee_mp::CollMap) {
    if map == ROAD_MAP {
        animation.update_collision(collision, &ROAD_BINDINGS);
        for binding in archive_bindings(assets, map) {
            if !ROAD_BINDINGS.iter().any(|b| b.joint_index == binding.joint_index) {
                animation.update_collision(collision, std::slice::from_ref(binding));
            }
        }
    } else {
        animation.update_collision(collision, archive_bindings(assets, map));
    }
}

/// grAnime_801C8138(gobj, map, 0) on a fresh model: frame zero evaluated,
/// with the particles its keys ask for.
fn animate(
    animation: &mut BackgroundAnimation,
    map: u8,
    assets: &Assets,
    particles: &mut ParticleSystem,
    rng: &mut HsdRng,
) -> Result<()> {
    for event in animation.evaluate_initial_frame::<RetailTrig>().to_vec() {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(map, event.joint), event.matrix));
        particles.spawn::<RetailTrig>(
            assets.stage_key_bank(event.bank),
            request,
            rng,
            &mut DrawLog::default(),
        )?;
    }
    Ok(())
}

/// The scene's side of the stage's procs.
struct World<'a> {
    animations: &'a mut Animations,
    collision: &'a mut melee_mp::CollMap,
    rng: &'a mut HsdRng,
    limits: Limits,
    scale: f32,
}

impl World<'_> {
    fn model(&self, map: u8) -> &BackgroundAnimation {
        self.animations.get(&map).expect("Big Blue map model")
    }
    fn model_mut(&mut self, map: u8) -> &mut BackgroundAnimation {
        self.animations.get_mut(&map).expect("Big Blue map model")
    }
}

/// The JObj behind a body: a GObj's wrapper or a model bone.
enum Joint {
    Wrapper(u8),
    Bone(u8, usize),
}

fn joint(body: Body) -> Joint {
    match body {
        Body::Flyer(index) => Joint::Bone(FLYERS_MAP, FLYER_BONES[index]),
        Body::Falcon => Joint::Wrapper(FALCON_MAP),
        Body::Route => Joint::Wrapper(ROUTE_MAP),
        Body::Car(slot) => Joint::Bone(CARS_MAP, slot + 1),
        Body::Road => Joint::Wrapper(ROAD_MAP),
        Body::RoadBone(bone) => Joint::Bone(ROAD_MAP, bone),
        Body::Scenery(map) => Joint::Wrapper(map),
    }
}

impl Scene for World<'_> {
    fn scale(&self) -> f32 {
        self.scale
    }
    fn limits(&self) -> Limits {
        self.limits
    }
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
    fn floor(&mut self, at: Vec3, half_height: f32) -> (f32, Option<Vec3>) {
        floor(self.collision, at, half_height)
    }
    fn translation(&self, body: Body) -> Vec3 {
        match joint(body) {
            Joint::Wrapper(map) => self.model(map).gobj_translation(),
            Joint::Bone(map, bone) => self.model(map).joint_translation(bone),
        }
    }
    fn set_translation(&mut self, body: Body, translation: Vec3) {
        match joint(body) {
            Joint::Wrapper(map) => self.model_mut(map).set_gobj_translate(translation),
            Joint::Bone(map, bone) => self.model_mut(map).set_joint_translate(bone, translation),
        }
    }
    fn rotation_z(&self, body: Body) -> f32 {
        match joint(body) {
            Joint::Wrapper(map) => self.model(map).gobj_rotation().z,
            Joint::Bone(map, bone) => self.model(map).joint_rotation_z(bone),
        }
    }
    fn set_rotation_z(&mut self, body: Body, z: f32) {
        match joint(body) {
            Joint::Wrapper(map) => {
                let model = self.model_mut(map);
                let mut rotation = model.gobj_rotation();
                rotation.z = z;
                model.set_gobj_rotation(rotation);
            }
            Joint::Bone(map, bone) => self.model_mut(map).set_joint_rotation_z(bone, z),
        }
    }
    fn set_scale_of(&mut self, body: Body, scale: f32) {
        let scale = Vec3::new(scale, scale, scale);
        match joint(body) {
            Joint::Wrapper(map) => self.model_mut(map).set_gobj_scale(scale),
            Joint::Bone(map, bone) => self.model_mut(map).set_joint_scale(bone, scale),
        }
    }
    fn set_hidden(&mut self, body: Body, hidden: bool) {
        match joint(body) {
            Joint::Wrapper(map) => self.model_mut(map).set_hidden(hidden),
            Joint::Bone(map, bone) => self.model_mut(map).set_joint_hidden(bone, hidden),
        }
    }
    fn world_position(&mut self, body: Body) -> Vec3 {
        match joint(body) {
            Joint::Wrapper(map) => {
                let m = self.model_mut(map).gobj_matrix();
                Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3])
            }
            Joint::Bone(map, bone) => self.model_mut(map).joint_world_translation(bone),
        }
    }
    fn enable_joint(&mut self, joint: i32) {
        self.collision.joint_list_add(joint);
    }
    fn disable_joint(&mut self, joint: i32) {
        self.collision.joint_list_remove(joint);
    }
    fn snapshot_joint(&mut self, joint: i32) {
        self.collision.joint_snapshot_prev_pos(joint);
    }
    fn stitch_joints(&mut self) {
        self.collision.stitch_all_joints();
    }
}

/// grBigBlue_801EC58C (0x801EC58C): mpCheckMultiple on each road joint's
/// floor along x from `y + half` down to `y - half`; the highest hit and
/// its normal.
fn floor(collision: &mut melee_mp::CollMap, at: Vec3, half_height: f32) -> (f32, Option<Vec3>) {
    let top = at.y + half_height;
    let bottom = at.y - half_height;
    let mut highest = NO_FLOOR;
    let mut normal = None;
    for joint in ROAD_JOINTS {
        if let Some(hit) = collision.check_multiple(at.x, top, at.x, bottom, 1, -1, joint) {
            if highest < hit.pos.y {
                highest = hit.pos.y;
                normal = Some(hit.normal);
            }
        }
    }
    (highest, normal)
}

fn limits(stage_limits: &crate::stage_limits::StageLimits) -> Limits {
    let c = &stage_limits.camera;
    Limits {
        camera_top: c.top(),
        camera_bottom: c.bottom(),
        camera_left: c.left(),
        camera_right: c.right(),
        blast_left: c.blast_left(),
        blast_right: c.blast_right(),
    }
}

/// Items stay off on Big Blue until a flyer's item is witnessed.
struct NoItems;
impl FlyerItems for NoItems {
    fn any_enabled(&self) -> bool {
        false
    }
}

/// grBigBlue_801E57C0 (0x801E57C0): the maps in creation order with their
/// on_inits, then the road's joints marked (mpLib_80058044).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let scale = assets.stage_desc.parameters.map_scale;
    let mut animations = Animations::new();
    // grBigBlue_801E5B20: map 0x1F without an animation, at (0, 60, 60)
    // and scale 0.8, its root and first bone at the origin.
    let mut markers = load(assets, MARKERS_MAP)?;
    markers.clear_animation();
    markers.set_gobj_translate(parameters.markers_translate);
    let s = parameters.markers_scale;
    markers.set_gobj_scale(Vec3::new(s, s, s));
    markers.set_joint_translate(0, Vec3::ZERO);
    if markers.joint_count() > 1 {
        markers.set_joint_translate(1, Vec3::ZERO);
    }
    animations.insert(MARKERS_MAP, markers);
    // grBigBlue_801E5AE4, 801E6298, 801E6200: maps 0, 1 and 2 animate.
    for map in [0, 1, 2] {
        let mut model = load(assets, map)?;
        animate(&mut model, map, assets, particles, rng)?;
        animations.insert(map, model);
    }
    let limits = limits(&crate::stage_limits::StageLimits::new(assets));
    // grBigBlue_801E613C: animation, collision, the road.
    let mut road_model = load(assets, ROAD_MAP)?;
    animate(&mut road_model, ROAD_MAP, assets, particles, rng)?;
    bind(&mut road_model, assets, ROAD_MAP, collision);
    animations.insert(ROAD_MAP, road_model);
    // grBigBlue_801E6364: collision, the bones at unit scale.
    let mut cars_model = load(assets, CARS_MAP)?;
    cars_model.clear_animation();
    bind(&mut cars_model, assets, CARS_MAP, collision);
    cars_model.set_gobj_scale(Vec3::new(1.0, 1.0, 1.0));
    animations.insert(CARS_MAP, cars_model);
    // grBigBlue_801E6904: collision, unit scale, animation.
    let mut flyers_model = load(assets, FLYERS_MAP)?;
    bind(&mut flyers_model, assets, FLYERS_MAP, collision);
    flyers_model.set_gobj_scale(Vec3::new(1.0, 1.0, 1.0));
    animate(&mut flyers_model, FLYERS_MAP, assets, particles, rng)?;
    animations.insert(FLYERS_MAP, flyers_model);
    let mut world = World {
        animations: &mut animations,
        collision: &mut *collision,
        rng: &mut *rng,
        limits,
        scale,
    };
    let road = Road::new(&parameters, &mut world);
    // grBigBlue_801E6364: each car bone at the ground's scale times x0C.
    let car_scale = scale * parameters.car_scale;
    for slot in 0..SLOTS {
        world.set_scale_of(Body::Car(slot), car_scale);
    }
    let cars = Cars::new(&parameters, &mut world);
    let flyers = Flyers::new(&parameters, &mut world);
    // grBigBlue_801E57C0 hides map 0x20 once it is made.
    world.model_mut(FLYERS_MAP).set_hidden(true);
    // grBigBlue_801E8D64: the Falcon Flyer.
    let mut falcon = load(assets, FALCON_MAP)?;
    bind(&mut falcon, assets, FALCON_MAP, collision);
    falcon.set_gobj_scale(Vec3::new(scale, scale, scale));
    // grBigBlue_801E8D64 searches the road (at its creation pose) under an
    // uninitialised stack vector; the boundaries hold the origin's floor.
    let (found, _) = floor(collision, Vec3::ZERO, 500.0);
    let y = if found == NO_FLOOR { 0.0 } else { found } + parameters.falcon_height;
    falcon.set_gobj_translate(Vec3::new(0.0, y, 0.0));
    animate(&mut falcon, FALCON_MAP, assets, particles, rng)?;
    // The spawn markers 0..3 (Ground_801C2CF4) move by the Falcon Flyer's
    // translation over the ground's scale.
    let offset = falcon.gobj_translation();
    let inverse = 1.0 / scale;
    let offset = Vec3::new(offset.x * inverse, offset.y * inverse, offset.z * inverse);
    let spawns = animations.get_mut(&0).unwrap();
    for marker in 0..4 {
        let binding = assets
            .stage_desc
            .position_bindings
            .iter()
            .rfind(|b| b.stage_position == marker)
            .expect("Big Blue's spawn markers");
        anyhow::ensure!(binding.model_id == 0, "Big Blue's spawn marker {marker} off map 0");
        let bone = binding.joint_index as usize;
        let t = spawns.joint_translation(bone);
        spawns.set_joint_translate(bone, Vec3::new(t.x + offset.x, t.y + offset.y, t.z + offset.z));
    }
    animations.insert(FALCON_MAP, falcon);
    // grBigBlue_801E9F3C: the route platform, hidden.
    let mut route = load(assets, ROUTE_MAP)?;
    bind(&mut route, assets, ROUTE_MAP, collision);
    route.set_gobj_scale(Vec3::new(scale, scale, scale));
    animate(&mut route, ROUTE_MAP, assets, particles, rng)?;
    route.set_hidden(true);
    animations.insert(ROUTE_MAP, route);
    mark_joints(collision);
    let stage = BigBlueStage {
        parameters,
        road,
        cars,
        flyers,
    };
    Ok((SceneStage::BigBlue(Box::new(stage)), animations))
}

/// The joints' callbacks (recorded, applied at the next proc) and
/// grBigBlue_801E57C0's mpLib_80058044 on joints 0x21..0x41.
fn mark_joints(collision: &mut melee_mp::CollMap) {
    for joint in FLYER_JOINTS {
        collision.record_joint_events(joint, true);
    }
    for slot in 0..SLOTS {
        collision.record_joint_events(slot_joint(slot), true);
    }
    for joint in B11_JOINTS {
        collision.joint_set_b11(joint);
    }
}

/// A Big Blue gobj proc (s_link 4).
pub(crate) fn run_proc(state: &mut InitialState, map: u8) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::BigBlue(stage) = &mut state.stage else {
        unreachable!()
    };
    let stage = &mut **stage;
    let mut world = World {
        animations: &mut state.stage_animations,
        collision: &mut state.map,
        rng: &mut state.rng,
        limits: limits(&state.stage_limits),
        scale: assets.stage_desc.parameters.map_scale,
    };
    match map {
        ROAD_MAP => {
            // grBigBlue_801E61C4: the road, lb_800115F4 (in the dispatch),
            // Ground_801C2FE0.
            stage.road.step(&stage.parameters, &mut world);
        }
        CARS_MAP => {
            // fn_801EF60C since the last step: a fighter landing (kind 1).
            for slot in 0..SLOTS {
                let joint = slot_joint(slot);
                for event in world.collision.take_joint_events(joint).iter() {
                    if event.collision_type == 1 && event.ground_kind == 1 {
                        stage.cars.landed(&stage.parameters, joint, event.weight);
                    }
                }
            }
            let road = RoadMotion {
                y: stage.road.position.y,
                previous_y: stage.road.previous_position.y,
            };
            stage.cars.step(&stage.parameters, &mut world, road);
        }
        FLYERS_MAP => {
            // fn_801E8560 since the last step: fighters (1) and type 3.
            for (index, joint) in FLYER_JOINTS.into_iter().enumerate() {
                for event in world.collision.take_joint_events(joint).iter() {
                    if matches!(event.collision_type, 1 | 3) {
                        let at = Vec3::new(event.x, event.y, 0.0);
                        stage.flyers.ridden(&world, index, at, event.x50);
                    }
                }
            }
            stage.flyers.step_flyers(&stage.parameters, &mut world, &stage.cars, &NoItems);
        }
        FALCON_MAP => stage.flyers.step_falcon(&stage.parameters, &mut world, &stage.cars),
        ROUTE_MAP => stage.flyers.step_route(&stage.parameters, &mut world, &stage.cars),
        // The other maps' procs are empty.
        _ => return Ok(()),
    }
    let animation = state.stage_animations.get_mut(&map).unwrap();
    update_collision(animation, &assets, map, &mut state.map);
    Ok(())
}
