//! AObj-driven joints: `HSD_JObjAddAnimAll` / `ReqAnimAll` / `AnimAll`
//! over frames, with FObj byte streams built the way `anim_fobj.rs` builds
//! them. Expected joint values come from stepping an identical `AObj`
//! directly; expected matrices from `crate::mtx` in the C's op order.

mod anim_common;

use anim_common::Stream;
use hsd_anim::aobj::*;
use hsd_anim::dobj::DObj;
use hsd_anim::fobj::*;
use hsd_anim::jobj::*;
use hsd_anim::mobj::{MObj, MatAnim, Material, HSD_A_M_ALPHA, HSD_A_M_DIFFUSE_R};
use hsd_anim::mtx::{self, InverseTrig};
use hsd_types::{Mtx, Vec3};

/// Stand-in inverse trig; only the RObj-driven `0x36`/`0x37` ids use it.
struct StubTrig;
impl InverseTrig for StubTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        y * 0.5 + x * 0.25
    }
    fn asinf(x: f32) -> f32 {
        x * 0.5
    }
    fn acosf(x: f32) -> f32 {
        1.0 - x * 0.5
    }
}

const FLOAT: u8 = HSD_A_FRAC_FLOAT;

fn lin_desc(track: u8, v0: f32, wait: u32, v1: f32) -> FObjDesc {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 2).f32(v0).wait(wait).f32(v1);
    FObjDesc {
        length: s.bytes.len() as u32,
        startframe: 0.0,
        obj_type: track,
        frac_value: FLOAT,
        frac_slope: FLOAT,
        ad: s.finish(),
    }
}

fn con_desc(track: u8, keys: &[(f32, u32)]) -> FObjDesc {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, keys.len() as u32);
    for (v, w) in keys {
        s.f32(*v).wait(*w);
    }
    FObjDesc {
        length: s.bytes.len() as u32,
        startframe: 0.0,
        obj_type: track,
        frac_value: FLOAT,
        frac_slope: FLOAT,
        ad: s.finish(),
    }
}

fn aobj_desc(end_frame: f32, flags: u32, tracks: Vec<FObjDesc>) -> AObjDesc {
    AObjDesc {
        flags,
        end_frame,
        fobjdesc: tracks,
        obj_id: 0,
    }
}

fn anim_joint(desc: Option<AObjDesc>, children: Vec<AnimJoint>) -> AnimJoint {
    AnimJoint {
        aobjdesc: desc,
        flags: 0,
        children,
    }
}

fn assert_mtx_bits(got: &Mtx, want: &Mtx, what: &str) {
    for r in 0..3 {
        for c in 0..4 {
            assert_eq!(
                got.0[r][c].to_bits(),
                want.0[r][c].to_bits(),
                "{what}: [{r}][{c}]"
            );
        }
    }
}

/// Reference SRT state driven by the same AObj through the same rules as
/// `JObjUpdateFunc` for the SRT tracks.
#[derive(Clone, Copy)]
struct Ref {
    rot: Vec3,
    sca: Vec3,
    tra: Vec3,
}

fn apply_ref(r: &mut Ref, ty: u8, v: f32) {
    match ty {
        HSD_A_J_ROTX => r.rot.x = v,
        HSD_A_J_ROTY => r.rot.y = v,
        HSD_A_J_ROTZ => r.rot.z = v,
        HSD_A_J_TRAX => r.tra.x = v,
        HSD_A_J_TRAY => r.tra.y = v,
        HSD_A_J_TRAZ => r.tra.z = v,
        HSD_A_J_SCAX => r.sca.x = v,
        HSD_A_J_SCAY => r.sca.y = v,
        HSD_A_J_SCAZ => r.sca.z = v,
        _ => {}
    }
}

