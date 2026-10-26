//! Brinstar composition (grzebes.c): the acid (map 8), the two upper parts
//! with their plates (maps 6 and 7), the bubbles between them, the scenery
//! (maps 1 and 4) and the splashes (map 5), over their models, collision
//! joints, material items and particles, in grZebes_801D84A4's order.
mod hazard;
mod models;

pub(crate) use hazard::{acid_device, splash_models_tick};
pub(crate) use models::{load, pulse_ticks};

use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{bail, Result};
use gekko_math::HsdRng;
use hsd_particle::{
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    zebes::{
        acid::{Acid, Quake},
        bubbles::{
            capsule::Capsule, BubbleEngine, BubbleObject, Bubbles, LEFT_ANCHOR_POINT,
            LEFT_BOUND_POINT, LOWER_VERTEX_POINT, RIGHT_ANCHOR_POINT, RIGHT_BOUND_POINT,
            UPPER_VERTEX_POINT,
        },
        island::{Island, IslandEngine},
        plates::{Plate, PlateEngine, PlatePhase},
        procs::*,
        scenery::{AcidReaction, Ornament, OrnamentStep, REACTION_BONE},
        splash::{
            splash_crossings, LEFT_SEGMENTS, LEFT_WIDE_FROM, RIGHT_SEGMENTS, RIGHT_WIDE_FROM,
            SPLASH, WIDE_SPLASH,
        },
        Parameters,
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grZebes_801DB088's burst: grLib_801C96F8(0x7530, 0x1E).
const BURST_PARTICLE: u32 = 0x7530;
/// The stage particle bank grLib_801C96F8's 0x1E names.
const STAGE_BANK: u8 = 0x1E;
/// The common bank (hsd_8039F05C's bank 0).
const COMMON_BANK: u8 = 0;
/// The plates' hurt capsules (grMaterial_801C8DE0): map 6's from y -7 to 7,
/// radius 3; map 7's from -5 to 5, radius 2.
const LEFT_PLATE_HURT: (f32, f32) = (7.0, 3.0);
const RIGHT_PLATE_HURT: (f32, f32) = (5.0, 2.0);

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_zebes_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

/// The bones a map's plate and points use.
#[derive(Clone, Copy)]
struct Side {
    map: u8,
    crack: usize,
    plate: usize,
    wobble: [usize; 2],
    bindings: &'static [melee_gr::desc::JointMapping],
}
const LEFT: Side = Side {
    map: LEFT_MAP,
    crack: LEFT_CRACK_BONE,
    plate: LEFT_PLATE_BONE,
    wobble: LEFT_WOBBLE_BONES,
    bindings: &LEFT_BINDINGS,
};
const RIGHT: Side = Side {
    map: RIGHT_MAP,
    crack: RIGHT_CRACK_BONE,
    plate: RIGHT_PLATE_BONE,
    wobble: RIGHT_WOBBLE_BONES,
    bindings: &RIGHT_BINDINGS,
};

/// Ground_801C2ED0: the map's joints from its current pose, that pose
/// taken as the previous one too.
fn bind(animation: &mut BackgroundAnimation, side: Side, collision: &mut melee_mp::CollMap) {
    animation.update_collision(collision, side.bindings);
    for binding in side.bindings {
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// The bubbles' engine at the stage's creation: their models exist before
/// the match's first tick, and none bursts.
struct Creation<'a> {
    rng: &'a mut HsdRng,
}
impl BubbleEngine for Creation<'_> {
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
    fn create(&mut self) -> BubbleObject {
        BubbleObject::Initial
    }
    fn burst(&mut self, _: BubbleObject, _: Vec3) {
        unreachable!("grZebes_801DC9DC: a bubble burst during the stage's creation")
    }
}

/// grZebes_801D84A4 (0x801D84A4): maps 0, 1, 6 (whose creation makes map 7
/// and the bubbles), 4 and 8, with their draws (map 4's first wait, the
/// acid's first delay and height); the material items follow once the
/// pool exists ([`spawn_items`]).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let mut animations = Animations::new();
    // Map 0 has no on_init: no animation.
    let mut ground = load(assets, 0)?;
    ground.clear_animation();
    animations.insert(0, ground);
    // grZebes_801D9F30: animation 0, nothing on its frame zero.
    let mut reaction = load(assets, REACTION_MAP)?;
    ensure_no_keys(&mut reaction, REACTION_MAP)?;
    animations.insert(REACTION_MAP, reaction);
    // grZebes_801D9100 (map 7, inside map 6's creation): animation 0, then
    // Ground_801C2ED0.
    let mut right = load(assets, RIGHT_MAP)?;
    ensure_no_keys(&mut right, RIGHT_MAP)?;
    bind(&mut right, RIGHT, collision);
    let right_plate = Plate::new(right.joint_translation(RIGHT.wobble[0]).x);
    right.update_collision(collision, RIGHT.bindings);
    animations.insert(RIGHT_MAP, right);
    // grZebes_801D8644 (map 6): Ground_801C2ED0, animation 0, the bubbles.
    let mut left = load(assets, LEFT_MAP)?;
    bind(&mut left, LEFT, collision);
    ensure_no_keys(&mut left, LEFT_MAP)?;
    let left_plate = Plate::new(left.joint_translation(LEFT.wobble[0]).x);
    let bubbles = Bubbles::create(&parameters, &mut Creation { rng: &mut *rng });
    // mpJointSetB10(0), Ground_801C2FE0.
    collision.joint_set_b10(CHAIN_JOINT);
    left.update_collision(collision, LEFT.bindings);
    animations.insert(LEFT_MAP, left);
    // grZebes_801D9508: no animation yet; its first wait.
    let mut ornament_model = load(assets, ORNAMENT_MAP)?;
    ornament_model.clear_animation();
    let ornament = Ornament::create(rng);
    animations.insert(ORNAMENT_MAP, ornament_model);
    // grZebes_801D9798: animation 0, the first delay and height.
    let mut acid_model = load(assets, ACID_MAP)?;
    ensure_no_keys(&mut acid_model, ACID_MAP)?;
    let acid = Acid::create(&parameters, rng);
    acid_model.set_joint_translate_y(ACID_BONE, acid.level);
    animations.insert(ACID_MAP, acid_model);
    for key in SPLASH_KEYS {
        let mut splash = load(assets, SPLASH_MAP)?;
        splash.set_hidden(true);
        animations.insert(key, splash);
    }
    let surface = acid.surface();
    let pulse_ticks = pulse_ticks(assets)?;
    let stage = ZebesStage {
        parameters,
        acid,
        // Ground_801C438C twice: both the surface.
        floor: surface,
        previous_floor: surface,
        left_plate,
        right_plate,
        left_plate_seen: 0,
        right_plate_seen: 0,
        island: Island::new(),
        bubbles,
        ornament,
        reaction: AcidReaction::default(),
        splashes: 0,
        acid_device: false,
        left_plate_item: None,
        right_plate_item: None,
        bubble_item: None,
        pulse_ticks,
        live_splashes: Default::default(),
        splash_cursor: 0,
    };
    Ok((SceneStage::Zebes(Box::new(stage)), animations))
}

/// The splash models' scheduler keys (map 5 instances, at most
/// [`MOST_SPLASHES`] at once).
pub(crate) const SPLASH_KEYS: std::ops::Range<u8> =
    SPLASH_KEY_BASE..SPLASH_KEY_BASE + MOST_SPLASHES as u8;

/// A map's frame-zero evaluation; none of these maps has particle keys.
fn ensure_no_keys(animation: &mut BackgroundAnimation, map: u8) -> Result<()> {
    if !animation.evaluate_initial_frame::<RetailTrig>().is_empty() {
        bail!("Brinstar map {map}: frame-zero particle keys");
    }
    Ok(())
}

/// A bone's world translation, scale and rotation, which a material item
/// copies (it_2E6A_UnkMotion19_Phys).
fn anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// grZebes_801D9100's and grZebes_801D8644's material items, in creation
/// order: the right plate's on bone 15, the left plate's on bone 30, the
/// bubbles' at the origin; each takes hits (grMaterial_801C8E08) on its
/// capsule.
pub(crate) fn spawn_items(
    stage: &mut SceneStage,
    animations: &mut Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
) {
    let SceneStage::Zebes(stage) = stage else {
        return;
    };
    let mut plate = |side: Side, (half, radius): (f32, f32)| {
        let model = animations.get_mut(&side.map).unwrap();
        let id = crate::scene_items::spawn_yaku(
            items,
            gobjs,
            resources,
            map,
            anchor(model, side.wobble[0]),
        );
        let item = items.get_mut(id).unwrap();
        it_yaku::set_hurt_capsule(
            item,
            [Vec3::new(0.0, -half, 0.0), Vec3::new(0.0, half, 0.0)],
            radius,
        );
        it_yaku::set_hurt_enabled(item, true);
        id
    };
    stage.right_plate_item = Some(plate(RIGHT, RIGHT_PLATE_HURT));
    stage.left_plate_item = Some(plate(LEFT, LEFT_PLATE_HURT));
    // grMaterial_801C8D44 at grZe_803B7FF0's first point (the origin).
    let origin = melee_it::YakuAnchor {
        position: Vec3::ZERO,
        scale: Vec3::new(1.0, 1.0, 1.0),
        rotation: Vec3::ZERO,
    };
    let id = crate::scene_items::spawn_yaku(items, gobjs, resources, map, origin);
    it_yaku::set_hurt_enabled(items.get_mut(id).unwrap(), true);
    stage.bubble_item = Some(id);
}

/// The deferred calls Ground_801C0FB8 runs when the countdown releases the
/// stage, newest first: grZebes_801D9758 (the acid's device and its
/// countdown), then fn_801D94F0 (map 4's clock).
pub(crate) fn start(state: &mut InitialState) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.acid_device = true;
    stage.acid.start();
    stage.ornament.waiting = false;
}

