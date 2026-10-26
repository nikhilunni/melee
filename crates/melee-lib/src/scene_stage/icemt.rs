//! Icicle Mountain composition (gricemt.c, Versus): the base (map 0), the
//! background (map 8), the scroll controller (map 9) and the two field
//! segments it holds out of maps 1..6. The segments' models are loaded at
//! setup and kept hidden until made, so the tick path allocates nothing.
use crate::{
    assets::Assets,
    initial_state::InitialState,
    scene_stage::SceneStage,
};
use anyhow::{ensure, Result};
use gekko_math::{fma::fmadds, HsdRng};
use hsd_particle::{rng_sites::DrawLog, system::ParticleSystem};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    desc::JointMapping,
    icemt::{
        procs::{
            segment, IceMtStage, SegmentState, BACKGROUND_MAP, BASE_MAP, COLLAPSES,
            CONTROLLER_MAP, RESPAWN_POINTS, SEGMENT_MAPS, STAGE_JOINTS,
        },
        spacing, Controller, Parameters,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grIceMt_801F8CDC's block capsule (grMaterial_801C8DE0).
const BLOCK_HURT: ([Vec3; 2], f32) = ([Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)], 6.0);
/// fn_801F8C64: efSync_Spawn(0x445) where a block breaks.
const BREAK_EFFECT: u16 = 0x445;
/// grIceMt_801F7080: grLib_801C96F8(0x7530, 30, origin), the falling snow.
const SNOW_PARTICLE: u32 = 0x7530;
/// grIceMt_801FA0BC (@263): the first segment stands this far below the
/// origin, times the map scale.
const FIRST_SEGMENT_DROP: f32 = -20.0;
/// grIceMt_801F9ACC: a segment's origin this close to the view's makes its
/// spawn points current.
const MARKER_RANGE: f32 = 10.0;
/// stageGObj8_GObjProc: the background scrolls at 0.3 of the field and
/// wraps every 3500 units (times the map scale).
const BACKGROUND_RATE: f32 = 0.3;
const BACKGROUND_PERIOD: f32 = 3500.0;
/// stageGObj8_GObjProc's bone (Ground_801C3FA4(gobj, 8)).
const BACKGROUND_BONE: usize = 8;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let scale = assets.stage_desc.parameters.map_scale;
    if usize::from(map) >= assets.stage_desc.models.len() {
        // Ground_GetStageGObj past the archive's models: an identity JObj.
        return BackgroundAnimation::identity_root(&assets.stage, scale)
            .map_err(|e| anyhow::anyhow!("Icicle Mountain map {map}: {e}"));
    }
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Icicle Mountain map {map}: {e}"))?;
    animation.set_map_scale(scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_icemt_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

fn table<'a>(map: u8) -> impl Iterator<Item = &'a JointMapping> {
    let joints: &'a [JointMapping] = &STAGE_JOINTS;
    joints
        .iter()
        .filter(move |b| b.target_index == i16::from(map))
}

fn archive(assets: &Assets, map: u8) -> &[JointMapping] {
    assets
        .stage_desc
        .models
        .get(usize::from(map))
        .map_or(&[][..], |m| &m.joint_mappings[..])
}

/// Ground_801C2FE0's and Ground_801C3128's order: grIm_803E40B0's joints,
/// then the archive's (no joint is in both).
fn bindings<'a>(assets: &'a Assets, map: u8) -> impl Iterator<Item = &'a JointMapping> + 'a {
    table(map).chain(archive(assets, map).iter())
}

/// Ground_801C2FE0 (0x801C2FE0) on `map`: mpLib_80055E9C on each joint.
fn update_collision(
    model: &mut BackgroundAnimation,
    assets: &Assets,
    map: u8,
    collision: &mut melee_mp::CollMap,
) {
    for binding in bindings(assets, map) {
        model.update_collision(collision, std::slice::from_ref(binding));
    }
}

