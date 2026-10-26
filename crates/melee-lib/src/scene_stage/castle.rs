//! Princess Peach's Castle composition (grcastle.c): maps 0, 4, 3 and 6 at
//! creation, the switches and block sets at the start, the Bullet Bill's
//! maps during the match; the stage material items and camera subjects
//! those maps make.
use super::kraid::spawn_keys;
use melee_ft::fighter::RetailTrig;
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::{rng_sites::DrawLog, system::ParticleSystem};
use hsd_types::Vec3;
use melee_gr::{
    castle::{
        bills::{
            Bill, BillEngine, Cannon, BILL_ITEM_STATE, BLAST_DECAY, BLAST_FRAMES,
            BLAST_PHASE_STEP, BLAST_STRENGTH, BLOCK_CLEARANCE, CANNON_MAP, EXPLOSION_ITEM_STATE,
            EXPLOSION_MAP, FIRST_FLIGHT_MAP, FLIGHTS,
        },
        platforms::{BONES as PLATFORM_BONES, JOINTS as PLATFORM_JOINTS},
        procs::{CastleStage, Explosion, FOUNTAIN, MAP_ORDER, PLATFORM_MAP, ROOF_MAP},
        switches::{
            BlockEngine, BlockSet, SwitchEngine, BLOCK_MAPS, PLACES, SWITCH_CAPSULE,
            SWITCH_ITEM_STATE, SWITCH_MAPS, SWITCH_SCALE,
        },
        wind::FLAG_COUNT,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// The maps made during the match, loaded at creation and kept hidden
/// until made (the tick allocates nothing).
const RESERVED: [u8; 17] = [1, 2, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
/// Camera_80029044(2): an Auto subject.
const SUBJECT_STATE: melee_cm::SubjectState = melee_cm::SubjectState::Auto;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Peach's Castle map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<melee_gr::castle::Parameters> {
    melee_gr::desc::read_castle_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

fn bindings(assets: &Assets, map: u8) -> &[melee_gr::desc::JointMapping] {
    &assets.stage_desc.models[usize::from(map)].joint_mappings
}

fn map_scale(assets: &Assets) -> f32 {
    assets.stage_desc.parameters.map_scale
}

/// grCastle_801CD37C (0x801CD37C): maps 0, 4, 3, 6, then the reserved
/// models. The block sets' joints start disabled (mpLib_80057BC0).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let draws = &mut DrawLog::default();
    // Map 3's lights are animated: presentation reads them itself.
    let stage = CastleStage::new(parameters);
    let mut animations = Animations::new();
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        match map {
            // grCs_StageCallbacks[0] is empty: no animation is requested.
            0 => animation.clear_animation(),
            // grCastle_801CD5BC: animation 0.
            4 => spawn_keys(&mut animation, map, assets, particles, None, rng, draws)?,
            ROOF_MAP => {
                // grCastle_801CD658: animation 0 (none), the fountain.
                spawn_keys(&mut animation, map, assets, particles, None, rng, draws)?;
                let scale = map_scale(assets);
                let [x, y, z] = FOUNTAIN.1;
                let position = Vec3::new(x * scale, y * scale, z * scale);
                super::stadium::ground_effect(assets, particles, None, rng, draws, FOUNTAIN.0, position)?;
            }
            PLATFORM_MAP => {
                // grCastle_801CDA0C: Ground_801C2ED0, the platforms' heights
                // (mpLib_80057424 keeps the bind pose as the previous one),
                // Ground_801C2FE0. No animation is requested.
                animation.clear_animation();
                animation.update_collision(collision, bindings(assets, map));
                for binding in bindings(assets, map) {
                    collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
                }
                for (index, bone) in PLATFORM_BONES.into_iter().enumerate() {
                    animation.set_joint_translate_y(bone, stage.platforms[index].position);
                    collision.joint_snapshot_prev_pos(PLATFORM_JOINTS[index]);
                }
                animation.update_collision(collision, bindings(assets, map));
            }
            _ => unreachable!(),
        }
        animations.insert(map, animation);
    }
    for joint in melee_gr::castle::procs::DISABLED_JOINTS {
        collision.joint_list_remove(joint);
    }
    load_reserved(assets, &mut animations)?;
    Ok((SceneStage::Castle(Box::new(stage)), animations))
}

/// The models of the maps made during the match, hidden.
pub(crate) fn load_reserved(assets: &Assets, animations: &mut Animations) -> Result<()> {
    for map in RESERVED {
        let mut model = load(assets, map)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(map, model);
    }
    Ok(())
}

/// The scheduler and item-pool context a map's creation needs.
pub(crate) struct Scene<'a> {
    pub world: &'a mut hsd_gobj::TaggedWorld,
    pub objects: &'a mut crate::scene_stage::StageObjects,
    pub item_objects: &'a mut crate::scene_items::Objects,
    pub draws: &'a mut DrawLog,
}

