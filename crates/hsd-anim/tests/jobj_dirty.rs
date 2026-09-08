//! `JOBJ_MTX_DIRTY` propagation: the `HSD_JObjSetMtxDirty` macro and
//! `HSD_JObjSetMtxDirtySub` (`jobj.c:1445`), the `MTX_INDEP_*` and
//! `USER_DEF_MTX` exemptions, `HSD_JObjCheckDepend` (`jobj.c:30`), and the
//! flag setters' `JOBJ_CLASSICAL_SCALE` side effect.

use hsd_anim::jobj::*;
use hsd_types::Vec3;

fn chain(child_flags: u32, grand_flags: u32) -> (JObjTree, JObjId, JObjId, JObjId) {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new().position(1.0, 0.0, 0.0).child(
            JointSpec::new()
                .flags(child_flags)
                .position(0.0, 1.0, 0.0)
                .child(JointSpec::new().flags(grand_flags).position(0.0, 0.0, 1.0)),
        ),
    );
    let child = tree.child(root).unwrap();
    let grand = tree.child(child).unwrap();
    (tree, root, child, grand)
}

fn dirty(tree: &JObjTree, ids: &[JObjId]) -> Vec<bool> {
    ids.iter().map(|&i| tree.mtx_is_dirty(i)).collect()
}

#[test]
fn fresh_tree_is_dirty_and_setup_of_leaf_cleans_the_chain() {
    let (mut tree, root, child, grand) = chain(0, 0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, true, true]);
    tree.setup_matrix(grand);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, false]);
}

#[test]
fn setup_of_middle_node_leaves_leaf_dirty() {
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.setup_matrix(child);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, true]);
}

#[test]
fn srt_setter_dirties_the_subtree() {
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.setup_matrix(grand);
    tree.set_translate_x(root, 5.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, true, true]);

    tree.setup_matrix(grand);
    tree.set_rotation_z(child, 0.5);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, true, true]);

    tree.setup_matrix(grand);
    tree.add_scale_y(grand, 0.5);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, true]);
}

#[test]
fn already_dirty_node_stops_the_walk() {
    // Sub only recurses into children that are not already dirty, so a
    // clean grandchild under a dirty child is left alone by a root change.
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.setup_matrix(grand);
    tree.set_translate_x(child, 2.0); // child, grand dirty
    tree.setup_matrix(grand); // all clean
    tree.get_mut(child).flags |= JOBJ_MTX_DIRTY; // child dirty by hand, grand clean
    tree.set_translate_x(root, 3.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, true, false]);
}

#[test]
fn indep_parent_child_is_not_dirtied_by_its_parent() {
    let (mut tree, root, child, grand) = chain(JOBJ_MTX_INDEP_PARENT, 0);
    tree.setup_matrix(grand);
    tree.set_translate_x(root, 5.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, false, false]);
    // But its own setters still work and reach its children.
    tree.set_translate_x(child, 5.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, true, true]);
}

#[test]
fn indep_srt_setters_do_not_dirty() {
    let (mut tree, root, child, grand) = chain(JOBJ_MTX_INDEP_SRT, 0);
    tree.setup_matrix(grand);
    tree.set_translate(child, &Vec3::new(9.0, 9.0, 9.0));
    tree.set_scale_x(child, 3.0);
    tree.add_rotation_x(child, 1.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, false]);
    // The values did change; the matrix is simply not rebuilt.
    assert_eq!(tree.translation(child), Vec3::new(9.0, 9.0, 9.0));
    // An explicit dirty still propagates.
    tree.set_mtx_dirty(child);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, true, true]);
}

#[test]
fn user_def_mtx_is_never_dirty_but_still_forwards() {
    let (mut tree, root, child, grand) = chain(JOBJ_USER_DEF_MTX, 0);
    // Setting up the grandchild does not set up the root: make_mtx stops at
    // the user-defined child, which is never "dirty".
    tree.setup_matrix(grand);
    assert!(tree.mtx_is_dirty(root));
    tree.setup_matrix(root);
    assert!(!tree.mtx_is_dirty(child));
    assert!(
        tree.flags(child) & JOBJ_MTX_DIRTY != 0,
        "bit 6 from JObjInit never cleared"
    );
    // The macro sees a "clean" node and runs the sub, which recurses.
    tree.set_translate_x(root, 1.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, false, true]);
    // setup_matrix on the user-def node is a no-op: its mtx stays identity.
    tree.setup_matrix(child);
    assert_eq!(tree.get(child).mtx, hsd_types::Mtx::IDENTITY);
}

#[test]
fn instance_node_does_not_forward_dirty_to_its_child() {
    // JObjLoad does not load children under JOBJ_INSTANCE, so set the flag
    // after building the chain.
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.setup_matrix(grand);
    tree.set_flags(child, JOBJ_INSTANCE);
    tree.set_translate_x(root, 1.0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, true, false]);
}

