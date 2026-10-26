//! An Arwing's or Wolfen's pass: its path (map 2), ship (map 3 or 6) and,
//! for a near pass, the platform on its wing (map 10 or 13), created by
//! the spawner in grVenom_80204F20's nested order and gone, in GObj order,
//! the tick the path's animation ends.
use super::{bind, evaluate_frame_zero, follow, models};
use crate::{initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use hsd_gobj::{GObjId, TaggedWorld};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::venom::{
    arwings::{
        pass, path_animation, platform_joint, platform_map, ship_map, ManeuverStep, Pass, Platform,
        Ship, PATH_BONE, PATH_MAP, SHIP_BONE, WOLFEN,
    },
    procs::{bindings, Flight, PendingShot},
};

/// Player_GetPlayerSlotType's Gm_PKind_NA.
const NO_PLAYER: i32 = 3;

fn stage(state: &mut InitialState) -> &mut melee_gr::venom::procs::VenomStage {
    let SceneStage::Venom(stage) = &mut state.stage else {
        unreachable!("a Venom proc on another stage")
    };
    stage
}

fn flight(state: &mut InitialState) -> &mut Flight {
    stage(state).flight.as_mut().expect("a pass map without its pass")
}

/// `lb_8000B1CC(bone, NULL, &out)`: a bone's world translation.
fn bone_position(state: &mut InitialState, map: u8, bone: usize) -> Vec3 {
    let matrix = state.stage_animations.get_mut(&map).unwrap().joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// Ground_GetStageGObj (0x801C14D0) for a pass map: its GObj with the
/// Ground_801C1CD0 (s_link 1) and Ground_801C1D38 (s_link 4) procs, and a
/// fresh model under a fresh map-scale wrapper.
fn create_object(
    state: &mut InitialState,
    map: u8,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<GObjId> {
    ensure!(
        objects[usize::from(map)].is_none(),
        "Ground_GetStageGObj: a second map-{map} object while the first lives"
    );
    let object = world.create(3, 5, 0);
    state.stage_gobjs.create(&mut state.effects.gobjs, map);
    let tag = super::super::last::TAG_BASE + usize::from(map) * 3;
    world.add_tagged_proc(object, 1, tag);
    world.add_tagged_proc(object, 4, tag + 1);
    objects[usize::from(map)] = Some(object);
    let scale = state.assets.stage_desc.parameters.map_scale;
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        ensure!(
            !state
                .particles
                .has_joint_attachment(crate::initial_state::stage::joint_id(map, joint)),
            "Ground_GetStageGObj: map {map} recreated while generators hold its retired JObjs"
        );
    }
    model.reset_for_creation();
    model.set_gobj_translate(Vec3::ZERO);
    model.set_gobj_rotation(Vec3::ZERO);
    model.set_gobj_scale(Vec3::new(scale, scale, scale));
    model.set_hidden(false);
    Ok(object)
}

/// grVenom_80203EAC's HSD_GObj_SetupProc(gobj, gobj_proc, 4), after the
/// map's on_init.
fn add_map_proc(world: &mut TaggedWorld, object: GObjId, map: u8) {
    let tag = super::super::last::TAG_BASE + usize::from(map) * 3;
    world.add_tagged_proc(object, 4, tag + 2);
}

/// Ground_801C4A08 (0x801C4A08) on a pass map: the collision joints bound
/// to it off (Ground_801C3128 with mpLib_80057BC0), the GObj freed.
fn remove_object(
    state: &mut InitialState,
    map: u8,
    world: &mut TaggedWorld,
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
    if matches!(map, models::ARWING_MAP | models::WOLFEN_MAP) {
        model.constrain_joint::<RetailTrig>(0, None);
    }
}

/// The pass is over once its three maps are gone.
fn finish(state: &mut InitialState) {
    let stage = stage(state);
    if let Some(flight) = stage.flight {
        if !flight.path && flight.ship.is_none() && flight.platform.is_none() {
            stage.flight = None;
        }
    }
}

/// The spawner's grVenom_80203EAC(2) and the creations nested in it:
/// grVenom_80204F20 (the path) creates the ship, whose on_init
/// (grVenom_80205AD4 / 80206874) creates a near pass's platform
/// (grVenom_802056B0) or holds a far ship to the path's bone 5
/// (lb_8000C2F8), sets the ship's scale and draws an Arwing's first gun.
pub(super) fn create(
    state: &mut InitialState,
    slot: usize,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    ensure!(slot == 0, "grVenom_8020362C: slot {slot} outside the 1P stage");
    let venom = stage(state);
    ensure!(
        venom.flight.is_none(),
        "grVenom_8020362C: a pass while the last one's maps live"
    );
    let parameters = venom.parameters;
    let (kind, group) = (venom.spawner.kinds[slot], venom.spawner.groups[slot]);
    let map_scale = state.assets.stage_desc.parameters.map_scale;
    let ship_scale = map_scale * parameters.ship_scale;
    // Ground_GetStageGObj(2); grVenom_80204F20 then creates the ship.
    let path_object = create_object(state, PATH_MAP, world, objects)?;
    let ship_map = ship_map(group);
    let ship_object = create_object(state, ship_map, world, objects)?;
    let model = state.stage_animations.get_mut(&ship_map).unwrap();
    let switches: &[(usize, usize, bool)] = if group == WOLFEN {
        &models::WOLFEN_SWITCHES
    } else {
        &models::ARWING_SWITCHES
    };
    models::play(model, switches);
    let pass = pass(kind);
    let platform_map = platform_map(group);
    let mut platform = None;
    match pass {
        Pass::Near => {
            let object = create_object(state, platform_map, world, objects)?;
            // grVenom_802056B0: Ground_801C2ED0, mpJointListAdd,
            // Ground_801C2ED0 again.
            let model = state.stage_animations.get_mut(&platform_map).unwrap();
            bind(model, platform_map, &mut state.map);
            state.map.joint_list_add(platform_joint(group));
            bind(model, platform_map, &mut state.map);
            platform = Some(Platform {
                slot,
                position: Vec3::ZERO,
                bank: 0.0,
            });
            add_map_proc(world, object, platform_map);
        }
        Pass::Far => {
            // lb_8000C2F8: the RObjs resolve whenever the root is set up.
            let target = state
                .stage_animations
                .get_mut(&PATH_MAP)
                .unwrap()
                .joint_target(PATH_BONE);
            let model = state.stage_animations.get_mut(&ship_map).unwrap();
            model.constrain_joint::<RetailTrig>(0, Some(target));
        }
    }
    let model = state.stage_animations.get_mut(&ship_map).unwrap();
    model.set_gobj_scale(Vec3::new(ship_scale, ship_scale, ship_scale));
    let ship = Ship::create(slot, group, &parameters, &mut state.rng);
    add_map_proc(world, ship_object, ship_map);
    // grVenom_80204F20's tail: the path's scale (a near pass flattened in
    // depth), its first tick pending.
    let path_scale = match pass {
        Pass::Near => Vec3::new(map_scale, map_scale, map_scale * parameters.ship_scale),
        Pass::Far => Vec3::new(map_scale, map_scale, map_scale),
    };
    let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
    model.set_gobj_scale(path_scale);
    add_map_proc(world, path_object, PATH_MAP);
    stage(state).flight = Some(Flight {
        kind,
        group,
        path: true,
        path_starting: true,
        ship_map,
        ship: Some(ship),
        platform_map,
        platform,
    });
    Ok(())
}

/// grVenom_802053B0 (0x802053B0): the first tick starts the pass's
/// animation (grAnime_801C8138); a far path keeps its GObj unturned; the
/// pass ends with the animation (the blast-zone tests only pick sounds).
pub(super) fn path(
    state: &mut InitialState,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let flight = *flight(state);
    ensure!(flight.path, "grVenom_802053B0: a path its slot no longer names");
    if flight.path_starting {
        flight_mut(state).path_starting = false;
        let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
        model.reset_pose();
        model
            .select_animation(
                &state.assets.stage,
                &state.assets.stage_desc.models[usize::from(PATH_MAP)],
                path_animation(flight.kind),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        return evaluate_frame_zero(model, PATH_MAP);
    }
    let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
    if pass(flight.kind) == Pass::Far {
        let mut rotation = model.gobj_rotation();
        rotation.y = 0.0;
        model.set_gobj_rotation(rotation);
    }
    if model.first_animation_ended() {
        flight_mut(state).path = false;
        remove_object(state, PATH_MAP, world, objects);
    }
    Ok(())
}

fn flight_mut(state: &mut InitialState) -> &mut Flight {
    flight(state)
}

/// grVenom_80205F30 (0x80205F30; a Wolfen's grVenom_80206B70): with its
/// path gone the ship goes; a near ship maneuvers, follows the path's bone
/// 5 and hands its bone 1 to the platform; a far ship may fire.
pub(super) fn ship(
    state: &mut InitialState,
    map: u8,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let flight = *flight(state);
    if !flight.path {
        // The ship's article GObjs (xE0..xEC) are never set on Venom.
        remove_object(state, map, world, objects);
        flight_mut(state).ship = None;
        finish(state);
        return Ok(());
    }
    let parameters = stage(state).parameters;
    let mut ship = flight.ship.expect("the ship's controller");
    if flight.group == WOLFEN {
        let model = state.stage_animations.get_mut(&map).unwrap();
        model.set_joint_rotation_z(SHIP_BONE, 0.0);
    }
    match pass(flight.kind) {
        Pass::Near => {
            let model = state.stage_animations.get_mut(&map).unwrap();
            let mut rotation = model.gobj_rotation();
            rotation.y = 0.0;
            model.set_gobj_rotation(rotation);
            let ended = model.subtree_animation_ended(0);
            match ship.maneuver(flight.group, ended, &parameters, &mut state.rng) {
                ManeuverStep::Play(index) => {
                    let model = state.stage_animations.get_mut(&map).unwrap();
                    model.play_joint_switch(SHIP_BONE, index, 0.0, 1.0);
                }
                ManeuverStep::Level => {
                    let model = state.stage_animations.get_mut(&map).unwrap();
                    model.set_joint_rotation_z(SHIP_BONE, 0.0);
                }
                ManeuverStep::Hold => {}
            }
            // grVenom_802052E0's offsets are all zero.
            let position = bone_position(state, PATH_MAP, PATH_BONE);
            let model = state.stage_animations.get_mut(&map).unwrap();
            model.set_gobj_translate(position);
            let wing = bone_position(state, map, SHIP_BONE);
            let bank = state.stage_animations[&map].joint_rotation_z(SHIP_BONE);
            if let Some(platform) = flight_mut(state).platform.as_mut() {
                platform.position = wing;
                platform.bank = bank;
            }
            ship.advance();
        }
        Pass::Far => {
            let path = bone_position(state, PATH_MAP, PATH_BONE);
            let slot_type = |slot: usize| {
                let present = state
                    .fighters
                    .iter()
                    .any(|f| usize::from(f.0.player.id) == slot && !f.0.player.secondary);
                if present {
                    0
                } else {
                    NO_PLAYER
                }
            };
            let player_position = |slot: usize| {
                let fighter = state
                    .fighters
                    .iter()
                    .find(|f| usize::from(f.0.player.id) == slot && !f.0.player.secondary)
                    .expect("Player_LoadPlayerCoords: a present slot");
                crate::scene_fighter::with_fighter!(fighter, |f| f.player_position)
            };
            let mut rng = state.rng;
            let shot = ship.fire(
                flight.kind,
                flight.group,
                path,
                &mut rng,
                slot_type,
                player_position,
            );
            state.rng = rng;
            if let Some((shot, target)) = shot {
                // lb_8000B1CC on the gun: the root's RObjs resolve against
                // the path's bone 5 as it stands.
                let target_bone = state
                    .stage_animations
                    .get_mut(&PATH_MAP)
                    .unwrap()
                    .joint_target(PATH_BONE);
                let model = state.stage_animations.get_mut(&map).unwrap();
                model.constrain_joint::<RetailTrig>(0, Some(target_bone));
                let gun = bone_position(state, map, shot.bone);
                stage(state).pending_shot = Some(PendingShot {
                    gun,
                    target,
                    laser: shot.laser,
                });
            }
        }
    }
    flight_mut(state).ship = Some(ship);
    Ok(())
}

/// [`super::display`] for a far pass's ship.
pub(super) fn display_far_ship(state: &mut InitialState, flight: Flight) {
    if pass(flight.kind) != Pass::Far {
        return;
    }
    let target = state
        .stage_animations
        .get_mut(&PATH_MAP)
        .unwrap()
        .joint_target(PATH_BONE);
    let model = state.stage_animations.get_mut(&flight.ship_map).unwrap();
    model.constrain_joint::<RetailTrig>(0, Some(target));
    model.joint_matrix(0);
}

/// grVenom_80205758 (0x80205758): with its path gone the platform goes
/// (its joint off); else it stands on the ship's wing, flattened onto the
/// stage's plane when near it, at the ships' scale, and its joint follows.
pub(super) fn platform(
    state: &mut InitialState,
    map: u8,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) {
    let flight = *flight(state);
    if !flight.path {
        state.map.joint_list_remove(platform_joint(flight.group));
        remove_object(state, map, world, objects);
        flight_mut(state).platform = None;
        finish(state);
        return;
    }
    let parameters = stage(state).parameters;
    let scale = state.assets.stage_desc.parameters.map_scale * parameters.ship_scale;
    let (translation, hidden) = flight_mut(state)
        .platform
        .as_mut()
        .expect("the platform's controller")
        .place();
    let model = state.stage_animations.get_mut(&map).unwrap();
    model.set_hidden(hidden);
    model.set_gobj_translate(translation);
    model.set_gobj_scale(Vec3::new(scale, scale, scale));
    follow(state, map);
}

/// it_802E7654 for the shot the ship's proc fired, right after the proc:
/// nothing runs between them in retail but the ship's colour overlay and
/// its clock.
pub(crate) fn spawn_lasers(
    state: &mut InitialState,
    world: &mut TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
    bounds: &melee_it::ItemBounds,
) -> Result<()> {
    let SceneStage::Venom(stage) = &mut state.stage else {
        return Ok(());
    };
    let Some(shot) = stage.pending_shot.take() else {
        return Ok(());
    };
    let launch = it_arwinglaser::Launch {
        gun: shot.gun,
        target: shot.target,
        kind: shot.laser,
        joint: 0,
        scale: stage.parameters.ship_scale,
        map_scale: state.assets.stage_desc.parameters.map_scale,
    };
    crate::scene_items::spawn_arwing_laser(state, world, item_objects, launch, bounds)
}
