//! Kongo Jungle composition: the main stage's bridge and platform ends,
//! the barrel, in grKongo_801D523C's creation order.
use crate::{
    assets::Assets,
    initial_state::InitialState,
    scene_stage::SceneStage,
};
use anyhow::Result;
use gekko_math::HsdRng;
use hsd_particle::{rng_sites::DrawLog, system::ParticleSystem};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    desc::JointMapping,
    kongo::{
        planks::{BRIDGE_COLLISION_JOINT, END_JOINTS, SWING_JOINTS},
        Barrel, Bridge, Kongo, Swing, BARREL, MAIN, MAP_ORDER,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grKg_803E16E0: collision joint `joint_index` follows map
/// `target_index`'s descendant `extra`. Joints 2, 3, 5 and 6 are the
/// platform ends on map 10; 0 and 1 the logs (maps 1 and 2).
pub(crate) const COLLISION_BINDINGS: [JointMapping; 6] = [
    JointMapping {
        joint_index: 2,
        target_index: 10,
        extra: 19,
    },
    JointMapping {
        joint_index: 3,
        target_index: 10,
        extra: 22,
    },
    JointMapping {
        joint_index: 5,
        target_index: 10,
        extra: 43,
    },
    JointMapping {
        joint_index: 6,
        target_index: 10,
        extra: 44,
    },
    JointMapping {
        joint_index: 0,
        target_index: 1,
        extra: 0,
    },
    JointMapping {
        joint_index: 1,
        target_index: 2,
        extra: 2,
    },
];
/// The barrel model's turning JObj (descendant 1).
const BARREL_JOINT: usize = 1;

pub(crate) fn bindings(map: u8) -> impl Iterator<Item = &'static JointMapping> {
    COLLISION_BINDINGS
        .iter()
        .filter(move |b| b.target_index == i16::from(map))
}

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Kongo Jungle map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// `lb_8000B1CC(jobj, NULL, &out)` (0x8000B1CC): a bone's world translation.
fn world_position(animation: &mut BackgroundAnimation, bone: usize) -> Vec3 {
    let matrix = animation.joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// Ground_801C2FE0 for `map`: mpLib_80055E9C on each bound joint.
pub(crate) fn update_collision(
    animation: &mut BackgroundAnimation,
    map: &mut melee_mp::CollMap,
    id: u8,
) {
    for binding in bindings(id) {
        animation.update_collision(map, std::slice::from_ref(binding));
    }
}

/// grKongo_801D7134's model and map writes after the bridge step: the end
/// JObjs, the jolts into the swings, each plank's rotation, then the lines.
fn apply_bridge(
    bridge: &mut Bridge,
    swings: &mut [Swing; 2],
    parameters: &melee_gr::kongo::Parameters,
    main: &mut BackgroundAnimation,
    map: &mut melee_mp::CollMap,
) {
    let ends = END_JOINTS.map(|joint| main.joint_rotation_z(joint));
    let step = bridge.step(parameters, ends);
    for (side, &joint) in END_JOINTS.iter().enumerate() {
        let (tilt, angle) = step.ends[side];
        main.set_joint_rotation_x(joint, tilt);
        main.set_joint_rotation_z(joint, angle);
        if let Some(jolt) = step.jolts[side] {
            swings[side].velocity = jolt;
        }
    }
    for plank in &bridge.planks {
        main.set_joint_rotation_x(plank.joint, plank.angle);
    }
    map.joint_snapshot_prev_pos(BRIDGE_COLLISION_JOINT);
    for (line, sag) in bridge.line_offsets() {
        map.line_set_offset(line, 0.0, sag, 0.0, sag);
    }
}

/// grKongo_801D77E0's JObj writes: each swing turns its two JObjs.
fn apply_swings(swings: &[Swing; 2], main: &mut BackgroundAnimation) {
    for (swing, joints) in swings.iter().zip(SWING_JOINTS) {
        for joint in joints {
            main.set_joint_rotation_z(joint, swing.angle);
        }
    }
}

/// grKongo_801D523C (0x801D523C) from Ground creation to the last map's
/// on_init: the cold setup. `bridge` is grKg_803E188C as the previous
/// match left it (static data; a fresh boot's is zero).
pub(crate) fn initialize(
    assets: &Assets,
    bridge: Bridge,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    map: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_kongo_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let draws = &mut DrawLog::default();
    let mut animations = Animations::new();
    let mut bridge = bridge;
    let mut swings = [Swing::default(); 2];
    let mut barrel = None;
    let mut barrel_position = Vec3::ZERO;
    for id in MAP_ORDER {
        let mut animation = load(assets, id)?;
        if id == MAIN {
            // grKongo_801D5490: Ground_801C2ED0 binds the platform ends at
            // the bind pose before the animation's frame zero.
            for binding in bindings(id) {
                animation.update_collision(map, std::slice::from_ref(binding));
                map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
            }
        }
        // Map 0 has no on_init; the others start animation 0
        // (grAnime_801C8138).
        if id != 0 {
            let requests = animation.evaluate_initial_frame::<RetailTrig>().to_vec();
            super::garden::spawn_keys(&requests, id, assets, particles, None, rng, draws)?;
        }
        if id == MAIN {
            // grKongo_801D69B0: the planks at rest, one bridge step, the
            // swings at rest (grKongo_801D77E0(gobj, 1)), mpJointSetB10(4).
            bridge.reset_angles();
            for plank in &bridge.planks {
                animation.set_joint_rotation_x(plank.joint, plank.angle);
            }
            apply_bridge(&mut bridge, &mut swings, &parameters, &mut animation, map);
            swings = [Swing::default(); 2];
            apply_swings(&swings, &mut animation);
            map.joint_set_b10(BRIDGE_COLLISION_JOINT);
            // Ground_801C10B8 defers fn_801D542C to the match start;
            // Ground_801C2FE0.
            update_collision(&mut animation, map, id);
        }
        if id == BARREL {
            // grKongo_801D55D8: the barrel's two draws, then its place.
            barrel = Some(Barrel::new(&parameters, rng));
            barrel_position = world_position(&mut animation, BARREL_JOINT);
        }
        animations.insert(id, animation);
    }
    // grKongo_801D523C: the logs' collision joints start disabled.
    map.joint_list_remove(0);
    map.joint_list_remove(1);
    load_reserved(assets, &mut animations)?;
    Ok((
        SceneStage::Kongo(Box::new(Kongo {
            parameters,
            bridge,
            swings,
            event_timer: -1,
            last_event: -1,
            barrel: barrel.expect("the barrel's map"),
            barrel_position,
            barrel_angle: 0.0,
            catch_active: false,
            logs: [None; 2],
            passersby: [false; 3],
        })),
        animations,
    ))
}

/// The event maps' models, loaded once and kept hidden until an event
/// creates them (the tick path allocates nothing).
pub(crate) fn load_reserved(assets: &Assets, animations: &mut Animations) -> Result<()> {
    for map in melee_gr::kongo::EVENT_MAPS {
        let mut model = load(assets, map)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(map, model);
    }
    Ok(())
}

/// A Kongo Jungle gobj proc (s_link 4).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    if matches!(map, 1 | 2) {
        return step_log(state, map, world, objects);
    }
    if matches!(map, 7..=9) {
        let model = &state.stage_animations[&map];
        // grKongo_801D64BC: gone once its run ends or wraps.
        if model.ended() || model.rewound() {
            remove_object(state, map, world, objects);
        }
        return Ok(());
    }
    let SceneStage::Kongo(stage) = &mut state.stage else {
        unreachable!()
    };
    let stage = &mut **stage;
    match map {
        MAIN => {
            // fn_801D7700's landings since the last step (the bridge's
            // collision callback, deferred: only this proc reads them).
            let (left, _) = state.map.vtx_get_pos(melee_gr::kongo::planks::BRIDGE_LEFT_VERTEX);
            let (right, _) = state
                .map
                .vtx_get_pos(melee_gr::kongo::planks::BRIDGE_RIGHT_VERTEX);
            for event in state.map.take_joint_events(BRIDGE_COLLISION_JOINT) {
                if matches!(event.collision_type, 1 | 3) && event.ground_kind == 1 {
                    stage.bridge.land(&stage.parameters, event.x, left, right);
                }
            }
            let main = state.stage_animations.get_mut(&MAIN).unwrap();
            apply_bridge(
                &mut stage.bridge,
                &mut stage.swings,
                &stage.parameters,
                main,
                &mut state.map,
            );
            for swing in &mut stage.swings {
                swing.step(&stage.parameters);
            }
            apply_swings(&stage.swings, main);
            // grkongo.c:1191: Box eligibility is it_8026D324(Box), not a
            // stage constant. Items-on Kongo setup must stay fail-closed
            // until this actual switch eligibility and Box events are ported.
            let event = stage.tick_events(false, &mut state.rng);
            match event {
                None => {}
                Some(melee_gr::kongo::events::Event::Object(id)) => {
                    create_object(state, id, draws, world, objects)?;
                }
                Some(event) => {
                    unimplemented!("grKongo_801D7BBC: Kongo Jungle event {event:?} (map 11's path rider)")
                }
            }
            let main = state.stage_animations.get_mut(&MAIN).unwrap();
            update_collision(main, &mut state.map, MAIN);
            state.map.joint_refresh_island(BRIDGE_COLLISION_JOINT);
        }
        BARREL => {
            let step = stage.barrel.step(&stage.parameters, &mut state.rng);
            let model = state.stage_animations.get_mut(&BARREL).unwrap();
            model.set_joint_rotation_z(BARREL_JOINT, step.angle);
            let position = world_position(model, BARREL_JOINT);
            stage.barrel_position = position;
            stage.barrel_angle = step.angle;
            model.set_animation_rate(step.rate);
            step_hold(state)?;
        }
        _ => {}
    }
    Ok(())
}