/// Ground_801C32AC (0x801C32AC): mpLib_80057424 on each of `map`'s joints.
fn snapshot(assets: &Assets, map: u8, collision: &mut melee_mp::CollMap) {
    for binding in bindings(assets, map) {
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// Ground_801C3214 (0x801C3214): a map whose joints were disabled
/// (Ground_804D6950) has them enabled again.
fn enable(assets: &Assets, map: u8, released: &mut [bool; 7], collision: &mut melee_mp::CollMap) {
    if std::mem::take(&mut released[usize::from(map)]) {
        for binding in bindings(assets, map) {
            collision.joint_list_add(i32::from(binding.joint_index));
        }
    }
}

/// Ground_801C3260 / the tail of Ground_801C4A08: the first time, `map`'s
/// joints are disabled (mpLib_80057BC0) and the map marked.
fn release(assets: &Assets, map: u8, released: &mut [bool; 7], collision: &mut melee_mp::CollMap) {
    if !released[usize::from(map)] {
        released[usize::from(map)] = true;
        for binding in bindings(assets, map) {
            collision.joint_list_remove(i32::from(binding.joint_index));
        }
    }
}

/// What a segment's creation touches besides the stage itself.
struct Creation<'a> {
    assets: &'a Assets,
    animations: &'a mut Animations,
    collision: &'a mut melee_mp::CollMap,
    released: &'a mut [bool; 7],
}

impl Creation<'_> {
    /// stageGObj1..6_OnInit: Ground_801C2ED0 (the archive's joints, then
    /// grIm_803E40B0's: mpLib_800552B0, 80055E9C, 80057424; then
    /// Ground_801C3214), grAnime_801C8138(gobj, map, 0), grAnime_801C77FC
    /// (every animation loops), the spawn points' and blocks' bones, and on
    /// maps 2 and 4 the platforms that give way (grIceMt_801F91EC: their
    /// bones stopped, mpJointSetCb1 on their joints). The blocks' items
    /// follow once the item pool exists.
    fn on_init(&mut self, map: u8) -> Result<SegmentState> {
        let model = self.animations.get_mut(&map).unwrap();
        let description = &self.assets.stage_desc.models[usize::from(map)];
        model
            .recreate(&self.assets.stage, description)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        model.set_hidden(false);
        model.clear_animation();
        let assets = self.assets;
        for binding in archive(assets, map).iter().chain(table(map)) {
            model.update_collision(self.collision, std::slice::from_ref(binding));
            self.collision
                .joint_snapshot_prev_pos(i32::from(binding.joint_index));
        }
        enable(self.assets, map, self.released, self.collision);
        // grAnime_801C8138(gobj, map, 0): the pose reset, animation 0 when
        // the map has one, frame zero evaluated.
        model.reset_pose();
        if !description.animations.is_empty() {
            model
                .select_animation(&self.assets.stage, description, 0)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        ensure!(
            model.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "grAnime_801C8138: Icicle Mountain map {map} has frame-zero particle keys"
        );
        model.set_subtree_loops(0);
        let mut state = SegmentState::default();
        for (index, (_, site)) in COLLAPSES.iter().filter(|(m, _)| *m == map).enumerate() {
            // grIceMt_801F91EC: grAnime_801C7A04(gobj, bone, 7, 0) on each
            // bone, mpJointSetCb1(joint, gp, fn).
            let model = self.animations.get_mut(&map).unwrap();
            model.set_subtree_animation_rate(site.bones.0, 0.0);
            if let Some(bone) = site.bones.1 {
                model.set_subtree_animation_rate(bone, 0.0);
            }
            self.collision.record_joint_events(site.joint, true);
            state.collapses[index] = melee_gr::icemt::procs::Collapse::default();
        }
        Ok(state)
    }
}

/// Map 0's on_init (stageGObj0_OnInit): no animation; the respawn points'
/// translations.
fn init_base(model: &mut BackgroundAnimation) {
    model.reset_pose();
    model.clear_animation();
    for (bone, [x, y]) in RESPAWN_POINTS {
        model.set_joint_translate(bone, Vec3::new(x, y, 0.0));
    }
}

/// grIceMt_801F686C (0x801F686C), Versus: maps 0 and 8, Ground_801C3260
/// on maps 1..6, then map 9 (stageGObj9_OnInit: the first two segments'
/// draws, grIceMt_801FA0BC making them). The blocks' items are made once
/// the item pool exists ([`spawn_items`]).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    // stageGObj9_OnInit's draws come after maps 0 and 8 and the release of
    // maps 1..6, none of which draws.
    let controller = Controller::new(&parameters, rng);
    build(assets, parameters, controller, collision)
}

