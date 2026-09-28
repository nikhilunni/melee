//! Pokemon Stadium's forms as Ground maps: creation (`grStadium_801D10F8`
//! and each form's init), destruction (`Ground_801C4A08` and each form's
//! callback3), their collision (`Ground_801C2ED0`, `Ground_801C2FE0`) and
//! the engine operations the transformation controller issues.
use super::spawn_ground_effect;
use crate::{initial_state::stage::joint_id, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_particle::{rng_sites::DrawLog, system::SpawnRequest};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    desc::{JointMapping, ModelDesc},
    last::animation::BackgroundAnimation,
    stadium::{procs, Form, ScreenMode, StadiumEngine},
};

/// Particles the forms create (bank 30).
const FIRE_FLAME: u32 = 0x753F;
const FIRE_EMBER: u32 = 0x7549;
const ROCK_DUST: u32 = 0x7546;
const WATER_SPRAY: u32 = 0x7544;
const SPARKLE: u32 = 0x7548;
/// grStadium_801D1840: the fire form's flames, by Ground_801C3FA4 index.
const FIRE_GENERATORS: [(usize, u32); 7] = [
    (0x13, FIRE_FLAME),
    (0x14, FIRE_EMBER),
    (0x15, FIRE_EMBER),
    (0x16, FIRE_FLAME),
    (0x17, FIRE_FLAME),
    (0x18, FIRE_EMBER),
    (0x19, FIRE_FLAME),
];
/// grStadium_801D19F8: the subtree whose generators die with the fire form.
const FIRE_GENERATOR_ROOT: usize = 0x12;
/// grStadium_801D1A38: the water form's windmill (Ground_801C3FA4 index),
/// which carries collision joint 0 (GrPs3.dat's own binding).
const WINDMILL: usize = 7;
const WINDMILL_JOINT: i32 = 0;
/// grStadium_801D1B48: MTXDegToRad(-0.5F) per tick (sdata2 @436).
const WINDMILL_TURN: f32 = -0.008_726_646;
/// The water form's wheel maps (grStadium_801D1DE4).
const WATER_WHEELS: [u8; 2] = [7, 8];

/// `grDatFiles_801C6330` (0x801C6330): the first registered archive whose
/// map_head has the map, the stage's own first.
fn model_source(
    assets: &crate::assets::Assets,
    archive: Option<Form>,
    map: u8,
) -> Option<(&hsd_archive::Archive, &ModelDesc)> {
    let own = &assets.stage_desc.models[usize::from(map)];
    if own.present {
        return Some((&assets.stage, own));
    }
    let form = &assets.stage_forms[archive?.archive()?];
    let model = form.models.get(usize::from(map))?;
    model.present.then_some((&form.archive, model))
}

/// The stage table's joints for the map, and the archive's own bindings.
fn bindings(
    state: &InitialState,
    archive: Option<Form>,
    map: u8,
) -> (Vec<JointMapping>, Vec<JointMapping>) {
    let stage = procs::stage_joints(map).cloned().collect();
    let own = model_source(&state.assets, archive, map)
        .map(|(_, model)| model.joint_mappings.clone())
        .unwrap_or_default();
    (stage, own)
}

fn stadium(state: &mut InitialState) -> &mut melee_gr::stadium::Stadium {
    let SceneStage::Stadium(stage) = &mut state.stage else {
        unreachable!()
    };
    stage
}

/// `Ground_801C2FE0` (0x801C2FE0): the stage joints bound to the map, then
/// the archive's own bindings not already updated this call.
/// Runs every tick, so it borrows the bindings in place rather than
/// collecting them as [`bindings`] does.
pub(super) fn update_collision(state: &mut InitialState, archive: Option<Form>, map: u8) {
    let own = model_source(&state.assets, archive, map)
        .map_or(&[][..], |(_, model)| model.joint_mappings.as_slice());
    let animation = state.stage_animations.get_mut(&map).expect("live map");
    for binding in procs::stage_joints(map) {
        animation.update_collision(&mut state.map, std::slice::from_ref(binding));
    }
    for binding in own {
        if !procs::stage_joints(map).any(|s| s.joint_index == binding.joint_index) {
            animation.update_collision(&mut state.map, std::slice::from_ref(binding));
        }
    }
}

