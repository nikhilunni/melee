//! Great Bay composition (grgreatbay.c): maps 0, 3, 4, 2, 1, 9 (with the
//! giants, 5..8) and 10 in grGreatBay_801F4240's order; the lab's
//! floating platform, the turtle, the moon and Tingle's platform; the
//! water's splash (grGreatBay_801F6708).
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::{
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    desc::JointMapping,
    greatbay::{
        procs::{
            GreatBayStage, Moon, TinglePlatform, BACKGROUND_MAP, CREATION_ORDER, FLOAT_BONE,
            FLOAT_JOINT, GIANT_MAPS, MOON_MAP, SHELL_VERTICES, STAGE_JOINTS, STAGE_MAP,
            TINGLE_JOINT, TINGLE_MAP, TURTLE_MAP, WATER_MAP,
        },
        rand_range,
        turtle::{Turtle, TurtleEngine, BODY_BONE, SPLASH_PARTICLE, WAKE_PARTICLE},
        FloatFloor, Parameters,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// Stage particle bank 30 (grLib_801C96F8 / 801C97DC's bank argument).
const STAGE_PARTICLE_BANK: u8 = 30;
/// grGreatBay_801F5600: the balloon's pop and Tingle's splash.
const POP_PARTICLE: u32 = 0x7534;
const TINGLE_SPLASH_PARTICLE: u32 = 0x7537;
/// grGreatBay_801F5600: below this Tingle is gone and made again.
const TINGLE_LOST_Y: f32 = -50.0;
/// grGreatBay_801F5600: the pop's effect this far above the platform.
const POP_RISE: f32 = 10.0;
/// grGreatBay_801F6708 (@958): the water's surface.
const WATER_Y: f32 = -5.0;
/// grGreatBay_801F6708 (@620): the splash's scale per unit of the
/// fighter's camera radius.
const SPLASH_DIVISOR: f32 = 10.0;
/// grGreatBay_801F5460: Tingle's platform starts far away.
const PLATFORM_PARKED: f32 = 1000.0;
/// grGreatBay_801F63F4's offset from the turtle's body bone (grGb_803B81C8).
const ITEM_OFFSET: Vec3 = Vec3::new(0.99, 35.72, 0.0);

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let scale = assets.stage_desc.parameters.map_scale;
    if usize::from(map) >= assets.stage_desc.models.len() {
        // Ground_GetStageGObj past the archive's models: an identity JObj.
        return BackgroundAnimation::identity_root(&assets.stage, scale)
            .map_err(|e| anyhow::anyhow!("Great Bay map {map}: {e}"));
    }
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Great Bay map {map}: {e}"))?;
    animation.set_map_scale(scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_greatbay_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

/// The collision joints bound to `map` (grGb_803E3E60; the archive binds
/// none).
fn bindings(map: u8) -> impl Iterator<Item = &'static JointMapping> {
    STAGE_JOINTS
        .iter()
        .filter(move |b| b.target_index == i16::from(map))
}

/// Ground_801C2ED0 (0x801C2ED0): bind, transform and snapshot each joint.
fn bind(model: &mut BackgroundAnimation, map: u8, collision: &mut melee_mp::CollMap) {
    for binding in bindings(map) {
        model.update_collision(collision, std::slice::from_ref(binding));
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// Ground_801C2FE0 (0x801C2FE0) on `map`.
fn update_collision(model: &mut BackgroundAnimation, map: u8, collision: &mut melee_mp::CollMap) {
    for binding in bindings(map) {
        model.update_collision(collision, std::slice::from_ref(binding));
    }
}

/// grAnime_801C8138(gobj, map, index) during creation: the frame-zero
/// evaluation's particle keys.
fn play_initial(
    model: &mut BackgroundAnimation,
    map: u8,
    assets: &Assets,
    particles: &mut ParticleSystem,
    rng: &mut HsdRng,
) -> Result<()> {
    let requests = model.evaluate_initial_frame::<RetailTrig>().to_vec();
    super::garden::spawn_keys(&requests, map, assets, particles, None, rng, &mut DrawLog::default())
}

/// grGreatBay_801F4240 (0x801F4240): the maps in creation order with their
/// on_init. Tingle's creation draws (it_802ECA70: the facing query and
/// it_802EB6DC twice) are taken here, in their place; his item is made
/// once the pool exists ([`spawn_tingle`]).
pub(crate) fn initialize(
    assets: &Assets,
    items_on: bool,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let mut animations = Animations::new();
    let mut float_floor = None;
    let mut turtle = None;
    let mut moon = None;
    let mut tingle_draws = None;
    for map in CREATION_ORDER {
        let mut model = load(assets, map)?;
        match map {
            // grGreatBay_801F4404: animation 0 (none).
            0 => {}
            // grGreatBay_801F4440 / 801F44B0: animation 0, looping.
            BACKGROUND_MAP | WATER_MAP => play_initial(&mut model, map, assets, particles, rng)?,
            STAGE_MAP => {
                // grGreatBay_801F454C: Ground_801C2ED0, animation 0; the
                // turtle's head joint off; the platform's rest
                // (grGreatBay_801F5D4C, mpJointSetCb1(0)).
                bind(&mut model, map, collision);
                play_initial(&mut model, map, assets, particles, rng)?;
                collision.joint_list_remove(1);
                let rest = model.joint_translation(FLOAT_BONE);
                float_floor = Some(FloatFloor::new(rest.x, rest.y));
                collision.record_joint_events(FLOAT_JOINT, true);
            }
            TURTLE_MAP => {
                // grGreatBay_801F4694: Ground_801C2ED0, animation 0, the
                // turtle scaled and placed, its body's height, its wait.
                bind(&mut model, map, collision);
                play_initial(&mut model, map, assets, particles, rng)?;
                let s = model.gobj_scale();
                let k = parameters.turtle_scale;
                model.set_gobj_scale(Vec3::new(s.x * k, s.y * k, s.z * k));
                let mut t = model.gobj_translation();
                // retail: fadds kame_x + kame_x_offset_init.
                t.x = parameters.turtle_x + parameters.turtle_x_offset_init;
                t.y = parameters.turtle_y;
                model.set_gobj_translate(t);
                let body_y = model.joint_translation(BODY_BONE).y;
                turtle = Some(Turtle::new(&parameters, body_y, rng));
            }
            MOON_MAP => {
                // grGreatBay_801F5A00: animation 0 held at its first
                // frame, the giants made (grGreatBay_801F598C each), the
                // fall's wait.
                play_initial(&mut model, map, assets, particles, rng)?;
                model.set_rate(0.0);
                for giant in GIANT_MAPS {
                    let mut giant_model = load(assets, giant)?;
                    play_initial(&mut giant_model, giant, assets, particles, rng)?;
                    giant_model.set_rate(0.0);
                    animations.insert(giant, giant_model);
                }
                let [a, b] = parameters.moon_fall_wait;
                moon = Some(Moon {
                    timer: rand_range(i32::from(b), i32::from(a), rng) as i16,
                    started: [false; 4],
                });
            }
            TINGLE_MAP => {
                // grGreatBay_801F5460: Ground_801C2ED0, mpJointSetCb1(5),
                // mpJointListAdd(5), parked far away, Ground_801C2FE0,
                // mpLib_80057424(5), then Tingle (it_802ECA70).
                bind(&mut model, map, collision);
                collision.record_joint_events(TINGLE_JOINT, true);
                collision.joint_list_add(TINGLE_JOINT);
                model.set_gobj_translate(Vec3::new(PLATFORM_PARKED, PLATFORM_PARKED, 0.0));
                update_collision(&mut model, map, collision);
                collision.joint_snapshot_prev_pos(TINGLE_JOINT);
                tingle_draws = Some(TingleDraws::take(assets, rng)?);
            }
            _ => unreachable!(),
        }
        animations.insert(map, model);
    }
    let mut stage = stage(
        assets,
        parameters,
        float_floor.expect("map 2"),
        turtle.expect("map 1"),
        moon.expect("map 9"),
    )?;
    let draws = tingle_draws.expect("map 10");
    stage.tingle_draws = Some((draws.facing, draws.x, draws.wait));
    stage.items_on = items_on;
    Ok((SceneStage::GreatBay(Box::new(stage)), animations))
}

pub(crate) fn stage(
    assets: &Assets,
    parameters: Parameters,
    float_floor: FloatFloor,
    turtle: Turtle,
    moon: Moon,
) -> Result<GreatBayStage> {
    let mut stage = GreatBayStage {
        parameters,
        float_floor,
        float_x: 0.0,
        turtle,
        moon,
        tingle: TinglePlatform::default(),
        tingle_draws: None,
        water: false,
        wake: None,
        items_on: false,
        // Map 2's light set is animated (ground.c:2723-2747); the
        // presentation reads it itself.
        lights: Vec::new(),
    };
    let _ = assets;
    stage.tingle.position = Vec3::new(PLATFORM_PARKED, PLATFORM_PARKED, 0.0);
    Ok(stage)
}

/// The draws it_802ECA70 makes as it creates Tingle at the stage's creation,
/// before any fighter exists: it_8027B5B0's facing query (no fighter, so a
/// coin, ftLib_800864A8), then it_802EB6DC from his spawn callback and once
/// more: the second's results stand.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TingleDraws {
    /// it_8026B684's facing for the creation pass's collision.
    pub facing: f32,
    /// The second it_802EB6DC's place and wait draws.
    pub x: i32,
    pub wait: i32,
}
impl TingleDraws {
    fn take(assets: &Assets, rng: &mut HsdRng) -> Result<Self> {
        let item = assets.items.try_get(melee_types::ItemKind::Tincle);
        let item = item.ok_or_else(|| anyhow::anyhow!("GrGb.dat has no Tingle"))?;
        let facing = if rng.randi(2) != 0 { 1.0 } else { -1.0 };
        // The spawn callback's it_802EB6DC; it_802ECA70's own follows.
        it_tincle::reset_draws(item, rng);
        let (x, wait) = it_tincle::reset_draws(item, rng);
        Ok(Self { facing, x, wait })
    }
}

/// it_802ECA70's item, made once the item pool exists with the draws the
/// stage's creation took: Item_80268B18 (its spawn callback), the map's
/// scale and no facing (it_802ECA70's tail), it_802EB6DC.
pub(crate) fn spawn_tingle(
    stage: &mut SceneStage,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
    map_scale: f32,
) {
    let SceneStage::GreatBay(stage) = stage else {
        return;
    };
    let Some((facing, x, wait)) = stage.tingle_draws.take() else {
        return;
    };
    let draws = TingleDraws { facing, x, wait };
    let assets = resources.get(melee_types::ItemKind::Tincle);
    let id = crate::scene_items::spawn_tingle(items, gobjs, resources, map, draws.facing);
    let item = items.get_mut(id).unwrap();
    it_tincle::start(item, map_scale);
    it_tincle::reset_with(item, assets, draws.x, draws.wait);
    stage.tingle.item = Some(id);
}

/// The Great Bay procs (s_link 4, Tingle's platform's at 5).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    match map {
        STAGE_MAP => step_float_floor(state),
        TURTLE_MAP => step_turtle(state, draws),
        MOON_MAP => step_moon(state),
        TINGLE_MAP => step_tingle(state, draws, world, item_objects),
        _ => Ok(()),
    }
}

/// grGreatBay_801F4658 (0x801F4658): the platform's landings since the last
/// step weigh it (grGreatBay_801F60C4, for fighters and stage enemies),
/// it eases (grGreatBay_801F5E28), Ground_801C2FE0; lb_800115F4 runs in
/// the scene dispatch.
fn step_float_floor(state: &mut InitialState) -> Result<()> {
    let events = state.map.take_joint_events(FLOAT_JOINT);
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    for event in events.iter() {
        if matches!(event.collision_type, 1 | 3) {
            stage
                .float_floor
                .weigh(&parameters, event.x50, event.ground_kind == 1, event.x, stage.float_x);
        }
    }
    let (rotation, x, y) = stage.float_floor.step(&parameters);
    let model = state.stage_animations.get_mut(&STAGE_MAP).unwrap();
    model.set_joint_rotation_z(FLOAT_BONE, rotation);
    model.set_joint_translate_x(FLOAT_BONE, x);
    model.set_joint_translate_y(FLOAT_BONE, y);
    update_collision(model, STAGE_MAP, &mut state.map);
    // lb_8000B1CC(bone 49) as the next landings read it.
    let x = model.joint_matrix(FLOAT_BONE).0[0][3];
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.float_x = x;
    Ok(())
}

/// What the turtle's step does to the model, the map and the effects.
struct TurtleModel<'a> {
    model: &'a mut BackgroundAnimation,
    map: &'a mut melee_mp::CollMap,
    assets: &'a Assets,
    particles: &'a mut ParticleSystem,
    events: &'a mut melee_ef::fixture_spawns::EventSink,
    rng: &'a mut HsdRng,
    draws: &'a mut DrawLog,
    wake: &'a mut Option<usize>,
    error: Option<anyhow::Error>,
}
impl TurtleEngine for TurtleModel<'_> {
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
    fn rewound(&mut self) -> bool {
        self.model.first_animation_rewound()
    }
    fn ended(&mut self) -> bool {
        self.model.first_animation_ended()
    }
    fn play(&mut self, index: usize) {
        self.model.reset_pose();
        let result = self
            .model
            .select_animation(
                &self.assets.stage,
                &self.assets.stage_desc.models[usize::from(TURTLE_MAP)],
                index,
            )
            .map_err(|e| anyhow::anyhow!("{e}"));
        if let Err(e) = result {
            self.error.get_or_insert(e);
            return;
        }
        if !self.model.evaluate_initial_frame::<RetailTrig>().is_empty() {
            self.error.get_or_insert(anyhow::anyhow!(
                "grAnime_801C8138: the turtle's animation {index} has frame-zero particle keys"
            ));
        }
    }
    fn freeze(&mut self) {
        self.model.set_rate(0.0);
    }
    fn set_hidden(&mut self, hidden: bool) {
        self.model.set_hidden(hidden);
    }
    fn set_b11(&mut self, joint: i32, set: bool) {
        if set {
            self.map.joint_set_b11(joint);
        } else {
            self.map.joint_clear_b11(joint);
        }
    }
    fn enable_joint(&mut self, joint: i32, enabled: bool) {
        if enabled {
            self.map.joint_list_add(joint);
        } else {
            self.map.joint_list_remove(joint);
        }
    }
    fn set_root_x(&mut self, x: f32) {
        let mut t = self.model.gobj_translation();
        t.x = x;
        self.model.set_gobj_translate(t);
    }
    fn start_wake(&mut self) {
        // grLib_801C96F8(0x7531, 30, lb_8000B1CC(gobj jobj)).
        let m = self.model.gobj_matrix();
        let position = Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3]);
        match ground_effect(
            self.assets,
            self.particles,
            self.events,
            self.rng,
            self.draws,
            WAKE_PARTICLE,
            position,
        ) {
            Ok(id) => *self.wake = id,
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }
    }
    fn stop_wake(&mut self) {
        // grLib_801C9874 on xC8 when it is set.
        if let Some(id) = self.wake.take() {
            self.particles.kill_generator(id);
        }
    }
    fn splash(&mut self, bone: usize) {
        // grLib_801C97DC(0x7533, 30, jobj): a detached generator there.
        let matrix = self.model.joint_matrix(bone);
        let mut request = SpawnRequest::new(STAGE_PARTICLE_BANK, SPLASH_PARTICLE, 0);
        request.joint = Some((joint_id(TURTLE_MAP, bone), matrix));
        self.events.spawn(&request, true, false);
        match self.particles.spawn::<RetailTrig>(
            &self.assets.particle_bank,
            request,
            self.rng,
            self.draws,
        ) {
            Ok(id) => self.particles.pending_generators.push(id),
            Err(e) => {
                self.error.get_or_insert(e.into());
            }
        }
    }
    fn body_y(&mut self) -> f32 {
        self.model.joint_translation(BODY_BONE).y
    }
    fn set_body_y(&mut self, y: f32) {
        self.model.set_joint_translate_y(BODY_BONE, y);
    }
    fn set_rotation_y(&mut self, bone: usize, angle: f32) {
        self.model.set_joint_rotation_y(bone, angle);
    }
}

