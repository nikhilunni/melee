//! Mushroom Kingdom II composition (grinishie2.c): the backdrop (map 0) and
//! the stage (map 2) in grInishie2_801FCBC4's order, then the event maps
//! map 2's timers make: the lifts (maps 3, 4), Birdo's platform (map 15)
//! and Pidgit's carpet (map 1). Their models are loaded at setup and kept
//! hidden until made, so the tick path allocates nothing.
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    desc::JointMapping,
    inishie2::{
        procs::{
            Inishie2Stage, BACKDROP_MAP, BIRDO_JOINT, BIRDO_MAP, CARPET_FLIGHT_BONE, CARPET_JOINT,
            CARPET_MAP, CARPET_ROOT_BONE, DISABLED_AT_SETUP, EVENT_MAPS, LIFT_JOINTS, LIFT_MAPS,
            MAP_ORDER, STAGE_JOINTS, STAGE_MAP,
        },
        Carpet, Controller, Lift, Parameters,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grInishie2_801FCE04's material switches on map 2:
/// grAnime_801C7FF8(gobj, bone, 7, index, 0.0, 1.0), the waterfall's.
const STAGE_MATERIALS: [(usize, usize); 3] = [(1, 0), (6, 1), (5, 2)];

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let scale = assets.stage_desc.parameters.map_scale;
    if usize::from(map) >= assets.stage_desc.models.len() {
        // Ground_GetStageGObj past the archive's models: an identity JObj.
        return BackgroundAnimation::identity_root(&assets.stage, scale)
            .map_err(|e| anyhow::anyhow!("Mushroom Kingdom II map {map}: {e}"));
    }
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Mushroom Kingdom II map {map}: {e}"))?;
    animation.set_map_scale(scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_inishie2_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Every collision joint bound to `map`: grI2_803E4A60's, then the
/// archive's not already listed (Ground_801C2FE0's order and epoch).
fn bindings(assets: &Assets, map: u8) -> impl Iterator<Item = &JointMapping> {
    let table = STAGE_JOINTS
        .iter()
        .filter(move |b| b.target_index == i16::from(map));
    let archive = assets
        .stage_desc
        .models
        .get(usize::from(map))
        .map_or(&[][..], |m| &m.joint_mappings[..])
        .iter()
        .filter(move |b| {
            !STAGE_JOINTS
                .iter()
                .any(|t| t.target_index == i16::from(map) && t.joint_index == b.joint_index)
        });
    table.chain(archive)
}

/// Ground_801C2FE0 (0x801C2FE0) on `map`: mpLib_80055E9C on each bound joint.
fn update_collision(
    model: &mut BackgroundAnimation,
    assets: &Assets,
    map: u8,
    collision: &mut melee_mp::CollMap,
) {
    for binding in bindings(assets, map) {
        model.update_collision(collision, std::slice::from_ref(binding));
    }
}

/// Ground_801C2ED0 (0x801C2ED0): bind, transform and snapshot each joint
/// (mpLib_800552B0, 80055E9C, 80057424), then Ground_801C3214: a map whose
/// joints Ground_801C4A08 disabled has them enabled again.
fn bind(state: &mut InitialState, map: u8) {
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&map).unwrap();
    for binding in bindings(&assets, map) {
        model.update_collision(&mut state.map, std::slice::from_ref(binding));
        state.map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    if std::mem::take(&mut stage.released[usize::from(map)]) {
        for binding in bindings(&assets, map) {
            state.map.joint_list_add(i32::from(binding.joint_index));
        }
    }
}

/// grInishie2_801FCBC4 (0x801FCBC4): map 0 (grInishie2_801FCDC8: its
/// empty animation 0), map 2 (grInishie2_801FCE04: the waterfall's
/// materials, the four timers' draws, the fog's points), then every
/// collision joint but the stage's own disabled.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let mut animations = Animations::new();
    let mut controller = None;
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        match map {
            BACKDROP_MAP => {}
            STAGE_MAP => {
                // Ground_GetStageGObj requests nothing; on_init switches
                // the waterfall's subtrees.
                animation.clear_animation();
                for (bone, index) in STAGE_MATERIALS {
                    animation.play_material_subtree(bone, index);
                }
                controller = Some(Controller::new(&parameters, rng));
            }
            _ => unreachable!(),
        }
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "Mushroom Kingdom II map {map}: unexpected setup particle key"
        );
        animations.insert(map, animation);
    }
    for joint in DISABLED_AT_SETUP {
        collision.joint_list_remove(joint);
    }
    load_reserved(assets, &mut animations)?;
    let stage = stage(assets, parameters, controller.expect("map 2"))?;
    Ok((SceneStage::Inishie2(Box::new(stage)), animations))
}