/// The operations of `grStadium_801D4548` on the live scene.
pub(super) struct Engine<'a> {
    pub state: &'a mut InitialState,
    pub draws: &'a mut DrawLog,
    pub world: &'a mut hsd_gobj::TaggedWorld,
    pub objects: &'a mut [Option<hsd_gobj::GObjId>],
    /// The controller's registered archive this tick.
    pub archive: Option<Form>,
    /// This tick's external read completion, and where its use is noted.
    pub stage_read: crate::StageRead,
    pub consumed: &'a mut crate::ConsumedEvents,
    pub error: Option<anyhow::Error>,
}
impl Engine<'_> {
    fn attempt(&mut self, result: Result<()>) {
        if let Err(error) = result {
            self.error.get_or_insert(error);
        }
    }
    fn animation(&mut self, form: Form) -> &mut BackgroundAnimation {
        self.state
            .stage_animations
            .get_mut(&form.map())
            .expect("live form model")
    }
}
impl StadiumEngine for Engine<'_> {
    fn rng(&mut self) -> &mut HsdRng {
        &mut self.state.rng
    }
    fn read_completed(&mut self, form: Form, poll: u32) -> bool {
        let completed = match self.stage_read {
            crate::StageRead::Default => {
                melee_gr::stadium::transform::default_read_completed(form, poll)
            }
            crate::StageRead::Completed => true,
            crate::StageRead::InFlight => false,
            crate::StageRead::Unrecorded => {
                self.error.get_or_insert(anyhow::anyhow!(
                    "grStadium_801D42B8: the recording lacks the form archive read's \
                     completion (re-record with the stage_io tick tracer)"
                ));
                false
            }
        };
        self.consumed.stage_read = Some(completed);
        completed
    }
    fn announce(&mut self, mode: ScreenMode) {
        super::show(self.state, mode);
    }
    fn warn_standing(&mut self, form: Form) {
        stadium(self.state).sinking[usize::from(form.map())] = true;
    }
    fn scale_y(&mut self, form: Form) -> f32 {
        self.animation(form).wrapper_scale_y()
    }
    fn set_scale_y(&mut self, form: Form, scale: f32) {
        self.animation(form).set_wrapper_scale_y(scale);
    }
    fn set_translate_y(&mut self, form: Form, translate: f32) {
        self.animation(form).set_wrapper_translate_y(translate);
    }
    fn freeze(&mut self, form: Form) {
        self.animation(form).freeze();
    }
    fn create(&mut self, form: Form) {
        let result = create(self, form.map());
        self.attempt(result);
    }
    fn destroy(&mut self, form: Form) {
        let result = destroy(self, form.map());
        self.attempt(result);
    }
    fn settle(&mut self, form: Form) {
        stadium(self.state).settled[usize::from(form.map())] = true;
    }
    fn enable_line(&mut self, line: i32) {
        self.state.map.line_enable(line);
    }
    fn disable_line(&mut self, line: i32) {
        self.state.map.line_disable(line);
    }
    fn stitch_all(&mut self) {
        self.state.map.stitch_all_joints();
    }
    fn sparkle(&mut self, position: Vec3) {
        let result = spawn_ground_effect(self.state, SPARKLE, position, self.draws).map(|_| ());
        self.attempt(result);
    }
    fn quake(&mut self) {
        let state = &mut *self.state;
        state
            .quakes
            .request(&mut state.camera, melee_cm::QuakeKind::Loop);
    }
    fn map_scale(&self) -> f32 {
        self.state.assets.stage_desc.parameters.map_scale
    }
}

