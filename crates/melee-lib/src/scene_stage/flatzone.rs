//! Flat Zone composition: the engine half of the grflatzone.c procs (map
//! models, collision joints, the oil spill, the tools) and the stage's
//! setup, in grFlatzone_80216E78's creation order.
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    flatzone::{procs::MAP_ORDER, FlatZone, FlatZoneEngine, SpillHandle, SPILL_MATERIAL},
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// `mpCheckMultiple`'s and the tools' `joint_id_only`: collision joint 50,
/// the ground floor.
pub(crate) const GROUND_JOINT: i32 = 0x32;
/// mpCheckMultiple's checks: the floor only.
const FLOOR_CHECK: u32 = 1;

/// Ground_GetStageGObj for `map`: the archive model under the map-scale
/// wrapper; map 8 is past the archive's eight models and gets an empty JObj.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let scale = assets.stage_desc.parameters.map_scale;
    if usize::from(map) >= assets.stage_desc.models.len() {
        return BackgroundAnimation::identity_root(&assets.stage, scale)
            .map_err(|e| anyhow::anyhow!("Flat Zone map {map}: {e}"));
    }
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Flat Zone map {map}: {e}"))?;
    animation.set_map_scale(scale);
    Ok(animation)
}

/// The archive's collision bindings of `map` (Flat Zone's StageData lists
/// none of its own).
pub(crate) fn bindings(assets: &Assets, map: u8) -> &[melee_gr::desc::JointMapping] {
    assets
        .stage_desc
        .models
        .get(usize::from(map))
        .map_or(&[], |model| &model.joint_mappings[..])
}

/// Ground_801C2ED0 (0x801C2ED0): bind each collision joint to its JObj,
/// transform it (mpLib_80055E9C) and take that as its previous position
/// (mpLib_80057424).
pub(crate) fn bind(model: &mut BackgroundAnimation, assets: &Assets, map: u8, coll: &mut melee_mp::CollMap) {
    for binding in bindings(assets, map) {
        model.update_collision(coll, std::slice::from_ref(binding));
        coll.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// grFlatzone_80216E78 (0x80216E78): the cold setup. `stale_event` is the
/// controller's uncleared xC5.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    coll: &mut melee_mp::CollMap,
    stale_event: u8,
) -> Result<(SceneStage, Animations)> {
    let parameters = melee_gr::desc::read_flatzone_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut animations = Animations::new();
    let mut stage = None;
    for key in MAP_ORDER {
        let mut model = load(assets, key)?;
        match key {
            // grFlatzone_80217030, grFlatzone_802173D4: grAnime_801C8138
            // with animation 0 (maps 0 and 1 have none).
            0 | 1 => {
                ensure!(
                    model.evaluate_initial_frame::<RetailTrig>().is_empty(),
                    "Flat Zone map {key}: unexpected setup particle key"
                );
            }
            // grFlatzone_8021707C: bind (no joints), z 1.0, animation 0;
            // then the first event wait.
            2 => {
                bind(&mut model, assets, key, coll);
                model.set_gobj_translate(Vec3::new(0.0, 0.0, 1.0));
                ensure!(
                    model.evaluate_initial_frame::<RetailTrig>().is_empty(),
                    "Flat Zone map 2: unexpected setup particle key"
                );
                stage = Some(FlatZone::create(parameters.clone(), stale_event, rng));
            }
            // grFlatzone_80217648, 80217CD4, 80217450, 80217DA8,
            // 80217E90: bind, hide; no animation is attached.
            _ => {
                model.clear_animation();
                bind(&mut model, assets, key, coll);
                model.set_hidden(true);
            }
        }
        animations.insert(key, model);
    }
    let mut stage = stage.expect("map 2 created");
    // grFlatzone_80218060(1): layout 2, variant 1.
    let mut engine = Engine::new(&mut animations, assets, coll);
    stage.first_layout(&mut engine, rng);
    ensure!(engine.tool.is_none(), "Flat Zone setup spawned a tool");
    Ok((SceneStage::FlatZone(Box::new(stage)), animations))
}

/// The engine operations over the scene's models and collision map.
struct Engine<'a> {
    animations: &'a mut Animations,
    assets: &'a Assets,
    coll: &'a mut melee_mp::CollMap,
    /// The proc's it_802EEFA8 call (one at most), made once it returns.
    tool: Option<(Vec3, i32, f32)>,
}

