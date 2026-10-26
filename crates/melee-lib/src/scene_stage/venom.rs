//! Venom composition (grvenom.c): the Great Fox (map 5) and its spawner,
//! the flight's sequence (map 7) and the tunnel floor (map 9), and the
//! passes of Arwings and Wolfens ([`passes`]), in grVenom_80203B18's order.
mod models;
mod passes;

pub(crate) use models::load;
pub(crate) use passes::spawn_lasers;

use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    venom::{
        arwings::{Spawner, PATH_MAP},
        procs::{bindings, VenomStage, FLIGHT_MAP, MARKER_MAP, SHIP_MAP, TUNNEL_MAP},
        sequence::{JointChange, Sequence, TUNNEL_JOINT},
        Parameters,
    },
};
use models::{ARWING_MAP, ARWING_PLATFORM_MAP, WOLFEN_MAP, WOLFEN_PLATFORM_MAP};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// The maps a pass creates, reserved at load.
const PASS_MAPS: [u8; 5] = [
    PATH_MAP,
    ARWING_MAP,
    WOLFEN_MAP,
    ARWING_PLATFORM_MAP,
    WOLFEN_PLATFORM_MAP,
];
/// The platforms' collision joints, off until a near pass comes
/// (grVenom_80203B18's mpLib_80057BC0(3), (4)).
const PLATFORM_JOINTS: [i32; 2] = [3, 4];

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_venom_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

/// grVenom_80203B18's two lb_80011A50 fields: from each side a constant
/// gust over its half of the stage (M_PI / 3 as the float 1.0471976).
pub(crate) fn winds() -> [melee_lb::radial_force::DirectionalGust; 2] {
    let gust = |x: f32, rectangle: [f32; 4]| melee_lb::radial_force::DirectionalGust {
        direction: Vec3::new(x, 1.0, -1.0),
        frames: -1,
        strength: 1.0,
        decay: 0.0,
        phase_step: 1.047_197_6,
        rectangle,
    };
    [
        gust(-1.0, [-100000.0, 10000.0, 0.0, -100000.0]),
        gust(1.0, [0.0, 10000.0, 100000.0, -100000.0]),
    ]
}