#[test]
fn single_joint_tracks_follow_the_aobj_frame_by_frame() {
    let desc = aobj_desc(
        10.0,
        0,
        vec![
            lin_desc(HSD_A_J_TRAX, 0.0, 10, 10.0),
            lin_desc(HSD_A_J_ROTZ, 0.0, 5, 1.5),
            con_desc(HSD_A_J_SCAY, &[(1.0, 3), (2.0, 4), (0.5, 3)]),
        ],
    );

    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new().position(1.0, 2.0, 3.0));
    tree.add_anim_all(root, Some(&anim_joint(Some(desc.clone()), vec![])), None);
    assert!(tree.get(root).aobj.is_some());
    tree.req_anim_all(root, 0.0);

    let mut oracle = AObj::load_desc(&desc);
    oracle.req_anim(0.0);
    let mut r = Ref {
        rot: Vec3::ZERO,
        sca: Vec3::new(1.0, 1.0, 1.0),
        tra: Vec3::new(1.0, 2.0, 3.0),
    };

    for frame in 0..14 {
        let cb = tree.anim_all::<StubTrig>(root);
        let mut ocb = AObjEndCallback::default();
        oracle.interpret_anim(&mut |ty, v| apply_ref(&mut r, ty, v), &mut ocb);
        assert_eq!(cb, ocb, "frame {frame} counters");
        // Running until the end frame; counted as ended exactly once, at
        // frame 10; a stopped AObj is not counted afterwards.
        let want_cb = match frame {
            0..=9 => AObjEndCallback {
                ended: 0,
                running: 1,
            },
            10 => AObjEndCallback {
                ended: 1,
                running: 0,
            },
            _ => AObjEndCallback {
                ended: 0,
                running: 0,
            },
        };
        assert_eq!(cb, want_cb, "frame {frame}");
        assert_eq!(cb.should_invoke(), frame == 10);

        assert_eq!(tree.translation(root), r.tra, "frame {frame} translate");
        assert_eq!(tree.scale(root), r.sca, "frame {frame} scale");
        assert_eq!(tree.rotation_z(root), r.rot.z, "frame {frame} rotz");
        assert_eq!(
            tree.get(root).aobj.as_ref().unwrap().curr_frame,
            oracle.curr_frame
        );

        if frame < 10 {
            assert!(tree.mtx_is_dirty(root), "frame {frame}: setters mark dirty");
        }
        let got = *tree.get_mtx(root);
        let mut want = Mtx::ZERO;
        mtx::hsd_mtx_srt(&mut want, &r.sca, &r.rot, &r.tra, None);
        assert_mtx_bits(&got, &want, &format!("frame {frame}"));
        assert!(!tree.mtx_is_dirty(root));
    }
    assert_eq!(tree.translation_x(root), 10.0);
    assert!(tree.get(root).aobj.as_ref().unwrap().flags & AOBJ_NO_ANIM != 0);
}

#[test]
fn child_animation_composes_with_parent_animation() {
    let root_desc = aobj_desc(8.0, AOBJ_LOOP, vec![lin_desc(HSD_A_J_ROTY, 0.0, 8, 3.0)]);
    let child_desc = aobj_desc(8.0, AOBJ_LOOP, vec![lin_desc(HSD_A_J_TRAX, 0.0, 4, 2.0)]);

    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new().scale(2.0, 1.0, 0.5).child(
            JointSpec::new()
                .position(0.0, 1.0, 0.0)
                .rotation(0.1, 0.2, 0.3),
        ),
    );
    let child = tree.child(root).unwrap();
    tree.add_anim_all(
        root,
        Some(&anim_joint(
            Some(root_desc.clone()),
            vec![anim_joint(Some(child_desc.clone()), vec![])],
        )),
        None,
    );
    tree.req_anim_all(root, 0.0);

    let mut o_root = AObj::load_desc(&root_desc);
    let mut o_child = AObj::load_desc(&child_desc);
    o_root.req_anim(0.0);
    o_child.req_anim(0.0);
    let mut rr = Ref {
        rot: Vec3::ZERO,
        sca: Vec3::new(2.0, 1.0, 0.5),
        tra: Vec3::ZERO,
    };
    let mut rc = Ref {
        rot: Vec3::new(0.1, 0.2, 0.3),
        sca: Vec3::new(1.0, 1.0, 1.0),
        tra: Vec3::new(0.0, 1.0, 0.0),
    };

    for frame in 0..20 {
        let cb = tree.anim_all::<StubTrig>(root);
        let mut ocb = AObjEndCallback::default();
        o_root.interpret_anim(&mut |ty, v| apply_ref(&mut rr, ty, v), &mut ocb);
        o_child.interpret_anim(&mut |ty, v| apply_ref(&mut rc, ty, v), &mut ocb);
        assert_eq!(cb, ocb, "frame {frame}");
        assert_eq!(
            cb,
            AObjEndCallback {
                ended: 0,
                running: 2
            }
        );

        let got = *tree.get_mtx(child);
        let mut root_world = Mtx::ZERO;
        mtx::hsd_mtx_srt(&mut root_world, &rr.sca, &rr.rot, &rr.tra, None);
        let mut want = Mtx::ZERO;
        mtx::hsd_mtx_srt(&mut want, &rc.sca, &rc.rot, &rc.tra, Some(&rr.sca));
        let local = want;
        mtx::mtx_concat(&root_world, &local, &mut want);
        assert_mtx_bits(&got, &want, &format!("frame {frame} child"));
        assert_mtx_bits(
            &tree.get(root).mtx,
            &root_world,
            &format!("frame {frame} root"),
        );
    }
}

