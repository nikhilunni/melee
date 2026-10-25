//! Kongo Jungle N64 composition: the barrel's model (its path animation's
//! rate, the cannon's aim, the shot animation), the platforms' collision
//! joints and the bird's model, in grOldKongo_8020F46C's creation order.
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    oldkongo::{
        procs::{BARREL_MAP, BIRD_MAP, COLLISION_BINDINGS, MAP_ORDER, PLATFORM_MAP},
        BarrelEngine, BirdStep, OldKongo, Shot, BIRD_DEPTH, BIRD_SIDE,
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// The barrel model's cannon (Ground_801C3FA4(gobj, 1)) and the shot
/// animation's subtree (grAnime_801C7FF8(gobj, 2, ...)).
const CANNON: usize = 1;
const SHOT_BONE: usize = 2;
const SHOT_ANIMATION: usize = 2;
/// The bird model's two flights: from the right (descendant 3) or the left (1).
const BIRD_FROM_RIGHT: usize = 3;
const BIRD_FROM_LEFT: usize = 1;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Kongo Jungle N64 map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    if map == BARREL_MAP {
        // The shot and the catch replay their subtree switches without
        // decoding the archive in the tick.
        let model = &assets.stage_desc.models[usize::from(map)];
        for index in [SHOT_ANIMATION, CATCH_ANIMATION] {
            animation
                .prepare_subtree(&assets.stage, model, SHOT_BONE, index)
                .map_err(|e| anyhow::anyhow!("Kongo Jungle N64 map {map}: {e}"))?;
        }
    }
    Ok(animation)
}

/// `lb_8000B1CC(jobj, NULL, &out)` (0x8000B1CC): a bone's world translation.
fn world_position(animation: &mut BackgroundAnimation, bone: usize) -> Vec3 {
    let matrix = animation.joint_matrix(bone);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// Ground_801C2ED0 for map 3 (0x801C2ED0): grOk_803E6580's joints bound to
/// the platforms from the pose before any animation, transformed and taken
/// as the previous positions too.
fn bind_platforms(platforms: &mut BackgroundAnimation, map: &mut melee_mp::CollMap) {
    platforms.update_collision(map, &COLLISION_BINDINGS);
    for binding in &COLLISION_BINDINGS {
        map.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// grOldKongo_8020F46C (0x8020F46C): the cold setup. Maps 0, 3 and 1
/// evaluate animation 0's frame zero (grAnime_801C8138); map 3 binds its
/// collision first; the barrel and the bird draw their timers.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    map: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_old_kongo_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut animations = Animations::new();
    let mut barrel = None;
    let mut bird = None;
    for key in MAP_ORDER {
        let mut animation = load(assets, key)?;
        if key == BIRD_MAP {
            // stageGObj2_OnInit: hidden until its first flight restarts
            // the animation.
            animation.clear_animation();
            animation.set_hidden(true);
            bird = Some(melee_gr::oldkongo::Bird::create(&parameters, rng));
            animations.insert(key, animation);
            continue;
        }
        if key == PLATFORM_MAP {
            bind_platforms(&mut animation, map);
        }
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "Kongo Jungle N64 map {key}: unexpected setup particle key"
        );
        if key == BARREL_MAP {
            barrel = Some(melee_gr::oldkongo::Barrel::create(&parameters, rng));
        }
        animations.insert(key, animation);
    }
    let cannon = world_position(animations.get_mut(&BARREL_MAP).unwrap(), CANNON);
    let stage = OldKongo {
        parameters,
        barrel: barrel.unwrap(),
        bird: bird.unwrap(),
        cannon,
        cannon_aim: 0.0,
        catch_enabled: false,
    };
    Ok((SceneStage::OldKongo(stage), animations))
}

/// The barrel step's engine half over map 1's model.
struct BarrelModel<'a> {
    animation: &'a mut BackgroundAnimation,
}
impl BarrelEngine for BarrelModel<'_> {
    fn aim_cannon(&mut self, aim: f32) -> Vec3 {
        self.animation.set_joint_rotation_z(CANNON, aim);
        world_position(self.animation, CANNON)
    }
    fn set_rate(&mut self, rate: f32) {
        self.animation.set_rate(rate);
    }
    fn start_shot_animation(&mut self) {
        play_subtree(self.animation, SHOT_ANIMATION);
    }
    fn holds_fighter(&self, _fighter: usize) -> bool {
        // Fighter GObjs never leave p_link 8 during a match.
        true
    }
}

/// The catch's model animation (grAnime_801C7FF8(gobj, 2, 7, 1, ...)).
const CATCH_ANIMATION: usize = 1;
/// efSync_Spawn(0x405, gobj, &fighter_position): the catch's dust.
const CATCH_DUST: u16 = 0x405;