/// grIceMt_801F686C with map 9's controller as its draws left it: the
/// cold setup's and a saved boundary's shared construction.
pub(crate) fn build(
    assets: &Assets,
    parameters: Parameters,
    controller: Controller,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let mut animations = Animations::new();
    let mut base = load(assets, BASE_MAP)?;
    init_base(&mut base);
    animations.insert(BASE_MAP, base);
    // stageGObj8_OnInit: no animation, every animation loops,
    // Ground_801C4E70 (the sky's bounds), its speed zero.
    let mut background = load(assets, BACKGROUND_MAP)?;
    background.reset_pose();
    background.clear_animation();
    background.set_subtree_loops(0);
    animations.insert(BACKGROUND_MAP, background);
    load_reserved(assets, &mut animations)?;
    let mut released = [false; 7];
    for map in SEGMENT_MAPS {
        release(assets, map, &mut released, collision);
    }
    animations.insert(CONTROLLER_MAP, load(assets, CONTROLLER_MAP)?);
    let mut stage = stage(assets, parameters, controller)?;
    stage.released = released;
    place_first_segments(&mut stage, assets, &mut animations, collision)?;
    Ok((SceneStage::IceMt(Box::new(stage)), animations))
}

pub(crate) fn stage(assets: &Assets, parameters: Parameters, controller: Controller) -> Result<IceMtStage> {
    Ok(IceMtStage {
        parameters,
        setup_order: [
            BASE_MAP,
            BACKGROUND_MAP,
            CONTROLLER_MAP,
            controller.under as u8,
            controller.upper as u8,
        ],
        controller,
        segments: [None; 6],
        released: [false; 7],
        background_speed: 0.0,
        lights: melee_gr::battle::lights::load_model(
            &assets.stage,
            &assets.stage_desc,
            usize::from(BACKGROUND_MAP),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?,
    })
}

/// The segments' models, loaded once and hidden until made.
pub(crate) fn load_reserved(assets: &Assets, animations: &mut Animations) -> Result<()> {
    for map in SEGMENT_MAPS {
        let mut model = load(assets, map)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(map, model);
    }
    Ok(())
}

/// grIceMt_801FA0BC (0x801FA0BC): the under segment above the view
/// (`-20 * scale + spacing`, retail 0x801FA14C fmadds) and the upper one
/// in it (`-20 * scale`, 0x801FA224 fmuls), each made, placed, enabled
/// (Ground_801C3214), moved (Ground_801C2FE0) and snapshotted
/// (Ground_801C32AC); the upper one holds the view and gives the spawn
/// points (grIceMt_801F98A8); the respawn points are map 0's
/// (grIceMt_801FA854).
fn place_first_segments(
    stage: &mut IceMtStage,
    assets: &Assets,
    animations: &mut Animations,
    collision: &mut melee_mp::CollMap,
) -> Result<()> {
    let scale = assets.stage_desc.parameters.map_scale;
    let (under, upper) = (stage.controller.under as u8, stage.controller.upper as u8);
    let gap = spacing(&stage.parameters, scale, under, upper);
    for (map, y) in [
        (under, fmadds(FIRST_SEGMENT_DROP, scale, gap)),
        (upper, FIRST_SEGMENT_DROP * scale),
    ] {
        let mut creation = Creation {
            assets,
            animations,
            collision,
            released: &mut stage.released,
        };
        let state = creation.on_init(map)?;
        let model = animations.get_mut(&map).unwrap();
        model.set_gobj_translate_y(y);
        enable(assets, map, &mut stage.released, collision);
        update_collision(model, assets, map, collision);
        snapshot(assets, map, collision);
        stage.segments[usize::from(map) - 1] = Some(state);
    }
    let upper_state = stage.segments[usize::from(upper) - 1].as_mut().unwrap();
    upper_state.current = true;
    upper_state.markers_due = false;
    Ok(())
}

/// grMaterial_801C8CFC(8, 0, gp, bone, NULL, fn_801F8C64, NULL) for each
/// of a segment's blocks: its capsule (grMaterial_801C8DE0), hits on
/// (801C8E08) and grounded (801C8E68).
fn block_anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

fn arm_block(item: &mut melee_it::ItemCore) {
    let (offsets, radius) = BLOCK_HURT;
    it_yaku::set_hurt_capsule(item, offsets, radius);
    it_yaku::set_hurt_enabled(item, true);
    it_yaku::set_ground_or_air(item, melee_types::GroundOrAir::Ground);
}

/// The setup segments' blocks, under's then upper's, as their on_init
/// made them.
pub(crate) fn spawn_items(
    stage: &mut SceneStage,
    animations: &mut Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
) {
    let SceneStage::IceMt(stage) = stage else {
        return;
    };
    for segment_map in [stage.controller.under, stage.controller.upper] {
        let segment_map = segment_map as u8;
        let model = animations.get_mut(&segment_map).unwrap();
        let state = stage.segments[usize::from(segment_map) - 1].as_mut().unwrap();
        // The on_init made them before grIceMt_801FA0BC placed the segment:
        // they start where their bones stood then and follow them from
        // their first physics step.
        let placed = model.gobj_translation_y();
        model.set_gobj_translate_y(0.0);
        for (index, &bone) in segment(segment_map).block_bones.iter().enumerate() {
            let id = crate::scene_items::spawn_yaku(items, gobjs, resources, map, block_anchor(model, bone));
            arm_block(items.get_mut(id).unwrap());
            state.blocks[index] = Some(id);
        }
        model.set_gobj_translate_y(placed);
        for (index, &bone) in segment(segment_map).block_bones.iter().enumerate() {
            let item = items.get_mut(state.blocks[index].unwrap()).unwrap();
            it_yaku::set_anchor(item, block_anchor(model, bone));
        }
    }
}

/// grIceMt_801F7080 (0x801F7080), the stage's on_load after the fighters
/// exist: Ground_801C2FE0 on the live segments (maps 1..6 in order), then
/// the falling snow (grLib_801C96F8(0x7530, 30, origin)).
pub(crate) fn on_load(
    stage: &SceneStage,
    assets: &Assets,
    animations: &mut Animations,
    collision: &mut melee_mp::CollMap,
    particles: &mut ParticleSystem,
    events: &mut melee_ef::fixture_spawns::EventSink,
    rng: &mut HsdRng,
) -> Result<()> {
    let SceneStage::IceMt(stage) = stage else {
        return Ok(());
    };
    for map in SEGMENT_MAPS {
        if stage.segments[usize::from(map) - 1].is_some() {
            update_collision(animations.get_mut(&map).unwrap(), assets, map, collision);
        }
    }
    super::greatbay::ground_effect(
        assets,
        particles,
        events,
        rng,
        &mut DrawLog::default(),
        SNOW_PARTICLE,
        Vec3::ZERO,
    )?;
    Ok(())
}

/// fn_801F75EC (0x801F75EC), deferred by Ground_801C10B8 until the
/// countdown releases the stage: the controller starts scrolling.
pub(crate) fn start(state: &mut InitialState) {
    if let SceneStage::IceMt(stage) = &mut state.stage {
        stage.controller.waiting = false;
    }
}

/// An Icicle Mountain gobj proc (s_link 4).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    match map {
        BASE_MAP => Ok(()),
        BACKGROUND_MAP => {
            step_background(state);
            Ok(())
        }
        CONTROLLER_MAP => step_controller(state, world, objects, item_objects),
        1..=6 => {
            step_segment(state, map);
            Ok(())
        }
        _ => unreachable!("Icicle Mountain map {map}"),
    }
}

