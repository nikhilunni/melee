//! Pokemon Stadium composition: the stage-owned screen and transformation
//! controller, with the particles, collision and fighters they touch.
use crate::{
    assets::Assets,
    initial_state::InitialState,
    scene_fighter::{with_fighter, SceneFighter},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use hsd_particle::{rng_sites::DrawLog, system::SpawnRequest};
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    stadium::{procs, ScreenMode, ScreenPlayers, Stadium},
};
use std::collections::BTreeMap;

/// Bank of the stage's own particles (map_ptcl).
const STAGE_PARTICLE_BANK: u8 = 30;

/// `Ground_GetStageGObj` for the maps `grStadium_OnInit` creates, with
/// `grAnime_801C8138(map, 0)` for 1, 2 and 5 (their models carry no joint
/// animation, so frame zero requests nothing).
pub(crate) fn load_models(assets: &Assets) -> Result<BTreeMap<u8, BackgroundAnimation>> {
    let mut animations = BTreeMap::new();
    for map in procs::MAP_ORDER {
        let model = &assets.stage_desc.models[usize::from(map)];
        let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
            .map_err(|e| anyhow::anyhow!("Pokemon Stadium map {map}: {e}"))?;
        animation.set_map_scale(assets.stage_desc.parameters.map_scale);
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "unexpected Pokemon Stadium setup particle event"
        );
        animations.insert(map, animation);
    }
    Ok(animations)
}

/// The collision edits of `grStadium_OnInit` (0x801D101C), in C order:
/// map 5's `Ground_801C2ED0` binds, updates and snapshots joint 4 (its
/// `mpJointListAdd(4)` finds the joint enabled); `grStadium_801D13E0`
/// disables the pit lines; then every form's joints are disabled and the
/// frame is stitched to the base arena.
pub(crate) fn initialize_collision(
    map: &mut melee_mp::CollMap,
    animations: &mut BTreeMap<u8, BackgroundAnimation>,
) {
    let base = animations.get_mut(&5).expect("base arena model");
    let bindings: Vec<_> = procs::stage_joints(5).cloned().collect();
    base.update_collision(map, &bindings);
    for binding in &bindings {
        map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
    for line in procs::PIT_LINES {
        map.line_disable(line);
    }
    for joint in procs::DISABLED_AT_INIT {
        map.joint_list_remove(joint);
    }
    let (frame, arena) = procs::STITCHED_AT_INIT;
    map.stitch_joints(frame, arena);
}

/// A map's gobj proc (s_link 4). Map 0 has none.
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    draws: &mut DrawLog,
    radial_forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    match map {
        1 => run_screen(state, draws),
        2 => {
            // grStadium_801D1520: the controller once released, then
            // lb_800115F4 and Ground_801C2FE0 (map 2 binds no joint).
            let SceneStage::Stadium(stage) = &mut state.stage else {
                unreachable!()
            };
            if !stage.transformation.waiting_for_start {
                stage.transformation.tick();
            }
            radial_forces.tick();
            let wind = radial_forces.wind_state();
            for fighter in &mut state.fighters {
                with_fighter!(fighter, |f| f.stage_wind = wind);
            }
            Ok(())
        }
        5 => {
            // grStadium_801D1604: Ground_801C2FE0.
            update_form_collision(state, map);
            Ok(())
        }
        _ => unreachable!("Pokemon Stadium map {map} has no gobj proc"),
    }
}

/// `Ground_801C2FE0` (0x801C2FE0): each stage joint bound to the map
/// follows its JObj.
fn update_form_collision(state: &mut InitialState, map: u8) {
    let bindings: Vec<_> = procs::stage_joints(map).cloned().collect();
    state
        .stage_animations
        .get_mut(&map)
        .expect("live form model")
        .update_collision(&mut state.map, &bindings);
}