/// grLib_801C96F8 (0x801C96F8) with the stage's pieces apart (see
/// `stadium::spawn_ground_effect`).
pub(crate) fn ground_effect(
    assets: &Assets,
    particles: &mut ParticleSystem,
    events: &mut melee_ef::fixture_spawns::EventSink,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
    kind: u32,
    position: Vec3,
) -> Result<Option<usize>> {
    let Some(descriptor) = assets.particle_bank.descriptor(kind) else {
        return Ok(None);
    };
    let owns_transform = descriptor.kind & 0x20000 != 0;
    let scale = assets.stage_desc.parameters.map_scale;
    let mut request = SpawnRequest::new(STAGE_PARTICLE_BANK, kind, 0);
    request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
        translation: position,
        scale: Vec3::new(scale, scale, scale),
        status: if owns_transform { 0 } else { 1 },
        ..Default::default()
    });
    events.spawn(&request, true, false);
    let id = particles.spawn::<RetailTrig>(&assets.particle_bank, request, rng, draws)?;
    if let Some(id) = id {
        let generator = particles.generators.iter().find(|g| g.id == id).unwrap();
        let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
        transform.camera_facing = 0;
        particles.set_application_transform(id, transform);
    }
    Ok(id)
}

/// grGreatBay_801F499C (0x801F499C): the turtle's state, Ground_801C2FE0,
/// its shell's vertices (grGreatBay_801F660C), mpLib_80055E24(2), and an
/// item draw as it rises (grGreatBay_801F63F4).
fn step_turtle(state: &mut InitialState, draws: &mut DrawLog) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let model = state.stage_animations.get_mut(&TURTLE_MAP).unwrap();
    let mut engine = TurtleModel {
        model,
        map: &mut state.map,
        assets: &assets,
        particles: &mut state.particles,
        events: &mut state.effects.events,
        rng: &mut state.rng,
        draws,
        wake: &mut stage.wake,
        error: None,
    };
    stage.turtle.step(&parameters, &mut engine);
    if let Some(error) = engine.error {
        return Err(error);
    }
    let model = state.stage_animations.get_mut(&TURTLE_MAP).unwrap();
    update_collision(model, TURTLE_MAP, &mut state.map);
    for (vertex, bone) in SHELL_VERTICES {
        let m = model.joint_matrix(bone);
        state.map.vtx_set_pos(vertex, m.0[0][3], m.0[1][3]);
    }
    // mpLib_80055E24(2): the moved shell lines are reclassified, then the island.
    state.map.joint_update_dynamics_and_island(2);
    // The splashes' pending generators (hsd_8039F6CC) bake their JObj's
    // matrix at the particle pass, after the head's turn above.
    super::publish_joint_matrices(model, TURTLE_MAP, &mut state.particles, &mut state.effects.events);
    let turtle = &mut stage.turtle;
    if turtle.phase == melee_gr::greatbay::turtle::Phase::Rise && !turtle.item_checked {
        let probability = parameters.turtle_item_prob;
        turtle.item_checked = item_draw(model, &assets, probability, stage.items_on, &mut state.rng)?;
    }
    Ok(())
}

