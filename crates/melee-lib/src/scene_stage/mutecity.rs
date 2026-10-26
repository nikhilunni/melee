//! Mute City composition (grmutecity.c): the city and its route (map
//! 0x1E), the platform (map 0x1D) and the cars (map 2) in
//! grMuteCity_801EFC6C's order; the route's script, the track's floor and
//! the cars' items and effects over the scene's collision, camera limits,
//! items and particles.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
    stage_limits::{Edge, StageLimits},
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
    fzero::{CarJoints, CARS},
    last::animation::BackgroundAnimation,
    mutecity::{
        cars::{CarEngine, Cars, Splines},
        procs::{
            Limits, MuteCityStage, CAR_MAP, CITY_MAP, LEFT_SPLINE_BONE,
            PLATFORM_BINDINGS, PLATFORM_MAP, REWIND_BONE, RIGHT_SPLINE_BONE, ROUTE_FRAMES,
            ROUTE_LIGHTS_BONE, TILT_ANIMATION, TILT_BONE, TRACK_START,
        },
        script::{Command, Script, ScriptEngine},
        track::{self, Ends, TrackCollision},
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grMaterial_801C8DE0(item, 0, 0, -12, 0, 0, 2, 15): a car's hurt capsule.
const CAR_HURT: [Vec3; 2] = [Vec3::new(0.0, 0.0, -12.0), Vec3::new(0.0, 0.0, 2.0)];
const CAR_HURT_RADIUS: f32 = 15.0;
/// it_802E6AEC's state for a car (grMaterial_801C8CFC(0, 2, ...)).
const CAR_ITEM_STATE: u16 = 2;
/// Lines 0x32 and 0x34 take this material while the platform rests on the
/// track (command 25).
const TRACK_MATERIAL_LINES: [i32; 2] = [0x32, 0x34];
const TRACK_MATERIAL: u32 = 0xC;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let model = &assets.stage_desc.models[usize::from(map)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
        .map_err(|e| anyhow::anyhow!("Mute City map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    if map == PLATFORM_MAP {
        // grMuteCity_801F01B4's grAnime_801C7FF8, decoded at load.
        animation
            .prepare_subtree(&assets.stage, model, TILT_BONE, TILT_ANIMATION)
            .map_err(|e| anyhow::anyhow!("Mute City platform tilt: {e}"))?;
    }
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<melee_gr::mutecity::Parameters> {
    melee_gr::desc::read_mute_city_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

fn read_splines(assets: &Assets) -> Result<std::sync::Arc<[hsd_archive::desc::spline::Spline; 2]>> {
    let model = &assets.stage_desc.models[usize::from(CITY_MAP)];
    let spline = |bone| {
        melee_gr::desc::bone_spline(&assets.stage, model, bone)
            .map_err(|e| anyhow::anyhow!("Mute City car spline {bone}: {e}"))
    };
    Ok(std::sync::Arc::new([
        spline(RIGHT_SPLINE_BONE)?,
        spline(LEFT_SPLINE_BONE)?,
    ]))
}

/// The platform's collision joints in Ground_801C2ED0's order (GrMc.dat's,
/// then grMc_803E30B0's) and Ground_801C2FE0's (the reverse grouping).
fn archive_bindings(assets: &Assets) -> &[melee_gr::desc::JointMapping] {
    &assets.stage_desc.models[usize::from(PLATFORM_MAP)].joint_mappings
}

/// Ground_801C2FE0 on the platform: grMc_803E30B0's joints, then the
/// archive's.
fn update_platform_collision(
    animation: &mut BackgroundAnimation,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) {
    animation.update_collision(map, &PLATFORM_BINDINGS);
    animation.update_collision(map, archive_bindings(assets));
}

/// The particles a model's frame-zero evaluation asks for.
fn spawn_keys(
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

/// Stage_GetCamBounds*Offset and Stage_GetBlastZone*Offset less the
/// camera's centre (grMuteCity_801EFDF8, x114..x130): `(a + c) - c`.
fn creation_limits(limits: &StageLimits) -> Limits {
    let c = &limits.camera;
    let (x, y) = (c.offset_x, c.offset_y);
    Limits {
        camera: melee_cm::Rect {
            top: c.top() - y,
            bottom: c.bottom() - y,
            left: c.left() - x,
            right: c.right() - x,
        },
        blast: melee_cm::Rect {
            top: c.blast_top() - y,
            bottom: c.blast_bottom() - y,
            left: c.blast_left() - x,
            right: c.blast_right() - x,
        },
    }
}

/// grMuteCity_801EFC6C (0x801EFC6C): map 0; map 0x1E's on_init
/// (grMuteCity_801EFDF8) with the platform's (grMuteCity_801F01B4) and the
/// cars' (grMuteCity_801F044C), the cars' first placement.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let mut animations = Animations::new();
    // Map 0 has no callbacks.
    animations.insert(0, load(assets, 0)?);
    // grMuteCity_801EFDF8: animation 0 (grAnime_801C8138), the route's loop.
    let mut city = load(assets, CITY_MAP)?;
    spawn_keys(&mut city, CITY_MAP, assets, particles, rng)?;
    city.set_loop_range(0.0, ROUTE_FRAMES);
    animations.insert(CITY_MAP, city);
    // grMuteCity_801F01B4: Ground_801C2ED0, animation 0, the loop, then
    // animation 1 on bone 6.
    let mut platform = load(assets, PLATFORM_MAP)?;
    bind_platform(&mut platform, assets, collision);
    spawn_keys(&mut platform, PLATFORM_MAP, assets, particles, rng)?;
    platform.set_loop_range(0.0, ROUTE_FRAMES);
    platform.play_prepared_subtree_at(TILT_BONE, TILT_ANIMATION, 0.0, 1.0);
    animations.insert(PLATFORM_MAP, platform);
    let limits = StageLimits::new(assets);
    let mut stage = MuteCityStage {
        parameters: read_parameters(assets)?,
        splines: read_splines(assets)?,
        script: Script::default(),
        track_raised: false,
        passing_sounds: false,
        ends: Ends::Ramps,
        right_parameter: TRACK_START,
        left_parameter: TRACK_START,
        right_point: Vec3::ZERO,
        left_point: Vec3::ZERO,
        limits: creation_limits(&limits),
        track_callbacks: false,
        tilt_target: 0.0,
        tilt: 0.0,
        // grMuteCity_801F0F4C; the word before the pack order is a fresh
        // boot's zero.
        cars: Cars::new(0),
        car_joints: CarJoints::new(assets.stage_desc.parameters.map_scale),
        hazards: melee_gr::desc::read_mute_city_hazards(&assets.stage)
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    };
    // grMuteCity_801F1A34(cars, city): the first placement makes no item.
    let city = animations.get_mut(&CITY_MAP).unwrap();
    let splines = std::sync::Arc::clone(&stage.splines);
    let lanes = lanes(&splines, city);
    let middle = stage.track_middle();
    let mut engine = CreationCars {
        joints: &mut stage.car_joints,
    };
    stage.cars.step(&stage.parameters, &lanes, middle, &mut engine);
    mark_track(collision);
    Ok((SceneStage::MuteCity(Box::new(stage)), animations))
}

/// mpJointSetB10(4); mpJointSetCb1(4, gp, fn_801F2B58).
fn mark_track(collision: &mut melee_mp::CollMap) {
    collision.joint_set_b10(track::JOINT);
    collision.record_joint_events(track::JOINT, true);
}

/// The collision a saved boundary holds from the creation: the platform's
/// joints bound from its pose before any animation, the track's joint
/// marked.
pub(crate) fn bind_creation_collision(
    assets: &Assets,
    collision: &mut melee_mp::CollMap,
) -> Result<()> {
    let mut platform = load(assets, PLATFORM_MAP)?;
    bind_platform(&mut platform, assets, collision);
    mark_track(collision);
    Ok(())
}

/// Ground_801C2ED0 (0x801C2ED0) on the platform from its pose before any
/// animation: GrMc.dat's joints, then grMc_803E30B0's, each taken as its
/// previous position too.
fn bind_platform(
    animation: &mut BackgroundAnimation,
    assets: &Assets,
    collision: &mut melee_mp::CollMap,
) {
    for bindings in [archive_bindings(assets), &PLATFORM_BINDINGS[..]] {
        for binding in bindings {
            animation.update_collision(collision, std::slice::from_ref(binding));
            collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
        }
    }
}

/// The creation's car step has no camera, items or effects: nothing is
/// near the stage, damaged or close.
struct CreationCars<'a> {
    joints: &'a mut CarJoints,
}
impl CarEngine for CreationCars<'_> {
    fn effect_at(&mut self, kind: u32, _: Vec3) {
        unreachable!("grMuteCity_801EFDF8's car step spawns {kind:#x}")
    }
    fn follow_car(&mut self, _: usize, kind: u32) -> bool {
        unreachable!("grMuteCity_801EFDF8's car step spawns {kind:#x}")
    }
    fn end_car_effects(&mut self, _: usize) {
        unreachable!("grMuteCity_801EFDF8's car step ends effects")
    }
    fn create_item(&mut self, car: usize) -> Option<u32> {
        unimplemented!("grMuteCity_801F1A34 at creation: car {car} near the stage needs an item before the pool")
    }
    fn destroy_item(&mut self, _: u32) {
        unreachable!()
    }
    fn quake(&mut self) {
        unimplemented!("grMuteCity_801F1A34 at creation: a car passing the stage")
    }
    fn camera_z(&self) -> f32 {
        unreachable!("no wreck at creation")
    }
    fn rng(&mut self) -> &mut HsdRng {
        unreachable!("no draw at creation")
    }
    fn joints(&mut self) -> &mut CarJoints {
        self.joints
    }
}

/// The two splines and their joints' world matrices (map 0x1E's bones 9
/// and 5; lb_8000B1CC sets each joint's matrix up).
fn lanes<'a>(
    splines: &'a [hsd_archive::desc::spline::Spline; 2],
    city: &mut BackgroundAnimation,
) -> Splines<'a> {
    Splines {
        right: &splines[0],
        right_matrix: city.joint_matrix(RIGHT_SPLINE_BONE),
        left: &splines[1],
        left_matrix: city.joint_matrix(LEFT_SPLINE_BONE),
    }
}

