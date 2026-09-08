//! JObj world-matrix composition: two- and three-level hierarchies with
//! known transforms, the expected matrices computed here with the same
//! `crate::mtx` op sequence `HSD_JObjMakeMatrix` uses (`jobj.c:138-196`),
//! compared bit for bit.
//!
//! A native-C oracle for `jobj.c` is impractical: the file depends on the
//! HSD class system (`hsdNew`, method tables), the object allocator, the id
//! table and RObj/DObj/spline, none of which can be compiled in isolation
//! the way `mtx.c` and `fobj.c` were. The expected values below therefore
//! follow the C operation order through the already-oracled `mtx` kernels.

use hsd_anim::jobj::*;
use hsd_anim::mtx;
use hsd_anim::quat::Quaternion;
use hsd_types::{Mtx, Vec3};

fn assert_mtx_bits(got: &Mtx, want: &Mtx, what: &str) {
    for r in 0..3 {
        for c in 0..4 {
            assert_eq!(
                got.0[r][c].to_bits(),
                want.0[r][c].to_bits(),
                "{what}: [{r}][{c}] got {} want {}",
                got.0[r][c],
                want.0[r][c]
            );
        }
    }
}

fn vmul(a: &Vec3, b: &Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

/// `HSD_MtxSRT` local matrix, then `PSMTXConcat(parent, local, out)`.
fn expect_world(
    scale: &Vec3,
    rot: &Vec3,
    trans: &Vec3,
    parent_scl: Option<&Vec3>,
    parent_world: Option<&Mtx>,
) -> Mtx {
    let mut m = Mtx::ZERO;
    mtx::hsd_mtx_srt(&mut m, scale, rot, trans, parent_scl);
    match parent_world {
        Some(p) => {
            let local = m;
            mtx::mtx_concat(p, &local, &mut m);
            m
        }
        None => m,
    }
}

const S0: Vec3 = Vec3::new(1.5, 0.75, 2.0);
const R0: Vec3 = Vec3::new(0.3, -1.1, 0.7);
const T0: Vec3 = Vec3::new(10.0, -3.5, 0.25);
const S1: Vec3 = Vec3::new(0.5, 1.25, 1.0);
const R1: Vec3 = Vec3::new(-0.2, 0.4, 1.9);
const T1: Vec3 = Vec3::new(1.0, 2.0, -4.0);
const S2: Vec3 = Vec3::new(2.5, 0.2, 1.75);
const R2: Vec3 = Vec3::new(1.0, 0.1, -0.6);
const T2: Vec3 = Vec3::new(-0.5, 7.0, 3.0);

fn spec(s: Vec3, r: Vec3, t: Vec3) -> JointSpec {
    JointSpec::new()
        .scale(s.x, s.y, s.z)
        .rotation(r.x, r.y, r.z)
        .position(t.x, t.y, t.z)
}

#[test]
fn root_alone_is_srt_without_compensation() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec(S0, R0, T0));
    assert!(tree.mtx_is_dirty(root));
    assert_eq!(tree.get(root).mtx, Mtx::IDENTITY, "JObjLoad sets identity");

    let want = expect_world(&S0, &R0, &T0, None, None);
    let got = *tree.get_mtx(root);
    assert_mtx_bits(&got, &want, "root");
    assert!(!tree.mtx_is_dirty(root));
    // Accumulated scale is the joint's own scale when there is no parent.
    assert_eq!(tree.get(root).scl, Some(S0));
}

#[test]
fn two_level_child_uses_parent_scl_then_concat() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec(S0, R0, T0).child(spec(S1, R1, T1)));
    let child = tree.child(root).unwrap();

    // Asking for the child sets up the parent first (jobj.c:142).
    let got = *tree.get_mtx(child);
    assert!(!tree.mtx_is_dirty(root));

    let root_world = expect_world(&S0, &R0, &T0, None, None);
    let want = expect_world(&S1, &R1, &T1, Some(&S0), Some(&root_world));
    assert_mtx_bits(&got, &want, "child");
    assert_mtx_bits(&tree.get(root).mtx, &root_world, "root via child");
    assert_eq!(tree.get(child).scl, Some(vmul(&S1, &S0)));
}

