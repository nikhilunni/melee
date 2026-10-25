//! Onett composition (gronett.c): the street (map 5), the drugstore roof
//! (map 4) and the cars (map 3) in grOnett_801E3738's order, the cars'
//! material items, the awnings', roof's and cars' callbacks.
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
    last::animation::BackgroundAnimation,
    onett::{
        cars::{CarEngine, Cars, RIGHT_END},
        procs::{
            OnettStage, AWNING_BONES, AWNING_JOINTS, AWNING_PARTICLE, CAR_BONES, CAR_MAP,
            CAR_PARTICLE, CAR_SMOKE_BONES, CHIMNEY_BONE, CHIMNEY_OFFSET, MAP_ORDER,
            ROOF_BLINK_BONES, ROOF_JOINTS, ROOF_MAP, SMOKE_PARTICLE, STREET_MAP,
            SUBJECT_DEPTH, SUBJECT_EXTENTS,
        },
        Awning, RoofEngine, Roof, Smoke, SmokeStep,
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// `coll->x34_flags.b1234` of the colliders grOnett_801E54B4 counts.
const COUNTED_COLLIDERS: std::ops::RangeInclusive<u8> = 1..=3;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Onett map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<melee_gr::onett::Parameters> {
    melee_gr::desc::read_onett_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

fn bindings(assets: &Assets, map: u8) -> &[melee_gr::desc::JointMapping] {
    &assets.stage_desc.models[usize::from(map)].joint_mappings
}

/// Ground_801C2ED0: a map's archive joints from the pose before any
/// animation, that pose taken as the previous one too.
fn bind(animation: &mut BackgroundAnimation, assets: &Assets, map: u8, collision: &mut melee_mp::CollMap) {
    animation.update_collision(collision, bindings(assets, map));
    for binding in bindings(assets, map) {
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
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
        particles.spawn::<RetailTrig>(assets.stage_key_bank(event.bank), request, rng, &mut DrawLog::default())?;
    }
    Ok(())
}

/// The roof's animation (grAnime_801C7FF8 on bone 0) at `rate`.
fn play_roof(animation: &mut BackgroundAnimation, assets: &Assets, index: usize, rate: f32) -> Result<()> {
    let model = &assets.stage_desc.models[usize::from(ROOF_MAP)];
    animation
        .play_subtree(&assets.stage, model, 0, index, 0.0, rate)
        .map_err(|e| anyhow::anyhow!("Onett roof animation {index}: {e}"))
}

/// grOnett_801E3738 (0x801E3738): maps 0, 2, 5, 4, 3 with their draws
/// (map 5's smoke pause, map 3's discarded HSD_Randi(4)), the cars' items
/// made once the pool exists ([`spawn_items`]).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let mut animations = Animations::new();
    let (mut smoke, mut cars, mut awnings) = (None, None, None);
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        match map {
            // grOt_803E27E0[0] is empty: no animation is requested.
            0 => animation.clear_animation(),
            // grOnett_801E38DC: animation 0.
            2 => spawn_keys(&mut animation, map, assets, particles, rng)?,
            STREET_MAP => {
                // grOnett_801E3A34: Ground_801C2ED0, then animation 0 (none).
                bind(&mut animation, assets, map, collision);
                spawn_keys(&mut animation, map, assets, particles, rng)?;
                awnings = Some(AWNING_BONES.map(|bone| Awning::new(animation.joint_translation(bone).y)));
                smoke = Some(Smoke::new(&parameters, rng));
                animation.update_collision(collision, bindings(assets, map));
            }
            ROOF_MAP => {
                // grOnett_801E3CE4: Ground_801C2ED0, animation 0 held at 0.
                bind(&mut animation, assets, map, collision);
                animation.clear_animation();
                play_roof(&mut animation, assets, 0, 0.0)?;
                for joint in ROOF_JOINTS {
                    collision.record_joint_events(joint, true);
                }
            }
            CAR_MAP => {
                // grOnett_801E41C8: animation 0, the cars parked and hidden.
                spawn_keys(&mut animation, map, assets, particles, rng)?;
                for bone in CAR_BONES {
                    animation.set_joint_translate_x(bone, RIGHT_END);
                    animation.set_joint_hidden(bone, true);
                }
                cars = Some(Cars::new(rng));
            }
            _ => unreachable!(),
        }
        animations.insert(map, animation);
    }
    let stage = OnettStage {
        parameters,
        awnings: awnings.unwrap(),
        smoke: smoke.unwrap(),
        subject: None,
        camera_phase: 0,
        roof: Roof::new(),
        cars: cars.unwrap(),
        car_items: [None; 4],
        smoke_generator: None,
        lights: melee_gr::battle::lights::load_model(
            &assets.stage,
            &assets.stage_desc,
            usize::from(STREET_MAP),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?,
    };
    Ok((SceneStage::Onett(Box::new(stage)), animations))
}

fn anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// grOnett_801E41C8's grMaterial_801C8CFC(0, 0, gp, car, grOnett_801E5030,
/// NULL, NULL) per car, its hits on (0x801C8E08) and no hitlag
/// (it_80274C60).
pub(crate) fn spawn_items(
    stage: &mut SceneStage,
    animations: &mut Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
) {
    let SceneStage::Onett(stage) = stage else {
        return;
    };
    let model = animations.get_mut(&CAR_MAP).unwrap();
    for (index, bone) in CAR_BONES.into_iter().enumerate() {
        let id = crate::scene_items::spawn_yaku(items, gobjs, resources, map, anchor(model, bone));
        let item = items.get_mut(id).unwrap();
        it_yaku::set_hurt_enabled(item, true);
        item.hitlag_enabled = false;
        stage.car_items[index] = Some(id);
    }
}

/// The deferred calls Ground_801C0FB8 runs when the countdown releases the
/// stage, newest first: grOnett_801E41B0 (the cars may go), then
/// grOnett_801E3930 (the awnings' callbacks, lane A's camera subject).
pub(crate) fn start(state: &mut InitialState) {
    let SceneStage::Onett(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.cars.waiting = false;
    for joint in AWNING_JOINTS {
        state.map.record_joint_events(joint, true);
    }
    let mut subject = melee_cm::Subject::new(melee_cm::SubjectState::Inactive);
    let [left, right, top, bottom] = SUBJECT_EXTENTS;
    subject.target_extents.left = left;
    subject.target_extents.right = right;
    subject.target_extents.top = top;
    subject.target_extents.bottom = bottom;
    subject.target_extents.radius = SUBJECT_DEPTH;
    subject.position = Vec3::ZERO;
    subject.bone_position = Vec3::ZERO;
    stage.subject = Some(subject);
}

/// grLib_801C9808(kind, bank, bone): a generator on a model bone.
fn spawn_on_bone(
    state: &mut InitialState,
    map: u8,
    bone: usize,
    (kind, bank): (u32, u8),
    draws: &mut DrawLog,
) -> Result<()> {
    let matrix = state.stage_animations.get_mut(&map).unwrap().joint_matrix(bone);
    let mut request = SpawnRequest::new(bank, kind, 0);
    request.joint = Some((joint_id(map, bone), matrix));
    state.effects.events.spawn(&request, false, false);
    let assets = std::sync::Arc::clone(&state.assets);
    let particle_bank = if bank == 0 {
        &assets.common_particle_bank
    } else {
        &assets.particle_bank
    };
    state
        .particles
        .spawn::<RetailTrig>(particle_bank, request, &mut state.rng, draws)?;
    Ok(())
}

struct RoofModel<'a> {
    model: &'a mut BackgroundAnimation,
    assets: &'a Assets,
    map: &'a mut melee_mp::CollMap,
    error: Option<anyhow::Error>,
}
impl RoofEngine for RoofModel<'_> {
    fn animation_ended(&self) -> bool {
        self.model.first_animation_ended()
    }
    fn play(&mut self, index: usize) {
        if let Err(e) = play_roof(self.model, self.assets, index, 1.0) {
            self.error.get_or_insert(e);
        }
    }
    fn set_solid(&mut self, joint: i32, solid: bool) {
        if solid {
            self.map.joint_list_add(joint);
        } else {
            self.map.joint_list_remove(joint);
        }
    }
    fn blink(&mut self, hidden: bool) {
        if hidden {
            for bone in ROOF_BLINK_BONES {
                self.model.set_joint_hidden(bone, true);
            }
        } else {
            self.model.set_hidden(false);
        }
    }
}