#[test]
fn end_callback_counters_sum_over_the_tree() {
    let short = aobj_desc(2.0, 0, vec![lin_desc(HSD_A_J_TRAX, 0.0, 2, 1.0)]);
    let long = aobj_desc(6.0, 0, vec![lin_desc(HSD_A_J_TRAY, 0.0, 6, 1.0)]);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new()
            .child(JointSpec::new())
            .child(JointSpec::new()),
    );
    let a = tree.child(root).unwrap();
    let b = tree.next(a).unwrap();
    // Root has no aobj; a is short, b is long.
    tree.add_anim_all(
        root,
        Some(&anim_joint(
            None,
            vec![
                anim_joint(Some(short), vec![]),
                anim_joint(Some(long), vec![]),
            ],
        )),
        None,
    );
    assert!(tree.get(root).aobj.is_none());
    tree.req_anim_all(root, 0.0);

    let mut seen = Vec::new();
    for _ in 0..8 {
        let cb = tree.anim_all::<StubTrig>(root);
        seen.push((cb.ended, cb.running, cb.should_invoke()));
    }
    // Frames 0,1: both running. Frame 2: a reaches end -> ended, b running:
    // not invoked. Frames 3..5: a stays stopped and is not counted at all
    // (NO_ANIM returns before the counters); b running. Frame 6: b ends.
    assert_eq!(
        seen,
        vec![
            (0, 2, false),
            (0, 2, false),
            (1, 1, false),
            (0, 1, false),
            (0, 1, false),
            (0, 1, false),
            (1, 0, true),
            (0, 0, false),
        ]
    );
    assert_eq!(tree.translation_x(a), 1.0);
    assert_eq!(tree.translation_y(b), 1.0);
}

#[test]
fn req_anim_all_restarts_at_the_given_frame() {
    let desc = aobj_desc(10.0, 0, vec![lin_desc(HSD_A_J_TRAX, 0.0, 10, 10.0)]);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    tree.add_anim_all(root, Some(&anim_joint(Some(desc.clone()), vec![])), None);
    tree.req_anim_all(root, 4.0);
    tree.anim_all::<StubTrig>(root); // FIRST_PLAY: evaluates at 4 without advancing

    let mut oracle = AObj::load_desc(&desc);
    oracle.req_anim(4.0);
    let mut v = 0.0;
    oracle.interpret_anim(&mut |_, x| v = x, &mut AObjEndCallback::default());
    assert_eq!(tree.translation_x(root), v);
    assert_eq!(tree.get(root).aobj.as_ref().unwrap().curr_frame, 4.0);

    // Flags without bit 0 leave the joint AObj alone.
    tree.req_anim_all_by_flags(root, 0x7FE, 0.0);
    assert_eq!(tree.get(root).aobj.as_ref().unwrap().curr_frame, 4.0);
    tree.req_anim_all_by_flags(root, 1, 0.0);
    assert_eq!(tree.get(root).aobj.as_ref().unwrap().curr_frame, 0.0);
}

