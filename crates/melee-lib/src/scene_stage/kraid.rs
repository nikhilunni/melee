//! Brinstar Depths composition (grkraid.c): Kraid and his effects (maps 4
//! and 1) and the stage's turn (map 3) over their models, collision joints
//! and particles, in grKraid_OnInit's creation order.
use crate::{
    assets::Assets,
    initial_state::{stage::joint_id, InitialState},
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::{
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Vec3;
use melee_ft::{fighter::RetailTrig, input::PadSample};
use melee_gr::{
    kraid::{
        procs::{KraidStage, EFFECTS_MAP, KRAID_MAP, MAP_ORDER, STAGE_BINDINGS, STAGE_MAP},
        Kraid, Turn, TurnEngine,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grKraid_801FE3B4: bone 13's subtree loops animation 2.
const IDLE_BONE: usize = 13;
const IDLE_ANIMATION: usize = 2;
/// grKraid_801FE440: bone 18's subtree shakes while the stage turns.
const SHAKE_BONE: usize = 18;
/// The root the turn rotates (Ground_801C3FA4(gobj, 0)).
const TURN_BONE: usize = 0;
/// PAD_BUTTON_A in HSD_PadStatus.button.
const PAD_A: u32 = 0x100;
/// grKraid_801FE2D0 wraps its request past the last of map 1's seven
/// animations.
const LAST_EFFECT: i32 = 6;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Brinstar Depths map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    if map == STAGE_MAP {
        let model = &assets.stage_desc.models[usize::from(STAGE_MAP)];
        for (bone, index) in SUBTREE_SWITCHES {
            animation
                .prepare_subtree(&assets.stage, model, bone, index)
                .map_err(|e| anyhow::anyhow!("Brinstar Depths map {map}: {e}"))?;
        }
    }
    Ok(animation)
}

/// `lb_8000B1CC(jobj, NULL, &out)` (0x8000B1CC): a bone's world translation.
fn world_position(animation: &mut BackgroundAnimation, bone: usize) -> Vec3 {
    let matrix = animation.joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// Map 3's grAnime_801C7FF8 switches: the idle loop and both turn poses,
/// decoded at load so the turn replays them without allocating.
const SUBTREE_SWITCHES: [(usize, usize); 3] = [
    (IDLE_BONE, IDLE_ANIMATION),
    (SHAKE_BONE, melee_gr::kraid::SHAKE),
    (SHAKE_BONE, melee_gr::kraid::STILL),
];

/// grAnime_801C7FF8(gobj, `bone`, 7, `index`, 0.0, 1.0) on map 3.
fn play_subtree(animation: &mut BackgroundAnimation, bone: usize, index: usize) {
    animation.play_prepared_subtree(bone, index);
}

/// The particles a model's frame-zero evaluation asks for, at once.
pub(crate) fn spawn_keys(
    animation: &mut BackgroundAnimation,
    map: u8,
    assets: &Assets,
    particles: &mut ParticleSystem,
    events: Option<&mut melee_ef::fixture_spawns::EventSink>,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
) -> Result<()> {
    let mut events = events;
    for event in animation.evaluate_initial_frame::<RetailTrig>() {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(map, event.joint), event.matrix));
        if let Some(events) = events.as_deref_mut() {
            events.spawn(&request, false, false);
        }
        particles.spawn::<RetailTrig>(assets.stage_key_bank(event.bank), request, rng, draws)?;
    }
    Ok(())
}

/// grKraid_OnInit: the models and controllers, with the setup's draws
/// (Kraid's first wait, and whatever the frame-zero keys spawn).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_kraid_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut animations = Animations::new();
    let mut kraid = None;
    for key in MAP_ORDER {
        let mut animation = load(assets, key)?;
        match key {
            KRAID_MAP | EFFECTS_MAP => {
                // grKraid_801FE818 / 801FE1EC request no animation; the
                // OnInit hides both models after creating them.
                animation.clear_animation();
                animation.set_hidden(true);
                if key == KRAID_MAP {
                    kraid = Some(Kraid::create(&parameters, rng));
                }
            }
            _ => {
                if key == STAGE_MAP {
                    // Ground_801C2ED0: grKr_803E4C78's joints on bone 12.
                    animation.update_collision(collision, &STAGE_BINDINGS);
                    for binding in &STAGE_BINDINGS {
                        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
                    }
                }
                // grAnime_801C8138(gobj, map, 0).
                spawn_keys(
                    &mut animation,
                    key,
                    assets,
                    particles,
                    None,
                    rng,
                    &mut DrawLog::default(),
                )?;
                if key == STAGE_MAP {
                    play_subtree(&mut animation, IDLE_BONE, IDLE_ANIMATION);
                }
            }
        }
        animations.insert(key, animation);
    }
    let stage = KraidStage {
        parameters,
        turn: Turn::new(),
        kraid: kraid.unwrap(),
        effect_request: 0,
        second_port_buttons: 0,
    };
    Ok((SceneStage::Kraid(Box::new(stage)), animations))
}

struct Stage<'a> {
    animation: &'a mut BackgroundAnimation,
    quakes: &'a mut crate::quake::Quakes,
    camera: &'a mut melee_cm::GameCamera,
    gobjs: &'a mut hsd_gobj::GObjAddresses,
}
impl TurnEngine for Stage<'_> {
    fn play_shake(&mut self, index: usize) {
        play_subtree(self.animation, SHAKE_BONE, index);
    }
    fn shake_ended(&self) -> bool {
        self.animation.subtree_animation_ended(SHAKE_BONE)
    }
    fn turn(&mut self, radians: f32) {
        self.animation.add_rotation_z(TURN_BONE, radians);
        self.quakes
            .request(self.camera, melee_cm::QuakeKind::Loop, self.gobjs);
    }
}

