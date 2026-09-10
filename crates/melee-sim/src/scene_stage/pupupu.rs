//! Dream Land composition: model queries and particle allocation surround the
//! stage-owned controller, in Ground's callback order.
use crate::{
    initial_state::{stage::joint_id, InitialState},
    scene_fighter::{with_fighter, SceneFighter},
    scene_stage::SceneStage,
};
use anyhow::Result;
use hsd_particle::{rng_sites::DrawLog, system::SpawnRequest};
use melee_ft::fighter::RetailTrig;

/// ftLib_800864A8 / 800866DC: vote by the current camera-target bone,
/// with zero on the positive side. The controller performs a tie RNG draw.
fn fighter_sides(fighters: &mut [SceneFighter; 2]) -> i32 {
    fighters
        .iter_mut()
        .map(|fighter| {
            with_fighter!(fighter, |f| {
                if f.status.disabled {
                    return 0;
                }
                let camera = &f.core.attributes.camera;
                let position = melee_ft::fighter::caches::bone_position(
                    &mut f.core.skeleton,
                    f.core.animation.root,
                    camera.camera_zoom_target_bone as usize,
                    camera.zoom_offset,
                );
                if position.x < 0.0 {
                    -1
                } else {
                    1
                }
            })
        })
        .sum()
}

pub(crate) fn run_proc(state: &mut InitialState, map: u8, draws: &mut DrawLog) -> Result<()> {
    let SceneStage::Pupupu(stage) = &mut state.stage else {
        unreachable!()
    };
    let selected = match map {
        7 => {
            let ended = state.stage_animations[&7].ended();
            // Querying a bone can materialize a matrix; retail only asks when
            // entering its turn phase, not on every callback.
            let sides = if stage.phase == melee_gr::pupupu::Phase::Turning && stage.entering {
                fighter_sides(&mut state.fighters)
            } else {
                0
            };
            stage.tick_whispy(ended, sides, &mut state.rng)
        }
        1 => stage.take_secondary_animation(),
        6 => stage.cloud_animation(),
        8 => {
            stage.tick_background();
            None
        }
        _ => None,
    };
    if let Some(selected) = selected {
        let animation = state.stage_animations.get_mut(&map).unwrap();
        animation
            .select_animation(
                &state.assets.stage,
                &state.assets.stage_desc.models[map as usize],
                selected,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        for event in animation.evaluate_initial_frame::<RetailTrig>() {
            let mut request = SpawnRequest::new(event.bank, event.kind, 0);
            request.joint = Some((joint_id(map, event.joint), event.matrix));
            state.effects.events.spawn(&request, false, false);
            state.particles.spawn::<RetailTrig>(
                &state.assets.particle_bank,
                request,
                &mut state.rng,
                draws,
            )?;
        }
    }
    Ok(())
}
