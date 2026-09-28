//! Battlefield composition: the background swap controller's engine
//! operations (`grBattle_BG_Callback2`, retail 0x8021A3BC), applied to live,
//! preloaded background models in C order.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use hsd_particle::{rng_sites::DrawLog, system::SpawnRequest};
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    battle::{
        BackgroundEvent, CURRENT_BACKGROUND_OVERLAY, PREVIOUS_BACKGROUND_OVERLAY, TRANSITION_MAP,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

/// Backgrounds grBattle_80219D84 can create after grBattle_OnInit.
const RESERVED_BACKGROUNDS: [u8; 2] = [2, 4];

/// `Ground_GetStageGObj` (0x801C14D0): one map's model under its map-scale
/// wrapper, with animation 0 attached (grAnime_801C8138).
pub(crate) fn load_model(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// Storage for the models the swap controller animates or creates later.
/// `grBattle_BG_Callback0` (0x8021A344) leaves map 3 hidden and without
/// animation; maps 2 and 4 are not live until created.
pub(crate) fn load_reserved(
    assets: &Assets,
    animations: &mut BTreeMap<u8, BackgroundAnimation>,
) -> Result<()> {
    let mut transition = load_model(assets, TRANSITION_MAP)?;
    transition.clear_animation();
    transition.set_hidden(true);
    animations.insert(TRANSITION_MAP, transition);
    for map in RESERVED_BACKGROUNDS {
        if let std::collections::btree_map::Entry::Vacant(entry) = animations.entry(map) {
            entry.insert(load_model(assets, map)?);
        }
    }
    Ok(())
}

/// A Battlefield map's gobj proc (s_link 4). Only map 3's controller acts:
/// maps 0, 1, 2 and 4 have empty callbacks; map 6 updates the static
/// collision transform and the empty quake list, guarded at restoration.
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut [Option<hsd_gobj::GObjId>; 10],
) -> Result<()> {
    match map {
        TRANSITION_MAP => {}
        0 | 1 | 2 | 4 | 6 => return Ok(()),
        _ => unimplemented!("grbattle.c:165-170: demo/event background"),
    }
    let SceneStage::Battlefield(stage) = &mut state.stage else {
        unreachable!()
    };
    let animations = &state.stage_animations;
    let event = stage.tick(
        &mut state.rng,
        || animations[&TRANSITION_MAP].first_animation_ended(),
        |map| objects[usize::from(map)].is_some(),
        |map| animations[&map].overlay.complete,
    );
    let assets = std::sync::Arc::clone(&state.assets);
    match event {
        BackgroundEvent::None => {}
        BackgroundEvent::AttachTransition => {
            let transition = state.stage_animations.get_mut(&TRANSITION_MAP).unwrap();
            transition
                .select_animation(
                    &assets.stage,
                    &assets.stage_desc.models[usize::from(TRANSITION_MAP)],
                    0,
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let requests = transition.evaluate_initial_frame::<RetailTrig>();
            assert!(requests.is_empty(), "Battlefield transition particles");
        }
        BackgroundEvent::ShowTransition => show_transition(state),
        BackgroundEvent::Swap { previous, current } => {
            show_transition(state);
            fade(state, &assets, previous, PREVIOUS_BACKGROUND_OVERLAY);
            create_background(state, &assets, current, draws, world, objects)?;
            fade(state, &assets, current, CURRENT_BACKGROUND_OVERLAY);
        }
        BackgroundEvent::Retire { previous } => {
            // Ground_801C4A08 (0x801C4A08) removes the GObj and its procs. It
            // does not visit particle generators (no hsd_8039D5DC): each one
            // holds a reference to its JObj (hsd_8039EFAC ref_INC) and keeps
            // emitting from that JObj's last matrix, which nothing refreshes.
            if let Some(object) = objects[usize::from(previous)].take() {
                world.destroy(object);
            }
            state
                .stage_animations
                .get_mut(&TRANSITION_MAP)
                .unwrap()
                .set_hidden(true);
        }
    }
    Ok(())
}

/// grbattle.c:386-388: clear JOBJ_HIDDEN while the root still carries it.
fn show_transition(state: &mut InitialState) {
    let transition = state.stage_animations.get_mut(&TRANSITION_MAP).unwrap();
    if transition.hidden() {
        transition.set_hidden(false);
    }
}

/// `grMaterial_801C9604(gobj, script, 0)` (0x801C9604): restart the color
/// overlay and interpret its first frame immediately.
fn fade(state: &mut InitialState, assets: &Assets, map: u8, script: usize) {
    let animation = state.stage_animations.get_mut(&map).unwrap();
    animation.overlay_script = Some(script);
    animation
        .overlay
        .start(&assets.stage_desc.material_scripts[script]);
}

/// `grBattle_80219D84` (0x80219D84): Ground_GetStageGObj registers the
/// Ground_801C1CD0 (s_link 1) and Ground_801C1D38 (s_link 4) procs, then
/// grBattle_GObj{1,2,4}_Callback0 attaches animation 0 and evaluates frame
/// zero, whose particle keys spawn at once; the empty gobj proc joins s_link 4.
/// grMaterial_801C94D8's material class and the render priority only affect
/// drawing.
fn create_background(
    state: &mut InitialState,
    assets: &Assets,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut [Option<hsd_gobj::GObjId>; 10],
) -> Result<()> {
    let object = world.create(3, 5, 0);
    for (index, link) in [1, 4, 4].into_iter().enumerate() {
        world.add_tagged_proc(object, link, super::last::TAG_BASE + usize::from(map) * 3 + index);
    }
    objects[usize::from(map)] = Some(object);
    let animation = state.stage_animations.get_mut(&map).unwrap();
    // A generator still bound to the retired instance's JObjs would share
    // the recreated model's joint ids and follow its matrices.
    for joint in 0..animation.joint_count() {
        ensure!(
            !state.particles.has_joint_attachment(joint_id(map, joint)),
            "grBattle_80219D84: map {map} recreated while generators hold its retired JObjs"
        );
    }
    animation
        .recreate(&assets.stage, &assets.stage_desc.models[usize::from(map)])
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for event in animation.evaluate_initial_frame::<RetailTrig>() {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(map, event.joint), event.matrix));
        state.effects.events.spawn(&request, false, false);
        state.particles.spawn::<RetailTrig>(
            &assets.particle_bank,
            request,
            &mut state.rng,
            draws,
        )?;
    }
    // hsd_8039D214 (0x8039D214) sets up each attached JObj's matrix when the
    // generators update later this tick: the frame-zero pose, not the cache.
    super::publish_joint_matrices(
        animation,
        map,
        &mut state.particles,
        &mut state.effects.events,
    );
    Ok(())
}
