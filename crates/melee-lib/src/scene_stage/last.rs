//! FD's stage actions applied to live, preloaded background models.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::Result;
use hsd_particle::{generator::ApplicationTransform, rng_sites::DrawLog, system::SpawnRequest};
use hsd_types::Vec3;
use melee_gr::last::{animation::BackgroundAnimation, AnimationStatus, StageAction};
use std::collections::BTreeMap;

/// grLast_8021B920 case 1: base subtrees keep their own animation clocks.
pub(crate) fn load_animations(assets: &Assets) -> Result<BTreeMap<u8, BackgroundAnimation>> {
    let mut result = BTreeMap::new();
    for map in 0..assets.stage_desc.models.len() {
        let model = &assets.stage_desc.models[map];
        let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        animation
            .prepare_switches(&assets.stage, model)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        if (4..9).contains(&map) {
            animation.clear_animation();
            const BASE_ANIMATIONS: [usize; 5] = [0, 4, 6, 8, 10];
            animation.play_prepared(2, BASE_ANIMATIONS[map - 4], true);
            animation.play_prepared(1, 0, false);
            animation.clear_joint_loop(1);
            if map == 4 {
                animation.play_prepared(16, 0, false);
            }
        }
        if map == 7 {
            animation.set_joint_scale(5, Vec3::new(1.0, 1.0, 1.0));
        }
        result.insert(map as u8, animation);
    }
    Ok(result)
}