/// stageGObj8_GObjProc (0x801F8B10): bone 8 moves by `0.3 * speed` against
/// the field (retail 0x801F8B38 fmuls, 0x801F8B78 fsubs) and wraps half a
/// period either way (0x801F8BA0 fnmsubs, 0x801F8BD0 fmadds).
fn step_background(state: &mut InitialState) {
    let SceneStage::IceMt(stage) = &state.stage else {
        unreachable!()
    };
    let scale = state.assets.stage_desc.parameters.map_scale;
    let step = BACKGROUND_RATE * stage.background_speed;
    let model = state.stage_animations.get_mut(&BACKGROUND_MAP).unwrap();
    let mut y = model.joint_translation(BACKGROUND_BONE).y - step;
    if y > 0.5 * (BACKGROUND_PERIOD * scale) {
        y = gekko_math::fma::fnmsubs(BACKGROUND_PERIOD, scale, y);
    } else if y < 0.5 * -(BACKGROUND_PERIOD * scale) {
        y = fmadds(BACKGROUND_PERIOD, scale, y);
    }
    model.set_joint_translate_y(BACKGROUND_BONE, y);
}

/// stageGObj9_GObjProc (0x801F7728): once started, the scroll's speed
/// (grIceMt_801FA364), both segments moved down by it (grIceMt_801F96E0),
/// the segments swapped (grIceMt_801F9ACC) and the background's speed
/// (grIceMt_801F9668).
fn step_controller(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    if stage.controller.waiting {
        return Ok(());
    }
    let parameters = stage.parameters;
    let speed = stage.controller.step_scroll(&parameters, &mut state.rng);
    // grIceMt_801F96E0(ids, -speed): HSD_JObjAddTranslationY (fadds).
    for map in [stage.controller.under, stage.controller.upper] {
        if map != -1 {
            let model = state.stage_animations.get_mut(&(map as u8)).unwrap();
            let y = model.gobj_translation_y() + -speed;
            model.set_gobj_translate_y(y);
        }
    }
    swap_segments(state, world, objects, item_objects)?;
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    stage.background_speed = speed;
    anchor_blocks(state);
    Ok(())
}