#[test]
fn remove_anim_all_drops_aobjs_by_flag_bit() {
    let desc = aobj_desc(10.0, 0, vec![lin_desc(HSD_A_J_TRAX, 0.0, 10, 10.0)]);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new().child(JointSpec::new()));
    let child = tree.child(root).unwrap();
    let aj = anim_joint(Some(desc.clone()), vec![anim_joint(Some(desc), vec![])]);
    tree.add_anim_all(root, Some(&aj), None);
    tree.remove_anim_all_by_flags(root, 0x7FE);
    assert!(tree.get(root).aobj.is_some() && tree.get(child).aobj.is_some());
    tree.remove_anim(root);
    assert!(tree.get(root).aobj.is_none() && tree.get(child).aobj.is_some());
    tree.remove_anim_all(root);
    assert!(tree.get(child).aobj.is_none());
    // Animating without an AObj counts nothing.
    assert_eq!(tree.anim_all::<StubTrig>(root), AObjEndCallback::default());
}

#[test]
fn anim_joint_flag_bit0_controls_classical_scale() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    tree.setup_matrix(root);
    let mut aj = anim_joint(None, vec![]);
    aj.flags = 1;
    tree.add_anim_all(root, Some(&aj), None);
    assert!(tree.flags(root) & JOBJ_CLASSICAL_SCALE != 0);
    assert!(tree.mtx_is_dirty(root), "toggle dirties");
    tree.setup_matrix(root);
    aj.flags = 0;
    tree.add_anim_all(root, Some(&aj), None);
    assert!(tree.flags(root) & JOBJ_CLASSICAL_SCALE == 0);
    // HSD_JObjClearFlags tests `(flags ^ arg) & CLASSICAL_SCALE`, which is
    // zero when the bit is set and about to be cleared: no dirty (jobj.c:1022).
    assert!(!tree.mtx_is_dirty(root));
}

#[test]
fn add_anim_all_pairs_children_positionally_and_tolerates_short_chains() {
    let desc = aobj_desc(1.0, 0, vec![lin_desc(HSD_A_J_TRAX, 0.0, 1, 1.0)]);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new()
            .child(JointSpec::new())
            .child(JointSpec::new())
            .child(JointSpec::new()),
    );
    let a = tree.child(root).unwrap();
    let b = tree.next(a).unwrap();
    let c = tree.next(b).unwrap();
    // Anim tree has only two children: a gets none, b gets one, c is past the end.
    let aj = anim_joint(
        None,
        vec![anim_joint(None, vec![]), anim_joint(Some(desc), vec![])],
    );
    tree.add_anim_all(root, Some(&aj), None);
    assert!(tree.get(a).aobj.is_none());
    assert!(tree.get(b).aobj.is_some());
    assert!(tree.get(c).aobj.is_none());
}

#[test]
fn sort_anim_hoists_the_branch_track() {
    let desc = aobj_desc(
        4.0,
        0,
        vec![
            lin_desc(HSD_A_J_TRAX, 0.0, 4, 1.0),
            lin_desc(HSD_A_J_TRAY, 0.0, 4, 1.0),
            con_desc(HSD_A_J_BRANCH, &[(0.0, 2), (1.0, 2)]),
            lin_desc(HSD_A_J_TRAZ, 0.0, 4, 1.0),
        ],
    );
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    tree.add_anim_all(root, Some(&anim_joint(Some(desc), vec![])), None);
    let types: Vec<u8> = tree
        .get(root)
        .aobj
        .as_ref()
        .unwrap()
        .fobj
        .iter()
        .map(|f| f.obj_type)
        .collect();
    assert_eq!(
        types,
        [HSD_A_J_BRANCH, HSD_A_J_TRAX, HSD_A_J_TRAY, HSD_A_J_TRAZ]
    );

    // Without a BRANCH track the order is untouched.
    let mut a = AObj::load_desc(&aobj_desc(
        1.0,
        0,
        vec![
            lin_desc(HSD_A_J_TRAX, 0.0, 1, 1.0),
            lin_desc(HSD_A_J_TRAY, 0.0, 1, 1.0),
        ],
    ));
    jobj_sort_anim(&mut a);
    assert_eq!(a.fobj[0].obj_type, HSD_A_J_TRAX);
}