/// A Brinstar gobj proc (s_link 4).
pub(crate) fn run_proc(state: &mut InitialState, map: u8, draws: &mut DrawLog) -> Result<()> {
    match map {
        REACTION_MAP => reaction(state),
        LEFT_MAP => left_part(state, draws)?,
        RIGHT_MAP => right_part(state, draws)?,
        ORNAMENT_MAP => ornament(state),
        ACID_MAP => {
            acid(state);
            hazard::splash_procs(state)?;
        }
        _ => {}
    }
    Ok(())
}

/// grZebes_801D9F84 (0x801D9F84).
fn reaction(state: &mut InitialState) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let model = state.stage_animations.get_mut(&REACTION_MAP).unwrap();
    let ended = model.subtree_animation_ended(REACTION_BONE);
    if let Some(index) = stage.reaction.tick(stage.floor, ended) {
        // grAnime_801C7FF8(gobj, 3, 7, index, 0.0, 1.0); a shake stops
        // looping (grAnime_801C78FC(gobj, 3, 7)).
        model.play_prepared_subtree(REACTION_BONE, index);
        if index != melee_gr::zebes::scenery::STILL {
            model.clear_subtree_loops(REACTION_BONE);
        }
    }
}

/// grZebes_801D95B8 (0x801D95B8): map 4 follows map 6's bone 14 and plays
/// its animations in turn.
fn ornament(state: &mut InitialState) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let platform = state.stage_animations[&LEFT_MAP].joint_translation(LEFT_PLATFORM_BONE);
    let model = state.stage_animations.get_mut(&ORNAMENT_MAP).unwrap();
    model.set_gobj_translate(platform);
    let ended = model.first_animation_ended();
    if let OrnamentStep::Play(index) = stage.ornament.tick(ended, &mut state.rng) {
        model
            .select_animation(
                &state.assets.stage,
                &state.assets.stage_desc.models[usize::from(ORNAMENT_MAP)],
                index,
            )
            .expect("Brinstar map 4's animations are prepared at load");
        // grAnime_801C8138 ends with HSD_JObjAnimAll: frame zero now (map 4
        // has no particle keys).
        let keys = model.evaluate_initial_frame::<RetailTrig>();
        assert!(keys.is_empty(), "Brinstar map 4: particle keys");
    }
}