pub(crate) const TAG_BASE: usize = 1 << 15;
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let status = AnimationStatus {
        layer_animation_stopped: std::array::from_fn(|index| {
            state.stage_animations[&(index as u8 + 4)].joint_ended(1)
        }),
        tilt_frame: state.stage_animations[&7].joint_frame(2),
        tilt_rewound: state.stage_animations[&7].joint_rewound(2),
        material_fade_complete: std::array::from_fn(|index| {
            state.stage_animations[&(index as u8 + 4)].overlay.complete
        }),
    };
    let SceneStage::FinalDestination(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.run_stage_proc(map, &status, &mut state.rng);
    if map == 7 {
        let bg = stage.ground.background.as_ref().unwrap();
        state
            .stage_animations
            .get_mut(&7)
            .unwrap()
            .set_background_rotation(bg.applied_pitch, bg.applied_yaw);
        super::publish_joint_matrices(
            state.stage_animations.get_mut(&7).unwrap(),
            7,
            &mut state.particles,
            &mut state.effects.events,
        );
    }
    for action in stage.actions.drain(..) {
        match action {
            StageAction::MaterialFade { map, script } => {
                let animation = state.stage_animations.get_mut(&map).unwrap();
                animation.overlay_script = Some(script);
                animation
                    .overlay
                    .start(&state.assets.stage_desc.material_scripts[script]);
                // grLast_8021B920 explicitly invokes 801C9698 again after 801C9604.
                animation
                    .overlay
                    .tick(&state.assets.stage_desc.material_scripts[script]);
            }
            StageAction::CreateMap(map) => {
                state
                    .stage_animations
                    .get_mut(&map)
                    .unwrap()
                    .reset_for_creation();
                let object = world.create(3, 5, 0);
                for (index, link) in [1, 4, 4].into_iter().enumerate() {
                    world.add_tagged_proc(object, link, TAG_BASE + usize::from(map) * 3 + index);
                }
                objects[usize::from(map)] = Some(object);
            }
            StageAction::DestroyMap(map) => {
                if let Some(object) = objects[usize::from(map)].take() {
                    world.destroy(object);
                }
                for joint in 0..state.stage_animations[&map].joint_count() {
                    state.particles.expire_joint(joint_id(map, joint));
                }
            }
            StageAction::PlayAnimation {
                map,
                joint,
                animation,
                subtree,
            } => {
                state.stage_animations.get_mut(&map).unwrap().play_prepared(
                    usize::from(joint),
                    usize::from(animation),
                    subtree,
                );
            }
            StageAction::StopAnimation { map, joint } => {
                state
                    .stage_animations
                    .get_mut(&map)
                    .unwrap()
                    .clear_joint_loop(usize::from(joint));
            }
            StageAction::RequestAnimation { map, joint } => {
                state
                    .stage_animations
                    .get_mut(&map)
                    .unwrap()
                    .set_joint_rate(usize::from(joint), 0.0);
            }
            StageAction::RemoveJointGenerators { map, joint } => {
                assert_eq!(joint, 0, "grLib_801C9854 whole model");
                for joint in 0..state.stage_animations[&map].joint_count() {
                    state.particles.expire_joint(joint_id(map, joint));
                }
            }
            StageAction::CreateTiltGenerator => {
                // grLast_8021B920 case 9 -> grLib_801C96F8: bank 30, particle 30001.
                let matrix = state.stage_animations.get_mut(&7).unwrap().joint_matrix(5);
                let mut request = SpawnRequest::new(30, 30001, 0);
                let scale = state.assets.stage_desc.parameters.map_scale;
                request.application_transform = Some(ApplicationTransform {
                    translation: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                    scale: Vec3::new(scale, scale, scale),
                    status: 1,
                    ..Default::default()
                });
                state.effects.events.spawn(&request, true, false);
                let id = state.particles.spawn::<melee_ft::fighter::RetailTrig>(
                    &state.assets.particle_bank,
                    request,
                    &mut state.rng,
                    draws,
                )?;
                if let Some(id) = id {
                    let mut transform = (**state
                        .particles
                        .generators
                        .iter()
                        .find(|g| g.id == id)
                        .unwrap()
                        .application_transform
                        .as_ref()
                        .unwrap())
                    .clone();
                    transform.camera_facing = 0;
                    state.particles.set_application_transform(id, transform);
                }
                let bg = stage.ground.background.as_mut().unwrap();
                bg.generator_id = id;
                bg.generator_present = id.is_some();
            }
            StageAction::UpdateTiltGeneratorTransform => {
                let id = stage
                    .ground
                    .background
                    .as_ref()
                    .unwrap()
                    .generator_id
                    .unwrap();
                let matrix = state.stage_animations.get_mut(&7).unwrap().joint_matrix(5);
                let mut transform = (**state
                    .particles
                    .generators
                    .iter()
                    .find(|g| g.id == id)
                    .unwrap()
                    .application_transform
                    .as_ref()
                    .unwrap())
                .clone();
                // grLast_8021ADD0: transform basis points, subtract the origin,
                // normalize, then recover the emitter's Euler orientation.
                let origin = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
                let basis = |axis| {
                    let mut point = Vec3::ZERO;
                    hsd_anim::mtx::mtx_mult_vec(&matrix, &axis, &mut point);
                    melee_lb::dynamics::arithmetic::normalize(
                        melee_lb::dynamics::arithmetic::difference(point, origin),
                    )
                };
                transform.translation = origin;
                transform.rotation = melee_lb::orientation::from_partial_basis(
                    basis(Vec3::new(0.0, 0.0, 1.0)),
                    basis(Vec3::new(0.0, 1.0, 0.0)),
                );
                transform.status = 0;
                state.particles.set_application_transform(id, transform);
            }
            StageAction::RemoveTiltGenerator => {
                if let Some(id) = stage
                    .ground
                    .background
                    .as_mut()
                    .unwrap()
                    .generator_id
                    .take()
                {
                    state.particles.expire_generator(id);
                }
            }
            StageAction::Quake => state.effects.request_camera_quake(1, Vec3::ZERO),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn fd_background_completes_a_full_cycle_with_cloned_continuation() {
        let files =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
        if !melee_test_support::require_files([files.join("GrNLa.dat")]) {
            return;
        }
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox),
                PlayerConfig::new(Port::P2, Character::Marth),
            ],
        )
        .with_seed(Seed(42));
        let assets = GameAssets::load(files, &config).unwrap();
        let mut game = Match::new(&assets, config).unwrap();
        let mut branch = game.clone();
        let mut view = presentation::Presentation::new(&game).unwrap();
        let mut phases = std::collections::BTreeSet::new();
        let mut last = 0;
        for tick in 0..28_000 {
            game.step(&Inputs::default())
                .unwrap_or_else(|e| panic!("tick {tick}: {e}"));
            branch.step(&Inputs::default()).unwrap();
            view.capture(&game).unwrap();
            let crate::scene_stage::SceneStage::FinalDestination(stage) =
                &game.engine.state().stage
            else {
                unreachable!()
            };
            let phase = stage.ground.phase as u16;
            phases.insert(phase);
            if phase != last {
                eprintln!("tick {tick}: phase {phase}");
                if phase == 13 {
                    assert_eq!(
                        game.engine.state().stage_animations[&4].overlay.color,
                        [3, 3, 3, 4]
                    );
                }
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&branch).unwrap()
                );
                branch.clone_from(&game);
                last = phase;
                if phase == 1 && phases.len() == 17 {
                    return;
                }
            }
        }
        panic!("background did not complete its cycle: {phases:?}");
    }
}