struct CarModels<'a> {
    model: &'a mut BackgroundAnimation,
    items: &'a mut melee_it::ItemPool,
    item_assets: &'a melee_it::desc::ItemAssets,
    car_items: [Option<u32>; 4],
    view: (f32, f32, f32),
    fighters: Vec<(f32, i32)>,
    camera_phase: &'a mut i32,
    rng: &'a mut HsdRng,
    particles: &'a mut ParticleSystem,
    events: &'a mut melee_ef::fixture_spawns::EventSink,
    bank: &'a hsd_particle::bank::ParticleBank,
    draws: &'a mut DrawLog,
    error: Option<anyhow::Error>,
}
impl CarEngine for CarModels<'_> {
    fn translate_x(&self, car: usize) -> f32 {
        self.model.joint_translation(CAR_BONES[car]).x
    }
    fn set_translate_x(&mut self, car: usize, x: f32) {
        self.model.set_joint_translate_x(CAR_BONES[car], x);
    }
    fn set_translate_z(&mut self, car: usize, z: f32) {
        self.model.set_joint_translate_z(CAR_BONES[car], z);
    }
    fn set_body_rotation_y(&mut self, car: usize, radians: f32) {
        self.model.set_joint_rotation_y(CAR_BONES[car] + 1, radians);
    }
    fn add_body_rotation_y(&mut self, car: usize, radians: f32) {
        let y = self.model.joint_rotation(CAR_BONES[car] + 1).y;
        self.model.set_joint_rotation_y(CAR_BONES[car] + 1, y + radians);
    }
    fn set_hidden(&mut self, car: usize, hidden: bool) {
        self.model.set_joint_hidden(CAR_BONES[car], hidden);
    }
    fn item_state(&mut self, car: usize, state: u16) {
        let id = self.car_items[car].expect("a car's item");
        it_yaku::change_state(self.items.get_mut(id).unwrap(), state, self.item_assets);
    }
    /// grLib_801C9808(0x2C, 0, car_jobjs2[car]): the common bank's smoke.
    fn smoke(&mut self, car: usize) {
        let bone = CAR_SMOKE_BONES[car];
        let matrix = self.model.joint_matrix(bone);
        let (kind, bank) = CAR_PARTICLE;
        let mut request = SpawnRequest::new(bank, kind, 0);
        request.joint = Some((joint_id(CAR_MAP, bone), matrix));
        self.events.spawn(&request, false, false);
        if let Err(e) = self.particles.spawn::<RetailTrig>(self.bank, request, self.rng, self.draws) {
            self.error.get_or_insert(anyhow::anyhow!("{e}"));
        }
    }
    fn ground_view(&self) -> (f32, f32, f32) {
        self.view
    }
    fn fighters(&self) -> &[(f32, i32)] {
        &self.fighters
    }
    fn set_camera_phase(&mut self, phase: i32) {
        *self.camera_phase = phase;
    }
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
}