pub(crate) fn stage(assets: &Assets, parameters: Parameters, controller: Controller) -> Result<Inishie2Stage> {
    Ok(Inishie2Stage {
        parameters,
        controller,
        lifts: [None; 2],
        carpet: None,
        birdo: None,
        released: [false; 16],
        lights: melee_gr::battle::lights::load_model(
            &assets.stage,
            &assets.stage_desc,
            usize::from(STAGE_MAP),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?,
    })
}

/// The event maps' models, loaded once and hidden until made.
pub(crate) fn load_reserved(assets: &Assets, animations: &mut Animations) -> Result<()> {
    for map in EVENT_MAPS {
        let mut model = load(assets, map)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(map, model);
    }
    Ok(())
}

/// A Mushroom Kingdom II gobj proc (s_link 4).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    match map {
        STAGE_MAP => step_controller(state, world, objects, item_objects),
        3 | 4 => step_lift(state, map, world, objects),
        CARPET_MAP => step_carpet(state, world, objects),
        BIRDO_MAP => {
            step_birdo_platform(state);
            Ok(())
        }
        _ => Ok(()),
    }
}

/// grInishie2_801FCFCC (0x801FCFCC): the lifts (801FD0B4), Birdo
/// (801FD3CC), the carpet (801FD7A8), the music (801FCF2C: sound only);
/// lb_800115F4 runs in the scene dispatch.
fn step_controller(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    for lift in 0..2 {
        let SceneStage::Inishie2(stage) = &mut state.stage else {
            unreachable!()
        };
        if stage.controller.lift_due(lift) {
            create_object(state, LIFT_MAPS[lift], world, objects, item_objects)?;
            // mpLib_80057BC0(12 / 13): the lift's joint waits for its delay.
            state.map.joint_list_remove(LIFT_JOINTS[lift]);
        }
    }
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    if stage.controller.birdo_due() {
        create_object(state, BIRDO_MAP, world, objects, item_objects)?;
    }
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    if stage.controller.carpet_due() {
        create_object(state, CARPET_MAP, world, objects, item_objects)?;
    }
    // grInishie2_801FCF2C: a stock match has no timer, so the music is
    // decided at once (lbAudioAx only otherwise).
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.controller.music_decided = true;
    Ok(())
}

/// grInishie2_801FCCDC (0x801FCCDC) for an event map: Ground_GetStageGObj
/// (the GObj and its Ground_801C1CD0 / Ground_801C1D38 procs), the map's
/// on_init, then its gobj proc.
fn create_object(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let SceneStage::Inishie2(stage) = &state.stage else {
        unreachable!()
    };
    ensure!(
        !stage.live(map),
        "grInishie2_801FCCDC: a second map-{map} object while the first lives"
    );
    let object = world.create(3, 5, 0);
    state.stage_gobjs.create(&mut state.effects.gobjs, map);
    for (index, link) in [1, 4].into_iter().enumerate() {
        world.add_tagged_proc(object, link, super::last::TAG_BASE + usize::from(map) * 3 + index);
    }
    objects[usize::from(map)] = Some(object);
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        ensure!(
            !state
                .particles
                .has_joint_attachment(crate::initial_state::stage::joint_id(map, joint)),
            "grInishie2_801FCCDC: map {map} remade while generators hold its retired JObjs"
        );
    }
    if usize::from(map) < state.assets.stage_desc.models.len() {
        model
            .recreate(&state.assets.stage, &state.assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        // Ground_GetStageGObj requests no animation; the on_init does.
        model.clear_animation();
    } else {
        model.reset_for_creation();
    }
    model.set_hidden(false);
    match map {
        3 | 4 => init_lift(state, map)?,
        CARPET_MAP => init_carpet(state)?,
        BIRDO_MAP => init_birdo_platform(state, world, item_objects)?,
        _ => unreachable!(),
    }
    world.add_tagged_proc(object, 4, super::last::TAG_BASE + usize::from(map) * 3 + 2);
    Ok(())
}