/// grZebes_801D99E0 (0x801D99E0): the acid moves; its bone and the stage's
/// floor (Ground_801C438C) follow. The fog and light colours are display
/// only.
fn acid(state: &mut InitialState) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let mut quakes = Quakes {
        quakes: &mut state.quakes,
        camera: &mut state.camera,
        gobjs: &mut state.effects.gobjs,
    };
    let parameters = stage.parameters;
    stage.acid.tick(&parameters, &mut state.rng, &mut quakes);
    let model = state.stage_animations.get_mut(&ACID_MAP).unwrap();
    model.set_joint_translate_y(ACID_BONE, stage.acid.level);
    let surface = stage.acid.surface();
    stage.set_floor(surface);
}

struct Quakes<'a> {
    quakes: &'a mut crate::quake::Quakes,
    camera: &'a mut melee_cm::GameCamera,
    gobjs: &'a mut hsd_gobj::GObjAddresses,
}
impl Quake for Quakes<'_> {
    fn quake(&mut self) {
        self.quakes
            .request(self.camera, melee_cm::QuakeKind::Loop, self.gobjs);
    }
}

/// A plate's model and item.
struct PlateModel<'a> {
    model: &'a mut BackgroundAnimation,
    item: Option<&'a mut melee_it::ItemCore>,
    side: Side,
}
impl PlateEngine for PlateModel<'_> {
    fn play(&mut self, animation: usize) {
        self.model
            .play_subtree_tracks(self.side.crack, animation, 0.0, 1.0);
    }
    fn stop_looping(&mut self) {
        self.model.clear_subtree_loops(self.side.crack);
    }
    fn ended(&self) -> bool {
        self.model.subtree_track_ended(self.side.crack)
    }
    fn request_frame(&mut self, frame: f32) {
        self.model.request_subtree_frame(self.side.crack, frame);
    }
    fn set_wobble(&mut self, x: f32) {
        for bone in self.side.wobble {
            self.model.set_joint_translate_x(bone, x);
        }
    }
    fn set_hits(&mut self, on: bool) {
        if let Some(item) = self.item.as_deref_mut() {
            it_yaku::set_hurt_enabled(item, on);
        }
    }
}