/// Ground_801C2ED0 (0x801C2ED0) for `map`: its joints from its current
/// pose, that pose taken as the previous one too.
fn bind(animation: &mut BackgroundAnimation, map: u8, collision: &mut melee_mp::CollMap) {
    for binding in bindings(map) {
        animation.update_collision(collision, std::slice::from_ref(binding));
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// Ground_801C2FE0 (0x801C2FE0) for `map`: its joints follow their bones.
fn follow(state: &mut InitialState, map: u8) {
    let model = state.stage_animations.get_mut(&map).unwrap();
    for binding in bindings(map) {
        model.update_collision(&mut state.map, std::slice::from_ref(binding));
    }
}

/// A map's frame-zero evaluation (grAnime_801C8138's HSD_JObjAnimAll);
/// Venom's maps have no particle keys.
fn evaluate_frame_zero(animation: &mut BackgroundAnimation, map: u8) -> Result<()> {
    ensure!(
        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
        "Venom map {map}: frame-zero particle keys"
    );
    Ok(())
}

/// grVenom_80203B18 (0x80203B18): maps 4 and 0, the slots and the first
/// wait (no draw: yakumono_param x0 and x4 are both 600), maps 5, 9 and 7,
/// the platforms' joints off; the pass maps reserved. The winds are the
/// scene's ([`winds`]).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let mut animations = Animations::new();
    // grVenom_80203FD4 and 80203F98: grAnime_801C8138 on models without
    // animations; Ground_801C4E70's points only set the fog's range.
    for map in [4, MARKER_MAP] {
        let mut model = load(assets, map)?;
        evaluate_frame_zero(&mut model, map)?;
        animations.insert(map, model);
    }
    let spawner = Spawner::create(&parameters, rng);
    // grVenom_802040F0: Ground_801C2ED0, then the switches (requested, not
    // evaluated).
    let mut ship = load(assets, SHIP_MAP)?;
    ship.clear_animation();
    bind(&mut ship, SHIP_MAP, collision);
    models::play(&mut ship, &models::SHIP_SWITCHES);
    animations.insert(SHIP_MAP, ship);
    // grVenom_80204DD4: Ground_801C2ED0, the wrapper's x and y scale 1.
    let mut tunnel = load(assets, TUNNEL_MAP)?;
    bind(&mut tunnel, TUNNEL_MAP, collision);
    let scale = assets.stage_desc.parameters.map_scale;
    tunnel.set_gobj_scale(Vec3::new(1.0, 1.0, scale));
    animations.insert(TUNNEL_MAP, tunnel);
    // grVenom_80204428: the tunnel floor off; then grAnime_801C8138(map 7).
    collision.joint_list_remove(TUNNEL_JOINT);
    let mut flight = load(assets, FLIGHT_MAP)?;
    evaluate_frame_zero(&mut flight, FLIGHT_MAP)?;
    animations.insert(FLIGHT_MAP, flight);
    for joint in PLATFORM_JOINTS {
        collision.joint_list_remove(joint);
    }
    reserve_pass_maps(assets, &mut animations)?;
    let stage = VenomStage {
        parameters,
        spawner,
        sequence: Sequence::default(),
        flight: None,
        pending_shot: None,
    };
    Ok((SceneStage::Venom(Box::new(stage)), animations))
}

/// grVenom_802040F0's switches on the Great Fox (requested, not evaluated).
pub(crate) fn play_ship_switches(model: &mut BackgroundAnimation) {
    models::play(model, &models::SHIP_SWITCHES);
}

/// The pass maps' models, loaded once and kept hidden until a pass creates
/// them (the tick path allocates nothing).
pub(crate) fn reserve_pass_maps(assets: &Assets, animations: &mut Animations) -> Result<()> {
    for map in PASS_MAPS {
        let mut model = load(assets, map)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(map, model);
    }
    Ok(())
}

/// The display pass (HSD_JObjDispAll on the ship): a far ship's root
/// resolves its RObjs against the path's bone 5 as the tick left it, and
/// keeps the local rotation resolveCnsOrientation recovers, which the next
/// resolution's column lengths read.
pub(crate) fn display(state: &mut InitialState) {
    let SceneStage::Venom(stage) = &state.stage else {
        return;
    };
    if let Some(flight) = stage.flight {
        if flight.path && flight.ship.is_some() {
            passes::display_far_ship(state, flight);
        }
    }
}

/// A Venom gobj proc (s_link 4).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    match map {
        SHIP_MAP => great_fox(state, world, objects)?,
        TUNNEL_MAP => follow(state, TUNNEL_MAP),
        FLIGHT_MAP => sequence(state),
        PATH_MAP => passes::path(state, world, objects)?,
        ARWING_MAP | WOLFEN_MAP => passes::ship(state, map, world, objects)?,
        ARWING_PLATFORM_MAP | WOLFEN_PLATFORM_MAP => passes::platform(state, map, world, objects),
        _ => {}
    }
    Ok(())
}

/// grVenom_80204284 (0x80204284): map 0 follows map 5's GObj translation
/// (both stay at the origin; the markers do not move), the spawner
/// (grVenom_8020362C) and Ground_801C2FE0. The smash taunt's timer only
/// runs on a 1P stage (fn_802040B4), and lb_800115F4 runs in the scene
/// dispatch.
fn great_fox(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let translation = state.stage_animations[&SHIP_MAP].gobj_translation();
    let marker = state.stage_animations.get_mut(&MARKER_MAP).unwrap();
    ensure!(
        translation == Vec3::ZERO && marker.gobj_translation() == Vec3::ZERO,
        "grVenom_80204284: the Great Fox moved its markers"
    );
    marker.set_gobj_translate(translation);
    let SceneStage::Venom(stage) = &mut state.stage else {
        unreachable!()
    };
    let live = stage.flight.is_some_and(|f| f.path);
    let groups = stage.sequence.groups();
    if let Some(slot) = stage
        .spawner
        .tick(&stage.parameters, groups, live, &mut state.rng)
    {
        passes::create(state, slot, world, objects)?;
    }
    follow(state, SHIP_MAP);
    Ok(())
}

/// grVenom_8020454C (0x8020454C): the flight's flags from its animation's
/// frame, and the tunnel floor with them.
fn sequence(state: &mut InitialState) {
    let frame = state.stage_animations[&FLIGHT_MAP]
        .first_animation_frame()
        .expect("grAnime_801C8318: map 7's clock");
    let SceneStage::Venom(stage) = &mut state.stage else {
        unreachable!()
    };
    for change in stage.sequence.step(frame).into_iter().flatten() {
        match change {
            JointChange::On => state.map.joint_list_add(TUNNEL_JOINT),
            JointChange::Off => state.map.joint_list_remove(TUNNEL_JOINT),
        }
    }
}