/// The blocks follow their bones (it_2E6A_UnkMotion19_Phys reads them at
/// the items' physics, after every Ground proc this tick).
fn anchor_blocks(state: &mut InitialState) {
    let SceneStage::IceMt(stage) = &state.stage else {
        unreachable!()
    };
    for map in SEGMENT_MAPS {
        let Some(segment_state) = &stage.segments[usize::from(map) - 1] else {
            continue;
        };
        let model = state.stage_animations.get_mut(&map).unwrap();
        for (index, &bone) in segment(map).block_bones.iter().enumerate() {
            if let Some(id) = segment_state.blocks[index] {
                if let Some(item) = state.items.get_mut(id) {
                    it_yaku::set_anchor(item, block_anchor(model, bone));
                }
            }
        }
    }
}

/// grIceMt_801F9ACC (0x801F9ACC): the spawn points of the segment at the
/// view, which segment holds it, and a swap once the upper segment left
/// the view upward (`cur > 0`) or downward (`cur < -spacing`); then the
/// respawn points (grIceMt_801FA854, map 0's: unchanged).
fn swap_segments(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let scale = state.assets.stage_desc.parameters.map_scale;
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    let (under, upper) = (stage.controller.under, stage.controller.upper);
    if under == -1 || upper == -1 {
        return Ok(());
    }
    let (under, upper) = (under as u8, upper as u8);
    let y0 = spacing(&stage.parameters, scale, under, upper);
    let cur = state.stage_animations[&upper].gobj_translation_y();
    fn segment_mut(stage: &mut IceMtStage, map: u8) -> &mut SegmentState {
        stage.segments[usize::from(map) - 1].as_mut().unwrap()
    }
    if (if cur < 0.0 { -cur } else { cur }) < MARKER_RANGE {
        segment_mut(stage, upper).markers_due = true;
    } else {
        let above = cur + y0;
        if (if above < 0.0 { -above } else { above }) < MARKER_RANGE {
            segment_mut(stage, under).markers_due = true;
        }
    }
    let (enter, leave) = if cur < 0.5 * -y0 { (under, upper) } else { (upper, under) };
    let entering = segment_mut(stage, enter);
    if !entering.current {
        entering.current = true;
        entering.markers_due = true;
    }
    segment_mut(stage, leave).current = false;
    let parameters = stage.parameters;
    if cur > 0.0 {
        // The top segment goes; the view's moves up, a new one below.
        remove_segment(state, under, world, objects);
        let SceneStage::IceMt(stage) = &mut state.stage else {
            unreachable!()
        };
        stage.controller.under = i16::from(upper);
        let next = stage.controller.pick_segment(&parameters, &mut state.rng);
        stage.controller.upper = i16::from(next);
        let y1 = spacing(&parameters, scale, upper, next);
        create_segment(state, next, cur - y1, world, objects, item_objects)?;
        markers_now(state, upper);
    } else if cur < -y0 {
        // The view's segment goes; the top one takes it, a new one above.
        remove_segment(state, upper, world, objects);
        let SceneStage::IceMt(stage) = &mut state.stage else {
            unreachable!()
        };
        stage.controller.upper = i16::from(under);
        let next = stage.controller.pick_segment(&parameters, &mut state.rng);
        stage.controller.under = i16::from(next);
        let y1 = spacing(&parameters, scale, next, under);
        // retail 0x801F9F74 / 0x801F9F78: fadds, fadds.
        create_segment(state, next, y1 + (cur + y0), world, objects, item_objects)?;
        markers_now(state, under);
    }
    Ok(())
}