/// grZebes_801DA528 for one side, then the plate's collision animation when
/// its phase changed (from frame 30 as it breaks, from 0 as it grows).
fn step_plate(state: &mut InitialState, side: Side) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let (plate, seen, item) = if side.map == LEFT_MAP {
        (
            &mut stage.left_plate,
            &mut stage.left_plate_seen,
            stage.left_plate_item,
        )
    } else {
        (
            &mut stage.right_plate,
            &mut stage.right_plate_seen,
            stage.right_plate_item,
        )
    };
    let model = state.stage_animations.get_mut(&side.map).unwrap();
    let mut engine = PlateModel {
        model,
        item: item.and_then(|id| state.items.get_mut(id)),
        side,
    };
    let phase = plate.step(&stage.parameters, &mut state.rng, &mut engine) as i32;
    if *seen != phase {
        *seen = phase;
        let model = state.stage_animations.get_mut(&side.map).unwrap();
        let start = match PlatePhase::from_saved(phase as u8) {
            Some(PlatePhase::Breaking) => Some((melee_gr::zebes::plates::CRACK_ANIMATION, 30.0)),
            Some(PlatePhase::Growing) => Some((melee_gr::zebes::plates::GROWTH_ANIMATION, 0.0)),
            _ => None,
        };
        if let Some((animation, frame)) = start {
            model.play_subtree_tracks(side.plate, animation, frame, 1.0);
            model.clear_subtree_loops(side.plate);
        }
    }
}

