//! Poké Floats composition (grpura.c): maps 0, 1 (the sky) and 4 (the
//! sequence) in grPura_80211D00's order, the sequence's on_init making its
//! camera subjects and every Pokémon (grPura_802125F0); each tick the
//! Pokémon follow the sequence's bones, the sequence moves the collision
//! and the vertices the animated Pokémon carry.
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
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    pura::{
        procs::{model, FLOATER_KEYS, SEQUENCE_MAP, SKY_MAP},
        Pura, Sky, BOUNDED_JOINTS, FLOATERS, REFRESHED_JOINT, SUBJECTS, SUBJECT_EXTENT,
        VERTICES,
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// Ground_GetStageGObj for a scheduler key: its model under the map-scale
/// wrapper.
pub(crate) fn load(assets: &Assets, key: u8) -> Result<BackgroundAnimation> {
    let map = model(key);
    let desc = &assets.stage_desc.models[usize::from(map)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, desc)
        .map_err(|e| anyhow::anyhow!("Poké Floats map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// The sequence's collision joints (GrPu.dat's map 4 bindings).
fn sequence_bindings(assets: &Assets) -> &[melee_gr::desc::JointMapping] {
    &assets.stage_desc.models[usize::from(SEQUENCE_MAP)].joint_mappings
}

/// Ground_801C33C0(4, joint) (0x801C33C0): the sequence's bone bound to
/// collision joint `joint` (grPu_StageData has no joints of its own).
fn sequence_bone(assets: &Assets, joint: i32) -> Result<usize> {
    let binding = sequence_bindings(assets)
        .iter()
        .rev()
        .find(|b| i32::from(b.joint_index) == joint);
    let binding = binding.ok_or_else(|| anyhow::anyhow!("Poké Floats: no bone for joint {joint}"))?;
    Ok(binding.extra as usize)
}

/// grAnime_801C8138(gobj, map, 0): frame zero, with the particles its keys
/// ask for.
fn evaluate(
    animation: &mut BackgroundAnimation,
    key: u8,
    assets: &Assets,
    particles: &mut ParticleSystem,
    rng: &mut HsdRng,
) -> Result<()> {
    for event in animation.evaluate_initial_frame::<RetailTrig>().to_vec() {
        let mut request = SpawnRequest::new(event.bank, event.kind, 0);
        request.joint = Some((joint_id(key, event.joint), event.matrix));
        particles.spawn::<RetailTrig>(
            assets.stage_key_bank(event.bank),
            request,
            rng,
            &mut DrawLog::default(),
        )?;
    }
    Ok(())
}

/// Ground_801C2ED0 (0x801C2ED0) on the sequence from its pose before any
/// animation: each joint transformed and taken as its previous position.
pub(crate) fn bind_sequence(
    animation: &mut BackgroundAnimation,
    assets: &Assets,
    collision: &mut melee_mp::CollMap,
) {
    let bindings = sequence_bindings(assets);
    animation.update_collision(collision, bindings);
    for binding in bindings {
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// grPura_80212CD4 (0x80212CD4): a camera subject on each of the first 25
/// bones under the sequence's root (Camera_80029020, then its position
/// and extents), active while the bone is shown.
fn create_subjects(sequence: &mut BackgroundAnimation) -> Result<([melee_cm::Subject; SUBJECTS], [usize; SUBJECTS])> {
    let bones = sequence.child_bones(0);
    ensure!(bones.len() >= SUBJECTS, "Poké Floats: {} sequence bones", bones.len());
    let subject_bones: [usize; SUBJECTS] = std::array::from_fn(|i| bones[i]);
    let subjects = std::array::from_fn(|i| {
        let mut subject = melee_cm::Subject::new(melee_cm::SubjectState::Active);
        place_subject(&mut subject, sequence, subject_bones[i]);
        subject.target_extents.top = SUBJECT_EXTENT;
        subject.target_extents.bottom = -SUBJECT_EXTENT;
        subject.target_extents.left = -SUBJECT_EXTENT;
        subject.target_extents.right = SUBJECT_EXTENT;
        subject.extents = subject.target_extents;
        subject
    });
    Ok((subjects, subject_bones))
}

/// grPura_80212EF4's body: the subject at its bone, active while shown.
fn place_subject(subject: &mut melee_cm::Subject, sequence: &mut BackgroundAnimation, bone: usize) {
    subject.position = sequence.joint_world_translation(bone);
    subject.state = if sequence.joint_hidden(bone) {
        melee_cm::SubjectState::Inactive
    } else {
        melee_cm::SubjectState::Active
    };
}

/// grPura_802125F0's placement of one Pokémon on its bone: the GObj's JObj
/// at the bone's translation (hidden with it), its model's root at the
/// origin, both scaled by the bone's x scale (times the table's 1.0 below
/// 2.0).
fn place_floater(floater: &mut BackgroundAnimation, sequence: &mut BackgroundAnimation, bone: usize) {
    floater.set_gobj_translate(sequence.joint_translation(bone));
    if sequence.joint_hidden(bone) {
        floater.set_hidden(true);
    }
    floater.set_joint_translate(0, Vec3::ZERO);
    // `if (scale < 2.0f) scale *= desc->x4;` with every x4 1.0.
    let scale = sequence.joint_scale(bone).x;
    let uniform = Vec3::new(scale, scale, scale);
    sequence.set_joint_scale(bone, uniform);
    floater.set_gobj_scale(uniform);
}

/// grPura_80211D00 (0x80211D00): maps 0, 1 and 4, the sequence's on_init
/// creating the subjects and the Pokémon.
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let mut animations = Animations::new();
    // stageGObj0_OnInit: animation 0.
    let mut background = load(assets, 0)?;
    evaluate(&mut background, 0, assets, particles, rng)?;
    animations.insert(0, background);
    // stageGObj1_OnInit: animation 0, then the sky's colours.
    let mut sky_model = load(assets, SKY_MAP)?;
    evaluate(&mut sky_model, SKY_MAP, assets, particles, rng)?;
    animations.insert(SKY_MAP, sky_model);
    let sky = Sky::new(rng);
    // stageGObj4_OnInit: Ground_801C2ED0 and animation 0 (Ground_JObjInline1).
    let mut sequence = load(assets, SEQUENCE_MAP)?;
    bind_sequence(&mut sequence, assets, collision);
    evaluate(&mut sequence, SEQUENCE_MAP, assets, particles, rng)?;
    let (subjects, subject_bones) = create_subjects(&mut sequence)?;
    // grPura_802125F0: each Pokémon's on_init (stageGObj2_OnInit: its
    // animation), then its placement.
    for (&(_, joint), &key) in FLOATERS.iter().zip(&FLOATER_KEYS) {
        let mut floater = load(assets, key)?;
        evaluate(&mut floater, key, assets, particles, rng)?;
        place_floater(&mut floater, &mut sequence, sequence_bone(assets, joint)?);
        animations.insert(key, floater);
    }
    animations.insert(SEQUENCE_MAP, sequence);
    let stage = Pura {
        sky,
        subjects,
        subject_bones,
    };
    Ok((SceneStage::Pura(Box::new(stage)), animations))
}

/// A Poké Floats gobj proc (s_link 4). Map 0's is empty; map 4's
/// lb_800115F4 runs in the scene's dispatch.
pub(crate) fn run_proc(state: &mut InitialState, key: u8) -> Result<()> {
    match key {
        0 => {}
        SKY_MAP => {
            let SceneStage::Pura(stage) = &mut state.stage else {
                unreachable!()
            };
            stage.sky.step(&mut state.rng);
        }
        SEQUENCE_MAP => step_sequence(state),
        _ => follow_bone(state, key)?,
    }
    Ok(())
}

/// stageGObj2_GObjProc (0x8021231C): the Pokémon's GObj JObj takes its
/// bone's translation, rotation and scale, and its hidden flag.
fn follow_bone(state: &mut InitialState, key: u8) -> Result<()> {
    let index = FLOATER_KEYS.iter().position(|&k| k == key);
    ensure!(index.is_some(), "Poké Floats: no Pokémon under key {key}");
    let joint = FLOATERS[index.unwrap()].1;
    let bone = sequence_bone(&state.assets, joint)?;
    let sequence = state.stage_animations.get(&SEQUENCE_MAP).unwrap();
    let translation = sequence.joint_translation(bone);
    let rotation = sequence.joint_rotation_quat(bone);
    let scale = sequence.joint_scale(bone);
    let bone_hidden = sequence.joint_hidden(bone);
    let floater = state.stage_animations.get_mut(&key).unwrap();
    floater.set_gobj_translate(translation);
    floater.set_gobj_rotation_quat(&rotation);
    floater.set_gobj_scale(scale);
    if bone_hidden && !floater.hidden() {
        floater.set_hidden(true);
    } else if !bone_hidden && floater.hidden() {
        floater.set_hidden(false);
    }
    Ok(())
}

/// stageGObj4_GObjProc (0x80211FD8): the subjects (grPura_80212EF4), the
/// collision joints (Ground_801C2FE0), the vertices (grPura_80213030) and
/// mpLib_80055E24(0x18).
fn step_sequence(state: &mut InitialState) {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::Pura(stage) = &mut state.stage else {
        unreachable!()
    };
    let sequence = state.stage_animations.get_mut(&SEQUENCE_MAP).unwrap();
    for (subject, &bone) in stage.subjects.iter_mut().zip(&stage.subject_bones) {
        place_subject(subject, sequence, bone);
    }
    sequence.update_collision(&mut state.map, sequence_bindings(&assets));
    for &(vertex, map, bone) in &VERTICES {
        let position = state
            .stage_animations
            .get_mut(&map)
            .unwrap()
            .joint_world_translation(bone);
        state.map.vtx_set_pos(vertex, position.x, position.y);
    }
    for joint in BOUNDED_JOINTS {
        state.map.joint_update_bounding(joint);
    }
    state.map.joint_update_dynamics_and_island(REFRESHED_JOINT);
}
