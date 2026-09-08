//! `HSD_JObjLoadJoint` / `HSD_JObjAddAnimAll` (jobj.c) archive integration.

mod anim_common;
mod load_common;

use anim_common::Stream;
use hsd_anim::aobj::{AOBJ_LOOP, AOBJ_NO_ANIM, AObj, AObjDesc};
use hsd_anim::fobj::{FObjDesc, HSD_A_FRAC_FLOAT, HSD_A_OP_LIN};
use hsd_anim::jobj::*;
use hsd_anim::load::{LoadError, attach_anim_joint, load_joint_tree};
use hsd_anim::mobj::{RENDER_NO_ZUPDATE, RENDER_TOON, RENDER_XLU};
use hsd_anim::mtx::InverseTrig;
use hsd_archive::{Archive, desc};
use hsd_types::{Mtx, Vec3};
use load_common::{Builder, FObj, Joint, MObj};

struct UnusedTrig;
impl InverseTrig for UnusedTrig {
    fn atan2f(_: f32, _: f32) -> f32 {
        panic!("translation does not need inverse trig")
    }
    fn asinf(_: f32) -> f32 {
        panic!("translation does not need inverse trig")
    }
    fn acosf(_: f32) -> f32 {
        panic!("translation does not need inverse trig")
    }
}

struct Scene {
    archive: Archive,
    root: desc::JObjDesc,
    anim: desc::AnimJoint,
    /// Root, first child, grandchild, second child (allocation/bone order).
    offsets: [u32; 4],
}

fn translation_track() -> FObjDesc {
    let mut stream = Stream::new();
    stream.pack(HSD_A_OP_LIN, 2).f32(2.0).wait(6).f32(14.0);
    FObjDesc {
        length: stream.bytes.len() as u32,
        startframe: 0.0,
        obj_type: HSD_A_J_TRAX,
        frac_value: HSD_A_FRAC_FLOAT,
        frac_slope: HSD_A_FRAC_FLOAT,
        ad: stream.finish(),
    }
}

fn joint_fixture() -> Joint {
    Joint {
        rotation: [-0.0, 0.25, -0.75],
        scale: [1.25, 0.5, 2.0],
        position: [5.0, -3.0, f32::from_bits(1)],
        ..Joint::default()
    }
}

fn scene() -> Scene {
    let mut b = Builder::new();
    let track = translation_track();
    let data = b.push_bytes(&track.ad);
    b.align(4);
    let fobj = b.fobj(&FObj {
        length: track.length,
        type_: track.obj_type,
        frac_value: track.frac_value,
        frac_slope: track.frac_slope,
        ad: Some(data),
        ..FObj::default()
    });
    let aobj = b.aobj(AOBJ_LOOP, 6.0, Some(fobj), 0, false);
    let grand_anim = b.animjoint(None, None, Some(aobj), None, 1);
    let second_anim = b.animjoint(None, None, None, None, 0);
    let first_anim = b.animjoint(Some(grand_anim), Some(second_anim), None, None, 0);
    let root_anim = b.animjoint(Some(first_anim), None, None, None, 0);

    let grandchild = b.joint(&Joint {
        flags: JOBJ_SKELETON,
        ..joint_fixture()
    });
    let second = b.joint(&Joint {
        flags: JOBJ_HIDDEN,
        ..joint_fixture()
    });
    let first = b.joint(&Joint {
        flags: JOBJ_SKELETON,
        child: Some(grandchild),
        next: Some(second),
        ..joint_fixture()
    });
    let root = b.joint(&Joint {
        flags: JOBJ_SKELETON_ROOT | JOBJ_CLASSICAL_SCALE,
        child: Some(first),
        ..joint_fixture()
    });
    b.public(root, "root");
    b.public(root_anim, "anim");
    let archive = b.archive();
    Scene {
        root: desc::read_public_jobj(&archive, "root").unwrap(),
        anim: desc::read_public_animjoint(&archive, "anim").unwrap(),
        archive,
        offsets: [root, first, grandchild, second],
    }
}

fn vector_bits(value: Vec3) -> [u32; 3] {
    [value.x.to_bits(), value.y.to_bits(), value.z.to_bits()]
}