/// Fighter_UnkProcessGrab_8006CA5C's ftColl_8007BA0C (0x8007BA0C) for one
/// fighter: the barrel's catch device, once the match started. On a catch
/// (grOldKongo_80210454) the barrel plays its catch animation and the dust,
/// then the fighter enters BarrelWait (ftCo_800C0874 -> ftCo_8009EB18).
pub(crate) fn catch(state: &mut InitialState, player: usize) -> Result<()> {
    let SceneStage::OldKongo(stage) = &mut state.stage else {
        return Ok(());
    };
    if !stage.catch_enabled {
        return Ok(());
    }
    let (catchable, position) = crate::scene_fighter::with_fighter!(&state.fighters[player], |f| (
        f.barrel_catchable(),
        f.core.physics.position
    ));
    if !catchable
        || !stage.barrel.try_catch(
            &stage.parameters,
            stage.cannon,
            player,
            position,
            &mut state.rng,
        )
    {
        return Ok(());
    }
    // Ground_801C5440(gp, 0, 0x129): the catch sound only plays.
    let animation = state.stage_animations.get_mut(&BARREL_MAP).unwrap();
    play_subtree(animation, CATCH_ANIMATION);
    // grMaterial_801C9604(gobj, x6C, 0): a material colour script that
    // only draws.
    state.effects.spawn_positional::<RetailTrig>(
        CATCH_DUST,
        position,
        &state.assets.common_particle_bank,
        &mut state.particles,
        &mut state.rng,
    )?;
    // ftLib_80086C18(keep, 0xD, 0x1E): the rumble.
    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
        .enter_barrel(&state.assets.fighters[player]))
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// grAnime_801C7FF8(gobj, 2, 7, `index`, 0.0, 1.0) on the barrel model.
fn play_subtree(animation: &mut BackgroundAnimation, index: usize) {
    animation.play_prepared_subtree(SHOT_BONE, index);
}

/// The scene's accessory1 for a fighter in BarrelWait (ftCo_8009EC44) and
/// its IASA's Ground_801C4DD0, which asks the barrel to fire.
pub(crate) fn ride(state: &mut InitialState, player: usize) {
    let SceneStage::OldKongo(stage) = &mut state.stage else {
        return;
    };
    let cannon = stage.cannon;
    let inside = crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
        f.ride_barrel(cannon);
        f.motion_state.id == melee_types::CommonMotionState::BarrelWait
    });
    // Only a death leaves BarrelWait while the barrel holds the fighter:
    // the shot empties the barrel's hold first.
    if stage.barrel.held == Some(player)
        && stage.barrel.catch == melee_gr::oldkongo::Catch::Holding
        && !inside
    {
        unimplemented!(
            "ftCo_8009EAF8 (ftdynamics.c:706): BarrelWait's death2_cb, Ground_801C4E20 -> \
             grOldKongo_802105C8"
        );
    }
}
pub(crate) fn relay_fire_request(state: &mut InitialState, player: usize) {
    let requested = crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
        std::mem::take(&mut f.core.barrel_fire_request)
    });
    if requested {
        if let SceneStage::OldKongo(stage) = &mut state.stage {
            stage.barrel.fire_early();
        }
    }
}

/// ftCo_8009EC70 for the barrel's shot.
pub(crate) fn fire(state: &mut InitialState, shot: Shot) -> Result<()> {
    let SceneStage::OldKongo(stage) = &state.stage else {
        unreachable!()
    };
    let p = &stage.parameters;
    let barrel_shot = melee_ft::fighter::barrel::BarrelShot {
        damage: p.shot_damage,
        set_knockback: p.shot_set_knockback,
        growth: p.shot_growth,
        base_knockback: p.shot_base_knockback,
        element: p.shot_element,
        angle: shot.angle,
    };
    let player = shot.fighter;
    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f.fire_from_barrel(
        shot.position,
        barrel_shot,
        &state.assets.fighters[player],
        &state.assets.stage_camera,
        &mut state.rng
    ))
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// A Kongo Jungle N64 gobj proc (s_link 4). Map 3's lb_800115F4 runs in
/// the scene dispatch. Returns a shot for the scene to deliver
/// (ftCo_8009EC70).
pub(crate) fn run_proc(state: &mut InitialState, key: u8) -> Result<Option<Shot>> {
    let SceneStage::OldKongo(stage) = &mut state.stage else {
        unreachable!()
    };
    match key {
        PLATFORM_MAP => {
            // Ground_801C2FE0: the platforms' joints follow their JObjs.
            state
                .stage_animations
                .get_mut(&PLATFORM_MAP)
                .unwrap()
                .update_collision(&mut state.map, &COLLISION_BINDINGS);
        }
        BARREL_MAP => {
            let mut engine = BarrelModel {
                animation: state.stage_animations.get_mut(&BARREL_MAP).unwrap(),
            };
            let (cannon, shot) = stage
                .barrel
                .tick(&stage.parameters, &mut state.rng, &mut engine);
            // Ground_801C4D70 (0x801C4D70).
            stage.cannon = cannon;
            stage.cannon_aim = stage.barrel.aim;
            return Ok(shot);
        }
        BIRD_MAP => {
            let animation = state.stage_animations.get_mut(&BIRD_MAP).unwrap();
            let ended = animation.first_animation_ended();
            match stage.bird.tick(&stage.parameters, ended, &mut state.rng) {
                BirdStep::Idle => {}
                BirdStep::Fly { height, from_right } => {
                    animation
                        .select_animation(
                            &state.assets.stage,
                            &state.assets.stage_desc.models[usize::from(BIRD_MAP)],
                            0,
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    ensure!(
                        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
                        "Kongo Jungle N64 bird: unexpected particle key"
                    );
                    // Camera_800307D0's bounds would push the entry outside
                    // a wider view; the bird only draws.
                    let x = if from_right { BIRD_SIDE } else { BIRD_DEPTH };
                    animation.set_gobj_translate(Vec3::new(x, height, BIRD_DEPTH));
                    let bone = if from_right {
                        BIRD_FROM_RIGHT
                    } else {
                        BIRD_FROM_LEFT
                    };
                    animation.set_joint_hidden(bone, false);
                }
                BirdStep::Land => animation.set_hidden(true),
            }
        }
        _ => {}
    }
    Ok(None)
}