/// grAnime_801C8138(gobj, map, index): the pose reset, the model
/// animation from frame zero, evaluated.
fn play(state: &mut InitialState, map: u8, index: usize) -> Result<()> {
    let model = state.stage_animations.get_mut(&map).unwrap();
    model.reset_pose();
    model
        .select_animation(
            &state.assets.stage,
            &state.assets.stage_desc.models[usize::from(map)],
            index,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    ensure!(
        model.evaluate_initial_frame::<RetailTrig>().is_empty(),
        "grAnime_801C8138: Mushroom Kingdom II map {map} animation {index} has frame-zero particle keys"
    );
    Ok(())
}

/// grInishie2_801FD198 (0x801FD198): Ground_801C2ED0, animation 0, the
/// lift's delay.
fn init_lift(state: &mut InitialState, map: u8) -> Result<()> {
    bind(state, map);
    play(state, map, 0)?;
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    let index = usize::from(map - LIFT_MAPS[0]);
    stage.lifts[index] = Some(Lift::new(index, &stage.parameters));
    Ok(())
}

/// grInishie2_801FD224 (0x801FD224): the delay, Ground_801C2FE0, and once
/// the animation ends the controller's wait rearmed and the map gone.
fn step_lift(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let index = usize::from(map - LIFT_MAPS[0]);
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    let lift = stage.lifts[index].as_mut().expect("a live lift");
    if lift.step() {
        state.map.joint_list_add(LIFT_JOINTS[index]);
    }
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&map).unwrap();
    update_collision(model, &assets, map, &mut state.map);
    // grAnime_801C83D0(gobj, 0, 1).
    if model.ended() {
        let SceneStage::Inishie2(stage) = &mut state.stage else {
            unreachable!()
        };
        let wait = stage.parameters.lift_waits[index];
        stage.controller.lifts[index].arm(wait, &mut state.rng);
        remove_object(state, map, world, objects);
    }
    Ok(())
}

/// grInishie2_801FDE8C (0x801FDE8C): grInishie2_801FD824 (the side, joint
/// 0 on, the animation facing in, the root beyond the blast zone), then
/// Ground_801C2ED0.
fn init_carpet(state: &mut InitialState) -> Result<()> {
    let camera = state.assets.stage_camera;
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    let carpet = Carpet::spawn(
        &stage.parameters,
        camera.blast_left(),
        camera.blast_right(),
        &mut state.rng,
    );
    stage.carpet = Some(carpet);
    state.map.joint_list_add(CARPET_JOINT);
    play(state, CARPET_MAP, usize::from(carpet.animation))?;
    let model = state.stage_animations.get_mut(&CARPET_MAP).unwrap();
    model.set_joint_translate(CARPET_ROOT_BONE, carpet.root);
    bind(state, CARPET_MAP);
    Ok(())
}

/// grInishie2_801FDED8 (0x801FDED8): Ground_801C2FE0, the flight
/// (grInishie2_801FD9EC), then the carpet leaves once past a blast zone
/// on its way out, rearming the controller's wait.
fn step_carpet(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&CARPET_MAP).unwrap();
    update_collision(model, &assets, CARPET_MAP, &mut state.map);
    // grAnime_801C83D0(gobj, 1, 1): bone 1's joint animation ended.
    let ended = model.joint_ended(CARPET_FLIGHT_BONE);
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    let carpet = stage.carpet.as_mut().expect("a live carpet");
    if ended {
        let camera = assets.stage_camera;
        carpet.flight_ended(&stage.parameters, camera.left(), camera.right(), &mut state.rng);
        let animation = carpet.animation;
        let root = carpet.advance();
        play(state, CARPET_MAP, usize::from(animation))?;
        let model = state.stage_animations.get_mut(&CARPET_MAP).unwrap();
        model.set_joint_translate(CARPET_ROOT_BONE, root);
    } else {
        let model = state.stage_animations.get_mut(&CARPET_MAP).unwrap();
        carpet.step = model.joint_translation(CARPET_FLIGHT_BONE);
    }
    // lb_8000B1CC on the root: past either blast zone.
    let model = state.stage_animations.get_mut(&CARPET_MAP).unwrap();
    let x = model.joint_matrix(CARPET_ROOT_BONE).0[0][3];
    let camera = assets.stage_camera;
    let outside = x > camera.blast_right() || x < camera.blast_left();
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    if outside && stage.carpet.expect("a live carpet").leaving {
        let wait = stage.parameters.carpet_wait;
        stage.controller.carpet.arm(wait, &mut state.rng);
        remove_object(state, CARPET_MAP, world, objects);
    }
    Ok(())
}

