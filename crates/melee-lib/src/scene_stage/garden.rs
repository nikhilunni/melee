//! Jungle Japes composition: map models and their animations, Cranky's
//! and the Klaptrap's controllers, in grGarden_80202B70's creation order.
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
    garden::{
        procs::{CRANKY, KLAPTRAP, KLAPTRAP_JAW, KLAPTRAP_ROOT, MAP_ORDER},
        Garden,
    },
    last::animation::{BackgroundAnimation, ParticleRequest},
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Jungle Japes map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// Spawn the particle keys an animation's evaluation requested.
pub(crate) fn spawn_keys(
    requests: &[ParticleRequest],
    map: u8,
    assets: &Assets,
    particles: &mut ParticleSystem,
    events: Option<&mut melee_ef::fixture_spawns::EventSink>,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
) -> Result<()> {
    let mut events = events;
    for event in requests {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(map, event.joint), event.matrix));
        if let Some(events) = events.as_deref_mut() {
            events.spawn(&request, false, false);
        }
        particles.spawn::<RetailTrig>(assets.stage_key_bank(event.bank), request, rng, draws)?;
    }
    Ok(())
}

/// grGarden_80202B70 (0x80202B70) from Ground creation to the last map's
/// on_init: the cold setup. The Klaptrap's item is created by the scene
/// once the item pool exists (`klaptrap_item_anchor`).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_garden_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let draws = &mut DrawLog::default();
    let mut animations = Animations::new();
    let mut cranky = None;
    let mut klaptrap = None;
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        if map == KLAPTRAP {
            // grGarden_802031A4 attaches no animation: the Klaptrap rests
            // in its bind pose until its first leap.
            animation.clear_animation();
            klaptrap = Some(melee_gr::garden::Klaptrap::new(&parameters, rng));
        } else {
            if map == CRANKY {
                // grGarden_80203004 draws before grAnime_801C8138.
                cranky = Some(melee_gr::garden::Cranky::new(&parameters, rng));
            }
            let requests = animation.evaluate_initial_frame::<RetailTrig>().to_vec();
            spawn_keys(&requests, map, assets, particles, None, rng, draws)?;
        }
        animations.insert(map, animation);
    }
    Ok((
        SceneStage::Garden(Garden {
            parameters,
            cranky: cranky.expect("Cranky's map"),
            klaptrap: klaptrap.expect("the Klaptrap's map"),
            water_active: false,
        }),
        animations,
    ))
}

/// it_2E6A_UnkMotion19_Phys's source: the jaw's world translation, local
/// scale and rotation (lb_8000B1CC, HSD_JObjGetScale, HSD_JObjGetRotation).
pub(crate) fn klaptrap_jaw(animations: &mut Animations) -> melee_it::YakuAnchor {
    let model = animations.get_mut(&KLAPTRAP).expect("the Klaptrap's model");
    let matrix = model.joint_matrix(KLAPTRAP_JAW);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(KLAPTRAP_JAW),
        rotation: model.joint_rotation(KLAPTRAP_JAW),
    }
}

/// grGarden_80203560 (0x80203560): a fighter entering the river makes the
/// splash generator through grLib_801C96F8 (a new AppSRT, status 1, at the
/// splash point, scaled by the map scale), then scales it again by the
/// fighter's size (three separate fmuls).
pub(crate) fn splash(
    assets: &Assets,
    particles: &mut ParticleSystem,
    splash: melee_gr::garden::Splash,
    rng: &mut HsdRng,
) -> Result<()> {
    let map_scale = assets.stage_desc.parameters.map_scale;
    let scale = (1.0 * map_scale) * splash.scale;
    let mut request = SpawnRequest::new(
        melee_gr::garden::SPLASH_BANK,
        melee_gr::garden::SPLASH_PARTICLE,
        0,
    );
    request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
        translation: splash.position,
        scale: Vec3::new(scale, scale, scale),
        status: 1,
        ..Default::default()
    });
    particles.spawn::<RetailTrig>(&assets.particle_bank, request, rng, &mut DrawLog::default())?;
    Ok(())
}

/// A Jungle Japes gobj proc (s_link 4).
pub(crate) fn run_proc(state: &mut InitialState, map: u8, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::Garden(stage) = &mut state.stage else {
        unreachable!()
    };
    match map {
        CRANKY => {
            let model = &state.stage_animations[&CRANKY];
            let cycled = model.first_animation_ended() || model.first_animation_rewound();
            if let Some(next) = stage
                .cranky
                .tick(&stage.parameters, cycled, &mut state.rng)
            {
                select(state, CRANKY, next, draws)?;
            }
        }
        KLAPTRAP => {
            let animations = &state.stage_animations;
            let step = stage.klaptrap.tick(
                &stage.parameters,
                || animations[&KLAPTRAP].first_animation_ended(),
                &mut state.rng,
            );
            if let Some(motion) = step.item_state {
                crate::scene_items::change_yaku_state(state, motion);
            }
            if step.start_leap {
                select(state, KLAPTRAP, 0, draws)?;
                let SceneStage::Garden(stage) = &state.stage else {
                    unreachable!()
                };
                let x = melee_gr::garden::Klaptrap::leap_x(&stage.parameters, &mut state.rng);
                let model = state.stage_animations.get_mut(&KLAPTRAP).unwrap();
                let mut translation = model.joint_translation(KLAPTRAP_ROOT);
                translation.x = x;
                model.set_joint_translate(KLAPTRAP_ROOT, translation);
            }
            if step.remove_animation {
                state
                    .stage_animations
                    .get_mut(&KLAPTRAP)
                    .unwrap()
                    .clear_animation();
            }
            let anchor = klaptrap_jaw(&mut state.stage_animations);
            crate::scene_items::anchor_yaku(state, anchor);
        }
        // grGarden_80202E48 is lb_800115F4 alone (the scene dispatch);
        // the other maps' procs are empty.
        _ => {}
    }
    Ok(())
}

/// grAnime_801C8138(gobj, map, index): the model's animation `index`
/// from frame zero, its keys' particles spawned now.
fn select(state: &mut InitialState, map: u8, index: usize, draws: &mut DrawLog) -> Result<()> {
    let animation = state.stage_animations.get_mut(&map).unwrap();
    animation
        .select_animation(
            &state.assets.stage,
            &state.assets.stage_desc.models[usize::from(map)],
            index,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let requests = animation.evaluate_initial_frame::<RetailTrig>().to_vec();
    spawn_keys(
        &requests,
        map,
        &state.assets,
        &mut state.particles,
        Some(&mut state.effects.events),
        &mut state.rng,
        draws,
    )
}
