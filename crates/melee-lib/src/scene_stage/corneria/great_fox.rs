//! The Great Fox's proc (grCorneria_801DD9A8) and its cannons' material
//! items and shots.
use super::{carry_markers, corneria, follow, passes};
use crate::{initial_state::InitialState, scene_stage::SceneStage};
use anyhow::Result;
use hsd_particle::rng_sites::DrawLog;
use hsd_types::Vec3;
use melee_gr::corneria::{
    backgrounds::{scenery, under},
    great_fox::{
        CannonStep, Scenery, BREAK_EFFECT, BROKEN_BONE, CANNON_JOINT, CHARGE_EFFECT,
        CHARGING_STATE, INTACT_BONE, LEFT_CANNON_BONE, MUZZLE_EFFECT, RESTING_STATE,
        RIGHT_CANNON_BONE,
    },
    procs::SHIP_MAP,
};
use melee_types::ItemKind;

/// The common particle bank (grLib_801C9808's bank 0) and the stage's.
const COMMON_BANK: u8 = 0;
const STAGE_BANK: u8 = 0x1E;
/// grCn_803E1FE8 / grCn_803E2000 (grZakoGenerator_Spawn): the hull's
/// segments in bone 0's frame where the sea splashes, and their
/// generators (grCorneria_801E2454: 0x7534; 801E2480: 0x7530).
const BOW_SEGMENTS: [[Vec3; 2]; 1] = [[Vec3::new(-59.0, -58.0, 18.0), Vec3::new(-22.0, -73.0, 0.0)]];
const KEEL_SEGMENTS: [[Vec3; 2]; 2] = [
    [Vec3::new(-81.0, -21.0, 28.0), Vec3::new(14.0, -59.0, 128.0)],
    [Vec3::new(14.0, -59.0, 128.0), Vec3::new(0.0, -75.0, 173.0)],
];
const BOW_SPLASH: u32 = 0x7534;
const KEEL_SPLASH: u32 = 0x7530;
/// grZakoGenerator_801CA394's x10 for both: the chance a crossing splashes.
const SPLASH_CHANCE: f32 = 0.3;
/// The material items' hurt capsule (grMaterial_801C8DE0): x 4 to 16,
/// radius 4.
const CANNON_HURT: ([Vec3; 2], f32) = ([Vec3::new(4.0, 0.0, 0.0), Vec3::new(16.0, 0.0, 0.0)], 4.0);
/// A fighter within this height of the left cannon fires it at once.
const AIM_HEIGHT: f32 = 10.0;