/// grGreatBay_801F63F4 (0x801F63F4): the body bone's point (grGb_803B81C8)
/// over the blast zone's floor takes one draw against kame_item_prob; with
/// items off no kind is offered (it_8026D324), so nothing is made.
fn item_draw(
    model: &mut BackgroundAnimation,
    assets: &Assets,
    probability: f32,
    items_on: bool,
    rng: &mut HsdRng,
) -> Result<bool> {
    let position = model.joint_point(BODY_BONE, ITEM_OFFSET);
    if position.y <= assets.stage_camera.blast_bottom() {
        return Ok(false);
    }
    if rng.randf() > probability {
        return Ok(true);
    }
    ensure!(
        !items_on,
        "grgreatbay.c:1349: the turtle's item with items on is not ported"
    );
    Ok(true)
}

/// grGreatBay_801F5AF8 (0x801F5AF8): the fall's wait; at its end the moon
/// and the giants play (rate 1) until the moon's animation wraps
/// (lb_8000B134), when they hold again and a new wait is drawn. The giants'
/// sounds (x12) are not modelled.
fn step_moon(state: &mut InitialState) -> Result<()> {
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    let assets = std::sync::Arc::clone(&state.assets);
    if stage.moon.timer > 0 {
        stage.moon.timer -= 1;
        if stage.moon.timer == 0 {
            for map in std::iter::once(MOON_MAP).chain(GIANT_MAPS) {
                let model = state.stage_animations.get_mut(&map).unwrap();
                model.reset_pose();
                model
                    .select_animation(&assets.stage, &assets.stage_desc.models[usize::from(map)], 0)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                ensure!(
                    model.evaluate_initial_frame::<RetailTrig>().is_empty(),
                    "grAnime_801C8138: Great Bay map {map} has frame-zero particle keys"
                );
                model.set_rate(1.0);
            }
            stage.moon.started = [false; 4];
        }
        return Ok(());
    }
    let moon = state.stage_animations.get_mut(&MOON_MAP).unwrap();
    // Unreachable with GrGb.dat: map 9's loop flag is 0, so
    // grAnime_801C8138 never sets AOBJ_LOOP and the fall (4690 frames)
    // stops on AOBJ_NO_ANIM, never AOBJ_REWINDED; the moon falls once.
    if moon.first_animation_rewound() {
        unimplemented!("grGreatBay_801F5AF8 (grgreatbay.c:834): the moon's fall ends");
    }
    Ok(())
}

