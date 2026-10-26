//! The acid's collision device (grZebes_801DCBFC, registered by
//! grZebes_801D9758 as type 3) and the splashes it makes where a fighter
//! enters (grZebes_801DA3F4: a map 5 model, at most ten).
use super::SPLASH_KEYS;
use crate::{initial_state::InitialState, scene_stage::SceneStage};
use anyhow::Result;
use hsd_particle::{rng_sites::DrawLog, system::SpawnRequest};
use hsd_types::Vec3;
use melee_ft::fighter::{stage_hazard::StageHit, RetailTrig};
use melee_gr::zebes::procs::{Splash, ACID_DEVICE, MOST_SPLASHES, SPLASH_MAP};

/// ftColl_8007BAC0 (0x8007BAC0)'s device loop for Brinstar, in
/// Fighter_8006CB94 between the item hits (ftColl_8007925C) and the logs'
/// resolution: ftCo_800C0A28 (type 3: the device's cooldown), the
/// callback, and the hit when the fighter is hurtable (ftColl_GetHitStatus
/// is 0). ftCo_800C0B20 follows on every stage: Brinstar's
/// Ground_801C5700 (grZebes_801DCCB8) marks no line.
pub(crate) fn acid_device(
    state: &mut InitialState,
    player: usize,
    draws: &mut DrawLog,
) -> Result<()> {
    let SceneStage::Zebes(stage) = &state.stage else {
        return Ok(());
    };
    if !stage.acid_device {
        return Ok(());
    }
    let (surface, motion) = (stage.floor, stage.floor_motion());
    let hit_at = stage.parameters.acid_hit;
    let fighter = &state.fighters[player];
    let (ready, hurtable, position, previous) = crate::scene_fighter::with_fighter!(fighter, |f| (
        f.core.stage_hazard_ready(),
        f.core.hurtable_by_stage(),
        f.physics.position,
        f.physics.previous_position
    ));
    if !ready {
        return Ok(());
    }
    // grZebes_801DCBFC (0x801DCBFC): below the surface it hurts; coming
    // from above it (the previous position moved with the surface,
    // 0x801DCC44 fadds), it splashes. Ground_801C53EC(400002) and (0x66)
    // are sounds.
    if position.y >= surface {
        return Ok(());
    }
    let previous_y = previous.y + motion;
    if previous_y > surface {
        splash(state, position, draws)?;
    }
    if hurtable {
        let hit = StageHit::read(&state.assets.stage, hit_at)
            .map_err(|e| anyhow::anyhow!("Brinstar's acid hit: {e}"))?;
        let assets = &state.assets.fighters[player];
        crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
            .core
            .take_stage_hit(&hit, ACID_DEVICE, assets));
    }
    Ok(())
}

/// grZebes_801DA3F4 (0x801DA3F4), through Ground_801C43A4: a splash model
/// (grZebes_801D93DC selects its animation, whose frame-zero key spawns
/// its generator on the fresh model) placed at the fighter.
fn splash(state: &mut InitialState, position: Vec3, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    if stage.splashes >= MOST_SPLASHES {
        return Ok(());
    }
    // The pool slots in turn, so a generator outliving its splash keeps
    // its slot's matrices longest.
    let slots = SPLASH_KEYS.len() as u8;
    let key = (1..=slots)
        .map(|step| SPLASH_KEYS.start + (stage.splash_cursor + step) % slots)
        .find(|key| stage.live_splashes.iter().all(|s| s.key != *key))
        .expect("a free splash model under the limit");
    stage.splash_cursor = key - SPLASH_KEYS.start;
    let object = state.effects.gobjs.create(hsd_gobj::GObjClass::Stage);
    stage.live_splashes.push(Splash { key, object });
    stage.splashes += 1;
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&key).unwrap();
    for joint in 0..model.joint_count() {
        anyhow::ensure!(
            !state
                .particles
                .has_joint_attachment(super::splash_joint(key, joint)),
            "grZebes_801DA3F4: a splash model reused while generators hold its last JObjs"
        );
    }
    model
        .recreate(
            &assets.stage,
            &assets.stage_desc.models[usize::from(SPLASH_MAP)],
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    model.set_gobj_translate(Vec3::ZERO);
    model.set_hidden(false);
    for event in model.evaluate_initial_frame::<RetailTrig>() {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((super::splash_joint(key, event.joint), event.matrix));
        state.effects.events.spawn(&request, false, false);
        state.particles.spawn::<RetailTrig>(
            assets.stage_key_bank(event.bank),
            request,
            &mut state.rng,
            draws,
        )?;
    }
    model.set_gobj_translate(position);
    crate::scene_stage::publish_joint_matrices(
        model,
        key,
        &mut state.particles,
        &mut state.effects.events,
    );
    Ok(())
}

/// Each splash model's Ground_801C1CD0 (s_link 1), after the maps':
/// their GObjs come last.
pub(crate) fn splash_models_tick(state: &mut InitialState, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::Zebes(stage) = &state.stage else {
        return Ok(());
    };
    let assets = std::sync::Arc::clone(&state.assets);
    for splash in stage.live_splashes.iter() {
        let model = state.stage_animations.get_mut(&splash.key).unwrap();
        for event in model.tick::<RetailTrig>() {
            let mut request = SpawnRequest::new(event.bank, event.kind, 0);
            request.joint = Some((super::splash_joint(splash.key, event.joint), event.matrix));
            state.effects.events.spawn(&request, false, false);
            state.particles.spawn::<RetailTrig>(
                assets.stage_key_bank(event.bank),
                request,
                &mut state.rng,
                draws,
            )?;
        }
        crate::scene_stage::publish_joint_matrices(
            model,
            splash.key,
            &mut state.particles,
            &mut state.effects.events,
        );
        state.map.finish_ground_animation();
    }
    Ok(())
}

/// Each splash's grZebes_801D9410 (0x801D9410), after map 8's proc: it
/// sits at the acid's surface and goes when its animation ends
/// (grZebes_801DA4FC).
pub(crate) fn splash_procs(state: &mut InitialState) -> Result<()> {
    let SceneStage::Zebes(stage) = &mut state.stage else {
        unreachable!()
    };
    let surface = stage.floor;
    let mut index = 0;
    while index < stage.live_splashes.len() {
        let splash = stage.live_splashes.iter().nth(index).copied().unwrap();
        let model = state.stage_animations.get_mut(&splash.key).unwrap();
        let mut translate = model.gobj_translation();
        translate.y = surface;
        model.set_gobj_translate(translate);
        crate::scene_stage::publish_joint_matrices(
            model,
            splash.key,
            &mut state.particles,
            &mut state.effects.events,
        );
        if model.first_animation_ended() {
            stage.splashes -= 1;
            model.set_hidden(true);
            state
                .effects
                .gobjs
                .destroy(hsd_gobj::GObjClass::Stage, splash.object);
            stage.live_splashes.remove(index);
        } else {
            index += 1;
        }
    }
    Ok(())
}