/// grKongo_801D5340 (0x801D5340) for an event map: Ground_GetStageGObj
/// (the GObj and its Ground_801C1CD0 / Ground_801C1D38 procs), the map's
/// on_init, then its gobj proc.
fn create_object(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let SceneStage::Kongo(stage) = &state.stage else {
        unreachable!()
    };
    anyhow::ensure!(
        !stage.live(map),
        "grKongo_801D5340: a second map-{map} object while the first lives"
    );
    let object = world.create(3, 5, 0);
    state.stage_gobjs.create(&mut state.effects.gobjs, map);
    for (index, link) in [1, 4, 4].into_iter().enumerate() {
        world.add_tagged_proc(object, link, super::last::TAG_BASE + usize::from(map) * 3 + index);
    }
    objects[usize::from(map)] = Some(object);
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        anyhow::ensure!(
            !state.particles.has_joint_attachment(crate::initial_state::stage::joint_id(map, joint)),
            "grKongo_801D5340: map {map} recreated while generators hold its retired JObjs"
        );
    }
    model
        .recreate(&state.assets.stage, &state.assets.stage_desc.models[usize::from(map)])
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    model.set_hidden(false);
    // on_init: grAnime_801C8138's frame zero first.
    let requests = model.evaluate_initial_frame::<RetailTrig>().to_vec();
    super::garden::spawn_keys(
        &requests,
        map,
        &state.assets,
        &mut state.particles,
        Some(&mut state.effects.events),
        &mut state.rng,
        draws,
    )?;
    let SceneStage::Kongo(stage) = &mut state.stage else {
        unreachable!()
    };
    let offset = if map <= 2 {
        let log = melee_gr::kongo::objects::Log::new(map, &mut state.rng);
        stage.logs[usize::from(map - 1)] = Some(log);
        log.offset
    } else {
        // grAnime_801C78FC: the passer-by runs once.
        state.stage_animations.get_mut(&map).unwrap().clear_loops();
        stage.passersby[usize::from(map - 7)] = true;
        melee_gr::kongo::objects::passerby_offset(map, &mut state.rng)
    };
    let model = state.stage_animations.get_mut(&map).unwrap();
    let mut translation = model.gobj_translation();
    translation.x = offset;
    model.set_gobj_translate(translation);
    Ok(())
}

