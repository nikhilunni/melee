//! An Arwing's or Wolfen's pass: its path (map 1), ship (map 2 or 10) and,
//! for a near pass, the platform on its wing (map 13 or 16), created by
//! the spawner in grCorneria_801DDAC4's nested order and gone, in GObj
//! order, the tick the path's animation ends.
use super::{bind, corneria, evaluate_frame_zero, follow, models};
use crate::initial_state::InitialState;
use anyhow::{ensure, Result};
use hsd_gobj::{GObjId, TaggedWorld};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    corneria::{
        arwings::{
            pass, path_animation, platform_joint, platform_map, ship_map, ship_position,
            ManeuverStep, Pass, Ship, Shot, PATH_BONE, PATH_DROP, PATH_DROP_BONE, PATH_MAP,
            SHIP_BONE, WOLFEN,
        },
        procs::{bindings, Flight, PendingShot, SHIP_MAP},
    },
    venom::arwings::Platform,
};

/// Player_GetPlayerSlotType's Gm_PKind_NA.
const NO_PLAYER: i32 = 3;
/// HSD_JObjSetRotationY(-1.5707964f): a ship (and a far pass's path)
/// turned a quarter.
const QUARTER_TURN: f32 = -1.5707964;
/// Ground_UpdateStarFoxArwingVisibility's depth on Corneria.
const PLATFORM_DEPTH: f32 = 10.0;

fn flight(state: &mut InitialState) -> &mut Flight {
    corneria(state)
        .stage
        .flight
        .as_mut()
        .expect("a pass map without its pass")
}