#[test]
fn branch_track_toggles_hidden_on_the_subtree_and_node_only_on_the_node() {
    let branch = aobj_desc(
        6.0,
        0,
        vec![con_desc(HSD_A_J_BRANCH, &[(0.0, 2), (1.0, 2), (0.3, 2)])],
    );
    let node = aobj_desc(6.0, 0, vec![con_desc(HSD_A_J_NODE, &[(1.0, 3), (0.0, 3)])]);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new().child(JointSpec::new().child(JointSpec::new())));
    let child = tree.child(root).unwrap();
    let grand = tree.child(child).unwrap();
    tree.add_anim_all(
        root,
        Some(&anim_joint(
            Some(branch),
            vec![anim_joint(Some(node), vec![])],
        )),
        None,
    );
    tree.req_anim_all(root, 0.0);

    let hidden = |t: &JObjTree| [root, child, grand].map(|i| t.flags(i) & JOBJ_HIDDEN != 0);

    tree.anim_all::<StubTrig>(root); // frame 0: BRANCH 0 -> hide all; NODE 1 -> show child
    assert_eq!(hidden(&tree), [true, false, true]);
    tree.anim_all::<StubTrig>(root); // frame 1: same values
    assert_eq!(hidden(&tree), [true, false, true]);
    tree.anim_all::<StubTrig>(root); // frame 2: BRANCH 1 -> show all
    assert_eq!(hidden(&tree), [false, false, false]);
    tree.anim_all::<StubTrig>(root); // frame 3: NODE 0 -> hide child only
    assert_eq!(hidden(&tree), [false, true, false]);
    tree.anim_all::<StubTrig>(root); // frame 4: BRANCH 0.3 (<= 0.5) -> hide all
    assert_eq!(hidden(&tree), [true, true, true]);
}

#[test]
fn scale_tracks_floor_tiny_values_at_1e_3() {
    let desc = aobj_desc(
        3.0,
        0,
        vec![
            con_desc(HSD_A_J_SCAX, &[(0.0, 1), (-0.0005, 1), (-0.5, 1)]),
            con_desc(HSD_A_J_SCAY, &[(1e-3, 1), (0.0009999, 1), (2.0, 1)]),
            con_desc(HSD_A_J_SCAZ, &[(-1e-3, 1), (-1e-3, 2)]),
        ],
    );
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    tree.add_anim_all(root, Some(&anim_joint(Some(desc), vec![])), None);
    tree.req_anim_all(root, 0.0);
    tree.anim_all::<StubTrig>(root);
    assert_eq!(tree.scale(root), Vec3::new(1e-3, 1e-3, -1e-3));
    tree.anim_all::<StubTrig>(root);
    assert_eq!(tree.scale(root), Vec3::new(1e-3, 1e-3, -1e-3));
    tree.anim_all::<StubTrig>(root);
    assert_eq!(tree.scale(root), Vec3::new(-0.5, 2.0, -1e-3));
}