/// Ground_801C4A08 (0x801C4A08) on an event map: the collision joints
/// bound to it disabled (Ground_801C3128 with mpLib_80057BC0), the GObj
/// freed. Generators still bound to its JObjs keep their last matrices.
fn remove_object(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) {
    for binding in bindings(map) {
        state.map.joint_list_remove(i32::from(binding.joint_index));
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
    let SceneStage::Kongo(stage) = &mut state.stage else {
        unreachable!()
    };
    match map {
        1 | 2 => stage.logs[usize::from(map - 1)] = None,
        _ => stage.passersby[usize::from(map - 7)] = false,
    }
}

/// grKongo_801D7E78 (0x801D7E78): where an event object stands.
fn object_position(state: &mut InitialState, map: u8) -> Vec3 {
    let model = state.stage_animations.get_mut(&map).unwrap();
    world_position(model, melee_gr::kongo::objects::PLACE_JOINT)
}

/// grKongo_801D6198 (0x801D6198): a log's step.
fn step_log(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    use melee_gr::kongo::objects::{log_joint, LogStep};
    let joint = log_joint(map);
    // fn_801D7E60's landings since the last step.
    let stood = state
        .map
        .take_joint_events(joint)
        .iter()
        .any(|event| event.ground_kind == 2);
    let ended = state.stage_animations[&map].ended();
    let position = object_position(state, map);
    let SceneStage::Kongo(stage) = &state.stage else {
        unreachable!()
    };
    let live = melee_gr::kongo::EVENT_MAPS.map(|other| other != map && stage.live(other));
    let mut positions = [Vec3::ZERO; 5];
    let mut count = 0;
    for (other, live) in melee_gr::kongo::EVENT_MAPS.into_iter().zip(live) {
        if live {
            positions[count] = object_position(state, other);
            count += 1;
        }
    }
    let SceneStage::Kongo(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters.clone();
    let log = stage.logs[usize::from(map - 1)]
        .as_mut()
        .expect("a live log");
    if stood {
        // Retail 0x801D7E68..0x801D7E70: C6 = (s16)(C8 + 1).
        // Repeated callbacks before this step all assign the same value.
        log.stood_on();
    }
    let step = log.step(&parameters, ended, || {
        melee_gr::kongo::objects::crowded(position, positions[..count].iter().copied())
    });
    if matches!(step, LogStep::Sink | LogStep::Remove) {
        unimplemented!("grKongo_801D6198 (grkongo.c:579): log lifecycle {step:?} needs a retail witness");
    }
    match step {
        LogStep::Nothing => {}
        LogStep::Land => {
            // Ground_801C2ED0 (bind, transform, snapshot), Ground_801C2FE0,
            // mpJointListAdd and the landing callback.
            let model = state.stage_animations.get_mut(&map).unwrap();
            for binding in bindings(map) {
                model.update_collision(&mut state.map, std::slice::from_ref(binding));
                state.map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
            }
            update_collision(model, &mut state.map, map);
            state.map.joint_list_add(joint);
            state.map.record_joint_events(joint, true);
        }
        LogStep::Sink => {
            let model = state.stage_animations.get_mut(&map).unwrap();
            model
                .select_animation(
                    &state.assets.stage,
                    &state.assets.stage_desc.models[usize::from(map)],
                    1,
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let requests = model.evaluate_initial_frame::<RetailTrig>().to_vec();
            anyhow::ensure!(requests.is_empty(), "a sinking log's frame-zero keys");
            state.map.joint_list_remove(joint);
            state.map.record_joint_events(joint, false);
        }
        LogStep::Remove => remove_object(state, map, world, objects),
    }
    Ok(())
}

/// grKongo_801D577C's hold switch (grkongo.c:436-497), after the spin.
fn step_hold(state: &mut InitialState) -> Result<()> {
    use melee_gr::kongo::barrel::{Hold, Kept};
    let SceneStage::Kongo(stage) = &mut state.stage else {
        unreachable!()
    };
    let barrel = &mut stage.barrel;
    match barrel.hold {
        Hold::Empty => {
            // grKongo_801D8078: a Klaptrap within reach is caught.
            if klaptrap_in_reach(state) {
                unimplemented!("grKongo_801D8078: the barrel catches a Klaptrap (it_802E20D8)");
            }
            return Ok(());
        }
        Hold::Holding => {
            if barrel.kept.is_none() {
                unimplemented!("grkongo.c:461: a holding barrel lost its keep (falls into the launch)");
            }
            let old = barrel.hold_timer;
            barrel.hold_timer = old.wrapping_sub(1);
            if old >= 0 {
                return Ok(());
            }
            barrel.hold = Hold::Launch;
        }
        Hold::Launch => {}
        Hold::Clearing => {
            if !klaptrap_in_reach(state) {
                let SceneStage::Kongo(stage) = &mut state.stage else {
                    unreachable!()
                };
                stage.barrel.hold = Hold::Empty;
            }
            return Ok(());
        }
    }
    // grkongo.c:466-492: the launch along the barrel's facing.
    let SceneStage::Kongo(stage) = &mut state.stage else {
        unreachable!()
    };
    let [damage, _, growth, set_knockback, base_knockback, element] = stage.parameters.launch_hit;
    let position = stage.barrel_position;
    let shot = melee_ft::fighter::barrel::BarrelShot {
        damage: damage as i32,
        growth: growth as i32,
        set_knockback: set_knockback as i32,
        base_knockback: base_knockback as i32,
        element: element as i32,
        angle: stage.barrel.launch_degrees(),
    };
    let kept = stage.barrel.kept;
    stage.barrel.hold = Hold::Clearing;
    match kept {
        Some(Kept::Fighter(index)) => {
            crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| {
                let f: &mut melee_ft::fighter::Fighter = f;
                f.fire_from_barrel(
                    position,
                    shot,
                    &state.assets.fighters[index],
                    &state.assets.stage_camera,
                    &mut state.rng,
                )
                .map_err(|e| anyhow::anyhow!("{e}"))
            })?;
        }
        Some(Kept::Item(_)) => {
            unimplemented!("it_802E2330: the barrel launches a Klaptrap")
        }
        None => unreachable!("a launching barrel holds something"),
    }
    Ok(())
}

/// grKongo_801D8078 (0x801D8078): a Klaptrap item within the catch radius.
fn klaptrap_in_reach(state: &InitialState) -> bool {
    let live = state
        .items
        .iter()
        .any(|item| item.kind == melee_types::ItemKind::Klap && !item.destroyed);
    if live {
        unimplemented!("grKongo_801D8078: a Klaptrap item on Kongo Jungle");
    }
    false
}

/// ftColl_8007BA0C (0x8007BA0C) on Kongo Jungle: the barrel's catch
/// device (ftCo_800C0A28's type-2 test, then fn_801D8134), from the
/// fighter's grab proc.
pub(crate) fn barrel_catch(state: &mut InitialState, player: usize) -> Result<()> {
    use melee_gr::kongo::barrel::{Hold, Kept};
    let SceneStage::Kongo(stage) = &mut state.stage else {
        return Ok(());
    };
    if !stage.catch_active {
        return Ok(());
    }
    let fighter = &state.fighters[player].0;
    // ftCo_800C0A28: bury_timer_2, BarrelWait, x1A6A bit 5 (x2224_b2,
    // a stamina-mode flag, is never set in a versus match).
    if !fighter.barrel_catchable()
    {
        return Ok(());
    }
    if !barrel_reaches(stage, fighter.physics.position) {
        return Ok(());
    }
    // fn_801D8134: the hold time, the keep, the hold; the sound, the
    // material flash (no state), efSync_Spawn(0x405) at the fighter.
    let position = fighter.physics.position;
    stage.barrel.hold_timer =
        melee_gr::kongo::Barrel::draw_hold(&stage.parameters, &mut state.rng);
    stage.barrel.kept = Some(Kept::Fighter(player));
    stage.barrel.hold = Hold::Holding;
    state.effects.spawn_positional::<RetailTrig>(
        0x405,
        position,
        &state.assets.common_particle_bank,
        &mut state.particles,
        &mut state.rng,
    )?;
    // ftCo_800C0874 -> ftCo_8009EB18.
    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
        let f: &mut melee_ft::fighter::Fighter = f;
        f.enter_barrel(&state.assets.fighters[player])
            .map_err(|e| anyhow::anyhow!("{e}"))
    })?;
    Ok(())
}