/// A bone's world translation, scale and rotation, which a material item
/// copies (it_2E6A_UnkMotion19_Phys).
fn anchor(model: &mut melee_gr::last::animation::BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// grCorneria_801DD674's grMaterial_801C8CFC calls: the left cannon's item
/// (state 0), then the right's (state 2), both taking hits.
pub(super) fn spawn_cannon_items(
    animations: &mut super::Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
) -> [Option<u32>; 2] {
    let model = animations.get_mut(&SHIP_MAP).unwrap();
    let mut spawn = |bone: usize, state: u16| {
        let id = crate::scene_items::spawn_yaku(items, gobjs, resources, map, anchor(model, bone));
        let item = items.get_mut(id).unwrap();
        if state != 0 {
            it_yaku::change_state(item, state, resources.get(ItemKind::PokemonRandom));
        }
        it_yaku::set_hurt_capsule(item, CANNON_HURT.0, CANNON_HURT.1);
        it_yaku::set_hurt_enabled(item, true);
        Some(id)
    };
    [spawn(LEFT_CANNON_BONE, RESTING_STATE), spawn(RIGHT_CANNON_BONE, CHARGING_STATE)]
}

/// grCorneria_801DD9A8 (0x801DD9A8): once the countdown has released it,
/// the drift, the descent, the sea's splashes, the cannon and the
/// spawner; then (always) Ground_801C2FE0, the markers
/// (grCorneria_801E1878) and the things riding the ship. The lights
/// (grCorneria_801E2110) only draw; lb_800115F4 runs in the scene
/// dispatch.
pub(super) fn tick(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
    draws: &mut DrawLog,
) -> Result<()> {
    if !corneria(state).stage.waiting {
        move_ship(state);
        splash(state, &BOW_SEGMENTS, BOW_SPLASH, draws)?;
        splash(state, &KEEL_SEGMENTS, KEEL_SPLASH, draws)?;
        cannon(state, world, item_objects, draws)?;
        // grCorneria_801E0E40 answers the debug camera's pad only.
        spawner(state, world, objects)?;
    }
    follow(state, SHIP_MAP);
    let translation = corneria(state).stage.great_fox.translation();
    let assets = std::sync::Arc::clone(&state.assets);
    let (arena, camera) = carry_markers(&assets, &mut state.stage_animations, translation)?;
    let corneria = corneria(state);
    corneria.arena = arena;
    corneria.camera = camera;
    let limits = corneria.limits(assets.stage_desc.parameters.map_scale);
    state.stage_limits = limits;
    follow_cannons(state);
    follow_lasers(state);
    Ok(())
}

/// grCorneria_801E1970 and grCorneria_801E2228: the drift moves the ship;
/// the descent reads which background the camera's interest is over.
fn move_ship(state: &mut InitialState) {
    let scale = state.assets.stage_desc.parameters.map_scale;
    let interest = state.camera.transform.interest.x;
    let SceneStage::Corneria(corneria) = &mut state.stage else {
        unreachable!()
    };
    let stage = &mut corneria.stage;
    let parameters = stage.parameters;
    stage.great_fox.drift(&parameters, &mut state.rng);
    let backgrounds = stage.backgrounds;
    let over = || -> Scenery {
        let map = under(backgrounds, scale, interest)
            .expect("grcorneria.c:2437: grCorneria_801E08CC, the camera over no background");
        scenery(map)
    };
    stage.great_fox.descend(scale, over, &mut state.rng);
    let translation = stage.great_fox.translation();
    state
        .stage_animations
        .get_mut(&SHIP_MAP)
        .unwrap()
        .set_gobj_translate(translation);
}

/// grZakoGenerator_801CA43C (0x801CA43C) over the hull's `segments` at the
/// sea's height: a segment that crosses it splashes three times in ten
/// (HSD_Randf), at the crossing (retail 0x801CA560 / 578: fmadds), or at a
/// drawn point of a level segment.
fn splash(
    state: &mut InitialState,
    segments: &[[Vec3; 2]],
    particle: u32,
    draws: &mut DrawLog,
) -> Result<()> {
    let scale = state.assets.stage_desc.parameters.map_scale;
    let height = corneria(state).stage.great_fox.sea_level(scale);
    for segment in segments {
        let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
        let (a, b) = (model.joint_point(0, segment[0]), model.joint_point(0, segment[1]));
        let above = a.y > height && b.y > height;
        let below = a.y < height && b.y < height;
        if above || below || state.rng.randf() >= SPLASH_CHANCE {
            continue;
        }
        let rise = b.y - a.y;
        // The hull's segments are never level as the ship bobs: a drawn
        // point along one (HSD_Randf) is unwitnessed.
        assert!(
            gekko_math::msl::fabsf(rise) > 0.0001,
            "grzakogenerator.c:95: a level segment's random splash point"
        );
        let t = (height - a.y) / rise;
        let position = Vec3::new(
            gekko_math::fma::fmadds(t, b.x - a.x, a.x),
            height,
            gekko_math::fma::fmadds(t, b.z - a.z, a.z),
        );
        let assets = std::sync::Arc::clone(&state.assets);
        crate::scene_stage::zebes::ground_effect(
            &assets,
            &mut state.particles,
            &mut state.effects.events,
            &mut state.rng,
            draws,
            (particle, STAGE_BANK),
            position,
        )?;
    }
    Ok(())
}

/// grCorneria_801E1348 (0x801E1348): one step of the cannon.
fn cannon(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
    draws: &mut DrawLog,
) -> Result<()> {
    // Phase 2's scan: lb_8000B1CC on the left cannon, then each fighter's
    // cur_pos (ftLib_80086644) in the fighter list.
    let aim = state
        .stage_animations
        .get_mut(&SHIP_MAP)
        .unwrap()
        .joint_point(LEFT_CANNON_BONE, Vec3::ZERO)
        .y;
    let fighters = &state.fighters;
    let at_height = || {
        fighters.iter().any(|f| {
            let y = crate::scene_fighter::with_fighter!(f, |f| f.physics.position.y);
            gekko_math::msl::fabsf(aim - y) <= AIM_HEIGHT
        })
    };
    let SceneStage::Corneria(corneria) = &mut state.stage else {
        unreachable!()
    };
    let parameters = corneria.stage.parameters;
    let step = corneria.stage.cannon.step(&parameters, &mut state.rng, at_height);
    let items = corneria.stage.cannon_items;
    let assets = std::sync::Arc::clone(&state.assets);
    let yaku = assets.items.get(ItemKind::PokemonRandom);
    let cannon_items = |state: &mut InitialState, apply: &dyn Fn(&mut melee_it::ItemCore)| {
        for id in items.into_iter().flatten() {
            if let Some(item) = state.items.get_mut(id) {
                apply(item);
            }
        }
    };
    match step {
        CannonStep::Nothing => {}
        CannonStep::Charge => {
            for bone in [LEFT_CANNON_BONE, RIGHT_CANNON_BONE] {
                crate::scene_stage::onett::spawn_on_bone(state, SHIP_MAP, bone, (CHARGE_EFFECT, COMMON_BANK), draws)?;
            }
            cannon_items(state, &|item| it_yaku::change_state(item, CHARGING_STATE, yaku));
            cannon_items(state, &|item| it_yaku::set_hurt_enabled(item, false));
        }
        CannonStep::Aim => {
            // hsd_8039D580 on each cannon: its generators stop.
            for bone in [LEFT_CANNON_BONE, RIGHT_CANNON_BONE] {
                let joint = crate::initial_state::stage::joint_id(SHIP_MAP, bone);
                state.particles.expire_joint_generators(joint);
            }
            cannon_items(state, &|item| it_yaku::change_state(item, RESTING_STATE, yaku));
        }
        CannonStep::Open => cannon_items(state, &|item| it_yaku::set_hurt_enabled(item, true)),
        CannonStep::Fire { bone, offset, laser } => {
            crate::scene_stage::onett::spawn_on_bone(state, SHIP_MAP, bone, (MUZZLE_EFFECT, COMMON_BANK), draws)?;
            let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
            let muzzle = gobj_point(model, offset);
            crate::scene_items::spawn_great_fox_laser(state, world, item_objects, muzzle, offset, laser)?;
        }
        CannonStep::Break => {
            for bone in [LEFT_CANNON_BONE, RIGHT_CANNON_BONE] {
                crate::scene_stage::onett::spawn_on_bone(state, SHIP_MAP, bone, (BREAK_EFFECT, COMMON_BANK), draws)?;
            }
            state.quakes.request(
                &mut state.camera,
                melee_cm::QuakeKind::Large,
                &mut state.effects.gobjs,
            );
            let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
            model.set_joint_hidden(INTACT_BONE, true);
            model.set_joint_hidden(BROKEN_BONE, false);
            state.map.joint_list_remove(CANNON_JOINT);
        }
    }
    Ok(())
}

/// `lb_8000B1CC(gobj->hsd_obj, &offset, &out)`: a point in the Great Fox
/// GObj's frame (the map-scale wrapper).
fn gobj_point(model: &mut melee_gr::last::animation::BackgroundAnimation, offset: Vec3) -> Vec3 {
    let matrix = model.gobj_matrix();
    let mut out = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(&matrix, &offset, &mut out);
    out
}

/// grCorneria_801DCE1C (0x801DCE1C): the spawner; a pass may be a far one
/// when the Great Fox and 300 to its left are both over the sea or the
/// coast (grCn_CheckFar, grCorneria_801E0A74).
fn spawner(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let scale = state.assets.stage_desc.parameters.map_scale;
    let SceneStage::Corneria(corneria) = &mut state.stage else {
        unreachable!()
    };
    let stage = &mut corneria.stage;
    let live = stage.flight.is_some_and(|f| f.path);
    let backgrounds = stage.backgrounds;
    let x = stage.great_fox.translation().x;
    let far_allowed = || {
        let over_open_water = |x: f32| {
            let map = under(backgrounds, scale, x)
                .expect("grcorneria.c:2507: grCorneria_801E0A74, the Great Fox over no background");
            matches!(scenery(map), Scenery::Sea | Scenery::Coast)
        };
        over_open_water(x) && over_open_water(x - 300.0)
    };
    let parameters = stage.parameters;
    if let Some(slot) = stage.spawner.tick(&parameters, live, far_allowed, &mut state.rng) {
        passes::create(state, slot, world, objects)?;
    }
    Ok(())
}

/// it_2E6A_UnkMotion19_Phys's JObjs: the cannons' items stand on bones 2
/// and 3 as the ship left them.
fn follow_cannons(state: &mut InitialState) {
    let items = corneria(state).stage.cannon_items;
    let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
    for (id, bone) in items.into_iter().zip([LEFT_CANNON_BONE, RIGHT_CANNON_BONE]) {
        let Some(id) = id else { continue };
        let anchor = anchor(model, bone);
        if let Some(item) = state.items.get_mut(id) {
            it_yaku::set_anchor(item, anchor);
        }
    }
}

/// What the shots read of the Great Fox as it now stands: each charging
/// shot's muzzle (itGreatfoxlaser_UnkMotion1_Anim's lb_8000B1CC on x20's
/// JObj at x28), and the drift an aimed ship's laser follows
/// (itArwinglaser_UnkMotion3_Phys's grCorneria_801DDCF0).
pub(crate) fn follow_lasers(state: &mut InitialState) {
    let drift = corneria(state).stage.great_fox.drift_step();
    let model = state.stage_animations.get_mut(&SHIP_MAP).unwrap();
    for item in state.items.iter_mut() {
        if item.destroyed {
            continue;
        }
        match item.kind {
            ItemKind::GreatFoxLaser => {
                let muzzle = gobj_point(model, it_greatfoxlaser::offset(item));
                it_greatfoxlaser::follow(item, muzzle);
            }
            ItemKind::ArwingLaser if matches!(it_arwinglaser::laser_state(item).kind, 1 | 3) => {
                it_arwinglaser::follow_stage(item, drift);
            }
            _ => {}
        }
    }
}

/// fn_801E12D4 (0x801E12D4), a cannon's item hit: the beam damage counts
/// (xCA0 as a float); past x38 both items stop taking hits and the next
/// step breaks the cannons.
pub(crate) fn material_hit(state: &mut InitialState, item: u32, damage: i32) {
    let SceneStage::Corneria(corneria) = &mut state.stage else {
        unreachable!()
    };
    let stage = &mut corneria.stage;
    if !stage.cannon_items.contains(&Some(item)) {
        return;
    }
    let parameters = stage.parameters;
    if stage.cannon.take_damage(&parameters, damage as f32) {
        for id in stage.cannon_items.into_iter().flatten() {
            if let Some(item) = state.items.get_mut(id) {
                it_yaku::set_hurt_enabled(item, false);
            }
        }
    }
}