/// grGreatBay_801F5600's tail once Tingle fell below -50 and his teardown
/// ran: it_802ECA70 makes him again (the facing query against the
/// fighters, his spawn callback's it_802EB6DC, the map's scale, it_802EB6DC
/// again) and the platform's counts restart (its place and ledge flags
/// stay).
pub(crate) fn tingle_lost(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
) {
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!("Tingle off Great Bay")
    };
    if !std::mem::take(&mut stage.tingle.respawn) {
        return;
    }
    let id = crate::scene_items::respawn_tingle(state, world, item_objects);
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    let p = &mut stage.tingle;
    p.item = Some(id);
    p.motion = 0;
    p.last_motion = 0;
    p.popped = false;
    p.splashed = false;
    p.riders = 0;
    p.last_riders = 0;
    p.weight = 0.0;
}

/// grGreatBay_801F6708 (0x801F6708), the water's fighter device (ftCo_800C07F8
/// type 6, registered at the stage's start), from Fighter_8006CB94's
/// ftColl_8007BAC0: a fighter not on a ledge falling through y -5 splashes
/// (grLib_801C96F8(0x7537) scaled by its camera radius over 10, retail
/// 0x801F6774 fdivs, 0x801F67D8.. fmuls), with a sound.
pub(crate) fn water_splash(state: &mut InitialState, player: usize, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::GreatBay(stage) = &state.stage else {
        return Ok(());
    };
    if !stage.water {
        return Ok(());
    }
    let (current, previous, on_ledge, radius) =
        crate::scene_fighter::with_fighter!(&state.fighters[player], |f| (
            f.physics.position,
            f.physics.previous_position,
            f.status.on_ledge,
            f.camera.target_extents.radius,
        ));
    if on_ledge || current.y >= WATER_Y || previous.y <= WATER_Y {
        return Ok(());
    }
    let scale = radius / SPLASH_DIVISOR;
    let position = Vec3::new(current.x, WATER_Y, current.z);
    let assets = std::sync::Arc::clone(&state.assets);
    if let Some(id) = ground_effect(
        &assets,
        &mut state.particles,
        &mut state.effects.events,
        &mut state.rng,
        draws,
        TINGLE_SPLASH_PARTICLE,
        position,
    )? {
        scale_effect(&mut state.particles, id, scale);
    }
    Ok(())
}