/// The route's command effects (grMuteCity_801F04B8's switch).
struct Route<'a> {
    city: &'a mut BackgroundAnimation,
    platform: &'a mut BackgroundAnimation,
    map: &'a mut melee_mp::CollMap,
    limits: &'a mut StageLimits,
    stage: &'a mut MuteCityStage,
    map_scale: f32,
}
impl ScriptEngine for Route<'_> {
    fn restart_platform(&mut self) {
        self.platform.request_frame(1.0);
    }
    fn run(&mut self, command: Command) {
        let own = self.stage.limits;
        match command {
            Command::Nothing | Command::Wait(_) => {}
            Command::StopRoute => {
                self.city.set_joint_rates(0.0);
                self.platform.set_rate(0.0);
                self.city.set_bone_rates(ROUTE_LIGHTS_BONE, 1.0);
                self.platform.set_bone_rates(TILT_BONE, 1.0);
            }
            Command::ResumeRoute => {
                self.city.set_joint_rates(1.0);
                self.platform.set_rate(1.0);
            }
            Command::RemoveJoint(joint) => self.map.joint_list_remove(joint),
            Command::AddJoint(joint) => self.map.joint_list_add(joint),
            Command::TrackCallbacks(on) => self.stage.track_callbacks = on,
            Command::LowerTrack => {
                self.map.joint_list_remove(track::JOINT);
                self.stage.track_raised = false;
            }
            Command::RaiseTrack(start) => {
                self.map.joint_list_add(track::JOINT);
                self.stage.track_raised = true;
                self.stage.left_parameter = start;
                self.stage.right_parameter = start;
            }
            Command::HidePlatformBone(bone) => {
                if bone < self.platform.joint_count() {
                    self.platform.set_joint_hidden(bone, true);
                }
            }
            Command::ShowPlatformBone(bone) => {
                if bone < self.platform.joint_count() {
                    self.platform.set_joint_hidden(bone, false);
                }
            }
            Command::CameraTop(v) => {
                self.limits.set_camera(Edge::Top, v * self.map_scale);
                if v > own.camera.top {
                    self.limits.set_blast(Edge::Top, v + (own.blast.top - own.camera.top));
                }
            }
            Command::CameraBottom(v) => {
                self.limits.set_camera(Edge::Bottom, v * self.map_scale);
                if v < own.camera.bottom {
                    self.limits
                        .set_blast(Edge::Bottom, v + (own.blast.bottom - own.camera.bottom));
                }
            }
            Command::CameraLeft(v) => {
                self.limits.set_camera(Edge::Left, v * self.map_scale);
                if v < own.camera.left {
                    self.limits.set_blast(Edge::Left, v + (own.blast.left - own.camera.left));
                }
            }
            Command::CameraRight(v) => {
                self.limits.set_camera(Edge::Right, v * self.map_scale);
                if v > own.camera.right {
                    self.limits
                        .set_blast(Edge::Right, v + (own.blast.right - own.camera.right));
                }
            }
            Command::RestoreCamera => {
                self.limits.set_camera(Edge::Top, own.camera.top);
                self.limits.set_camera(Edge::Bottom, own.camera.bottom);
                self.limits.set_camera(Edge::Left, own.camera.left);
                self.limits.set_camera(Edge::Right, own.camera.right);
                self.limits.set_blast(Edge::Top, own.blast.top);
                self.limits.set_blast(Edge::Bottom, own.blast.bottom);
                self.limits.set_blast(Edge::Left, own.blast.left);
                self.limits.set_blast(Edge::Right, own.blast.right);
            }
            Command::PassingSounds(on) => self.stage.passing_sounds = on,
            // The hazard sign, the overlays, the audio: drawing and sound.
            Command::Warning(_)
            | Command::CityOverlay
            | Command::Audio
            | Command::PlatformOverlay => {}
            Command::PlatformTilt(target) => self.stage.tilt_target = target,
            Command::TrackMaterial(on) => {
                let material = if on { TRACK_MATERIAL } else { 0 };
                for line in TRACK_MATERIAL_LINES {
                    self.map.line_set_material(line, material);
                }
            }
            Command::TrackEnds(bits) => self.stage.ends = Ends::from_bits(bits),
        }
    }
}