/// The deferred calls Ground_801C0FB8 runs at the countdown's end, newest
/// first: grCastle_801CD9B4 (the platforms' floor callbacks), then
/// grCastle_801CD610 (the moat, the switches and their block sets, the
/// switch and Bill countdowns).
pub(crate) fn start(state: &mut InitialState, scene: &mut Scene<'_>) -> Result<()> {
    for joint in PLATFORM_JOINTS {
        state.map.record_joint_events(joint, true);
    }
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.moat_active = true;
    // grCastle_801CF7B0: maps 20, 19, 18, each making its block set.
    for map in SWITCH_MAPS {
        create_map(state, scene, map)?;
    }
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    stage.selector = Some(melee_gr::castle::switches::SwitchSelector::new(
        &parameters,
        &mut state.rng,
    ));
    // grCastle_801CDFD8.
    stage.bill_timer = Some(melee_gr::castle::bills::BillTimer::new(&parameters, &mut state.rng));
    Ok(())
}

/// grCastle_801CD4D0 (0x801CD4D0) mid-match: Ground_GetStageGObj (the
/// GObj and its Ground_801C1CD0 / Ground_801C1D38 procs), the map's
/// on_init, then its gobj proc.
fn create_map(state: &mut InitialState, scene: &mut Scene<'_>, map: u8) -> Result<()> {
    ensure!(
        scene.objects[usize::from(map)].is_none(),
        "grCastle_801CD4D0: a second map-{map} object while the first lives"
    );
    let object = scene.world.create(3, 5, 0);
    state.stage_gobjs.create(&mut state.effects.gobjs, map);
    for (index, link) in [1, 4, 4].into_iter().enumerate() {
        scene
            .world
            .add_tagged_proc(object, link, super::last::TAG_BASE + usize::from(map) * 3 + index);
    }
    scene.objects[usize::from(map)] = Some(object);
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        ensure!(
            !state.particles.has_joint_attachment(joint_id(map, joint)),
            "grCastle_801CD4D0: map {map} recreated while generators hold its retired JObjs"
        );
    }
    model
        .recreate(&assets.stage, &assets.stage_desc.models[usize::from(map)])
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    model.reset_gobj_joint(map_scale(&assets));
    on_init(state, scene, map)
}

/// The map's on_init (grCs_StageCallbacks).
fn on_init(state: &mut InitialState, scene: &mut Scene<'_>, map: u8) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&map).unwrap();
    match map {
        18..=20 => {
            // grCastle_801CF0F4: no animation, hidden, scaled, its blocks.
            model.clear_animation();
            model.set_hidden(true);
            let scale = SWITCH_SCALE * map_scale(&assets);
            model.set_gobj_scale(Vec3::new(scale, scale, scale));
            let index = switch_index(map);
            create_map(state, scene, BLOCK_MAPS[index])
        }
        5 | 7 | 17 => {
            // grCastle_801CEACC: no animation, hidden, its joints off.
            model.clear_animation();
            model.set_hidden(true);
            if map == 7 {
                state.map.joint_list_remove(8);
            }
            let SceneStage::Castle(stage) = &mut state.stage else {
                unreachable!()
            };
            let parameters = stage.parameters;
            let index = BLOCK_MAPS.iter().position(|&m| m == map).unwrap();
            let items_on = state.item_spawner.is_some();
            let mut engine = Blocks {
                map,
                animations: &mut state.stage_animations,
                collision: &mut state.map,
                items_on,
            };
            stage.blocks[index] = Some(BlockSet::new(map, &parameters, &mut engine));
            Ok(())
        }
        8..=16 => create_bill(state, scene, map),
        CANNON_MAP => {
            // grCastle_801CE7E8: Ground_801C2ED0, animation 0.
            spawn_new_keys(state, scene.draws, map)?;
            let SceneStage::Castle(stage) = &mut state.stage else {
                unreachable!()
            };
            stage.cannon = Some(Cannon::new(&stage.parameters));
            Ok(())
        }
        EXPLOSION_MAP => create_explosion(state, scene),
        _ => unreachable!("castle map {map} made mid-match"),
    }
}