/// grIceMt_801F9ACC's inline grIceMt_801F98A8 on the segment that stays:
/// its spawn points go to Ground_801C2D0C at once.
fn markers_now(state: &mut InitialState, map: u8) {
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    if let Some(segment_state) = stage.segments[usize::from(map) - 1].as_mut() {
        segment_state.markers_due = false;
    }
}

/// setupStageCallbacks (0x801F71E8) during the match: Ground_GetStageGObj
/// (the GObj and its Ground_801C1CD0 / Ground_801C1D38 procs), the
/// segment's on_init (its blocks' items made there), its gobj proc; then
/// grIceMt_801F9ACC's placement: SetTranslateY, Ground_801C32AC,
/// Ground_801C2FE0, Ground_801C3214.
fn create_segment(
    state: &mut InitialState,
    map: u8,
    y: f32,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let object = world.create(3, 5, 0);
    state.stage_gobjs.create(&mut state.effects.gobjs, map);
    for (index, link) in [1, 4].into_iter().enumerate() {
        world.add_tagged_proc(object, link, super::last::TAG_BASE + usize::from(map) * 3 + index);
    }
    objects[usize::from(map)] = Some(object);
    let model = state.stage_animations.get(&map).unwrap();
    for joint in 0..model.joint_count() {
        ensure!(
            !state
                .particles
                .has_joint_attachment(crate::initial_state::stage::joint_id(map, joint)),
            "setupStageCallbacks: map {map} remade while generators hold its retired JObjs"
        );
    }
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    let mut creation = Creation {
        assets: &assets,
        animations: &mut state.stage_animations,
        collision: &mut state.map,
        released: &mut stage.released,
    };
    let mut segment_state = creation.on_init(map)?;
    let model = state.stage_animations.get_mut(&map).unwrap();
    for (index, &bone) in segment(map).block_bones.iter().enumerate() {
        let id = crate::scene_items::spawn_yaku_live(
            &mut state.items,
            &mut state.effects.gobjs,
            &assets.items,
            &mut state.map,
            world,
            item_objects,
            block_anchor(model, bone),
        );
        arm_block(state.items.get_mut(id).unwrap());
        segment_state.blocks[index] = Some(id);
    }
    world.add_tagged_proc(object, 4, super::last::TAG_BASE + usize::from(map) * 3 + 2);
    model.set_gobj_translate_y(y);
    snapshot(&assets, map, &mut state.map);
    update_collision(model, &assets, map, &mut state.map);
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    enable(&assets, map, &mut stage.released, &mut state.map);
    stage.segments[usize::from(map) - 1] = Some(segment_state);
    Ok(())
}