/// `grStadium_801D10F8` (0x801D10F8): Ground_GetStageGObj (the model under
/// its map-scale wrapper, Ground_801C1CD0 at s_link 1 and Ground_801C1D38
/// at s_link 4), the map's init, then its gobj proc at s_link 4.
fn create(engine: &mut Engine, map: u8) -> Result<()> {
    let assets = std::sync::Arc::clone(&engine.state.assets);
    let (archive, model) = model_source(&assets, engine.archive, map)
        .ok_or_else(|| anyhow::anyhow!("Pokemon Stadium map {map} has no model"))?;
    let mut animation =
        BackgroundAnimation::load_model(archive, model).map_err(|e| anyhow::anyhow!("{e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    for joint in 0..animation.joint_count() {
        ensure!(
            !engine
                .state
                .particles
                .has_joint_attachment(joint_id(map, joint)),
            "grStadium_801D10F8: map {map} recreated while generators hold its retired JObjs"
        );
    }
    let object = engine.world.create(3, 5, 0);
    let tag = super::super::last::TAG_BASE + usize::from(map) * 3;
    engine.world.add_tagged_proc(object, 1, tag);
    engine.world.add_tagged_proc(object, 4, tag + 1);
    engine.objects[usize::from(map)] = Some(object);
    engine.state.stage_animations.insert(map, animation);
    initialize(engine, map, model)?;
    engine.world.add_tagged_proc(object, 4, tag + 2);
    Ok(())
}

/// The map's init (grPs_StageCallbacks[map].on_init).
fn initialize(engine: &mut Engine, map: u8, model: &ModelDesc) -> Result<()> {
    if WATER_WHEELS.contains(&map) {
        // grStadium_801D1DE4: grAnime_801C8138 only.
        return evaluate_frame_zero(engine, map);
    }
    bind_collision(engine, map, model);
    list_released_joints(engine, map, model);
    let list = |engine: &mut Engine, joint: i32| engine.state.map.joint_list_add(joint);
    match map {
        5 => list(engine, 4),
        4 => list(engine, 3),
        6 => list(engine, 5),
        3 => {
            list(engine, 1);
            list(engine, 2);
        }
        9 => list(engine, 7),
        _ => unreachable!("Pokemon Stadium form map {map}"),
    }
    evaluate_frame_zero(engine, map)?;
    let stage = stadium(engine.state);
    stage.settled[usize::from(map)] = false;
    stage.sinking[usize::from(map)] = false;
    match map {
        6 => {
            // grStadium_801D1720: grLib_801C96F8(0x7546, 30, origin).
            let id = spawn_ground_effect(engine.state, ROCK_DUST, Vec3::ZERO, engine.draws)?;
            stadium(engine.state).generators[6] = id;
        }
        3 => {
            // grStadium_801D1840: grLib_801C9808 on seven flame joints.
            for (bone, kind) in FIRE_GENERATORS {
                let matrix = engine.animation(Form::Fire).joint_matrix(bone);
                let mut request = SpawnRequest::new(30, kind, 0);
                request.joint = Some((joint_id(3, bone), matrix));
                engine.state.effects.events.spawn(&request, false, false);
                engine.state.particles.spawn::<RetailTrig>(
                    &engine.state.assets.particle_bank,
                    request,
                    &mut engine.state.rng,
                    engine.draws,
                )?;
            }
        }
        9 => {
            // grStadium_801D1A38: the wheels, the windmill held back until
            // settled, the spray, and classical scaling for the squash.
            for wheel in WATER_WHEELS {
                create(engine, wheel)?;
            }
            engine.state.map.joint_list_remove(WINDMILL_JOINT);
            let id = spawn_ground_effect(engine.state, WATER_SPRAY, Vec3::ZERO, engine.draws)?;
            stadium(engine.state).generators[9] = id;
            engine
                .animation(Form::Water)
                .set_wrapper_flags_all(hsd_anim::jobj::JOBJ_CLASSICAL_SCALE);
        }
        _ => {}
    }
    Ok(())
}

/// `grAnime_801C8138(gobj, map, 0)`: frame zero's particle keys spawn now.
fn evaluate_frame_zero(engine: &mut Engine, map: u8) -> Result<()> {
    let state = &mut *engine.state;
    let animation = state.stage_animations.get_mut(&map).unwrap();
    for event in animation.evaluate_initial_frame::<RetailTrig>() {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(map, event.joint), event.matrix));
        state.effects.events.spawn(&request, false, false);
        state.particles.spawn::<RetailTrig>(
            &state.assets.particle_bank,
            request,
            &mut state.rng,
            engine.draws,
        )?;
    }
    Ok(())
}