fn switch_index(map: u8) -> usize {
    SWITCH_MAPS.iter().position(|&m| m == map).unwrap()
}

/// efLib_SpawnParticleEffect's (0x8005D174) common-bank kinds that attach
/// with an AppSRT taking the root's scale (eflib.c:900-916).
const ROOT_SCALED_KINDS: [u32; 9] = [0x2, 0x6, 0xA3, 0xA7, 0xAA, 0xF2, 0x12A, 0x132, 0x133];

/// A freshly made map's frame-zero particle keys (grAnime_801C8138). A
/// common-bank key goes through efLib_Cb_DPtcl -> efLib_SpawnParticleEffect
/// rather than grLib_801C99C0: the explosion's 0xF2 attaches with an
/// AppSRT carrying the root's (the map-scale wrapper's) scale.
fn spawn_new_keys(state: &mut InitialState, draws: &mut DrawLog, map: u8) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&map).unwrap();
    let root_scale = model.gobj_scale();
    for event in model.evaluate_initial_frame::<RetailTrig>() {
        let mut request = hsd_particle::system::SpawnRequest::new(event.bank, event.kind, 0);
        let joint = joint_id(map, event.joint);
        request.joint = Some((joint, event.matrix));
        let scaled = event.bank == 0 && ROOT_SCALED_KINDS.contains(&event.kind);
        if scaled {
            request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                scale: root_scale,
                ..Default::default()
            });
        } else {
            ensure!(
                event.bank == 30 || event.kind < 1000,
                "efLib_SpawnParticleEffect: castle key {}/{}",
                event.bank,
                event.kind
            );
        }
        state.effects.events.spawn(&request, false, false);
        let id = state.particles.spawn::<RetailTrig>(
            assets.stage_key_bank(event.bank),
            request,
            &mut state.rng,
            draws,
        )?;
        if let (true, Some(id)) = (scaled, id) {
            // eflib_create_generator_add_appsrt: type bits 9-10 off, B11 on.
            let generator = state.particles.generator_mut(id).unwrap();
            state.effects.events.flags(joint, 0x600, 0x800);
            generator.flags = (generator.flags & !0x600) | 0x800;
        }
    }
    Ok(())
}

/// A Camera_80029044(2) subject with these extents (left, right, top,
/// bottom). Camera_80029044 links it after every item's subject.
fn subject(state: &InitialState, extents: [f32; 4]) -> Result<melee_cm::Subject> {
    ensure!(
        state.items.iter().all(|item| item.camera.is_none()),
        "Camera_80029044: a castle subject would follow a framed item's"
    );
    let mut subject = melee_cm::Subject::new(SUBJECT_STATE);
    let [left, right, top, bottom] = extents;
    subject.target_extents.left = left;
    subject.target_extents.right = right;
    subject.target_extents.top = top;
    subject.target_extents.bottom = bottom;
    Ok(subject)
}

/// A bone's world position, scale and rotation (lb_8000B1CC,
/// HSD_JObjGetScale, HSD_JObjGetRotation).
fn anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// The GObj's JObj (the wrapper) as a material item anchor.
fn gobj_anchor(model: &mut BackgroundAnimation) -> melee_it::YakuAnchor {
    let matrix = model.gobj_matrix();
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.gobj_scale(),
        rotation: model.gobj_rotation(),
    }
}

