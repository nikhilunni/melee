use gekko_math::rng::HsdRng;
use hsd_anim::{
    aobj::{AObj, AOBJ_NO_ANIM},
    jobj::{JObjTree, JointSpec, JOBJ_USE_QUATERNION},
    mtx::InverseTrig,
};
use hsd_archive::desc::{FigaTrack, FigaTree};
use hsd_types::Vec3;
use melee_ft::{
    anim::{
        attach::{attach_motion, MotionRemap, PartFlags},
        FighterAnimation, Motion, MotionFlags, PartAnimation,
    },
    desc::PartTable,
};
use melee_lb::anim::{attach_joint_tracks_without_translation, AttachError};

struct RetailTrig;
impl InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}
fn skeleton() -> (JObjTree, FighterAnimation) {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new().child(JointSpec::new().child(JointSpec::new().child(JointSpec::new()))),
    );
    let state = FighterAnimation::new(&tree, root);
    (tree, state)
}
fn track(ty: u8) -> FigaTrack {
    FigaTrack {
        obj_type: ty,
        ..FigaTrack::default()
    }
}
fn motion(blend_frames: f32) -> Motion {
    Motion {
        id: 2,
        flags: MotionFlags(0),
        blend_frames,
        remap: None,
        animation: FigaTree {
            frames: 3.0,
            nodes: vec![0, 1, 1, 0],
            tracks: vec![track(1), track(2)],
            ..FigaTree::default()
        },
    }
}

#[test]
fn masked_joints_consume_no_node_but_locked_joints_consume_all_tracks() {
    let (mut tree, mut state) = skeleton();
    state.parts[1].flags.0 |= PartFlags::CONDITIONAL;
    state.parts[1].motion_mask = 2;
    state.parts[2].flags.0 |= PartFlags::LOCKED;
    let animation = FigaTree {
        nodes: vec![1, 1, 1],
        tracks: vec![track(1), track(2), track(3)],
        ..FigaTree::default()
    };
    attach_motion(&mut tree, &state.parts, &animation, 0, None).unwrap();
    assert!(tree.get(state.parts[1].joint).aobj.is_none());
    assert!(tree.get(state.parts[2].joint).aobj.is_none());
    assert_eq!(
        tree.get(state.parts[3].joint).aobj.as_ref().unwrap().fobj[0].obj_type,
        3
    );
}

#[test]
fn translation_filter_stops_at_the_first_rejected_track() {
    let (mut tree, state) = skeleton();
    let root = state.root;
    let animation = FigaTree {
        type_: 1,
        frames: 9.0,
        ..FigaTree::default()
    };
    for tracks in [
        vec![track(1), track(5), track(2)],
        vec![track(1), track(6), track(2)],
        vec![track(1), track(7), track(2)],
    ] {
        attach_joint_tracks_without_translation(&mut tree, root, &animation, &tracks).unwrap();
        let aobj = tree.get(root).aobj.as_ref().unwrap();
        assert_eq!(aobj.fobj.len(), 1);
        assert_eq!(aobj.fobj[0].obj_type, 1);
        assert_eq!(aobj.end_frame, 9.0);
    }
    let before = tree.clone();
    assert_eq!(
        attach_joint_tracks_without_translation(&mut tree, root, &animation, &[track(5), track(1)]),
        Err(AttachError::NoAcceptedTracks)
    );
    assert_eq!(tree, before);
    attach_joint_tracks_without_translation(&mut tree, root, &animation, &[]).unwrap();
    assert_eq!(tree, before);
}

#[test]
fn remapping_consumes_missing_bones_and_filters_only_destination_translations() {
    let (mut tree, mut state) = skeleton();
    state.parts[1].flags.0 |= PartFlags::TRANSLATION;
    let remap = MotionRemap {
        source: PartTable {
            joint_to_part: vec![None, Some(1), Some(2)],
            part_to_joint: vec![],
        },
        destination: PartTable {
            joint_to_part: vec![],
            part_to_joint: vec![None, Some(1), Some(3)],
        },
        source_masks: vec![0; 3],
    };
    let animation = FigaTree {
        nodes: vec![1, 1, 3],
        tracks: vec![track(3), track(5), track(1), track(5), track(2)],
        ..FigaTree::default()
    };
    attach_motion(&mut tree, &state.parts, &animation, 0, Some(&remap)).unwrap();
    assert!(tree.get(state.root).aobj.is_none());
    let translation = tree.get(state.parts[1].joint).aobj.as_ref().unwrap();
    assert_eq!(translation.fobj[0].obj_type, 5);
    let filtered = tree.get(state.parts[3].joint).aobj.as_ref().unwrap();
    assert_eq!(filtered.fobj.len(), 1);
    assert_eq!(filtered.fobj[0].obj_type, 1);
}

