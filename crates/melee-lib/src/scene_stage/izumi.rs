//! Fountain of Dreams composition: the platform controllers' engine
//! operations (models, collision joints) and the stage's setup, in
//! grIzumi_801CBB88's creation order.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::{
    generator::ApplicationTransform,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    desc::JointMapping,
    izumi::{
        procs::{model, PLATFORMS},
        Izumi, Platform, PlatformStep, Visibility,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// Map whose JObjs carry the collision joints (grIz_803E0D60).
pub(crate) const COLLISION_MAP: u8 = 3;
/// grIz_803E0D60: collision joint `joint_index` follows map 3's descendant
/// `extra`. Joints 0 and 1 are the left and right platforms, 2 the fountain.
pub(crate) const COLLISION_BINDINGS: [JointMapping; 3] = [
    JointMapping {
        joint_index: 0,
        target_index: 3,
        extra: 1,
    },
    JointMapping {
        joint_index: 1,
        target_index: 3,
        extra: 2,
    },
    JointMapping {
        joint_index: 2,
        target_index: 3,
        extra: 3,
    },
];
/// Map-3 descendants marking where each platform model stands (grizumi.c:401, 413).
const PLATFORM_ANCHORS: [usize; 2] = [4, 6];
/// Descendant of the platform model whose height sets the pillar's unit
/// scale (grIzumi_801CCBDC).
const PLATFORM_TOP: usize = 2;
/// Map 2 shows the platform tops: descendants 2 and 3 copy map 3's
/// collision JObjs (grIzumi_801CCA64, grIzumi_801CC0D4).
const REFLECTION_MAP: u8 = 2;
const REFLECTED_TOPS: [usize; 2] = [2, 3];
/// grIzumi_801CBE64: stage bank 30 generators placed by grLib_801C96F8.
const STAGE_PARTICLE_BANK: u8 = 30;
const FOUNTAIN_PARTICLES: [(u32, Vec3); 2] = [
    (30004, Vec3::new(0.0, 0.0, 0.0)),
    (30006, Vec3::new(0.0, 1.0, -27.0)),
];

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Fountain of Dreams map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// Spawn the frame-zero particle keys of a model's attached animation.
fn spawn_keys(
    requests: &[melee_gr::last::animation::ParticleRequest],
    key: u8,
    assets: &Assets,
    particles: &mut ParticleSystem,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
) -> Result<()> {
    for event in requests {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(key, event.joint), event.matrix));
        particles.spawn::<RetailTrig>(&assets.particle_bank, request, rng, draws)?;
    }
    Ok(())
}

/// Ground_801C2ED0 (0x801C2ED0) for map 3: bind each collision joint to its
/// JObj, transform it (mpLib_80055E9C) and take the result as the previous
/// position too (mpLib_80057424).
fn bind_collision(fountain: &mut BackgroundAnimation, map: &mut melee_mp::CollMap) {
    for binding in &COLLISION_BINDINGS {
        fountain.update_collision(map, std::slice::from_ref(binding));
        map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// grIzumi_801CBB88 (0x801CBB88) from Ground creation to the end of map 3's
/// on_init (grIzumi_801CBE64): the cold setup.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    map: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters =
        melee_gr::desc::read_izumi_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))?;
    let draws = &mut DrawLog::default();
    let mut animations = Animations::new();
    // Maps 0 and 1: grAnime_801C8138 evaluates frame zero.
    for key in [0, 1] {
        let mut animation = load(assets, key)?;
        let requests = animation.evaluate_initial_frame::<RetailTrig>().to_vec();
        spawn_keys(&requests, key, assets, particles, rng, draws)?;
        animations.insert(key, animation);
    }
    // Map 3 (grIzumi_801CBE64): collision from the unanimated pose, then
    // its animation's frame zero.
    let mut fountain = load(assets, COLLISION_MAP)?;
    bind_collision(&mut fountain, map);
    let requests = fountain.evaluate_initial_frame::<RetailTrig>().to_vec();
    spawn_keys(&requests, COLLISION_MAP, assets, particles, rng, draws)?;
    animations.insert(COLLISION_MAP, fountain);
    // The reflection camera and the star model (grIzumi_801CCB18) only draw.
    spawn_fountain_particles(assets, particles, rng, draws)?;
    // Map 2 (grIzumi_801CCA64): frame zero, then the platform tops hide.
    let mut reflection = load(assets, REFLECTION_MAP)?;
    let requests = reflection.evaluate_initial_frame::<RetailTrig>().to_vec();
    spawn_keys(&requests, REFLECTION_MAP, assets, particles, rng, draws)?;
    for bone in REFLECTED_TOPS {
        reflection.set_joint_hidden(bone, true);
    }
    animations.insert(REFLECTION_MAP, reflection);
    let platforms =
        [0, 1].map(|side| create_platform(side, &parameters, assets, &mut animations, map, rng));
    let [left, right] = platforms;
    Ok((
        SceneStage::Izumi(Izumi {
            parameters,
            platforms: [left?, right?],
        }),
        animations,
    ))
}

/// grIzumi_801CBE64 (0x801CBE64), grizumi.c:392-403: grLib_801C96F8 with
/// the fountain's bank-30 programs. Each generator gets a new AppSRT
/// (psAddGeneratorAppSRT_begin, status 1) scaled by the map scale.
fn spawn_fountain_particles(
    assets: &Assets,
    particles: &mut ParticleSystem,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
) -> Result<()> {
    let scale = assets.stage_desc.parameters.map_scale;
    for (index, (kind, position)) in FOUNTAIN_PARTICLES.into_iter().enumerate() {
        // retail 0x801CBFB0..0x801CBFD8: separate fmuls by Ground_801C0498.
        let translation = if index == 0 {
            position
        } else {
            Vec3::new(position.x * scale, position.y * scale, position.z * scale)
        };
        let mut request = SpawnRequest::new(STAGE_PARTICLE_BANK, kind, 0);
        request.application_transform = Some(ApplicationTransform {
            translation,
            scale: Vec3::new(scale, scale, scale),
            status: 1,
            ..Default::default()
        });
        particles.spawn::<RetailTrig>(&assets.particle_bank, request, rng, draws)?;
    }
    Ok(())
}