/// Ground_801C4A08 (0x801C4A08) on a segment: its callback3 frees its
/// blocks' items (grMaterial_801C8CDC -> Item_8026A8EC, at once), its
/// joints are disabled the first time (Ground_804D6950), the GObj freed.
fn remove_segment(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    let segment_state = stage.segments[usize::from(map) - 1]
        .take()
        .expect("a live segment");
    for id in segment_state.blocks.into_iter().flatten() {
        let item = state.items.get_mut(id).expect("a block's item");
        item.destroyed = true;
        state.particles.walk_unowned_joint();
        state.effects.gobjs.mark();
        state.items.note_destroyed();
    }
    release(&assets, map, &mut stage.released, &mut state.map);
    state.stage_gobjs.destroy(&mut state.effects.gobjs, map);
    if let Some(object) = objects[usize::from(map)].take() {
        world.destroy(object);
    }
    let model = state.stage_animations.get_mut(&map).unwrap();
    for joint in 0..model.joint_count() {
        let id = crate::initial_state::stage::joint_id(map, joint);
        if state.particles.has_joint_attachment(id) {
            let matrix = model.released_joint_matrix(joint);
            state.effects.events.update_joint(id, matrix);
            state.particles.update_joint(id, matrix);
        }
    }
    model.clear_animation();
    model.set_hidden(true);
}

/// stageGObj1..6_GObjProc: on maps 2 and 4 the platforms' countdowns
/// (grIceMt_801F929C), the spawn points (grIceMt_801F98A8), then
/// Ground_801C2FE0.
fn step_segment(state: &mut InitialState, map: u8) {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::IceMt(stage) = &mut state.stage else {
        unreachable!()
    };
    let segment_state = stage.segments[usize::from(map) - 1]
        .as_mut()
        .expect("a live segment's proc");
    let model = state.stage_animations.get_mut(&map).unwrap();
    for (index, (_, site)) in COLLAPSES.iter().filter(|(m, _)| *m == map).enumerate() {
        let collapse = &mut segment_state.collapses[index];
        if collapse.active {
            collapse.timer += 1;
            if collapse.timer == site.remove_at {
                state.map.joint_list_remove(site.joint);
            } else if collapse.timer == site.add_at {
                state.map.joint_list_add(site.joint);
            }
            // grAnime_801C83D0(gobj, bone, 7).
            if model.subtree_animation_ended(site.bones.0) {
                collapse.active = false;
            }
        }
    }
    segment_state.markers_due = false;
    update_collision(model, &assets, map, &mut state.map);
}