#[test]
fn check_depend_redirties_a_clean_child_of_a_dirty_parent() {
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.setup_matrix(grand);
    // Clean parent: nothing happens.
    tree.check_depend(grand);
    assert!(!tree.mtx_is_dirty(grand));
    tree.get_mut(root).flags |= JOBJ_MTX_DIRTY; // parent dirty, child not told
    tree.check_depend(child);
    assert_eq!(dirty(&tree, &[root, child, grand]), [true, true, false]);
    // Now the grandchild's parent carries the bit, so it follows on its turn.
    tree.check_depend(grand);
    assert!(tree.mtx_is_dirty(grand));
}

#[test]
fn check_depend_user_def_mtx_needs_parent_dirty_and_not_indep_parent() {
    let (mut tree, root, child, _grand) = chain(JOBJ_USER_DEF_MTX, 0);
    tree.get_mut(child).flags &= !JOBJ_MTX_DIRTY;
    tree.setup_matrix(root);
    tree.check_depend(child);
    assert!(tree.flags(child) & JOBJ_MTX_DIRTY == 0, "clean parent");
    tree.get_mut(root).flags |= JOBJ_MTX_DIRTY;
    tree.check_depend(child);
    assert!(
        tree.flags(child) & JOBJ_MTX_DIRTY != 0,
        "dirty parent sets the bit"
    );

    let (mut tree, root, child, _grand) = chain(JOBJ_USER_DEF_MTX | JOBJ_MTX_INDEP_PARENT, 0);
    tree.get_mut(child).flags &= !JOBJ_MTX_DIRTY;
    tree.get_mut(root).flags |= JOBJ_MTX_DIRTY;
    tree.check_depend(child);
    assert!(
        tree.flags(child) & JOBJ_MTX_DIRTY == 0,
        "INDEP_PARENT exempts"
    );
}

#[test]
fn check_depend_ik_joints_are_always_dirty() {
    for f in [JOBJ_JOINT1, JOBJ_JOINT2, JOBJ_EFFECTOR] {
        let (mut tree, _root, child, grand) = chain(f, 0);
        tree.setup_matrix(grand);
        assert!(!tree.mtx_is_dirty(child));
        tree.check_depend(child);
        assert!(tree.mtx_is_dirty(child), "flags {f:#x}");
        assert!(!tree.mtx_is_dirty(grand), "check_depend does not propagate");
    }
}

#[test]
fn flag_setters_dirty_only_when_classical_scale_toggles() {
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.setup_matrix(grand);
    tree.set_flags(child, JOBJ_HIDDEN);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, false]);
    tree.set_flags(child, JOBJ_CLASSICAL_SCALE);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, true, true]);
    tree.setup_matrix(grand);
    // Setting a flag that is already set is not a toggle.
    tree.set_flags(child, JOBJ_CLASSICAL_SCALE);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, false]);
    // HSD_JObjClearFlags uses the same `(flags ^ arg) & CLASSICAL_SCALE`
    // test (jobj.c:1022): clearing a *set* bit XORs to zero and does not
    // dirty, while "clearing" an already-clear bit does. Kept literally.
    tree.clear_flags(child, JOBJ_CLASSICAL_SCALE);
    assert_eq!(tree.flags(child) & JOBJ_CLASSICAL_SCALE, 0);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, false, false]);
    tree.clear_flags(child, JOBJ_CLASSICAL_SCALE);
    assert_eq!(dirty(&tree, &[root, child, grand]), [false, true, true]);
    assert_eq!(tree.flags(child) & JOBJ_HIDDEN, JOBJ_HIDDEN);
}

#[test]
fn flags_all_walks_children_and_stops_at_instance() {
    let (mut tree, root, child, grand) = chain(0, 0);
    tree.set_flags_all(root, JOBJ_HIDDEN);
    assert!([root, child, grand]
        .iter()
        .all(|&i| tree.flags(i) & JOBJ_HIDDEN != 0));
    tree.clear_flags_all(child, JOBJ_HIDDEN);
    assert!(tree.flags(root) & JOBJ_HIDDEN != 0);
    assert!(tree.flags(child) & JOBJ_HIDDEN == 0);
    assert!(tree.flags(grand) & JOBJ_HIDDEN == 0);

    tree.set_flags(child, JOBJ_INSTANCE);
    tree.set_flags_all(root, JOBJ_LIGHTING);
    assert!(tree.flags(child) & JOBJ_LIGHTING != 0);
    assert!(
        tree.flags(grand) & JOBJ_LIGHTING == 0,
        "instance blocks the walk"
    );
}

#[test]
fn get_mtx_rebuilds_only_when_dirty() {
    let (mut tree, root, child, _grand) = chain(0, 0);
    let first = *tree.get_mtx(child);
    // Poke the stored matrix; a clean node returns it unchanged.
    tree.get_mut(child).mtx.0[0][3] = 123.0;
    assert_eq!(tree.get_mtx(child).0[0][3], 123.0);
    // Dirtying the parent rebuilds it.
    tree.set_translate_x(root, tree.translation_x(root));
    assert_eq!(*tree.get_mtx(child), first);
}