/// grMaterial_801C8CFC(0, `item_state`, gp, jobj, ...) mid-match.
fn spawn_material(
    state: &mut InitialState,
    scene: &mut Scene<'_>,
    anchor: melee_it::YakuAnchor,
    item_state: u16,
) -> u32 {
    let assets = std::sync::Arc::clone(&state.assets);
    let id = crate::scene_items::spawn_yaku_live(
        &mut state.items,
        &mut state.effects.gobjs,
        &assets.items,
        &mut state.map,
        scene.world,
        scene.item_objects,
        anchor,
    );
    let item = state.items.get_mut(id).unwrap();
    it_yaku::change_state(item, item_state, assets.items.get(melee_types::ItemKind::PokemonRandom));
    id
}

/// grCastle_801CE260 (0x801CE260): a Bullet Bill's flight.
fn create_bill(state: &mut InitialState, scene: &mut Scene<'_>, map: u8) -> Result<()> {
    spawn_new_keys(state, scene.draws, map)?;
    let SceneStage::Castle(stage) = &state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let flight = usize::from(map - FIRST_FLIGHT_MAP);
    let subject = subject(state, parameters.bill_extents)?;
    let model = state.stage_animations.get_mut(&map).unwrap();
    let anchor = anchor(model, 0);
    let id = spawn_material(state, scene, anchor, BILL_ITEM_STATE);
    // it_80275414: its hits reach the items of its kind.
    state.items.get_mut(id).unwrap().strikes_kindred_items = true;
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.bill = Some(Bill::new(flight, &parameters));
    stage.bill_subject = Some(subject);
    stage.bill_item = Some(id);
    Ok(())
}

/// grCastle_801CE8E8 (0x801CE8E8): the explosion.
fn create_explosion(state: &mut InitialState, scene: &mut Scene<'_>) -> Result<()> {
    spawn_new_keys(state, scene.draws, EXPLOSION_MAP)?;
    let SceneStage::Castle(stage) = &state.stage else {
        unreachable!()
    };
    let subject = subject(state, stage.parameters.explosion_extents)?;
    let model = state.stage_animations.get_mut(&EXPLOSION_MAP).unwrap();
    let anchor = anchor(model, 0);
    let id = spawn_material(state, scene, anchor, EXPLOSION_ITEM_STATE);
    state.items.get_mut(id).unwrap().strikes_kindred_items = true;
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.explosion = Some(Explosion {
        subject: Some(subject),
        item: Some(id),
    });
    Ok(())
}

/// Ground_801C4A08 (0x801C4A08) on a map: the GObj freed. Generators
/// still bound to its JObjs keep their last matrices.
fn remove_map(state: &mut InitialState, scene: &mut Scene<'_>, map: u8) {
    state.stage_gobjs.destroy(&mut state.effects.gobjs, map);
    if let Some(object) = scene.objects[usize::from(map)].take() {
        scene.world.destroy(object);
    }
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        let id = joint_id(map, joint);
        if state.particles.has_joint_attachment(id) {
            let matrix = model.released_joint_matrix(joint);
            state.effects.events.update_joint(id, matrix);
            state.particles.update_joint(id, matrix);
        }
    }
    model.clear_animation();
    model.set_hidden(true);
}

/// grMaterial_801C8CDC on a stage material item.
fn destroy_material(state: &mut InitialState, scene: &mut Scene<'_>, id: u32) {
    crate::scene_items::destroy_yaku(state, scene.world, scene.item_objects, id);
}

/// A Peach's Castle gobj proc (s_link 4). Map 3's lb_800115F4 runs in the
/// scene dispatch before this.
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    scene: &mut Scene<'_>,
    forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    match map {
        ROOF_MAP => roof(state, scene, forces)?,
        PLATFORM_MAP => platforms(state),
        18..=20 => switch(state, scene, map),
        5 | 7 | 17 => blocks(state, map),
        8..=16 => bill(state, scene, map, forces)?,
        CANNON_MAP => cannon(state, scene.draws)?,
        EXPLOSION_MAP => explosion(state, scene),
        _ => {}
    }
    if let Some(animation) = state.stage_animations.get_mut(&map) {
        super::publish_joint_matrices(animation, map, &mut state.particles, &mut state.effects.events);
    }
    Ok(())
}