/// grInishie2_801FD4F0 (0x801FD4F0): the side, joint 0xE on, the platform
/// at the spawn point, Birdo made there (it_802ECD3C), Ground_801C2ED0.
fn init_birdo_platform(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let camera = state.assets.stage_camera;
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    let (position, facing) = melee_gr::inishie2::birdo_spawn(
        &stage.parameters,
        camera.blast_left(),
        camera.blast_right(),
        &mut state.rng,
    );
    stage.birdo = Some(None);
    state.map.joint_list_add(BIRDO_JOINT);
    let model = state.stage_animations.get_mut(&BIRDO_MAP).unwrap();
    model.set_gobj_translate(position);
    let item = crate::scene_items::spawn_zako(
        state,
        world,
        item_objects,
        melee_types::ItemKind::Kyasarin,
        position,
        facing,
    );
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.birdo = Some(item);
    bind(state, BIRDO_MAP);
    Ok(())
}

/// grInishie2_801FD448 (0x801FD448), from itKyasarin_Logic25_Destroyed:
/// the controller's Birdo wait rearmed, map 15 gone.
pub(crate) fn birdo_gone(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) {
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!("Birdo off Mushroom Kingdom II")
    };
    let wait = stage.parameters.birdo_wait;
    stage.controller.birdo.arm(wait, &mut state.rng);
    remove_object(state, BIRDO_MAP, world, objects);
}

/// grInishie2_801FD654 (0x801FD654): the platform follows Birdo, then
/// Ground_801C2FE0.
fn step_birdo_platform(state: &mut InitialState) {
    let SceneStage::Inishie2(stage) = &state.stage else {
        unreachable!()
    };
    let height = stage.parameters.birdo_platform_height;
    if let Some(Some(item)) = stage.birdo {
        // it_802ECD1C: the item's position (it may already be gone: the
        // pointer outlives it until map 15 is removed).
        let mut position = state.items.iter().find(|i| i.id == item).map(|i| i.position).expect("Birdo item");
        position.y += height;
        state
            .stage_animations
            .get_mut(&BIRDO_MAP)
            .unwrap()
            .set_gobj_translate(position);
    }
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&BIRDO_MAP).unwrap();
    update_collision(model, &assets, BIRDO_MAP, &mut state.map);
}

/// Ground_801C4A08 (0x801C4A08) on an event map: its joints disabled the
/// first time (Ground_804D6950), the GObj freed. Generators still bound to
/// its JObjs keep their last matrices.
pub(crate) fn remove_object(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::Inishie2(stage) = &mut state.stage else {
        unreachable!()
    };
    if !stage.released[usize::from(map)] {
        stage.released[usize::from(map)] = true;
        for binding in bindings(&assets, map) {
            state.map.joint_list_remove(i32::from(binding.joint_index));
        }
    }
    match map {
        CARPET_MAP => stage.carpet = None,
        3 | 4 => stage.lifts[usize::from(map - 3)] = None,
        BIRDO_MAP => stage.birdo = None,
        _ => unreachable!(),
    }
    state.stage_gobjs.destroy(&mut state.effects.gobjs, map);
    if let Some(object) = objects[usize::from(map)].take() {
        world.destroy(object);
    }
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        let id = crate::initial_state::stage::joint_id(map, joint);
        if state.particles.has_joint_attachment(id) {
            let matrix = model.released_joint_matrix(joint);
            state.effects.events.update_joint(id, matrix);
            state.particles.update_joint(id, matrix);
        }
    }
    model.clear_animation();
    model.set_hidden(true);
}