/// The bubbles' engine during the match.
struct Live<'a> {
    rng: &'a mut HsdRng,
    gobjs: &'a mut hsd_gobj::GObjAddresses,
    particles: &'a mut ParticleSystem,
    events: &'a mut melee_ef::fixture_spawns::EventSink,
    assets: &'a Assets,
    draws: &'a mut DrawLog,
    error: Option<anyhow::Error>,
}
impl BubbleEngine for Live<'_> {
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
    fn create(&mut self) -> BubbleObject {
        BubbleObject::Live(self.gobjs.create(hsd_gobj::GObjClass::Stage))
    }
    fn burst(&mut self, object: BubbleObject, position: Vec3) {
        // Ground_801C4A08.
        let address = match object {
            BubbleObject::Live(address) => address,
            BubbleObject::Initial => self.gobjs.outside(),
        };
        self.gobjs.destroy(hsd_gobj::GObjClass::Stage, address);
        if let Err(error) = ground_effect(
            self.assets,
            self.particles,
            self.events,
            self.rng,
            self.draws,
            (BURST_PARTICLE, STAGE_BANK),
            position,
        ) {
            self.error.get_or_insert(error);
        }
    }
}

/// `grLib_801C96F8(kind, bank, position)` (0x801C96F8): a generator whose
/// AppSRT carries the position, scaled by the map scale (as
/// stadium::spawn_ground_effect, in either bank).
pub(super) fn ground_effect(
    assets: &Assets,
    particles: &mut ParticleSystem,
    events: &mut melee_ef::fixture_spawns::EventSink,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
    (kind, bank): (u32, u8),
    position: Vec3,
) -> Result<Option<usize>> {
    let particle_bank = assets.stage_key_bank(if bank == COMMON_BANK { 0 } else { 1 });
    let Some(descriptor) = particle_bank.descriptor(kind) else {
        return Ok(None);
    };
    let owns_transform = descriptor.kind & 0x20000 != 0;
    let scale = assets.stage_desc.parameters.map_scale;
    let mut request = SpawnRequest::new(bank, kind, 0);
    request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
        translation: position,
        scale: Vec3::new(scale, scale, scale),
        status: if owns_transform { 0 } else { 1 },
        ..Default::default()
    });
    events.spawn(&request, true, false);
    let id = particles.spawn::<RetailTrig>(particle_bank, request, rng, draws)?;
    if let Some(id) = id {
        let generator = particles.generators.iter().find(|g| g.id == id).unwrap();
        let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
        transform.camera_facing = 0;
        particles.set_application_transform(id, transform);
    }
    Ok(id)
}

/// The top platform's animations on maps 6 and 7.
struct IslandModels<'a> {
    left: &'a mut BackgroundAnimation,
    right: &'a mut BackgroundAnimation,
}
impl IslandEngine for IslandModels<'_> {
    fn play(&mut self, animation: usize) {
        self.left
            .play_joint_track(LEFT_PLATFORM_BONE, animation, 0.0, 1.0);
        self.left.clear_joint_loop(LEFT_PLATFORM_BONE);
        self.right
            .play_joint_track(RIGHT_PART_BONE, animation, 0.0, 1.0);
        self.right.clear_joint_loop(RIGHT_PART_BONE);
    }
    fn ended(&self) -> bool {
        self.left.subtree_track_ended(LEFT_PLATFORM_BONE)
    }
}