#[test]
fn first_play_end_and_immediate_wait_restart() {
    let (mut tree, mut state) = skeleton();
    let motion = motion(0.0);
    state.set_animation(&mut tree, &motion, 0.0, 1.0).unwrap();
    let mut rng = HsdRng::new(42);
    for expected in [0.0f32, 1.0, 2.0] {
        state.step::<RetailTrig>(&mut tree);
        assert_eq!(state.frame.to_bits(), expected.to_bits());
        assert!(state
            .update_wait::<RetailTrig>(&mut tree, &mut rng, None, |_| &motion)
            .unwrap()
            .is_none());
    }
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(state.frame, 3.0);
    assert!(!state.frames_remaining(&tree));
    let choice = state
        .update_wait::<RetailTrig>(&mut tree, &mut rng, None, |_| &motion)
        .unwrap()
        .unwrap();
    assert_eq!(choice.draws, 0);
    assert_eq!(rng.seed, 42);
    assert_eq!(state.frame, 0.0);
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(state.frame, 1.0);
}

#[test]
fn blend_progress_saturates_but_active_tree_does_not_change() {
    let (mut tree, mut state) = skeleton();
    let mut motion = motion(2.0);
    motion.animation.frames = 9.0;
    let joint = state.parts[1].joint;
    tree.set_translate(joint, &Vec3::new(8.0, 0.0, 0.0));
    state.set_animation(&mut tree, &motion, 0.0, 1.0).unwrap();
    assert!(tree.get(joint).aobj.is_none());
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(state.blend_progress, 1.0);
    assert_eq!(tree.translation_x(joint), 4.0);
    assert_eq!(state.frame, 0.0);
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(tree.translation_x(joint), 0.0);
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(
        (state.blend_duration, state.blend_progress, state.frame),
        (2.0, 2.0, 2.0)
    );
}

#[test]
fn part_animation_runs_between_command_and_accessory_hooks_without_main_motion() {
    let (mut tree, mut state) = skeleton();
    let joint = state.parts[1].joint;
    state
        .blend_tree
        .set_translate(joint, &Vec3::new(6.0, 0.0, 0.0));
    state.parts[1].flags.0 |= PartFlags::PART_ANIMATION;
    state.step_with_hooks::<RetailTrig>(
        &mut tree,
        |state, tree| {
            assert_eq!(tree.translation_x(joint), 0.0);
            state.part_animations[0] = PartAnimation {
                duration: 2.0,
                rate: 1.0,
                current: 0,
                joints: [1].into_iter().collect(),
                ..PartAnimation::default()
            };
        },
        |state, tree| {
            assert_eq!(state.part_animations[0].progress, 1.0);
            assert_eq!(tree.translation_x(joint), 3.0);
        },
    );
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(tree.translation_x(joint), 6.0);
}

#[test]
fn copy_parts_bypass_interpolation_and_locked_parts_keep_their_pose() {
    let (mut tree, mut state) = skeleton();
    let copy = state.parts[1].joint;
    let locked = state.parts[2].joint;
    state.parts[1].flags.0 |= PartFlags::COPY;
    state.parts[2].flags.0 |= PartFlags::LOCKED;
    tree.set_translate(copy, &Vec3::new(10.0, 0.0, 0.0));
    tree.set_rotation_x(locked, 1.5);
    state
        .set_animation(&mut tree, &motion(6.0), 0.0, 1.0)
        .unwrap();
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(tree.translation_x(copy), 0.0);
    assert_eq!(tree.rotation_x(locked), 1.5);
    assert_eq!(tree.flags(copy) & JOBJ_USE_QUATERNION, 0);
}

#[test]
fn frames_remaining_uses_only_eligible_aobjs_and_selected_tree() {
    let (mut tree, mut state) = skeleton();
    let joint = state.parts[1].joint;
    tree.get_mut(joint).aobj = Some(AObj {
        flags: 0,
        ..AObj::default()
    });
    assert!(state.frames_remaining(&tree));
    state.parts[1].flags.0 |= PartFlags::LOCKED;
    assert!(!state.frames_remaining(&tree));
    state.parts[1].flags.0 &= !PartFlags::LOCKED;
    state.blend_duration = 6.0;
    assert!(!state.frames_remaining(&tree));
    state.blend_tree.get_mut(joint).aobj = Some(AObj {
        flags: 0,
        ..AObj::default()
    });
    assert!(state.frames_remaining(&tree));
    state.blend_tree.get_mut(joint).aobj.as_mut().unwrap().flags = AOBJ_NO_ANIM;
    assert!(!state.frames_remaining(&tree));
}

#[test]
fn pose_blending_uses_the_six_retail_fused_sites() {
    use melee_ft::anim::blend::blend_pose;
    let (mut tree, state) = skeleton();
    let root = state.root;
    let mut source = tree.get(root).clone();
    source.translate = Vec3::new(f32::from_bits(0x3f80_0001), 0.0, 0.0);
    tree.set_translate(root, &Vec3::new(-1.0, 0.0, 0.0));
    // (1+2^-23)*(1-2^-23)-1 = -2^-46. An unfused product rounds to
    // 1.0 first and yields zero, so this distinguishes the retail fmadds.
    blend_pose::<RetailTrig>(&source, &mut tree, root, f32::from_bits(0x3f7f_fffe), 1.0);
    assert_eq!(tree.translation_x(root).to_bits(), 0xa880_0000);
}