#[test]
fn four_joints_preserve_depth_first_bone_order_links_flags_and_srt_bits() {
    let scene = scene();
    let (tree, root) = load_joint_tree(&scene.archive, &scene.root).unwrap();
    assert_eq!(tree.len(), 4);
    assert_eq!(
        tree.depth_first(root).collect::<Vec<_>>(),
        tree.ids().collect::<Vec<_>>()
    );
    let expected_links = [
        (None, Some(JObjId(1)), None),
        (Some(root), Some(JObjId(2)), Some(JObjId(3))),
        (Some(JObjId(1)), None, None),
        (Some(root), None, None),
    ];
    let descriptors = scene.root.descendants();
    for (index, (descriptor, links)) in descriptors.iter().zip(expected_links).enumerate() {
        let id = JObjId(index);
        assert_eq!(tree.bone(root, index), Some(id));
        let node = tree.get(id);
        assert_eq!(node.id, scene.offsets[index]);
        assert_eq!((node.parent, node.child, node.next), links);
        assert_eq!(node.flags, descriptor.flags | JOBJ_MTX_DIRTY);
        assert_eq!(
            vector_bits(node.scale),
            joint_fixture().scale.map(f32::to_bits)
        );
        assert_eq!(
            vector_bits(node.translate),
            joint_fixture().position.map(f32::to_bits)
        );
        assert_eq!(
            [node.rotate.x, node.rotate.y, node.rotate.z].map(f32::to_bits),
            joint_fixture().rotation.map(f32::to_bits)
        );
        assert_eq!(node.rotate.w.to_bits(), 0);
        assert_eq!(node.mtx, Mtx::IDENTITY);
        assert_eq!(node.scl, None);
        assert!(node.aobj.is_none());
        assert!(node.dobj.is_empty());
    }
    assert_eq!(tree.bone(root, 4), None);
}

#[test]
fn root_siblings_and_quaternion_flag_keep_c_load_semantics() {
    let mut b = Builder::new();
    let sibling = b.joint(&joint_fixture());
    let matrix = b.here();
    let values = [1.0, -0.0, 0.0, 4.0, 0.0, 2.0, 0.0, 5.0, 0.0, 0.0, 3.0, 6.0];
    for value in values {
        b.push_f32(value);
    }
    let root = b.joint(&Joint {
        flags: JOBJ_USE_QUATERNION,
        next: Some(sibling),
        mtx: Some(matrix),
        ..joint_fixture()
    });
    let archive = b.archive();
    let desc = desc::JObjDesc::read(&archive, root).unwrap();
    let (tree, root) = load_joint_tree(&archive, &desc).unwrap();
    assert_eq!(tree.next(root), Some(JObjId(1)));
    assert_eq!(tree.parent(JObjId(1)), None);
    assert_eq!(tree.get(JObjId(1)).id, sibling);
    assert_eq!(
        tree.depth_first(root).collect::<Vec<_>>(),
        [root, JObjId(1)]
    );
    let node = tree.get(root);
    assert_eq!(
        [node.rotate.x, node.rotate.y, node.rotate.z].map(f32::to_bits),
        joint_fixture().rotation.map(f32::to_bits)
    );
    assert_eq!(node.rotate.w.to_bits(), 0);
    assert_eq!(
        node.envelopemtx
            .unwrap()
            .0
            .into_iter()
            .flatten()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        values.map(f32::to_bits)
    );
}

#[test]
fn grandchild_animation_matches_hand_built_runtime_at_frames_zero_and_three() {
    let scene = scene();
    let (mut loaded, root) = load_joint_tree(&scene.archive, &scene.root).unwrap();
    let spec = || {
        JointSpec::new()
            .scale(1.25, 0.5, 2.0)
            .rotation(-0.0, 0.25, -0.75)
            .position(5.0, -3.0, f32::from_bits(1))
    };
    let mut manual = JObjTree::new();
    let manual_root = manual.load_joint(&spec().child(spec().child(spec())).child(spec()));
    let aobj = AObjDesc {
        flags: AOBJ_LOOP,
        end_frame: 6.0,
        fobjdesc: vec![translation_track()],
        obj_id: 0,
    };
    manual.get_mut(JObjId(2)).aobj = Some(AObj::load_desc(&aobj));
    attach_anim_joint(&mut loaded, root, &scene.anim, &scene.archive).unwrap();
    for id in loaded.ids() {
        assert_eq!(loaded.get(id).aobj.is_some(), id == JObjId(2));
    }
    assert_eq!(loaded.flags(root) & JOBJ_CLASSICAL_SCALE, 0);
    assert_ne!(loaded.flags(JObjId(2)) & JOBJ_CLASSICAL_SCALE, 0);
    let animation = loaded.get(JObjId(2)).aobj.as_ref().unwrap();
    assert_ne!(
        animation.flags & AOBJ_NO_ANIM,
        0,
        "attachment does not request playback"
    );
    for (frame, expected) in [(0.0, 2.0_f32), (3.0, 8.0_f32)] {
        loaded.req_anim_all(root, frame);
        manual.req_anim_all(manual_root, frame);
        assert_eq!(
            loaded.anim_all::<UnusedTrig>(root),
            manual.anim_all::<UnusedTrig>(manual_root)
        );
        for id in loaded.ids() {
            assert_eq!(
                vector_bits(loaded.translation(id)),
                vector_bits(manual.translation(id))
            );
        }
        assert_eq!(
            loaded.translation_x(JObjId(2)).to_bits(),
            expected.to_bits()
        );
    }
}