/// grZebes_801D881C (0x801D881C): the left plate, the bubbles' pulse,
/// forces, capsule and loose bubbles, the top platform, the left points,
/// the chain's collision line, the splashes along the left part, then
/// Ground_801C2FE0 (lb_800115F4 runs in the scene dispatch).
fn left_part(state: &mut InitialState, draws: &mut DrawLog) -> Result<()> {
    step_plate(state, LEFT);
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let mut live = Live {
        rng: &mut state.rng,
        gobjs: &mut state.effects.gobjs,
        particles: &mut state.particles,
        events: &mut state.effects.events,
        assets: &assets,
        draws,
        error: None,
    };
    // Each bubble model's Ground_801C1CD0 advanced its pulse.
    stage.bubbles.age_pulses();
    stage
        .island
        .tick_pulse(&mut stage.bubbles, &parameters, &mut live);
    let broken = stage
        .bubbles
        .update(&parameters, stage.pulse_ticks, &mut live);
    // grZebes_801DC260 places the bubbles' models: display only.
    let capsule = stage.bubbles.capsule();
    stage
        .island
        .grow_loose(&mut stage.bubbles, &parameters, &mut live);
    let mut models = {
        let (mut left, mut right) = (None, None);
        for (key, animation) in state.stage_animations.iter_mut() {
            match *key {
                LEFT_MAP => left = Some(animation),
                RIGHT_MAP => right = Some(animation),
                _ => {}
            }
        }
        IslandModels {
            left: left.unwrap(),
            right: right.unwrap(),
        }
    };
    stage.island.tick(
        broken,
        &mut stage.bubbles,
        &parameters,
        &mut live,
        &mut models,
    );
    if let Some(error) = live.error {
        return Err(error);
    }
    if let Some(id) = stage.bubble_item {
        if let Some(item) = state.items.get_mut(id) {
            set_bubble_capsule(item, capsule);
        }
    }
    let left = state.stage_animations.get_mut(&LEFT_MAP).unwrap();
    stage.bubbles.points.anchors[0] = left.joint_point(LEFT_PLATFORM_BONE, LEFT_ANCHOR_POINT);
    stage.bubbles.points.bounds[0] = left.joint_point(LEFT_PLATFORM_BONE, LEFT_BOUND_POINT);
    if stage.island.phase == melee_gr::zebes::island::IslandPhase::Standing {
        state.map.joint_list_add(CHAIN_JOINT);
        state.map.joint_snapshot_prev_pos(CHAIN_JOINT);
        let upper = left.joint_point(LEFT_PLATFORM_BONE, UPPER_VERTEX_POINT);
        // HSD_ASSERT(0x299): map 7's bone 1.
        let right = state.stage_animations.get_mut(&RIGHT_MAP).unwrap();
        let lower = right.joint_point(RIGHT_PART_BONE, LOWER_VERTEX_POINT);
        let vertices = match stage.bubbles.line((upper.x, upper.y), (lower.x, lower.y)) {
            Ok(vertices) => vertices,
            Err(column) => unimplemented!(
                "grzebes.c:613-632: no held bubble reaches the chain line's column {column}, whose x \
                 the C leaves to its stack (column_x uninitialised)"
            ),
        };
        for (index, (x, y)) in vertices.into_iter().enumerate() {
            state.map.vtx_set_pos(index as i32, x, y);
        }
        state.map.joint_update_dynamics_and_island(CHAIN_JOINT);
    } else {
        state.map.joint_list_remove(CHAIN_JOINT);
    }
    splash_lines(
        state,
        &assets,
        LEFT_MAP,
        LEFT_PLATFORM_BONE,
        &LEFT_SEGMENTS,
        LEFT_WIDE_FROM,
        draws,
    )?;
    finish_side(state, LEFT);
    Ok(())
}