#[test]
fn rate_deferral_preserves_live_clocks_until_applied() {
    let (mut tree, mut state) = skeleton();
    state
        .set_animation(&mut tree, &motion(6.0), 0.0, 1.0)
        .unwrap();
    state.set_rate(&mut tree, 0.25, true);
    assert_eq!(state.saved_speed, 0.25);
    assert_eq!(state.speed, 1.0);
    assert_eq!(state.current_aobj(&tree).unwrap().framerate, 1.0);
    state.set_rate(&mut tree, 0.25, false);
    state.step::<RetailTrig>(&mut tree);
    state.step::<RetailTrig>(&mut tree);
    assert_eq!(state.frame, 0.25);
    assert_eq!(state.blend_progress, 0.5);
}

#[test]
fn extracted_root_motion_tracks_both_histories_and_clears_local_translation() {
    use melee_ft::anim::root_motion::{RootMotion, TranslationHistory};
    let (mut tree, state) = skeleton();
    let primary = state.parts[1].joint;
    let secondary = state.parts[2].joint;
    let mut motion = RootMotion {
        translation: primary,
        secondary,
        primary_history: TranslationHistory::default(),
        secondary_history: TranslationHistory::default(),
        effective_scale: 2.0,
        compensate_joint: None,
        pinned: false,
    };
    tree.set_translate(primary, &Vec3::new(3.0, 4.0, 5.0));
    tree.set_translate(secondary, &Vec3::new(1.0, 2.0, 3.0));
    motion.animate::<RetailTrig>(
        &mut tree,
        state.root,
        MotionFlags(MotionFlags::SECOND_ROOT),
        0.5,
    );
    assert_eq!(tree.translation(primary), Vec3::new(4.0, 4.0, 4.0));
    assert_eq!(tree.translation(secondary), Vec3::ZERO);
    assert_eq!(motion.primary_history.position, Vec3::new(2.0, 4.0, 6.0));
    assert_eq!(motion.primary_history.offset, Vec3::new(2.0, 4.0, 6.0));
    // Attribute-scale mode chooses the other scale and preserves old offsets.
    tree.set_translate(primary, &Vec3::new(8.0, 8.0, 8.0));
    motion.animate::<RetailTrig>(
        &mut tree,
        state.root,
        MotionFlags(MotionFlags::ATTRIBUTE_SCALE),
        0.5,
    );
    assert_eq!(tree.translation(primary), Vec3::ZERO);
    assert_eq!(motion.primary_history.previous, Vec3::new(2.0, 4.0, 6.0));
    assert_eq!(motion.primary_history.position, Vec3::new(4.0, 4.0, 4.0));
    assert_eq!(motion.primary_history.offset, Vec3::new(2.0, 0.0, -2.0));
    assert_eq!(
        motion.primary_history.previous_offset,
        Vec3::new(2.0, 4.0, 6.0)
    );
}

#[test]
fn blend_root_extraction_keeps_primary_joint_pointer_identity() {
    use melee_ft::anim::root_motion::{RootMotion, TranslationHistory};
    let (mut primary_tree, mut state) = skeleton();
    let translation = state.parts[1].joint;
    let secondary = state.parts[2].joint;
    let compensate = state.parts[3].joint;
    let mut motion = RootMotion {
        translation,
        secondary,
        primary_history: TranslationHistory::default(),
        secondary_history: TranslationHistory {
            position: Vec3::new(10.0, 0.0, 0.0),
            ..TranslationHistory::default()
        },
        effective_scale: 2.0,
        compensate_joint: Some(compensate),
        pinned: false,
    };
    primary_tree.set_translate(compensate, &Vec3::new(20.0, 0.0, 0.0));
    state
        .blend_tree
        .set_translate(translation, &Vec3::new(3.0, 0.0, 0.0));
    state
        .blend_tree
        .set_translate(secondary, &Vec3::new(4.0, 0.0, 0.0));
    motion.animate_blend::<RetailTrig>(
        &mut state.blend_tree,
        &mut primary_tree,
        state.root,
        MotionFlags(MotionFlags::SECOND_ROOT),
        1.0,
    );
    assert_eq!(state.blend_tree.translation_x(translation), -4.0);
    assert_eq!(state.blend_tree.translation_x(secondary), 4.0);
    assert_eq!(motion.primary_history.position.x, 10.0);
    assert_eq!(primary_tree.translation_x(compensate), 15.0);
    assert_eq!(state.blend_tree.translation_x(compensate), 0.0);
}

#[test]
fn same_character_checks_presence_before_conditional_mask_walk() {
    let (mut tree, mut state) = skeleton();
    state.parts[1].flags.0 |= PartFlags::CONDITIONAL;
    state.parts[1].motion_mask = 2;
    state.parts[2].flags.0 = 0;
    let animation = FigaTree {
        nodes: vec![0, 1],
        tracks: vec![track(1)],
        ..FigaTree::default()
    };
    attach_motion(&mut tree, &state.parts, &animation, 0, None).unwrap();
    assert!(tree.get(state.parts[1].joint).aobj.is_none());
    assert!(tree.get(state.parts[2].joint).aobj.is_some());
}