#[test]
fn callback_ids_are_recorded_as_events() {
    let iv: i32 = 0x3F80_0005;
    let as_f = f32::from_bits(iv as u32);
    let desc = aobj_desc(
        2.0,
        0,
        vec![
            // Two keys each: a single-key CON stream ends before op_intrp
            // is set and emits the uninitialised value (see anim_fobj.rs).
            con_desc(HSD_A_J_DPTCL, &[(as_f, 1), (as_f, 1)]),
            con_desc(HSD_A_J_JSOUND, &[(as_f, 1), (as_f, 1)]),
            con_desc(HSD_A_J_PTCLTGT, &[(as_f, 1), (as_f, 1)]),
            con_desc(HSD_A_J_SETBYTE0 + 3, &[(as_f, 1), (as_f, 1)]),
            con_desc(HSD_A_J_SETFLOAT0 + 7, &[(0.75, 1), (0.75, 1)]),
            con_desc(HSD_A_J_PATH, &[(1.5, 1), (-2.0, 1)]),
        ],
    );
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    tree.add_anim_all(root, Some(&anim_joint(Some(desc), vec![])), None);
    tree.req_anim_all(root, 0.0);
    tree.anim_all::<StubTrig>(root);
    assert_eq!(
        tree.events,
        vec![
            JObjEvent::DPtcl {
                jobj: root,
                lo: 5,
                hi: 0xFE_0000
            },
            JObjEvent::JSound(iv),
            JObjEvent::PtclTgt {
                jobj: root,
                value: iv
            },
            JObjEvent::SetByte {
                jobj: root,
                ty: 23,
                value: iv
            },
            JObjEvent::SetFloat {
                jobj: root,
                ty: 37,
                value: 0.75
            },
            JObjEvent::Path { jobj: root, t: 1.0 },
        ]
    );
    tree.events.clear();
    tree.anim_all::<StubTrig>(root);
    assert_eq!(
        tree.events.last(),
        Some(&JObjEvent::Path { jobj: root, t: 0.0 })
    );
    // The translation was never touched by PATH (deferred).
    assert_eq!(tree.translation(root), Vec3::ZERO);
}

#[test]
fn robj_style_matrix_ids_write_columns_and_redecompose() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new()
            .position(1.0, 2.0, 3.0)
            .child(JointSpec::new().position(0.0, 5.0, 0.0)),
    );
    let child = tree.child(root).unwrap();
    tree.setup_matrix(child);

    tree.update_func::<StubTrig>(
        child,
        HSD_A_J_MTX_COL3,
        ObjData::Vec(Vec3::new(7.0, 8.0, 9.0)),
    );
    let m = tree.get(child).mtx;
    assert_eq!((m.0[0][3], m.0[1][3], m.0[2][3]), (7.0, 8.0, 9.0));
    // Writing columns does not set dirty (the C touches mtx directly).
    assert!(!tree.mtx_is_dirty(child));
    // A float payload on a column id is ignored (only RObj sends these).
    tree.update_func::<StubTrig>(child, HSD_A_J_MTX_COL0, ObjData::Float(1.0));
    assert_eq!(tree.get(child).mtx, m);

    // 0x38: translate = (parent^-1 * mtx).translation, here mtx is root+
    // (7,8,9) world, so local translate is (6, 6, 6).
    tree.update_func::<StubTrig>(child, HSD_A_J_MTX_TRA, ObjData::Null);
    let mut inv = Mtx::ZERO;
    mtx::hsd_mtx_inverse_concat(&tree.get(root).mtx, &m, &mut inv);
    let mut want = Vec3::ZERO;
    mtx::hsd_mtx_get_translate(&inv, &mut want);
    assert_eq!(tree.translation(child), want);
    assert_eq!(want, Vec3::new(6.0, 6.0, 6.0));

    // 0x39 on a root uses the matrix as-is.
    tree.get_mut(root).mtx = Mtx([
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0, 0.0],
        [0.0, 0.0, 4.0, 0.0],
    ]);
    tree.update_func::<StubTrig>(root, HSD_A_J_MTX_SCA, ObjData::Null);
    let mut want = Vec3::ZERO;
    mtx::hsd_mtx_get_scale(&tree.get(root).mtx, &mut want);
    assert_eq!(tree.scale(root), want);
    assert_eq!(
        tree.translation(root),
        Vec3::new(1.0, 2.0, 3.0),
        "0x39 leaves translate"
    );

    // 0x37 goes through HSD_MtxGetRotation<T>, writing only x, y, z.
    tree.get_mut(root).rotate.w = 0.5;
    tree.update_func::<StubTrig>(root, HSD_A_J_MTX_ROT, ObjData::Null);
    let mut e = Vec3::ZERO;
    mtx::hsd_mtx_get_rotation::<StubTrig>(&tree.get(root).mtx, &mut e);
    let q = tree.rotation(root);
    assert_eq!((q.x, q.y, q.z, q.w), (e.x, e.y, e.z, 0.5));
}