/// grCastle_801CD8A8 (0x801CD8A8): the next switch (grCastle_801CF868),
/// the next Bill (grCastle_801CE19C), Ground_801C2FE0 (no joints),
/// lb_800115F4 (run by the dispatch), then the wind: grCastle_801D0BBC and
/// one grCastle_801D0D24 per flag (the flags' own motion is drawing only).
fn roof(
    state: &mut InitialState,
    scene: &mut Scene<'_>,
    forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let idle = stage
        .switches
        .map(|s| s.state == melee_gr::castle::switches::SwitchState::Hidden);
    if let Some(selector) = &mut stage.selector {
        if let Some(placement) = selector.step(&parameters, idle, &mut state.rng) {
            stage.switches[placement.switch].place(placement.place);
        }
    }
    let flight = stage.bill_timer.as_mut().and_then(|t| t.step(&mut state.rng));
    if let Some(flight) = flight {
        debug_assert!(flight < FLIGHTS);
        create_map(state, scene, FIRST_FLIGHT_MAP + flight as u8)?;
        // Ground_801C5440 / Ground_801C5694: sounds.
    }
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let field = forces
        .directional_mut()
        .ok_or_else(|| anyhow::anyhow!("grCastle_801D0BBC: the castle's wind field is gone"))?;
    stage.gusts.steer(field, &mut state.rng);
    for _ in 0..FLAG_COUNT {
        stage.gusts.gust(field, &mut state.rng);
    }
    Ok(())
}

/// grCastle_801CDC44 (0x801CDC44): each platform's weight since the last
/// step (grCastle_801CF750), its move, then Ground_801C2FE0.
fn platforms(state: &mut InitialState) {
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    for (index, joint) in PLATFORM_JOINTS.into_iter().enumerate() {
        for event in state.map.take_joint_events(joint).iter() {
            stage.platforms[index].stand(event.collision_type, event.x50);
        }
    }
    let parameters = stage.parameters;
    let model = state.stage_animations.get_mut(&PLATFORM_MAP).unwrap();
    for (index, bone) in PLATFORM_BONES.into_iter().enumerate() {
        if let Some(y) = stage.platforms[index].step(&parameters) {
            model.set_joint_translate_y(bone, y);
        }
    }
    let assets = std::sync::Arc::clone(&state.assets);
    model.update_collision(&mut state.map, bindings(&assets, PLATFORM_MAP));
}

struct SwitchModel<'a, 'b> {
    map: u8,
    index: usize,
    state: &'a mut InitialState,
    scene: &'a mut Scene<'b>,
}

impl SwitchModel<'_, '_> {
    fn stage(&mut self) -> &mut CastleStage {
        let SceneStage::Castle(stage) = &mut self.state.stage else {
            unreachable!()
        };
        stage
    }
    fn model(&mut self) -> &mut BackgroundAnimation {
        self.state.stage_animations.get_mut(&self.map).unwrap()
    }
}

impl SwitchEngine for SwitchModel<'_, '_> {
    fn set_hidden(&mut self, hidden: bool) {
        self.model().set_hidden(hidden);
    }
    fn hidden(&self) -> bool {
        self.state.stage_animations[&self.map].hidden()
    }
    fn reset_root(&mut self) {
        self.model().reset_joint_pose(0);
    }
    fn create_item(&mut self) {
        let anchor = gobj_anchor(self.model());
        let id = spawn_material(self.state, self.scene, anchor, SWITCH_ITEM_STATE);
        let item = self.state.items.get_mut(id).unwrap();
        let (a, b, radius) = SWITCH_CAPSULE;
        it_yaku::set_hurt_capsule(item, [Vec3::new(a[0], a[1], a[2]), Vec3::new(b[0], b[1], b[2])], radius);
        it_yaku::set_hurt_enabled(item, true);
        let index = self.index;
        self.stage().switch_items[index] = Some(id);
    }
    fn destroy_item(&mut self) {
        let index = self.index;
        if let Some(id) = self.stage().switch_items[index].take() {
            destroy_material(self.state, self.scene, id);
        }
    }
    fn start_blocks(&mut self) {
        let index = self.index;
        self.stage().blocks[index].as_mut().unwrap().start();
    }
    fn blocks_idle(&self) -> bool {
        let SceneStage::Castle(stage) = &self.state.stage else {
            unreachable!()
        };
        stage.blocks[self.index].as_ref().unwrap().state
            == melee_gr::castle::switches::BlocksState::Idle
    }
    fn press(&mut self) {
        let assets = std::sync::Arc::clone(&self.state.assets);
        let map = self.map;
        let model = self.model();
        model
            .select_animation(&assets.stage, &assets.stage_desc.models[usize::from(map)], 0)
            .expect("the switch's prepared animation");
        let draws = &mut *self.scene.draws;
        let state = &mut *self.state;
        let model = state.stage_animations.get_mut(&map).unwrap();
        if let Err(error) = spawn_keys(
            model,
            map,
            &assets,
            &mut state.particles,
            Some(&mut state.effects.events),
            &mut state.rng,
            draws,
        ) {
            panic!("grCastle_801CF308: the press's keys: {error}");
        }
        // Ground_801C5440: the press's sound; then a Small quake.
        state.quakes.request(
            &mut state.camera,
            melee_cm::QuakeKind::Small,
            &mut state.effects.gobjs,
        );
    }
    fn animation_ended(&self) -> bool {
        self.state.stage_animations[&self.map].first_animation_ended()
    }
    fn follow(&mut self, place: usize) {
        let (map, bone) = PLACES[place];
        let target = self.state.stage_animations.get_mut(&map).unwrap();
        let matrix = target.joint_matrix(bone);
        let position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
        let rotation = target.joint_rotation(bone);
        let model = self.model();
        model.set_gobj_translate(position);
        model.set_gobj_rotation(rotation);
    }
}