#[test]
fn mismatched_animation_shapes_leave_existing_animation_unchanged() {
    for extra in [false, true] {
        let mut scene = scene();
        let (mut tree, root) = load_joint_tree(&scene.archive, &scene.root).unwrap();
        attach_anim_joint(&mut tree, root, &scene.anim, &scene.archive).unwrap();
        let before = tree.clone();
        let first = scene.anim.child.as_mut().unwrap();
        if extra {
            first.next.as_mut().unwrap().child = Some(Box::new(desc::AnimJoint::default()));
        } else {
            first.child = None;
        }
        assert!(matches!(
            attach_anim_joint(&mut tree, root, &scene.anim, &scene.archive),
            Err(LoadError::MismatchedTreeShape { .. })
        ));
        assert_eq!(tree, before);
    }
}

#[test]
fn invalid_runtime_root_returns_error() {
    let scene = scene();
    assert!(matches!(
        attach_anim_joint(&mut JObjTree::new(), JObjId(0), &scene.anim, &scene.archive),
        Err(LoadError::InvalidJoint(JObjId(0)))
    ));
}

#[test]
fn attachment_ignores_root_siblings_like_add_anim_all() {
    let mut scene = scene();
    let (mut tree, root) = load_joint_tree(&scene.archive, &scene.root).unwrap();
    let sibling = tree.load_joint(&JointSpec::new());
    tree.get_mut(root).next = Some(sibling);
    scene.anim.next = Some(Box::new(desc::AnimJoint {
        robj_anim: Some(0),
        offset: 0,
        flags: 0,
        child: None,
        next: None,
        aobjdesc: None,
    }));
    attach_anim_joint(&mut tree, root, &scene.anim, &scene.archive).unwrap();
    assert!(tree.get(sibling).aobj.is_none());
}

#[test]
fn unsupported_animation_payload_is_atomic_even_on_a_late_sibling() {
    for object_reference in [false, true] {
        let mut scene = scene();
        let (mut tree, root) = load_joint_tree(&scene.archive, &scene.root).unwrap();
        attach_anim_joint(&mut tree, root, &scene.anim, &scene.archive).unwrap();
        let before = tree.clone();
        let sibling = scene.anim.child.as_mut().unwrap().next.as_mut().unwrap();
        if object_reference {
            sibling.aobjdesc = Some(Box::new(desc::AObjDesc {
                // A relocated reference to offset zero is not a null pointer.
                obj_id: 0,
                obj_id_is_link: true,
                ..desc::AObjDesc::default()
            }));
        } else {
            sibling.robj_anim = Some(0);
        }
        assert!(matches!(
            attach_anim_joint(&mut tree, root, &scene.anim, &scene.archive),
            Err(LoadError::UnsupportedFeature { .. })
        ));
        assert_eq!(tree, before);
    }
}

#[test]
fn attachment_uses_runtime_branch_track_sorting_and_replaces_previous_tracks() {
    let mut b = Builder::new();
    let track = translation_track();
    let data = b.push_bytes(&track.ad);
    b.align(4);
    let fobj = |type_, next| FObj {
        type_,
        next,
        length: track.length,
        ad: Some(data),
        frac_value: track.frac_value,
        frac_slope: track.frac_slope,
        ..FObj::default()
    };
    let branch = b.fobj(&fobj(HSD_A_J_BRANCH, None));
    let translation = b.fobj(&fobj(HSD_A_J_TRAX, Some(branch)));
    let aobj = b.aobj(AOBJ_LOOP, 6.0, Some(translation), 0, false);
    let anim = b.animjoint(None, None, Some(aobj), None, 0);
    let archive = b.archive();
    let mut anim = desc::AnimJoint::read(&archive, anim).unwrap();
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    attach_anim_joint(&mut tree, root, &anim, &archive).unwrap();
    let aobj = tree.get(root).aobj.as_ref().unwrap();
    assert_eq!(
        aobj.fobj.iter().map(|f| f.obj_type).collect::<Vec<_>>(),
        [HSD_A_J_BRANCH, HSD_A_J_TRAX]
    );
    anim.aobjdesc = None;
    attach_anim_joint(&mut tree, root, &anim, &archive).unwrap();
    assert!(tree.get(root).aobj.is_none());
}