/// `grStadium_801D1390` (0x801D1390): the audience flash, then the screen.
fn run_screen(state: &mut InitialState, draws: &mut DrawLog) -> Result<()> {
    if let Some(position) = Stadium::audience_flash(&mut state.rng) {
        let scale = state.assets.stage_desc.parameters.map_scale;
        // 801D1EBC..801D1ED8: three fmuls.
        let position =
            hsd_types::Vec3::new(position.x * scale, position.y * scale, position.z * scale);
        spawn_ground_effect(state, super::stadium::FLASH, position, draws)?;
    }
    let SceneStage::Stadium(stage) = &mut state.stage else {
        unreachable!()
    };
    let mut players = Players {
        fighters: &state.fighters,
    };
    stage
        .screen
        .tick(&stage.parameters, &mut state.rng, &mut players);
    Ok(())
}

const FLASH: u32 = melee_gr::stadium::FLASH_PARTICLE;

/// `grLib_801C96F8` (0x801C96F8): a stage particle whose AppSRT carries the
/// position; the generator itself sits at the origin. A descriptor without
/// its own AppSRT (kind 0x20000) gets one with status 1; an existing one
/// keeps status 0 and loses camera facing (xA2 = 0). The scale is the
/// map scale (1 * scale, fmuls).
pub(crate) fn spawn_ground_effect(
    state: &mut InitialState,
    kind: u32,
    position: hsd_types::Vec3,
    draws: &mut DrawLog,
) -> Result<Option<usize>> {
    let Some(descriptor) = state.assets.particle_bank.descriptor(kind) else {
        return Ok(None);
    };
    let owns_transform = descriptor.kind & 0x20000 != 0;
    let scale = state.assets.stage_desc.parameters.map_scale;
    let mut request = SpawnRequest::new(STAGE_PARTICLE_BANK, kind, 0);
    request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
        translation: position,
        scale: hsd_types::Vec3::new(scale, scale, scale),
        status: if owns_transform { 0 } else { 1 },
        ..Default::default()
    });
    state.effects.events.spawn(&request, true, false);
    let id = state.particles.spawn::<RetailTrig>(
        &state.assets.particle_bank,
        request,
        &mut state.rng,
        draws,
    )?;
    if let Some(id) = id {
        let generator = state
            .particles
            .generators
            .iter()
            .find(|g| g.id == id)
            .unwrap();
        let mut transform = (**generator.application_transform.as_ref().unwrap()).clone();
        transform.camera_facing = 0;
        state.particles.set_application_transform(id, transform);
    }
    Ok(id)
}

/// The Versus scene's calls into the screen (gm_16AE.c): the countdown
/// banner's start (fn_8016B7B4) and end (fn_8016B7F8, after Ground's start),
/// and GO's end (fn_8016B784).
pub(crate) fn show(state: &mut InitialState, mode: ScreenMode) {
    let SceneStage::Stadium(stage) = &mut state.stage else {
        return;
    };
    let mut players = Players {
        fighters: &state.fighters,
    };
    stage
        .screen
        .set_mode(mode, 0, &stage.parameters, &mut state.rng, &mut players);
}

struct Players<'a> {
    fighters: &'a [SceneFighter; 2],
}
impl Players<'_> {
    /// `Player_GetEntity(slot)`.
    fn fighter(&self, slot: i16) -> Option<&SceneFighter> {
        self.fighters
            .iter()
            .find(|f| with_fighter!(f, |f| i16::from(f.player.id) == slot))
    }
}
impl ScreenPlayers for Players<'_> {
    fn exists(&self, slot: i16) -> bool {
        self.fighter(slot).is_some()
    }
    fn suppressed(&self, slot: i16) -> bool {
        let fighter = self.fighter(slot).expect("existing player");
        with_fighter!(fighter, |f| f.status.disabled)
    }
    fn framed(&mut self, _slot: i16) -> bool {
        unimplemented!("grpstadium.c:1381-1434: grStadium_801D32D0 close-up framing")
    }
}