/// grCastle_801CF308 (0x801CF308), then its material item follows the
/// switch.
fn switch(state: &mut InitialState, scene: &mut Scene<'_>, map: u8) {
    let index = switch_index(map);
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let mut switch = stage.switches[index];
    let mut engine = SwitchModel {
        map,
        index,
        state,
        scene,
    };
    switch.step(&parameters, &mut engine);
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.switches[index] = switch;
    if let Some(id) = stage.switch_items[index] {
        let anchor = gobj_anchor(state.stage_animations.get_mut(&map).unwrap());
        it_yaku::set_anchor(state.items.get_mut(id).unwrap(), anchor);
    }
}

struct Blocks<'a> {
    map: u8,
    animations: &'a mut Animations,
    collision: &'a mut melee_mp::CollMap,
    items_on: bool,
}

impl Blocks<'_> {
    fn model(&self) -> &BackgroundAnimation {
        &self.animations[&self.map]
    }
    fn model_mut(&mut self) -> &mut BackgroundAnimation {
        self.animations.get_mut(&self.map).unwrap()
    }
}

impl BlockEngine for Blocks<'_> {
    fn hidden(&self, bone: usize) -> bool {
        self.model().joint_hidden(bone)
    }
    fn set_hidden(&mut self, bone: usize, hidden: bool) {
        self.model_mut().set_joint_hidden_only(bone, hidden);
    }
    fn scale_y(&self, bone: usize) -> f32 {
        self.model().joint_scale_y(bone)
    }
    fn set_scale_y(&mut self, bone: usize, y: f32) {
        self.model_mut().set_joint_scale_y(bone, y);
    }
    /// grCastle_801CDF54 (0x801CDF54): the first live flight map's first
    /// bone, within 40 in x and y (sqrtf__Ff of fmuls, fmuls, fadds).
    fn bill_near(&mut self, bone: usize) -> bool {
        let matrix = self.model_mut().joint_matrix(bone);
        let position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
        for flight in 0..FLIGHTS as u8 {
            let map = FIRST_FLIGHT_MAP + flight;
            let model = self.animations.get_mut(&map).unwrap();
            if model.hidden() {
                continue;
            }
            let matrix = model.joint_matrix(0);
            let dx = position.x - matrix.0[0][3];
            let dy = position.y - matrix.0[1][3];
            let distance = gekko_math::msl::sqrtf(dx * dx + dy * dy);
            return distance < BLOCK_CLEARANCE;
        }
        false
    }
    fn items_on(&self) -> bool {
        self.items_on
    }
    fn enable_joint(&mut self, joint: i32) {
        self.collision.joint_list_add(joint);
    }
    fn disable_joint(&mut self, joint: i32) {
        self.collision.joint_list_remove(joint);
    }
}