/// fn_801F9338 / fn_801F9448 / fn_801F9558, the platforms' floor
/// callbacks, for every call the proc that just ran made: a fighter
/// (x34 b1234 == 1) on a platform at rest sets it going: its bones play
/// (grAnime_801C7A04 rate 1, grAnime_801C7B24 frame 0) and stop looping
/// (grAnime_801C78FC), and its countdown starts.
pub(crate) fn apply_joint_events(state: &mut InitialState) {
    let SceneStage::IceMt(stage) = &mut state.stage else {
        return;
    };
    for (map, site) in COLLAPSES {
        let events = state.map.take_joint_events(site.joint);
        let Some(segment_state) = stage.segments[usize::from(map) - 1].as_mut() else {
            continue;
        };
        let index = COLLAPSES
            .iter()
            .filter(|(m, _)| *m == map)
            .position(|(_, s)| s.joint == site.joint)
            .unwrap();
        for event in events.iter() {
            let collapse = &mut segment_state.collapses[index];
            if event.collision_type == 1 && !collapse.active {
                collapse.active = true;
                collapse.timer = 0;
                let model = state.stage_animations.get_mut(&map).unwrap();
                for bone in std::iter::once(site.bones.0).chain(site.bones.1) {
                    model.set_subtree_animation_rate(bone, 1.0);
                    model.request_subtree_frame_all(bone, 0.0);
                    model.clear_subtree_loops_all(bone);
                }
            }
        }
    }
}

/// fn_801F8C64 (0x801F8C64), the blocks' hit-taken callback: hits off
/// (grMaterial_801C8E28), the block's bone hidden (its collision joint
/// hides at the next Ground_801C2FE0), efSync_Spawn(0x445) at the item, a
/// sound (Ground_801C53EC(310)) and a small quake.
pub(crate) fn material_hit(state: &mut InitialState, item: u32) -> Result<()> {
    let SceneStage::IceMt(stage) = &state.stage else {
        return Ok(());
    };
    let mut found = None;
    for map in SEGMENT_MAPS {
        if let Some(segment_state) = &stage.segments[usize::from(map) - 1] {
            if let Some(index) = segment_state.blocks.iter().position(|&b| b == Some(item)) {
                found = Some((map, segment(map).block_bones[index]));
            }
        }
    }
    let Some((map, bone)) = found else {
        return Ok(());
    };
    let block = state.items.get_mut(item).unwrap();
    it_yaku::set_hurt_enabled(block, false);
    let position = block.position;
    state
        .stage_animations
        .get_mut(&map)
        .unwrap()
        .set_joint_hidden(bone, true);
    state.effects.spawn_positional::<RetailTrig>(
        BREAK_EFFECT,
        position,
        &state.assets.common_particle_bank,
        &mut state.particles,
        &mut state.rng,
    )?;
    state.quakes.request(
        &mut state.camera,
        melee_cm::QuakeKind::Small,
        &mut state.effects.gobjs,
    );
    Ok(())
}

/// The respawn points (Ground_801C2D24(4..7): map 0's bones 18..21 at the
/// translations stageGObj0_OnInit gives them).
pub(crate) fn respawn_points(
    archive: &hsd_archive::Archive,
    stage: &melee_gr::desc::StageDesc,
) -> Result<[Vec3; 4]> {
    let mut base = BackgroundAnimation::load_model(archive, &stage.models[usize::from(BASE_MAP)])
        .map_err(|e| anyhow::anyhow!("Icicle Mountain map 0: {e}"))?;
    base.set_map_scale(stage.parameters.map_scale);
    init_base(&mut base);
    Ok(RESPAWN_POINTS.map(|(bone, _)| {
        let m = base.joint_matrix(bone);
        Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3])
    }))
}

/// The spawn points (Ground_801C2D24(0..3)) as the stage's creation left
/// them: the view's segment's.
pub(crate) fn spawn_point(stage: &SceneStage, animations: &mut Animations, slot: usize) -> Option<Vec3> {
    let SceneStage::IceMt(stage) = stage else {
        return None;
    };
    let map = stage.controller.upper as u8;
    let bone = segment(map).spawn_bones[slot];
    let m = animations.get_mut(&map).unwrap().joint_matrix(bone);
    Some(Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3]))
}