#[test]
fn material_chains_copy_colors_and_classify_blending() {
    let mut b = Builder::new();
    let colors = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
    let mat = b.material(colors, -0.0, 23.5);
    let class = b.data_cstr("hsd_mobj");
    let mut next = Some(b.dobj(None, None, None, None));
    for mode in [RENDER_XLU | RENDER_NO_ZUPDATE, RENDER_XLU, 0] {
        let material = b.mobj(&MObj {
            class_name: Some(class),
            rendermode: mode,
            mat: Some(mat),
            ..MObj::default()
        });
        next = Some(b.dobj(None, next, Some(material), None));
    }
    let root = b.joint(&Joint {
        u: next,
        ..joint_fixture()
    });
    let archive = b.archive();
    let descriptor = desc::JObjDesc::read(&archive, root).unwrap();
    let (mut tree, root) = load_joint_tree(&archive, &descriptor).unwrap();
    let objects = &tree.get(root).dobj;
    assert_eq!(objects.len(), 4);
    assert_eq!(
        objects.iter().map(|d| d.flags).collect::<Vec<_>>(),
        [2, 8, 4, 0]
    );
    for (object, mode) in objects
        .iter()
        .zip([0, RENDER_XLU, RENDER_XLU | RENDER_NO_ZUPDATE])
    {
        let m = object.mobj.as_ref().unwrap();
        assert_eq!(m.rendermode, mode | RENDER_TOON);
        for (actual, expected) in [m.mat.ambient, m.mat.diffuse, m.mat.specular]
            .into_iter()
            .zip(colors)
        {
            assert_eq!([actual.r, actual.g, actual.b, actual.a], expected);
        }
        assert_eq!(m.mat.alpha.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(m.mat.shininess.to_bits(), 23.5_f32.to_bits());
        assert!(m.aobj.is_none());
    }
    assert!(objects[3].mobj.is_none());
    // C receives MatAnimJoint separately: joint-only attachment leaves its AObj alone.
    tree.get_mut(root).dobj[0].mobj.as_mut().unwrap().aobj = Some(AObj::alloc());
    let before = tree.get(root).dobj.clone();
    attach_anim_joint(&mut tree, root, &desc::AnimJoint::default(), &archive).unwrap();
    assert_eq!(tree.get(root).dobj, before);
}

#[test]
fn unsupported_joint_payloads_are_errors() {
    let mut scene = scene();
    for flags in [JOBJ_INSTANCE, JOBJ_SPLINE, JOBJ_PTCL, JOBJ_JOINT1] {
        scene.root.flags = flags;
        assert!(matches!(load_joint_tree(&scene.archive, &scene.root),
            Err(LoadError::UnsupportedFlag { flags: actual, .. }) if actual == flags));
    }
    scene.root.flags = 0;
    scene.root.robjdesc = Some(0);
    assert!(matches!(
        load_joint_tree(&scene.archive, &scene.root),
        Err(LoadError::UnsupportedFeature {
            feature: "RObj constraints",
            ..
        })
    ));
}

#[test]
fn invalid_material_blending_returns_error_instead_of_runtime_panic() {
    let mut b = Builder::new();
    let mat = b.material([[0; 4]; 3], 1.0, 0.0);
    let mobj = b.mobj(&MObj {
        mat: Some(mat),
        rendermode: RENDER_NO_ZUPDATE,
        ..MObj::default()
    });
    let dobj = b.dobj(None, None, Some(mobj), None);
    let root = b.joint(&Joint {
        u: Some(dobj),
        ..joint_fixture()
    });
    let archive = b.archive();
    let descriptor = desc::JObjDesc::read(&archive, root).unwrap();
    assert!(matches!(
        load_joint_tree(&archive, &descriptor),
        Err(LoadError::UnsupportedFlag { .. })
    ));
}

#[test]
fn archive_read_error_remains_available_as_source() {
    use std::error::Error;
    let archive = Builder::new().archive();
    let result: Result<_, LoadError> = (|| {
        let descriptor = desc::JObjDesc::read(&archive, 0)?;
        load_joint_tree(&archive, &descriptor)
    })();
    let error = result.unwrap_err();
    assert!(matches!(error, LoadError::Archive(_)));
    assert!(error.source().is_some());
    assert!(error.to_string().contains("archive descriptor read failed"));
}