/// grIzumi_801CCBDC (0x801CCBDC): create one map-4 platform at map 3's
/// anchor, then run its controller once.
fn create_platform(
    side: usize,
    parameters: &melee_gr::izumi::Parameters,
    assets: &Assets,
    animations: &mut Animations,
    map: &mut melee_mp::CollMap,
    rng: &mut HsdRng,
) -> Result<Platform> {
    let anchor = world_position(
        animations.get_mut(&COLLISION_MAP).unwrap(),
        PLATFORM_ANCHORS[side],
    );
    let height = parameters.initial_heights[side];
    let mut model = load(assets, 4)?;
    model.set_gobj_translate(anchor);
    // grAnime_801C7FF8(gobj, 0, 7, 0, 0.0, 1.0) only for a surfaced start;
    // the request is evaluated by the next s_link 1 update.
    if height < 0.0 {
        model.clear_animation();
    }
    let top = world_position(&mut model, PLATFORM_TOP);
    // retail 0x801CCD38: fsubs, fdivs by Ground_801C0498.
    let full_height = (top.y - anchor.y) / assets.stage_desc.parameters.map_scale;
    let key = PLATFORMS[side];
    animations.insert(key, model);
    let mut platform = Platform::new(height, side as i16, 0.0, anchor.y, full_height);
    let step = platform.tick(parameters, rng);
    apply_step(&platform, step, key, assets, animations, map)?;
    // grizumi.c:407, 419: the rest height is stored after the first step.
    platform.rest_height = parameters.rest_height;
    Ok(platform)
}

/// `lb_8000B1CC(jobj, NULL, &out)` (0x8000B1CC): a bone's world translation.
fn world_position(animation: &mut BackgroundAnimation, bone: usize) -> Vec3 {
    let matrix = animation.joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// The engine half of grIzumi_801CC358 (0x801CC358), after its state step.
fn apply_step(
    platform: &Platform,
    step: PlatformStep,
    key: u8,
    assets: &Assets,
    animations: &mut Animations,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let model = animations.get_mut(&key).unwrap();
    match step.visibility {
        Visibility::Unchanged => {}
        Visibility::Hide => {
            model.set_hidden(true);
            model.clear_animation();
        }
        Visibility::ShowAndAnimate => {
            model.set_hidden(false);
            model
                .select_animation(&assets.stage, &assets.stage_desc.models[4], 0)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }
    if let Some((x, y)) = step.pillar_scale {
        let scale = model.joint_scale(0);
        model.set_joint_scale(0, Vec3::new(x, y, scale.z));
    }
    let binding = &COLLISION_BINDINGS[platform.collision_joint as usize];
    let fountain = animations.get_mut(&COLLISION_MAP).unwrap();
    if let Some(y) = step.collision_y {
        fountain.set_joint_translate_y(binding.extra as usize, y);
    }
    // mpLib_80055E9C(gp->xC8) ends every step.
    fountain.update_collision(map, std::slice::from_ref(binding));
    Ok(())
}

/// grIzumi_801CC0D4 (0x801CC0D4): map 2's platform tops follow map 3's
/// collision JObjs and hide below the water. lb_800115F4 runs in the scene
/// dispatch.
fn follow_platform_tops(animations: &mut Animations) {
    for (side, &bone) in REFLECTED_TOPS.iter().enumerate() {
        let translation =
            animations[&COLLISION_MAP].joint_translation(COLLISION_BINDINGS[side].extra as usize);
        let tops = animations.get_mut(&REFLECTION_MAP).unwrap();
        tops.set_joint_translate(bone, translation);
        let hidden = tops.joint_hidden(bone);
        if translation.y < 0.0 {
            if !hidden {
                tops.set_joint_hidden(bone, true);
            }
        } else if hidden {
            tops.set_joint_hidden(bone, false);
        }
    }
}

/// A Fountain of Dreams gobj proc (s_link 4).
pub(crate) fn run_proc(state: &mut InitialState, key: u8) -> Result<()> {
    let SceneStage::Izumi(stage) = &mut state.stage else {
        unreachable!()
    };
    // The maps whose JObjs this proc moves after their animation callback
    // published them.
    let moved: &[u8] = match key {
        0..=2 => &[],
        COLLISION_MAP => {
            follow_platform_tops(&mut state.stage_animations);
            &[REFLECTION_MAP]
        }
        _ => {
            let side = PLATFORMS
                .iter()
                .position(|&k| k == key)
                .expect("Fountain of Dreams platform key");
            ensure!(model(key) == Some(4), "platform model");
            let platform = &mut stage.platforms[side];
            let step = platform.tick(&stage.parameters, &mut state.rng);
            apply_step(
                platform,
                step,
                key,
                &state.assets,
                &mut state.stage_animations,
                &mut state.map,
            )?;
            &[key, COLLISION_MAP]
        }
    };
    // Generators attached to those JObjs read their world matrices in the
    // particle proc, after this s_link 4 proc moved them: republish so a
    // rising platform's splash is not a tick behind.
    for &map in moved {
        if let Some(animation) = state.stage_animations.get_mut(&map) {
            super::publish_joint_matrices(
                animation,
                map,
                &mut state.particles,
                &mut state.effects.events,
            );
        }
    }
    Ok(())
}