#[test]
fn three_level_accumulates_scale_componentwise() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec(S0, R0, T0).child(spec(S1, R1, T1).child(spec(S2, R2, T2))));
    let child = tree.child(root).unwrap();
    let grand = tree.child(child).unwrap();

    let got = *tree.get_mtx(grand);

    let root_world = expect_world(&S0, &R0, &T0, None, None);
    let child_world = expect_world(&S1, &R1, &T1, Some(&S0), Some(&root_world));
    let scl1 = vmul(&S1, &S0);
    let want = expect_world(&S2, &R2, &T2, Some(&scl1), Some(&child_world));
    assert_mtx_bits(&got, &want, "grandchild");
    assert_eq!(tree.get(grand).scl, Some(vmul(&S2, &scl1)));
}

#[test]
fn classical_scale_copies_parent_scl_instead_of_multiplying() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &spec(S0, R0, T0).child(
            spec(S1, R1, T1)
                .flags(JOBJ_CLASSICAL_SCALE)
                .child(spec(S2, R2, T2)),
        ),
    );
    let child = tree.child(root).unwrap();
    let grand = tree.child(child).unwrap();

    let got = *tree.get_mtx(grand);

    let root_world = expect_world(&S0, &R0, &T0, None, None);
    // The child still compensates by the parent's scl in its own SRT...
    let child_world = expect_world(&S1, &R1, &T1, Some(&S0), Some(&root_world));
    // ...but hands its parent's scl, not scale*scl, to the grandchild.
    assert_eq!(tree.get(child).scl, Some(S0));
    let want = expect_world(&S2, &R2, &T2, Some(&S0), Some(&child_world));
    assert_mtx_bits(&got, &want, "grandchild under classical child");
}

#[test]
fn classical_scale_root_has_no_scl_so_child_gets_no_compensation() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &spec(S0, R0, T0)
            .flags(JOBJ_CLASSICAL_SCALE)
            .child(spec(S1, R1, T1)),
    );
    let child = tree.child(root).unwrap();

    let got = *tree.get_mtx(child);
    assert_eq!(tree.get(root).scl, None);

    let root_world = expect_world(&S0, &R0, &T0, None, None);
    let want = expect_world(&S1, &R1, &T1, None, Some(&root_world));
    assert_mtx_bits(&got, &want, "child of classical root");
    assert_eq!(tree.get(child).scl, Some(S1));
}

#[test]
fn quaternion_flag_selects_srt_quat() {
    let mut tree = JObjTree::new();
    let root = tree
        .load_joint(&spec(S0, R0, T0).child(spec(S1, Vec3::ZERO, T1).flags(JOBJ_USE_QUATERNION)));
    let child = tree.child(root).unwrap();
    let q = Quaternion::new(0.1, -0.2, 0.3, 0.9);
    tree.set_rotation(child, &q);

    let got = *tree.get_mtx(child);

    let root_world = expect_world(&S0, &R0, &T0, None, None);
    let mut want = Mtx::ZERO;
    mtx::hsd_mtx_srt_quat(&mut want, &S1, &q, &T1, Some(&S0));
    let local = want;
    mtx::mtx_concat(&root_world, &local, &mut want);
    assert_mtx_bits(&got, &want, "quaternion child");
}

#[test]
fn quaternion_root_without_parent_scl() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec(S0, Vec3::ZERO, T0).flags(JOBJ_USE_QUATERNION));
    // JObjLoad leaves w at 0: a degenerate quaternion, kept literally.
    assert_eq!(tree.rotation(root), Quaternion::new(0.0, 0.0, 0.0, 0.0));
    let q = Quaternion::new(0.0, 0.6, 0.0, 0.8);
    tree.set_rotation(root, &q);
    let got = *tree.get_mtx(root);
    let mut want = Mtx::ZERO;
    mtx::hsd_mtx_srt_quat(&mut want, &S0, &q, &T0, None);
    assert_mtx_bits(&got, &want, "quaternion root");
}

#[test]
fn siblings_each_concat_against_the_same_parent() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &spec(S0, R0, T0)
            .child(spec(S1, R1, T1))
            .child(spec(S2, R2, T2)),
    );
    let a = tree.child(root).unwrap();
    let b = tree.next(a).unwrap();
    let root_world = expect_world(&S0, &R0, &T0, None, None);
    let want_a = expect_world(&S1, &R1, &T1, Some(&S0), Some(&root_world));
    let want_b = expect_world(&S2, &R2, &T2, Some(&S0), Some(&root_world));
    let got_b = *tree.get_mtx(b);
    let got_a = *tree.get_mtx(a);
    assert_mtx_bits(&got_a, &want_a, "sibling a");
    assert_mtx_bits(&got_b, &want_b, "sibling b");
}