struct Floor<'a> {
    map: &'a mut melee_mp::CollMap,
}
impl TrackCollision for Floor<'_> {
    fn snapshot(&mut self) {
        self.map.joint_snapshot_prev_pos(track::JOINT);
    }
    fn set_line(&mut self, line: i32, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.map.line_set_pos(line, x0, y0, x1, y1);
    }
    fn refresh(&mut self) {
        self.map.joint_update_dynamics_and_island(track::JOINT);
        self.map.joint_refresh_island(track::JOINT);
    }
}

/// The scene's side of a car step. The stage itself is out of `state`
/// while its cars step (see `city`).
struct SceneCars<'a> {
    assets: &'a Assets,
    state: &'a mut InitialState,
    world: &'a mut hsd_gobj::TaggedWorld,
    objects: &'a mut crate::scene_items::Objects,
    camera_z: f32,
    joints: &'a mut CarJoints,
    draws: &'a mut DrawLog,
    error: Option<anyhow::Error>,
}
impl SceneCars<'_> {
    fn note(&mut self, result: Result<Option<usize>, hsd_particle::Error>) -> Option<usize> {
        match result {
            Ok(id) => id,
            Err(e) => {
                self.error.get_or_insert(anyhow::anyhow!("{e}"));
                None
            }
        }
    }
}
fn anchor(joints: &mut CarJoints, car: usize) -> melee_it::YakuAnchor {
    melee_it::YakuAnchor {
        position: joints.world_position(car),
        scale: joints.scale(car),
        rotation: joints.rotation(car),
    }
}
impl CarEngine for SceneCars<'_> {
    /// grLib_801C96F8(kind, 0, &position) (0x801C96F8): the common bank's
    /// generator at the origin, its AppSRT at `position` with the ground's
    /// scale (a new AppSRT has status 1; a descriptor's own keeps its
    /// status and stops facing the camera).
    fn effect_at(&mut self, kind: u32, position: Vec3) {
        let bank = &self.assets.common_particle_bank;
        let Some(descriptor) = bank.descriptor(kind) else {
            return;
        };
        let owns_transform = descriptor.kind & 0x20000 != 0;
        let scale = self.assets.stage_desc.parameters.map_scale;
        let mut request = SpawnRequest::new(0, kind, 0);
        request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            translation: position,
            scale: Vec3::new(scale, scale, scale),
            status: if owns_transform { 0 } else { 1 },
            ..Default::default()
        });
        self.state.effects.events.spawn(&request, true, false);
        let spawned = self.state.particles.spawn::<RetailTrig>(
            bank,
            request,
            &mut self.state.rng,
            self.draws,
        );
        if let Some(id) = self.note(spawned) {
            let generator = self.state.particles.generators.iter().find(|g| g.id == id).unwrap();
            let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
            transform.camera_facing = 0;
            self.state.particles.set_application_transform(id, transform);
        }
    }
    /// grMuteCity_801F2AB0 (0x801F2AB0): grLib_801C9808(kind, kind / 1000,
    /// car joint), its AppSRT (psAddGeneratorAppSRT_begin(gen, 0) when the
    /// descriptor has none) at the ground's scale, not facing the camera,
    /// following the joint's translation (type: clear 0x600, set 0x800).
    fn follow_car(&mut self, car: usize, kind: u32) -> bool {
        let bank = &self.assets.common_particle_bank;
        let Some(descriptor) = bank.descriptor(kind) else {
            return false;
        };
        let owns_transform = descriptor.kind & 0x20000 != 0;
        let scale = self.assets.stage_desc.parameters.map_scale;
        let mut request = SpawnRequest::new((kind / 1000) as u8, kind, 0);
        request.joint = Some((joint_id(CAR_MAP, car), self.joints.matrix(car)));
        if !owns_transform {
            request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                status: 0,
                ..Default::default()
            });
        }
        self.state.effects.events.spawn(&request, false, false);
        let spawned = self.state.particles.spawn::<RetailTrig>(
            bank,
            request,
            &mut self.state.rng,
            self.draws,
        );
        let Some(id) = self.note(spawned) else {
            return false;
        };
        let generator = self.state.particles.generator_mut(id).unwrap();
        generator.flags = (generator.flags & !0x600) | 0x800;
        let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
        transform.camera_facing = 0;
        transform.scale = Vec3::new(scale, scale, scale);
        self.state.particles.set_application_transform(id, transform);
        true
    }
    fn end_car_effects(&mut self, car: usize) {
        self.state.particles.kill_joint(joint_id(CAR_MAP, car));
    }
    /// grMaterial_801C8CFC(0, 2, cars, car joint, grMuteCity_801F1A0C,
    /// grMc_803E3C6C[car], NULL), then grMaterial_801C8DE0 and 801C8E08.
    fn create_item(&mut self, car: usize) -> Option<u32> {
        let anchor = anchor(self.joints, car);
        let id = crate::scene_items::spawn_yaku_live(
            &mut self.state.items,
            &mut self.state.effects.gobjs,
            &self.assets.items,
            &mut self.state.map,
            self.world,
            self.objects,
            anchor,
        );
        let item = self.state.items.get_mut(id).unwrap();
        it_yaku::change_state(
            item,
            CAR_ITEM_STATE,
            self.assets.items.get(melee_types::ItemKind::PokemonRandom),
        );
        it_yaku::set_hurt_capsule(item, CAR_HURT, CAR_HURT_RADIUS);
        it_yaku::set_hurt_enabled(item, true);
        Some(id)
    }
    /// grMaterial_801C8CDC: Item_8026A8EC.
    fn destroy_item(&mut self, item: u32) {
        self.state.items.get_mut(item).unwrap().destroyed = true;
        crate::scene_items::tear_down(self.state, self.world, self.objects, item);
    }
    fn quake(&mut self) {
        let state = &mut *self.state;
        state
            .quakes
            .request(&mut state.camera, melee_cm::QuakeKind::Loop, &mut state.effects.gobjs);
    }
    fn camera_z(&self) -> f32 {
        self.camera_z
    }
    fn rng(&mut self) -> &mut HsdRng {
        &mut self.state.rng
    }
    fn joints(&mut self) -> &mut CarJoints {
        self.joints
    }
}