/// grCastle_801CEF04 (0x801CEF04).
fn blocks(state: &mut InitialState, map: u8) {
    let index = BLOCK_MAPS.iter().position(|&m| m == map).unwrap();
    let items_on = state.item_spawner.is_some();
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let mut engine = Blocks {
        map,
        animations: &mut state.stage_animations,
        collision: &mut state.map,
        items_on,
    };
    stage.blocks[index]
        .as_mut()
        .unwrap()
        .step(&parameters, &mut state.rng, &mut engine);
}

struct BillModel<'a, 'b> {
    map: u8,
    state: &'a mut InitialState,
    scene: &'a mut Scene<'b>,
    forces: &'a mut melee_lb::radial_force::RadialForces,
    error: Option<anyhow::Error>,
}

impl BillModel<'_, '_> {
    fn stage(&mut self) -> &mut CastleStage {
        let SceneStage::Castle(stage) = &mut self.state.stage else {
            unreachable!()
        };
        stage
    }
    fn nose(&mut self) -> Vec3 {
        let model = self.state.stage_animations.get_mut(&self.map).unwrap();
        let matrix = model.joint_matrix(0);
        Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
    }
    fn explode_inner(&mut self) -> Result<()> {
        if let Some(id) = self.stage().bill_item.take() {
            destroy_material(self.state, self.scene, id);
        }
        let position = self.nose();
        create_map(self.state, self.scene, EXPLOSION_MAP)?;
        let explosion = self.state.stage_animations.get_mut(&EXPLOSION_MAP).unwrap();
        explosion.set_gobj_translate(position);
        self.forces.insert(melee_lb::radial_force::RadialImpulse {
            center: position,
            frames: BLAST_FRAMES,
            strength: BLAST_STRENGTH,
            decay: BLAST_DECAY,
            phase_step: BLAST_PHASE_STEP,
        });
        let parameters = self.stage().parameters;
        let SceneStage::Castle(stage) = &mut self.state.stage else {
            unreachable!()
        };
        if let Some(timer) = &mut stage.bill_timer {
            timer.restart(&parameters, &mut self.state.rng);
        }
        stage.bill_subject = None;
        stage.bill = None;
        let map = self.map;
        remove_map(self.state, self.scene, map);
        Ok(())
    }
}

impl BillEngine for BillModel<'_, '_> {
    fn follow_subject(&mut self) {
        let position = self.nose();
        if let Some(subject) = &mut self.stage().bill_subject {
            subject.position = position;
            subject.bone_position = position;
        }
    }
    /// grCastle_801CD4D0(2), then grCastle_801CE3AC: the cannon above the
    /// Bill's second bone, turned by the flight's row.
    fn make_cannon(&mut self) {
        if let Err(error) = create_map(self.state, self.scene, CANNON_MAP) {
            self.error.get_or_insert(error);
            return;
        }
        let stage = self.stage();
        let lift = stage.parameters.cannon_lift;
        let flight = stage.bill.map_or(0, |b| b.flight);
        let rotation = stage.parameters.flights[flight].cannon_rotation;
        let model = self.state.stage_animations.get_mut(&self.map).unwrap();
        let position = model.joint_point(1, Vec3::new(0.0, lift, 0.0));
        let cannon = self.state.stage_animations.get_mut(&CANNON_MAP).unwrap();
        cannon.set_gobj_translate(position);
        cannon.set_gobj_rotation(rotation);
    }
    fn flight_ended(&self) -> bool {
        self.state.stage_animations[&self.map].ended()
    }
    fn land(&mut self, cannon_made: bool) {
        if cannon_made {
            remove_map(self.state, self.scene, CANNON_MAP);
            self.stage().cannon = None;
        }
    }
    fn explode(&mut self) {
        if let Err(error) = self.explode_inner() {
            self.error.get_or_insert(error);
        }
    }
}

/// grCastle_801CE578 (0x801CE578), then the item follows the Bill.
fn bill(
    state: &mut InitialState,
    scene: &mut Scene<'_>,
    map: u8,
    forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let mut bill = stage.bill.expect("a flying Bill's state");
    let mut engine = BillModel {
        map,
        state,
        scene,
        forces,
        error: None,
    };
    let gone = bill.step(&parameters, &mut engine);
    if let Some(error) = engine.error {
        return Err(error);
    }
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    if !gone {
        stage.bill = Some(bill);
        if let Some(id) = stage.bill_item {
            let anchor = anchor(state.stage_animations.get_mut(&map).unwrap(), 0);
            it_yaku::set_anchor(state.items.get_mut(id).unwrap(), anchor);
        }
    }
    Ok(())
}