#[test]
fn user_def_mtx_is_used_as_is_by_children() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &spec(S0, R0, T0).child(
            spec(S1, R1, T1)
                .flags(JOBJ_USER_DEF_MTX)
                .child(spec(S2, R2, T2)),
        ),
    );
    let child = tree.child(root).unwrap();
    let grand = tree.child(child).unwrap();

    let custom = Mtx([
        [0.0, -1.0, 0.0, 5.0],
        [1.0, 0.0, 0.0, 6.0],
        [0.0, 0.0, 1.0, 7.0],
    ]);
    tree.copy_mtx(child, &custom);
    // HSD_JObjMtxIsDirty is false for user-defined matrices even though bit 6 is set.
    assert!(tree.flags(child) & JOBJ_MTX_DIRTY != 0);
    assert!(!tree.mtx_is_dirty(child));

    let got = *tree.get_mtx(grand);
    // The child was never made, so it has no scl; the grandchild gets no
    // compensation and concatenates against the custom matrix.
    assert_eq!(tree.get(child).scl, None);
    assert_mtx_bits(&tree.get(child).mtx, &custom, "custom untouched");
    let want = expect_world(&S2, &R2, &T2, None, Some(&custom));
    assert_mtx_bits(&got, &want, "grandchild of user-def");
    // The root was not needed and stays dirty.
    assert!(tree.mtx_is_dirty(root));
}

#[test]
fn setup_matrix_sub_clears_dirty_even_with_user_def_mtx_flag() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec(S0, R0, T0).flags(JOBJ_USER_DEF_MTX));
    // Forced rebuild: make_mtx runs, dirty cleared once, IK arm skipped.
    tree.setup_matrix_sub(root);
    assert!(tree.flags(root) & JOBJ_MTX_DIRTY == 0);
    let want = expect_world(&S0, &R0, &T0, None, None);
    assert_mtx_bits(&tree.get(root).mtx, &want, "forced");
}

#[test]
fn ik_joint_flags_do_not_change_the_matrix_without_robj() {
    for f in [JOBJ_JOINT1, JOBJ_JOINT2, JOBJ_EFFECTOR] {
        let mut tree = JObjTree::new();
        let root = tree.load_joint(&spec(S0, R0, T0).child(spec(S1, R1, T1).flags(f)));
        let child = tree.child(root).unwrap();
        let got = *tree.get_mtx(child);
        let root_world = expect_world(&S0, &R0, &T0, None, None);
        let want = expect_world(&S1, &R1, &T1, Some(&S0), Some(&root_world));
        assert_mtx_bits(&got, &want, "ik-flagged child");
        assert!(!tree.mtx_is_dirty(child));
    }
}

#[test]
fn setters_feed_the_next_setup() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec(S0, R0, T0).child(spec(S1, R1, T1)));
    let child = tree.child(root).unwrap();
    let _ = tree.get_mtx(child);

    tree.set_translate(root, &Vec3::new(-1.0, -2.0, -3.0));
    tree.set_rotation_y(child, 0.25);
    tree.add_scale_z(child, 0.5);
    tree.add_translation_x(child, 1.0);
    assert_eq!(tree.translation(root), Vec3::new(-1.0, -2.0, -3.0));
    assert_eq!(tree.rotation_y(child), 0.25);
    assert_eq!(tree.scale_z(child), S1.z + 0.5);
    assert_eq!(tree.translation_x(child), T1.x + 1.0);

    let got = *tree.get_mtx(child);
    let t0 = Vec3::new(-1.0, -2.0, -3.0);
    let r1 = Vec3::new(R1.x, 0.25, R1.z);
    let s1 = Vec3::new(S1.x, S1.y, S1.z + 0.5);
    let t1 = Vec3::new(T1.x + 1.0, T1.y, T1.z);
    let root_world = expect_world(&S0, &R0, &t0, None, None);
    let want = expect_world(&s1, &r1, &t1, Some(&S0), Some(&root_world));
    assert_mtx_bits(&got, &want, "after setters");
}