/// `lb_8000B1CC(bone, NULL, &out)`: a bone's world translation.
fn bone_position(state: &mut InitialState, map: u8, bone: usize) -> Vec3 {
    let matrix = state.stage_animations.get_mut(&map).unwrap().joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// Ground_GetStageGObj (0x801C14D0) for a map made during the match: its
/// GObj with the Ground_801C1CD0 (s_link 1) and Ground_801C1D38 (s_link 4)
/// procs, and a fresh model under a fresh map-scale wrapper.
pub(super) fn create_object(
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

/// grCorneria_801DD534's HSD_GObj_SetupProc(gobj, gobj_proc, 4), after the
/// map's on_init.
pub(super) fn add_map_proc(world: &mut TaggedWorld, object: GObjId, map: u8) {
    let tag = super::super::last::TAG_BASE + usize::from(map) * 3;
    world.add_tagged_proc(object, 4, tag + 2);
}

/// Ground_801C4A08 (0x801C4A08) on a map made during the match: the
/// collision joints bound to it off, the GObj freed.
pub(super) fn remove_object(
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
    let stage = &mut corneria(state).stage;
    if let Some(flight) = stage.flight {
        if !flight.path && flight.ship.is_none() && flight.platform.is_none() {
            stage.flight = None;
        }
    }
}

/// The spawner's grCorneria_801DD534(1) and the creations nested in it:
/// grCorneria_801DDAC4 (the path) makes the ship, whose init
/// (grCorneria_801DE8E4 / 801DF8D0) makes a near pass's platform
/// (grCorneria_Arwing_801DE4C0) or holds a far ship to the path's bone 4
/// (lb_8000C2F8), sets the ship's scale and draws an Arwing's first gun;
/// then the path takes its scale (x70 in x only).
pub(super) fn create(
    state: &mut InitialState,
    slot: usize,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    ensure!(slot == 0, "grCorneria_801DCE1C: slot {slot} outside the 1P stage");
    let stage = &corneria(state).stage;
    ensure!(
        stage.flight.is_none(),
        "grCorneria_801DCE1C: a pass while the last one's maps live"
    );
    let parameters = stage.parameters;
    let (kind, group) = (stage.spawner.kinds[slot], stage.spawner.groups[slot]);
    let map_scale = state.assets.stage_desc.parameters.map_scale;
    let ship_scale = map_scale * parameters.ship_scale;
    let path_object = create_object(state, PATH_MAP, world, objects)?;
    let ship_map = ship_map(group);
    let ship_object = create_object(state, ship_map, world, objects)?;
    let model = state.stage_animations.get_mut(&ship_map).unwrap();
    let switches = if group == WOLFEN {
        &crate::scene_stage::venom::models::WOLFEN_SWITCHES[..]
    } else {
        &crate::scene_stage::venom::models::ARWING_SWITCHES[..]
    };
    models::play(model, switches);
    let pass = pass(kind);
    let platform_map = platform_map(group);
    let mut platform = None;
    match pass {
        Pass::Near => {
            let object = create_object(state, platform_map, world, objects)?;
            // grCorneria_Arwing_801DE4C0: Ground_801C2ED0, mpJointListAdd,
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
    let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
    model.set_gobj_scale(Vec3::new(ship_scale, map_scale, map_scale));
    add_map_proc(world, path_object, PATH_MAP);
    corneria(state).stage.flight = Some(Flight {
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

/// grCorneria_801DE024 (0x801DE024): the path at the Great Fox's height
/// and its bone 1 lowered 349; its first tick starts the pass's animation
/// (grAnime_801C8138); a far path turns a quarter and its root rises with
/// the descent (at most 150); the pass ends with the animation (the
/// blast-zone tests only pick sounds).
pub(super) fn path(
    state: &mut InitialState,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let height = state.stage_animations[&SHIP_MAP].gobj_translation().y;
    let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
    let mut translation = model.gobj_translation();
    translation.y = height;
    model.set_gobj_translate(translation);
    let dropped = model.joint_translation(PATH_DROP_BONE).y - PATH_DROP;
    model.set_joint_translate_y(PATH_DROP_BONE, dropped);
    let flight = *flight(state);
    ensure!(flight.path, "grCorneria_801DE024: a path its slot no longer names");
    if flight.path_starting {
        self::flight(state).path_starting = false;
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
    if pass(flight.kind) == Pass::Far {
        let descent = corneria(state).stage.great_fox.descent;
        let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
        let mut rotation = model.gobj_rotation();
        rotation.y = QUARTER_TURN;
        model.set_gobj_rotation(rotation);
        let rise = gekko_math::cmp::max(gekko_math::cmp::min(descent, 0.0), -150.0);
        model.set_joint_translate_y(0, gekko_math::msl::fabsf(rise));
    }
    let model = state.stage_animations.get_mut(&PATH_MAP).unwrap();
    if model.first_animation_ended() {
        self::flight(state).path = false;
        remove_object(state, PATH_MAP, world, objects);
    }
    Ok(())
}

/// grCorneria_801DED50 (0x801DED50; a Wolfen's grCorneria_801DFBCC): with
/// its path gone the ship goes; a near ship maneuvers, stands beside the
/// path's bone 4, hands its bone 1 to the platform and may fire its
/// cannon level; a far ship may fire at a fighter.
pub(super) fn ship(
    state: &mut InitialState,
    map: u8,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let flight = *flight(state);
    if !flight.path {
        // A level cannon still at its gun keeps the ship's freed bone
        // (xDF8); never witnessed.
        assert!(
            !charging_cannons(state),
            "itarwinglaser.c:384: a level cannon charging after its ship left"
        );
        // The ship's article GObjs (xE0..xEC) are never set on Corneria.
        remove_object(state, map, world, objects);
        self::flight(state).ship = None;
        finish(state);
        return Ok(());
    }
    let stage = &corneria(state).stage;
    let parameters = stage.parameters;
    let drift_x = stage.great_fox.offset[0];
    let mut ship = flight.ship.expect("the ship's controller");
    if flight.group == WOLFEN {
        let model = state.stage_animations.get_mut(&map).unwrap();
        model.set_joint_rotation_z(SHIP_BONE, 0.0);
    }
    match pass(flight.kind) {
        Pass::Near => {
            let model = state.stage_animations.get_mut(&map).unwrap();
            let mut rotation = model.gobj_rotation();
            rotation.y = QUARTER_TURN;
            model.set_gobj_rotation(rotation);
            let step = {
                let model = &state.stage_animations[&map];
                ship.maneuver(
                    flight.group,
                    || model.subtree_animation_ended(0),
                    &parameters,
                    &mut state.rng,
                )
            };
            let model = state.stage_animations.get_mut(&map).unwrap();
            match step {
                ManeuverStep::Play(index) => model.play_joint_switch(SHIP_BONE, index, 0.0, 1.0),
                ManeuverStep::Level => model.set_joint_rotation_z(SHIP_BONE, 0.0),
                ManeuverStep::Hold => {}
            }
            let bone = bone_position(state, PATH_MAP, PATH_BONE);
            let position = ship_position(flight.kind, bone, drift_x);
            let model = state.stage_animations.get_mut(&map).unwrap();
            model.set_gobj_translate(position);
            let wing = bone_position(state, map, SHIP_BONE);
            let bank = state.stage_animations[&map].joint_rotation_z(SHIP_BONE);
            if let Some(platform) = self::flight(state).platform.as_mut() {
                platform.position = wing;
                platform.bank = bank;
            }
            if ship.may_fire(&parameters, &mut state.rng) {
                let arena = corneria(state).arena;
                // grCorneria_801DEC08: outside on any side.
                let outside = wing.x > arena.right
                    || wing.x < arena.left
                    || wing.y > arena.top
                    || wing.y < arena.bottom;
                if let Some(Shot::Cannon { bone, laser }) = ship.cannon(flight.group, !outside, bank) {
                    let gun = bone_position(state, map, bone);
                    corneria(state).stage.pending_shot = Some(PendingShot {
                        gun,
                        target: None,
                        laser,
                    });
                }
            }
        }
        Pass::Far => {
            let open = ship.may_fire(&parameters, &mut state.rng);
            let bone = bone_position(state, PATH_MAP, PATH_BONE);
            let position = ship_position(flight.kind, bone, drift_x);
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
            let mut rng = state.rng;
            let shot = ship.aimed(flight.group, position.x, open, &mut rng, slot_type);
            state.rng = rng;
            if let Some(Shot::Aimed {
                bone,
                laser,
                target_slot,
            }) = shot
            {
                let fighter = state
                    .fighters
                    .iter()
                    .find(|f| usize::from(f.0.player.id) == target_slot && !f.0.player.secondary)
                    .expect("Player_LoadPlayerCoords: a present slot");
                let mut target = crate::scene_fighter::with_fighter!(fighter, |f| f.player_position);
                target.y += 5.0;
                // lb_8000B1CC on the gun: the root's RObjs resolve against
                // the path's bone 4 as it stands.
                let path_target = state
                    .stage_animations
                    .get_mut(&PATH_MAP)
                    .unwrap()
                    .joint_target(PATH_BONE);
                let model = state.stage_animations.get_mut(&map).unwrap();
                model.constrain_joint::<RetailTrig>(0, Some(path_target));
                let gun = bone_position(state, map, bone);
                corneria(state).stage.pending_shot = Some(PendingShot {
                    gun,
                    target: Some(target),
                    laser,
                });
            }
        }
    }
    ship.clock += 1;
    self::flight(state).ship = Some(ship);
    follow_guns(state, map, flight.group);
    Ok(())
}

/// The level cannons that have not yet left their gun.
fn charging_cannons(state: &InitialState) -> bool {
    state.items.iter().any(|item| {
        item.kind == melee_types::ItemKind::ArwingLaser
            && !item.destroyed
            && matches!(
                it_arwinglaser::laser_state(item),
                laser if matches!(laser.kind, 0 | 2) && !laser.launched
            )
    })
}

/// itArwinglaser_UnkMotion2_Phys's lb_8000B1CC(xDF8): each level cannon
/// still at its gun reads the gun as the ship now stands (one pass flies
/// at a time, so every such cannon is this ship's).
fn follow_guns(state: &mut InitialState, map: u8, group: i32) {
    if !charging_cannons(state) {
        return;
    }
    let bone = if group == WOLFEN {
        melee_gr::corneria::arwings::WOLFEN_GUN_BONE
    } else {
        melee_gr::corneria::arwings::LEFT_GUN_BONE
    };
    let gun = bone_position(state, map, bone);
    for item in state.items.iter_mut() {
        if item.kind == melee_types::ItemKind::ArwingLaser
            && !item.destroyed
            && matches!(it_arwinglaser::laser_state(item).kind, 0 | 2)
            && !it_arwinglaser::laser_state(item).launched
        {
            it_arwinglaser::follow_gun(item, gun);
        }
    }
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

/// grCorneria_801DE568 (0x801DE568): with its path gone the platform goes
/// (its joint off); else it stands on the ship's wing, flattened onto the
/// stage's plane when within 10 of it, at the ships' scale, and its joint
/// follows.
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
        self::flight(state).platform = None;
        finish(state);
        return;
    }
    let parameters = corneria(state).stage.parameters;
    let scale = state.assets.stage_desc.parameters.map_scale * parameters.ship_scale;
    let (translation, hidden) = self::flight(state)
        .platform
        .as_mut()
        .expect("the platform's controller")
        .place_within(PLATFORM_DEPTH);
    let model = state.stage_animations.get_mut(&map).unwrap();
    model.set_hidden(hidden);
    model.set_gobj_translate(translation);
    model.set_gobj_scale(Vec3::new(scale, scale, scale));
    follow(state, map);
}

/// it_802E72E0 / it_802E7654 for the shot the ship's proc fired, right
/// after the proc: nothing runs between them in retail but the ship's
/// colour flash and its clock.
pub(crate) fn spawn_lasers(
    state: &mut InitialState,
    world: &mut TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
    bounds: &melee_it::ItemBounds,
) -> Result<()> {
    let stage = &mut corneria(state).stage;
    let Some(shot) = stage.pending_shot.take() else {
        return Ok(());
    };
    let scale = stage.parameters.ship_scale;
    let stage_drift = stage.great_fox.drift_step();
    let map_scale = state.assets.stage_desc.parameters.map_scale;
    match shot.target {
        Some(target) => {
            let launch = it_arwinglaser::Launch {
                gun: shot.gun,
                target,
                kind: shot.laser,
                joint: 3,
                scale,
                map_scale,
                stage_drift,
            };
            crate::scene_items::spawn_arwing_laser(state, world, item_objects, launch, bounds)
        }
        None => crate::scene_items::spawn_cannon_laser(
            state,
            world,
            item_objects,
            it_arwinglaser::CannonLaunch {
                gun: shot.gun,
                kind: shot.laser,
                scale,
                map_scale,
            },
        ),
    }
}