/// A Mute City gobj proc (s_link 4).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    match map {
        CITY_MAP => city(state, draws, world, objects),
        PLATFORM_MAP => {
            // grMuteCity_801F0290: the tilt, then Ground_801C2FE0.
            let SceneStage::MuteCity(stage) = &mut state.stage else {
                unreachable!()
            };
            let tilt = stage.step_tilt();
            let platform = state.stage_animations.get_mut(&PLATFORM_MAP).unwrap();
            let z = platform.joint_rotation_z(TILT_BONE);
            platform.set_joint_rotation_z(TILT_BONE, z * tilt);
            let assets = std::sync::Arc::clone(&state.assets);
            update_platform_collision(platform, &assets, &mut state.map);
            Ok(())
        }
        // grMuteCity_801F04B4 is empty.
        _ => Ok(()),
    }
}

/// grMuteCity_801F0120 (0x801F0120): the script, the track's floor, the
/// cars' driving and placement; lb_800115F4 runs in the scene's dispatch.
fn city(
    state: &mut InitialState,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::MuteCity(stage) = &mut state.stage else {
        unreachable!()
    };
    // fn_801F2B58, the track's floor callback since the last step: only an
    // item (b1234 2, 3 or 5) on it (ground kind 2) while grMc_804D69D4 is
    // set acts (coll->x34_flags.b7).
    for event in state.map.take_joint_events(track::JOINT).iter() {
        ensure!(
            !(stage.track_callbacks
                && matches!(event.collision_type, 2 | 3 | 5)
                && event.ground_kind == 2),
            "fn_801F2B58 (grmutecity.c): an item on the track sets coll x34 b7"
        );
    }
    // grMuteCity_801F04B8.
    let mut city = state.stage_animations.remove(&CITY_MAP).unwrap();
    let frame = city.route_frame();
    let rewound = city.rewound_from(REWIND_BONE);
    let mut script = stage.script;
    {
        let mut route = Route {
            city: &mut city,
            platform: state.stage_animations.get_mut(&PLATFORM_MAP).unwrap(),
            map: &mut state.map,
            limits: &mut state.stage_limits,
            stage,
            map_scale: assets.stage_desc.parameters.map_scale,
        };
        script.step(frame, rewound, &mut route);
    }
    stage.script = script;
    // grMuteCity_801F0D20.
    if stage.track_raised {
        let splines = std::sync::Arc::clone(&stage.splines);
        let (right_parameter, right) = track::cross_plane(
            &splines[0],
            &city.joint_matrix(RIGHT_SPLINE_BONE),
            stage.right_parameter,
        );
        stage.right_parameter = right_parameter;
        let (left_parameter, left) = track::cross_plane(
            &splines[1],
            &city.joint_matrix(LEFT_SPLINE_BONE),
            stage.left_parameter,
        );
        stage.left_parameter = left_parameter;
        track::lay_floor(right, left, stage.ends, &mut Floor { map: &mut state.map });
        stage.right_point = right;
        stage.left_point = left;
    }
    // grMuteCity_801F1328, then grMuteCity_801F1A34.
    stage.cars.plan(&stage.parameters, &mut state.rng);
    // Camera_GetTransformPosition.
    let camera_z = state.camera.transform.position.z;
    let splines = std::sync::Arc::clone(&stage.splines);
    let lanes = lanes(&splines, &mut city);
    let middle = stage.track_middle();
    // The stage leaves `state` while its cars step: a wrecked car's item
    // goes through Item_8026A8EC's whole teardown, which reaches the rest.
    let placeholder = SceneStage::Battlefield(melee_gr::battle::Battlefield {
        phase: melee_gr::battle::BackgroundPhase::Waiting,
        timer: 0,
        current: None,
        previous: None,
        lights: Vec::new(),
    });
    let SceneStage::MuteCity(mut stage) = std::mem::replace(&mut state.stage, placeholder) else {
        unreachable!()
    };
    let stage_ref = &mut *stage;
    let mut engine = SceneCars {
        assets: &assets,
        state,
        world,
        objects,
        camera_z,
        joints: &mut stage_ref.car_joints,
        draws,
        error: None,
    };
    stage_ref
        .cars
        .step(&stage_ref.parameters, &lanes, middle, &mut engine);
    let error = engine.error.take();
    state.stage = SceneStage::MuteCity(stage);
    state.stage_animations.insert(CITY_MAP, city);
    if let Some(error) = error {
        return Err(error);
    }
    publish_cars(state);
    Ok(())
}