/// grAnime_801C8138(gobj, map, index), with its frame-zero particles.
fn select(state: &mut InitialState, map: u8, index: usize, draws: &mut DrawLog) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let animation = state.stage_animations.get_mut(&map).unwrap();
    animation
        .select_animation(
            &assets.stage,
            &assets.stage_desc.models[usize::from(map)],
            index,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    spawn_keys(
        animation,
        map,
        &assets,
        &mut state.particles,
        Some(&mut state.effects.events),
        &mut state.rng,
        draws,
    )?;
    super::publish_joint_matrices(
        animation,
        map,
        &mut state.particles,
        &mut state.effects.events,
    );
    Ok(())
}

/// A Brinstar Depths gobj proc (s_link 4). Map 3's lb_800115F4 runs in the
/// scene dispatch; maps 0 and 2 have empty procs.
pub(crate) fn run_proc(
    state: &mut InitialState,
    key: u8,
    pads: &[PadSample; 4],
    draws: &mut DrawLog,
) -> Result<()> {
    match key {
        EFFECTS_MAP => {
            // grKraid_801FE2D0: debug code left in retail. The second
            // port's A press (HSD_PadMasterStatus[1].trigger) steps map 1
            // through its animations.
            let SceneStage::Kraid(stage) = &mut state.stage else {
                unreachable!()
            };
            let buttons = pads[1].buttons.0;
            let trigger = buttons & (stage.second_port_buttons ^ buttons);
            stage.second_port_buttons = buttons;
            if trigger & PAD_A != 0 {
                stage.effect_request += 1;
                if stage.effect_request > LAST_EFFECT {
                    stage.effect_request = 0;
                }
                let index = stage.effect_request as usize;
                select(state, key, index, draws)?;
            }
        }
        STAGE_MAP => {
            let SceneStage::Kraid(stage) = &mut state.stage else {
                unreachable!()
            };
            let animation = state.stage_animations.get_mut(&key).unwrap();
            let mut engine = Stage {
                animation,
                quakes: &mut state.quakes,
                camera: &mut state.camera,
                gobjs: &mut state.effects.gobjs,
            };
            stage.turn.tick(&stage.parameters, &mut state.rng, &mut engine);
            // Ground_801C2FE0.
            let animation = state.stage_animations.get_mut(&key).unwrap();
            animation.update_collision(&mut state.map, &STAGE_BINDINGS);
        }
        KRAID_MAP => {
            let SceneStage::Kraid(stage) = &mut state.stage else {
                unreachable!()
            };
            // `(int) map->u.kraid.x4 == 0`.
            let turning = gekko_math::msl::fctiwz(stage.turn.direction) != 0;
            let ended = state.stage_animations[&key].first_animation_ended();
            let parameters = stage.parameters;
            let step = stage.kraid.tick(&parameters, turning, ended, &mut state.rng);
            if let Some(x) = step.rise {
                for map in [KRAID_MAP, EFFECTS_MAP] {
                    let animation = state.stage_animations.get_mut(&map).unwrap();
                    let mut translate = animation.gobj_translation();
                    translate.x = x;
                    animation.set_gobj_translate(translate);
                }
                for map in [KRAID_MAP, EFFECTS_MAP] {
                    state.stage_animations.get_mut(&map).unwrap().set_hidden(false);
                }
            }
            if let Some((hand, direction)) = step.swipe {
                let position = world_position(state.stage_animations.get_mut(&key).unwrap(), hand);
                let SceneStage::Kraid(stage) = &mut state.stage else {
                    unreachable!()
                };
                if stage.turn.swipe(&parameters, position.x, direction) {
                    // Ground_801C53EC(420005): sound; then the quake.
                    state.quakes.request(
                        &mut state.camera,
                        melee_cm::QuakeKind::Medium,
                        &mut state.effects.gobjs,
                    );
                }
            }
            if let Some(animation) = step.play {
                ensure!(animation >= 0, "grKraid_801FF068: animation {animation}");
                select(state, KRAID_MAP, animation as usize, draws)?;
                select(state, EFFECTS_MAP, animation as usize, draws)?;
            }
            if step.hide {
                for map in [KRAID_MAP, EFFECTS_MAP] {
                    state.stage_animations.get_mut(&map).unwrap().set_hidden(true);
                }
            }
        }
        _ => {}
    }
    Ok(())
}