#[test]
fn material_animation_runs_through_the_dobj_list_and_counts() {
    let mat_desc = aobj_desc(
        4.0,
        0,
        vec![
            lin_desc(HSD_A_M_DIFFUSE_R, 0.0, 4, 1.0),
            con_desc(HSD_A_M_ALPHA, &[(0.25, 2), (0.25, 2)]),
        ],
    );
    let mobj = MObj::load(0, Material::default(), None);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new().dobj(vec![DObj::load(Some(mobj.clone())), DObj::load(Some(mobj))]),
    );
    let mj = MatAnimJoint {
        matanim: vec![MatAnim {
            aobjdesc: Some(mat_desc.clone()),
        }],
        children: vec![],
    };
    tree.add_anim_all(root, None, Some(&mj));
    let dobjs = tree.dobj(root).unwrap();
    assert!(dobjs[0].mobj.as_ref().unwrap().aobj.is_some());
    assert!(
        dobjs[1].mobj.as_ref().unwrap().aobj.is_none(),
        "second dobj has no matanim"
    );
    tree.req_anim_all(root, 0.0);

    let mut oracle = AObj::load_desc(&mat_desc);
    oracle.req_anim(0.0);
    for frame in 0..6 {
        let cb = tree.anim_all::<StubTrig>(root);
        let mut ocb = AObjEndCallback::default();
        let mut r = None;
        let mut a = None;
        oracle.interpret_anim(
            &mut |ty, v| match ty {
                HSD_A_M_DIFFUSE_R => r = Some(v),
                HSD_A_M_ALPHA => a = Some(v),
                _ => {}
            },
            &mut ocb,
        );
        assert_eq!(cb, ocb, "frame {frame}: material aobj is counted");
        let mat = tree.dobj(root).unwrap()[0].mobj.as_ref().unwrap().mat;
        if let Some(r) = r {
            // (u8)(255.0 * fv): double product, truncating conversion.
            assert_eq!(
                mat.diffuse.r,
                (255.0f64 * f64::from(r)) as i32 as u8,
                "frame {frame}"
            );
        }
        if let Some(a) = a {
            assert_eq!(mat.alpha, 1.0 - a);
        }
    }
    let mat = tree.dobj(root).unwrap()[0].mobj.as_ref().unwrap().mat;
    assert_eq!(mat.diffuse.r, 255);
    assert_eq!(mat.alpha, 0.75);

    // MOBJ_ANIM (bit 2) is the DObj/MObj flag; bit 0 is the joint's.
    tree.remove_anim_all_by_flags(root, 1);
    assert!(tree.dobj(root).unwrap()[0]
        .mobj
        .as_ref()
        .unwrap()
        .aobj
        .is_some());
    tree.remove_anim_all_by_flags(root, 4);
    assert!(tree.dobj(root).unwrap()[0]
        .mobj
        .as_ref()
        .unwrap()
        .aobj
        .is_none());
}

#[test]
fn ptcl_and_spline_joints_have_no_dobj_list() {
    let mobj = MObj::load(0, Material::default(), None);
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new()
            .flags(JOBJ_PTCL)
            .dobj(vec![DObj::load(Some(mobj.clone()))]),
    );
    assert_eq!(tree.dobj(root), None);
    assert!(
        tree.get(root).dobj.is_empty(),
        "JObjLoad does not load dobjdesc for ptcl"
    );
    tree.add_dobj(root, DObj::load(Some(mobj.clone())));
    assert!(tree.get(root).dobj.is_empty());

    let plain = tree.load_joint(&JointSpec::new());
    tree.add_dobj(plain, DObj::load(None));
    tree.add_dobj(plain, DObj::load(Some(mobj)));
    let list = tree.dobj(plain).unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0].mobj.is_some(), "HSD_JObjAddDObj prepends");
}