impl<'a> Engine<'a> {
    fn new(animations: &'a mut Animations, assets: &'a Assets, coll: &'a mut melee_mp::CollMap) -> Self {
        Self {
            animations,
            assets,
            coll,
            tool: None,
        }
    }
    fn model(&mut self, map: u8) -> &mut BackgroundAnimation {
        self.animations.get_mut(&map).expect("Flat Zone model")
    }
}

impl FlatZoneEngine for Engine<'_> {
    fn play(&mut self, map: u8, animation: usize) {
        let assets = self.assets;
        let model = self.animations.get_mut(&map).expect("Flat Zone model");
        model.reset_pose();
        model
            .select_animation(&assets.stage, &assets.stage_desc.models[usize::from(map)], animation)
            .expect("Flat Zone animation");
        assert!(
            model.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "grAnime_801C8138: Flat Zone map {map} animation {animation} has frame-zero particle keys"
        );
    }
    fn remove_animation(&mut self, map: u8) {
        self.model(map).clear_animation();
    }
    fn hide(&mut self, map: u8) {
        self.model(map).set_hidden(true);
    }
    fn translation(&mut self, map: u8) -> Vec3 {
        self.model(map).gobj_translation()
    }
    fn set_translation(&mut self, map: u8, translation: Vec3) {
        self.model(map).set_gobj_translate(translation);
    }
    fn animation_ended(&mut self, map: u8) -> bool {
        self.model(map).ended()
    }
    fn find_floor(&mut self, x: f32, from_y: f32, to_y: f32) -> Option<(Vec3, i32)> {
        self.coll
            .check_multiple(x, from_y, x, to_y, FLOOR_CHECK, -1, GROUND_JOINT)
            .map(|hit| (hit.pos, hit.line_id))
    }
    fn spill(&mut self, position: Vec3, line: i32, radius: f32, frames: i32) -> Option<SpillHandle> {
        self.coll
            .dynamic_attrs
            .create(SPILL_MATERIAL, position, line, radius, frames)
    }
    fn release_spill(&mut self, handle: Option<SpillHandle>) {
        self.coll.dynamic_attrs.release(handle);
    }
    fn stitch(&mut self) {
        self.coll.stitch_all_joints();
    }
    fn blast_top(&self) -> f32 {
        self.assets.stage_camera.blast_top()
    }
    fn spawn_tool(&mut self, position: Vec3, kind: i32, facing: f32) {
        assert!(self.tool.is_none(), "grFlatzone_80217EF0 spawns one tool a frame");
        self.tool = Some((position, kind, facing));
    }
}

/// Ground_801C2FE0 for `map`: its collision joints follow their JObjs.
fn follow(state: &mut InitialState, map: u8) {
    let bindings = bindings(&state.assets, map);
    if bindings.is_empty() {
        return;
    }
    state
        .stage_animations
        .get_mut(&map)
        .unwrap()
        .update_collision(&mut state.map, bindings);
}

/// A Flat Zone gobj proc (s_link 4). Map 2's lb_800115F4 runs in the scene
/// dispatch.
pub(crate) fn run_proc(
    state: &mut InitialState,
    key: u8,
    world: &mut hsd_gobj::TaggedWorld,
    item_objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let SceneStage::FlatZone(stage) = &mut state.stage else {
        unreachable!()
    };
    let assets = std::sync::Arc::clone(&state.assets);
    let mut engine = Engine::new(&mut state.stage_animations, &assets, &mut state.map);
    match key {
        melee_gr::flatzone::CONTROLLER => stage.tick_controller(&mut engine, &mut state.rng),
        melee_gr::flatzone::OIL => stage.tick_oil(&mut engine, &mut state.rng),
        melee_gr::flatzone::PLATFORMS => stage.tick_platforms(&mut engine, &mut state.rng),
        melee_gr::flatzone::TOOLS => stage.tick_tools(&mut engine, &mut state.rng),
        4 | 6 => {
            // grFlatzone_80217D30 / 80217E04: set only by the controller's
            // cases 1 and 2.
            let index = usize::from(key == 6);
            assert!(
                !stage.unused_triggers[index],
                "grFlatzone_80217D30: map {key} triggered"
            );
        }
        _ => {}
    }
    let tool = engine.tool.take();
    follow(state, key);
    if key == melee_gr::flatzone::PLATFORMS {
        let SceneStage::FlatZone(stage) = &mut state.stage else {
            unreachable!()
        };
        let mut engine = Engine::new(&mut state.stage_animations, &assets, &mut state.map);
        stage.stitch_platforms(&mut engine);
    }
    if let Some((position, kind, facing)) = tool {
        crate::scene_items::spawn_tool(state, world, item_objects, position, kind, facing);
    }
    Ok(())
}