/// An Onett gobj proc (s_link 4).
pub(crate) fn run_proc(state: &mut InitialState, map: u8, draws: &mut DrawLog) -> Result<()> {
    step_map(state, map, draws)?;
    // Generators on these JObjs read their world matrices in the particle
    // proc, after this proc moved them.
    if let Some(animation) = state.stage_animations.get_mut(&map) {
        super::publish_joint_matrices(animation, map, &mut state.particles, &mut state.effects.events);
    }
    Ok(())
}

fn step_map(state: &mut InitialState, map: u8, draws: &mut DrawLog) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    match map {
        STREET_MAP => street(state, &assets, draws),
        ROOF_MAP => {
            let mut landings = Vec::new();
            for joint in ROOF_JOINTS {
                for event in state.map.take_joint_events(joint).iter() {
                    landings.push(*event);
                }
            }
            let SceneStage::Onett(stage) = &mut state.stage else {
                unreachable!()
            };
            let parameters = stage.parameters;
            for event in &landings {
                stage.roof.land(event.ground_kind, &parameters);
            }
            let model = state.stage_animations.get_mut(&ROOF_MAP).unwrap();
            let mut engine = RoofModel {
                model,
                assets: &assets,
                map: &mut state.map,
                error: None,
            };
            stage.roof.step(&parameters, &mut state.rng, &mut engine);
            if let Some(error) = engine.error {
                return Err(error);
            }
            let model = state.stage_animations.get_mut(&ROOF_MAP).unwrap();
            model.update_collision(&mut state.map, bindings(&assets, ROOF_MAP));
            Ok(())
        }
        CAR_MAP => cars(state, &assets, draws),
        _ => Ok(()),
    }
}