/// The generator's AppSRT scaled (grGreatBay_801F67A4: fmuls per axis).
fn scale_effect(particles: &mut ParticleSystem, id: usize, scale: f32) {
    let generator = particles.generators.iter().find(|g| g.id == id).unwrap();
    let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
    transform.scale = Vec3::new(
        transform.scale.x * scale,
        transform.scale.y * scale,
        transform.scale.z * scale,
    );
    particles.set_application_transform(id, transform);
}

/// fn_801F4520, deferred by Ground_801C10B8 to the stage's start: the
/// water's fighter device (ftCo_800C07F8(gobj, 6, grGreatBay_801F6708)).
pub(crate) fn start(state: &mut InitialState) {
    if let SceneStage::GreatBay(stage) = &mut state.stage {
        stage.water = true;
    }
}

/// grGreatBay_801F5600 (0x801F5600), s_link 5: Tingle's motion and place
/// (it_802ECCA4); gone below -50 he is made again (it_802EC830,
/// it_802ECA70); otherwise his riders' weight passes to him (it_802ECC98),
/// they sink or let him rise, the pop and splash effects show, and the
/// platform follows him (HSD_JObjSetTranslate, Ground_801C2FE0 twice).
fn step_tingle(
    state: &mut InitialState,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    // grGreatBay_801F5914's calls since the last step: fighters, items and
    // stage enemies standing on him.
    for event in state.map.take_joint_events(TINGLE_JOINT).iter() {
        if matches!(event.collision_type, 1..=3) {
            let p = &mut stage.tingle;
            p.ledge = event.env_flags & melee_types::mp::collide::LEDGE_GRAB_MASK;
            p.riders += 1;
            p.weight += event.x50 as f32 / 100.0;
        }
    }
    let item_id = stage.tingle.item.expect("Tingle's item");
    let item = state.items.get_mut(item_id).expect("Tingle's item");
    let (motion, position) = it_tincle::platform(item);
    stage.tingle.motion = i32::from(motion);
    stage.tingle.position = position;
    if position.y < TINGLE_LOST_Y {
        // it_802EC830 -> Item_8026A8EC (his teardown follows the proc and
        // makes him again, [`tingle_lost`]), mpLib_80057424(5).
        item.destroyed = true;
        state.map.joint_snapshot_prev_pos(TINGLE_JOINT);
        stage.tingle.respawn = true;
        return Ok(());
    }
    it_tincle::set_weight(item, stage.tingle.weight);
    let assets = std::sync::Arc::clone(&state.assets);
    let tingle_assets = assets.items.get(melee_types::ItemKind::Tincle);
    let p = &mut stage.tingle;
    use it_tincle::motion;
    match motion {
        motion::DESCEND => {
            state.map.joint_list_add(TINGLE_JOINT);
            let model = state.stage_animations.get_mut(&TINGLE_MAP).unwrap();
            update_collision(model, TINGLE_MAP, &mut state.map);
        }
        motion::BOB | motion::RISE => {
            if p.riders != 0 && p.ledge == 0 {
                it_tincle::sink(item, tingle_assets);
            }
        }
        motion::HOLD => {
            if p.riders == 0 {
                it_tincle::rise(item, tingle_assets);
            } else if p.last_riders != p.riders {
                it_tincle::sink(item, tingle_assets);
            }
        }
        motion::POPPED | motion::KNOCKED_OFF => {
            if !p.popped && it_tincle::timer(item) == 0 {
                let mut at = p.position;
                at.y += POP_RISE;
                p.popped = true;
                ground_effect(
                    &assets,
                    &mut state.particles,
                    &mut state.effects.events,
                    &mut state.rng,
                    draws,
                    POP_PARTICLE,
                    at,
                )?;
            }
            state.map.joint_list_remove(TINGLE_JOINT);
        }
        motion::FALL => {
            if !p.splashed && p.position.y <= 0.0 {
                p.splashed = true;
                ground_effect(
                    &assets,
                    &mut state.particles,
                    &mut state.effects.events,
                    &mut state.rng,
                    draws,
                    TINGLE_SPLASH_PARTICLE,
                    p.position,
                )?;
            }
        }
        // Case 9: a sound on the landing.
        motion::FLOAT_UP => {
            if p.last_motion != p.motion {
                p.popped = false;
                state.map.joint_list_add(TINGLE_JOINT);
            }
            if p.riders != 0 {
                it_tincle::sink(item, tingle_assets);
            }
        }
        _ => {}
    }
    let SceneStage::GreatBay(stage) = &mut state.stage else {
        unreachable!()
    };
    let p = &mut stage.tingle;
    p.last_motion = p.motion;
    let model = state.stage_animations.get_mut(&TINGLE_MAP).unwrap();
    model.set_gobj_translate(p.position);
    update_collision(model, TINGLE_MAP, &mut state.map);
    p.last_riders = p.riders;
    p.riders = 0;
    p.weight = 0.0;
    update_collision(model, TINGLE_MAP, &mut state.map);
    let _ = (world, item_objects);
    Ok(())
}