/// BarrelWait's IASA (ftCo_BarrelWait_IASA -> Ground_801C4DD0 ->
/// grKongo_801D8270): a held fighter's A or B fires the barrel at its next
/// step.
pub(crate) fn barrel_fire_request(state: &mut InitialState, player: usize) {
    if !matches!(state.stage, SceneStage::Kongo(_)) {
        return;
    }
    let requested = std::mem::take(&mut state.fighters[player].0.barrel_fire_request);
    if !requested {
        return;
    }
    if let SceneStage::Kongo(stage) = &mut state.stage {
        if stage.barrel.hold == melee_gr::kongo::barrel::Hold::Holding {
            stage.barrel.hold = melee_gr::kongo::barrel::Hold::Launch;
        }
    }
}

/// ftCo_8009EC44 (8009EC44), BarrelWait's accessory1: the fighter sits at
/// the barrel (Ground_801C4DA0).
pub(crate) fn barrel_accessory(state: &mut InitialState, player: usize) {
    let SceneStage::Kongo(stage) = &state.stage else {
        return;
    };
    let fighter = &mut state.fighters[player].0;
    if fighter.motion_state.id == melee_types::CommonMotionState::BarrelWait && !fighter.in_hitlag() {
        fighter.ride_barrel(stage.barrel_position);
    }
}

/// fn_801D8134 (0x801D8134): separate fsubs, fmuls and fadds at
/// 0x801D8190..0x801D81C0 test the squared three-dimensional distance.
pub(crate) fn barrel_reaches(stage: &Kongo, position: Vec3) -> bool {
    if stage.barrel.hold != melee_gr::kongo::barrel::Hold::Empty {
        return false;
    }
    let pos = stage.barrel_position;
    let dx = pos.x - position.x;
    let dy = pos.y - position.y;
    let dz = pos.z - position.z;
    let radius = stage.parameters.catch_radius;
    (dx * dx + dy * dy) + dz * dz < radius * radius
}