/// grOnett_801E3C60 (0x801E3C60): the awnings (grOnett_801E5214), the
/// smoke (801E5538), lane A's subject, then Ground_801C2FE0.
fn street(state: &mut InitialState, assets: &Assets, draws: &mut DrawLog) -> Result<()> {
    // grOnett_801E54B4's calls since the last step.
    let mut touches = Vec::new();
    for (index, joint) in AWNING_JOINTS.into_iter().enumerate() {
        for event in state.map.take_joint_events(joint).iter() {
            if COUNTED_COLLIDERS.contains(&event.collision_type) {
                touches.push((index, event.ground_kind));
            }
        }
    }
    let SceneStage::Onett(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    for &(index, ground_kind) in &touches {
        stage.awnings[index].touch(ground_kind, &parameters);
    }
    let steps = [
        stage.awnings[0].step(&parameters),
        stage.awnings[1].step(&parameters),
    ];
    for (index, (y, puff)) in steps.into_iter().enumerate() {
        let bone = AWNING_BONES[index];
        state
            .stage_animations
            .get_mut(&STREET_MAP)
            .unwrap()
            .set_joint_translate_y(bone, y);
        if puff {
            // Ground_801C5440's sound follows.
            spawn_on_bone(state, STREET_MAP, bone, AWNING_PARTICLE, draws)?;
        }
    }
    let SceneStage::Onett(stage) = &mut state.stage else {
        unreachable!()
    };
    match stage.smoke.step(&parameters, &mut state.rng) {
        SmokeStep::Nothing => {}
        SmokeStep::Stop => {
            if let Some(id) = stage.smoke_generator.take() {
                state.particles.expire_generator(id);
            } else {
                state.particles.walk_unowned_joint();
            }
        }
        SmokeStep::Start => {
            let model = state.stage_animations.get_mut(&STREET_MAP).unwrap();
            let [x, y, z] = CHIMNEY_OFFSET;
            let position = model.joint_point(CHIMNEY_BONE, Vec3::new(x, y, z));
            let id = super::stadium::spawn_ground_effect(state, SMOKE_PARTICLE.0, position, draws)?;
            let SceneStage::Onett(stage) = &mut state.stage else {
                unreachable!()
            };
            stage.smoke_generator = id;
        }
    }
    let SceneStage::Onett(stage) = &mut state.stage else {
        unreachable!()
    };
    let phase = stage.camera_phase;
    if let Some(subject) = &mut stage.subject {
        subject.state = if phase != 0 {
            melee_cm::SubjectState::Active
        } else {
            melee_cm::SubjectState::Inactive
        };
    }
    let model = state.stage_animations.get_mut(&STREET_MAP).unwrap();
    model.update_collision(&mut state.map, bindings(assets, STREET_MAP));
    Ok(())
}

/// grOnett_801E43E0 (0x801E43E0): both lanes, then the items follow their
/// cars.
fn cars(state: &mut InitialState, assets: &Assets, draws: &mut DrawLog) -> Result<()> {
    let view = melee_cm::view::ground_view(&state.rendered_camera);
    let fighters: Vec<(f32, i32)> = state
        .fighters
        .iter()
        .map(|f| {
            crate::scene_fighter::with_fighter!(f, |f| (
                f.physics.position.y,
                i32::from(f.core.standing_rank)
            ))
        })
        .collect();
    let SceneStage::Onett(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let model = state.stage_animations.get_mut(&CAR_MAP).unwrap();
    let mut engine = CarModels {
        model,
        items: &mut state.items,
        item_assets: assets.items.get(melee_types::ItemKind::PokemonRandom),
        car_items: stage.car_items,
        view: (view.left, view.center, view.right),
        fighters,
        camera_phase: &mut stage.camera_phase,
        rng: &mut state.rng,
        particles: &mut state.particles,
        events: &mut state.effects.events,
        bank: &assets.common_particle_bank,
        draws,
        error: None,
    };
    stage.cars.step(&parameters, &mut engine);
    if let Some(error) = engine.error {
        return Err(error);
    }
    let model = state.stage_animations.get_mut(&CAR_MAP).unwrap();
    for (index, bone) in CAR_BONES.into_iter().enumerate() {
        let id = stage.car_items[index].expect("a car's item");
        it_yaku::set_anchor(state.items.get_mut(id).unwrap(), anchor(model, bone));
    }
    Ok(())
}

/// grOnett_801E5030 (0x801E5030), the cars' damage-dealt callback.
pub(crate) fn car_struck(state: &mut InitialState, item: u32) {
    let SceneStage::Onett(stage) = &mut state.stage else {
        unreachable!()
    };
    if stage.car_of_item(item).is_some() {
        stage.cars.struck_something(&mut state.rng);
    }
}