/// grZebes_801D925C (0x801D925C): the right plate, the right points, the
/// splashes along the right part, then Ground_801C2FE0.
fn right_part(state: &mut InitialState, draws: &mut DrawLog) -> Result<()> {
    step_plate(state, RIGHT);
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let right = state.stage_animations.get_mut(&RIGHT_MAP).unwrap();
    stage.bubbles.points.anchors[1] = right.joint_point(RIGHT_PART_BONE, RIGHT_ANCHOR_POINT);
    stage.bubbles.points.bounds[1] = right.joint_point(RIGHT_PART_BONE, RIGHT_BOUND_POINT);
    splash_lines(
        state,
        &assets,
        RIGHT_MAP,
        RIGHT_PART_BONE,
        &RIGHT_SEGMENTS,
        RIGHT_WIDE_FROM,
        draws,
    )?;
    finish_side(state, RIGHT);
    Ok(())
}

/// Ground_801C2FE0 on a part's joints; its plate's item follows the
/// wobbling bone; generators on its bones read the moved matrices.
fn finish_side(state: &mut InitialState, side: Side) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let model = state.stage_animations.get_mut(&side.map).unwrap();
    model.update_collision(&mut state.map, side.bindings);
    let item = if side.map == LEFT_MAP {
        stage.left_plate_item
    } else {
        stage.right_plate_item
    };
    if let Some(item) = item.and_then(|id| state.items.get_mut(id)) {
        it_yaku::set_anchor(item, anchor(model, side.wobble[0]));
    }
    super::publish_joint_matrices(
        model,
        side.map,
        &mut state.particles,
        &mut state.effects.events,
    );
}

/// grMaterial_801C8DE0 with the bubbles' capsule, or grMaterial_801C8E28
/// with none (grZebes_801DBB60).
fn set_bubble_capsule(item: &mut melee_it::ItemCore, capsule: Option<Capsule>) {
    match capsule {
        None => it_yaku::set_hurt_enabled(item, false),
        Some(capsule) => {
            it_yaku::set_hurt_enabled(item, true);
            it_yaku::set_hurt_capsule(item, capsule.ends, capsule.radius);
        }
    }
}

/// grZakoGenerator_801CA43C with a part's segments on its `bone`, at the
/// acid's surface (Ground_801C4368).
fn splash_lines(
    state: &mut InitialState,
    assets: &Assets,
    map: u8,
    bone: usize,
    segments: &[[Vec3; 2]],
    wide_from: usize,
    draws: &mut DrawLog,
) -> Result<()> {
    let SceneStage::Zebes(stage) = &state.stage else {
        unreachable!()
    };
    let height = stage.floor;
    let model = state.stage_animations.get_mut(&map).unwrap();
    let mut world = [[Vec3::ZERO; 2]; 10];
    for (index, [start, end]) in segments.iter().enumerate() {
        world[index] = [
            model.joint_point(bone, *start),
            model.joint_point(bone, *end),
        ];
    }
    let mut result = Ok(());
    let (particles, events) = (&mut state.particles, &mut state.effects.events);
    splash_crossings(
        &world[..segments.len()],
        height,
        &mut state.rng,
        |position, index, rng| {
            // grZebes_801DCB64 / 801DCBB0.
            let kind = if index < wide_from {
                SPLASH
            } else {
                WIDE_SPLASH
            };
            if let Err(error) = ground_effect(
                assets,
                particles,
                events,
                rng,
                draws,
                (kind, COMMON_BANK),
                position,
            ) {
                result = Err(error);
            }
        },
    );
    result
}

/// A material item's hit-taken callback (yaku.x18): a plate's
/// (fn_801DA9D8 / fn_801DA9F0) gathers the damage and takes the launch's x
/// as its wobble speed; the bubbles' (fn_801DAC90) bursts one.
pub(crate) fn material_hit(state: &mut InitialState, item: u32, damage: i32, launch_x: f32) {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    if Some(item) == stage.left_plate_item {
        stage.left_plate.hit(damage as f32, launch_x);
    } else if Some(item) == stage.right_plate_item {
        stage.right_plate.hit(damage as f32, launch_x);
    } else if Some(item) == stage.bubble_item {
        stage.bubbles.hit();
    }
}

/// The joint id a splash model's generators attach to.
pub(crate) fn splash_joint(key: u8, joint: usize) -> usize {
    joint_id(key, joint)
}