/// grCastle_801CE860 (0x801CE860): the next animation, then a Loop quake.
fn cannon(state: &mut InitialState, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    let next = stage.cannon.as_mut().and_then(|c| c.step(&parameters));
    if let Some(index) = next {
        let assets = std::sync::Arc::clone(&state.assets);
        let model = state.stage_animations.get_mut(&CANNON_MAP).unwrap();
        model
            .select_animation(&assets.stage, &assets.stage_desc.models[usize::from(CANNON_MAP)], index)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        spawn_new_keys(state, draws, CANNON_MAP)?;
    }
    state.quakes.request(
        &mut state.camera,
        melee_cm::QuakeKind::Loop,
        &mut state.effects.gobjs,
    );
    Ok(())
}

/// grCastle_801CE9E8 (0x801CE9E8): the subject follows; at the
/// animation's end the item, the subject and the map go.
fn explosion(state: &mut InitialState, scene: &mut Scene<'_>) {
    let model = state.stage_animations.get_mut(&EXPLOSION_MAP).unwrap();
    let matrix = model.joint_matrix(0);
    let position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
    let ended = model.ended();
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let Some(explosion) = &mut stage.explosion else {
        unreachable!("the explosion's state")
    };
    if let Some(subject) = &mut explosion.subject {
        subject.position = position;
        subject.bone_position = position;
    }
    if ended {
        let item = explosion.item.take();
        stage.explosion = None;
        if let Some(id) = item {
            destroy_material(state, scene, id);
        }
        remove_map(state, scene, EXPLOSION_MAP);
    } else if let Some(id) = explosion.item {
        let anchor = anchor(state.stage_animations.get_mut(&EXPLOSION_MAP).unwrap(), 0);
        it_yaku::set_anchor(state.items.get_mut(id).unwrap(), anchor);
    }
}

/// fn_801CFAFC / fn_801CFB68 (a switch's yaku.x18 and x1C): the switch is
/// pressed; a fighter that pressed it freezes (ftLib_80086A4C).
pub(crate) fn switch_pressed(state: &mut InitialState, item: u32, fighter: Option<usize>) {
    let SceneStage::Castle(stage) = &mut state.stage else {
        unreachable!()
    };
    let Some(index) = stage.switch_of_item(item) else {
        return;
    };
    stage.switches[index].hit();
    if let Some(fighter) = fighter {
        let freeze = stage.parameters.switch_freeze as f32;
        crate::scene_fighter::with_fighter!(&mut state.fighters[fighter], |f| f
            .combat
            .freeze_request = freeze);
    }
}

/// grCastle_801D0A9C (0x801D0A9C): the moat's splash through
/// grLib_801C96F8, its AppSRT scaled by the fighter's size (three fmuls).
pub(crate) fn splash(
    assets: &Assets,
    particles: &mut ParticleSystem,
    splash: melee_gr::garden::Splash,
    rng: &mut HsdRng,
) -> Result<()> {
    let id = super::stadium::ground_effect(
        assets,
        particles,
        None,
        rng,
        &mut DrawLog::default(),
        melee_gr::castle::procs::SPLASH_PARTICLE,
        splash.position,
    )?;
    if let Some(id) = id {
        let generator = particles.generators.iter().find(|g| g.id == id).unwrap();
        let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
        transform.scale.x *= splash.scale;
        transform.scale.y *= splash.scale;
        transform.scale.z *= splash.scale;
        particles.set_application_transform(id, transform);
    }
    Ok(())
}

/// The stage subjects newest first: the explosion's, then the Bill's.
pub(crate) fn subjects(stage: &mut CastleStage) -> impl Iterator<Item = &mut melee_cm::Subject> {
    let explosion = stage.explosion.as_mut().and_then(|e| e.subject.as_mut());
    explosion.into_iter().chain(stage.bill_subject.as_mut())
}