/// `Ground_801C2ED0` (0x801C2ED0): bind (mpLib_800552B0), update
/// (mpLib_80055E9C) and snapshot (mpLib_80057424) the archive's own joints,
/// then the stage table's, before any animation is evaluated.
fn bind_collision(engine: &mut Engine, map: u8, model: &ModelDesc) {
    let state = &mut *engine.state;
    let animation = state.stage_animations.get_mut(&map).unwrap();
    for binding in model
        .joint_mappings
        .iter()
        .cloned()
        .chain(procs::stage_joints(map).cloned())
    {
        animation.update_collision(&mut state.map, std::slice::from_ref(&binding));
        state
            .map
            .joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// `Ground_801C3214` (0x801C3214): a map whose collision a destruction
/// released lists its joints again (Ground_801C3128: the stage table's,
/// then the archive's). `Ground_801C2ED0` calls it and each init repeats it.
fn list_released_joints(engine: &mut Engine, map: u8, model: &ModelDesc) {
    let released = &mut stadium(engine.state).released[usize::from(map)];
    if !std::mem::replace(released, false) {
        return;
    }
    for binding in procs::stage_joints(map)
        .cloned()
        .chain(model.joint_mappings.iter().cloned())
    {
        engine
            .state
            .map
            .joint_list_add(i32::from(binding.joint_index));
    }
}

/// `Ground_801C4A08` (0x801C4A08): the map's callback3, its collision
/// released (Ground_801C3128 with mpLib_80057BC0), then the GObj and its
/// procs removed. Generators the callback leaves keep their JObj's last
/// matrix (no hsd_8039D5DC).
fn destroy(engine: &mut Engine, map: u8) -> Result<()> {
    let joint = |engine: &mut Engine, joint: i32| engine.state.map.joint_list_remove(joint);
    match map {
        5 => joint(engine, 4),
        4 => joint(engine, 3),
        6 => {
            if let Some(id) = stadium(engine.state).generators[6].take() {
                engine.state.particles.expire_generator(id);
            }
            joint(engine, 5);
        }
        3 => {
            // grLib_801C9908 on the flame subtree: every attached generator
            // dies with its particles (type |= 0x80).
            let bones = engine.animation(Form::Fire).subtree(FIRE_GENERATOR_ROOT);
            for bone in bones {
                expire_attached(engine, joint_id(3, bone));
            }
            joint(engine, 1);
            joint(engine, 2);
        }
        9 => {
            if let Some(id) = stadium(engine.state).generators[9].take() {
                if let Some(generator) = engine.state.particles.generator_mut(id) {
                    generator.flags |= 0x80;
                }
                engine.state.particles.expire_generator(id);
            }
            for wheel in WATER_WHEELS {
                destroy(engine, wheel)?;
            }
            joint(engine, 7);
        }
        7 | 8 => {}
        _ => unreachable!("Pokemon Stadium form map {map}"),
    }
    let released = &mut stadium(engine.state).released[usize::from(map)];
    if !std::mem::replace(released, true) {
        let (stage, own) = bindings(engine.state, engine.archive, map);
        for binding in stage.iter().chain(&own) {
            joint(engine, i32::from(binding.joint_index));
        }
    }
    if let Some(object) = engine.objects[usize::from(map)].take() {
        engine.world.destroy(object);
    }
    engine.state.stage_animations.remove(&map);
    Ok(())
}

/// hsd_8039D4DC on each generator attached to the joint, type 0x80 set.
fn expire_attached(engine: &mut Engine, joint: usize) {
    let particles = &mut engine.state.particles;
    let ids: Vec<usize> = particles
        .generators
        .iter()
        .filter(|g| g.attachment_id == Some(joint))
        .map(|g| g.id)
        .collect();
    for id in ids {
        if let Some(generator) = particles.generator_mut(id) {
            generator.flags |= 0x80;
        }
        particles.expire_generator(id);
    }
}

/// `grStadium_801D1B48` (0x801D1B48), the water form's proc: the wheels
/// follow its squash, the windmill turns (801D1CB8 fadds), the windmill's
/// collision comes and goes with the settle and sink flags, then
/// Ground_801C2FE0 and the windmill's island hook (mpLib_8005667C).
pub(super) fn run_water(state: &mut InitialState, archive: Option<Form>) {
    let animations = &mut state.stage_animations;
    let scale = animations[&9].wrapper_scale_y();
    for wheel in WATER_WHEELS {
        if let Some(animation) = animations.get_mut(&wheel) {
            animation.set_wrapper_scale_y(scale);
        }
    }
    animations
        .get_mut(&9)
        .unwrap()
        .add_rotation_z(WINDMILL, WINDMILL_TURN);
    let stage = stadium(state);
    let settled = std::mem::replace(&mut stage.settled[9], false);
    let sinking = std::mem::replace(&mut stage.sinking[9], false);
    if settled {
        state.map.joint_list_add(WINDMILL_JOINT);
    }
    if sinking {
        state.map.joint_list_remove(WINDMILL_JOINT);
    }
    update_collision(state, archive, 9);
    state.map.joint_refresh_island(WINDMILL_JOINT);
}

/// hsd_8039D214 sets an attached JObj's matrix when its generator next
/// emits: publish the forms' current poses after the controller moved them.
pub(super) fn publish_forms(state: &mut InitialState) {
    for map in [3, 4, 5, 6, 7, 8, 9] {
        if let Some(animation) = state.stage_animations.get_mut(&map) {
            crate::scene_stage::publish_joint_matrices(
                animation,
                map,
                &mut state.particles,
                &mut state.effects.events,
            );
        }
    }
}