/// The cars' items follow their joints, and generators on them read the
/// joints this step moved.
fn publish_cars(state: &mut InitialState) {
    let SceneStage::MuteCity(stage) = &mut state.stage else {
        unreachable!()
    };
    for car in 0..CARS {
        let id = joint_id(CAR_MAP, car);
        if state.particles.has_joint_attachment(id) || state.effects.events.needs_joint_history() {
            let matrix = stage.car_joints.matrix(car);
            state.particles.update_joint(id, matrix);
            state.effects.events.update_joint(id, matrix);
        }
        if let Some(item) = stage.cars.cars[car].item {
            let anchor = anchor(&mut stage.car_joints, car);
            it_yaku::set_anchor(state.items.get_mut(item).unwrap(), anchor);
        }
    }
}

/// grMc_803E3C6C[car] (grMuteCity_801F173C ..), a car item's hit-taken
/// callback: the damage counts toward its smoke and explosion.
pub(crate) fn material_hit(state: &mut InitialState, item: u32, damage: i32) -> Result<()> {
    let SceneStage::MuteCity(stage) = &mut state.stage else {
        unreachable!()
    };
    let car = stage.cars.car_of_item(item);
    ensure!(car.is_some(), "Mute City: a hit on a material item no car owns");
    // it_2E6A_Logic117_DmgReceived converts xCA0 to float (0x802E7008..30).
    stage.cars.take_damage(car.unwrap(), damage as f32);
    Ok(())
}
