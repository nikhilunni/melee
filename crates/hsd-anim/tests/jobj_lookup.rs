//! Tree shape: the `JObjLoad` allocation order, the `ftParts` depth-first
//! bone index walk (`ftparts.c:405-441`), `HSD_JObjWalkTree`, and the
//! link surgery (`AddChild`, `Reparent`, `AddNext`, `GetPrev`) with the
//! `JOBJ_ROOT_*` bookkeeping.

use hsd_anim::jobj::*;

/// root
///  +- a
///  |   +- a1
///  |   +- a2
///  |       +- a2x
///  +- b
///  |   +- b1
///  +- c
///      +- c1
fn sample() -> (JObjTree, Vec<JObjId>) {
    let spec = JointSpec::new()
        .id(0)
        .child(
            JointSpec::new()
                .id(1)
                .child(JointSpec::new().id(2))
                .child(JointSpec::new().id(3).child(JointSpec::new().id(4))),
        )
        .child(JointSpec::new().id(5).child(JointSpec::new().id(6)))
        .child(JointSpec::new().id(7).child(JointSpec::new().id(8)));
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec);
    let ids: Vec<JObjId> = tree.depth_first(root).collect();
    (tree, ids)
}

#[test]
fn load_allocates_in_preorder_and_links_like_the_c() {
    let (tree, ids) = sample();
    assert_eq!(tree.len(), 9);
    // Allocation order is self, child chain, next chain: pre-order.
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(*id, JObjId(i));
        assert_eq!(tree.get(*id).id, i as u32);
    }
    let [root, a, a1, a2, a2x, b, b1, c, c1] = ids[..] else {
        panic!()
    };
    assert_eq!(tree.child(root), Some(a));
    assert_eq!(tree.next(a), Some(b));
    assert_eq!(tree.next(b), Some(c));
    assert_eq!(tree.next(c), None);
    assert_eq!(tree.child(a), Some(a1));
    assert_eq!(tree.next(a1), Some(a2));
    assert_eq!(tree.child(a2), Some(a2x));
    assert_eq!(tree.child(b), Some(b1));
    assert_eq!(tree.child(c), Some(c1));
    for id in [a, b, c] {
        assert_eq!(tree.parent(id), Some(root));
    }
    assert_eq!(tree.parent(a2x), Some(a2));
    assert_eq!(tree.parent(root), None);
    assert_eq!(tree.children(root).collect::<Vec<_>>(), [a, b, c]);
    assert_eq!(tree.children(a1).count(), 0);
}

#[test]
fn bone_index_follows_the_ftparts_walk() {
    let (tree, ids) = sample();
    let root = ids[0];
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(tree.bone(root, i), Some(*id));
    }
    assert_eq!(tree.bone(root, 9), None);
}

#[test]
fn bones_resolve_several_indices_like_bone() {
    let (tree, ids) = sample();
    let root = ids[0];
    let indices = [Some(7), None, Some(2), Some(9), Some(2), Some(0)];
    let expected = indices.map(|index| index.and_then(|i| tree.bone(root, i)));
    assert_eq!(tree.bones(root, indices), expected);
    assert_eq!(
        expected,
        [
            Some(ids[7]),
            None,
            Some(ids[2]),
            None,
            Some(ids[2]),
            Some(root)
        ]
    );
    assert_eq!(tree.bones(root, [None::<usize>; 3]), [None; 3]);
}

#[test]
fn instance_nodes_hide_their_children_from_the_walk() {
    let (mut tree, ids) = sample();
    let root = ids[0];
    let b = ids[5];
    tree.set_flags(b, JOBJ_INSTANCE);
    let walk: Vec<JObjId> = tree.depth_first(root).collect();
    assert_eq!(walk, [0, 1, 2, 3, 4, 5, 7, 8].map(JObjId));
    assert_eq!(tree.bone(root, 6), Some(JObjId(7)));
}

#[test]
fn walk_from_a_subtree_root_climbs_through_its_ancestors_like_the_c() {
    // ftParts always starts at the true root; the loop has no notion of a
    // subtree boundary, so starting at `a` continues into b and c.
    let (tree, ids) = sample();
    let walk: Vec<JObjId> = tree.depth_first(ids[1]).collect();
    assert_eq!(walk, ids[1..]);
    // From a leaf with no later siblings anywhere: just itself.
    assert_eq!(tree.depth_first(ids[8]).collect::<Vec<_>>(), [ids[8]]);
    // From a2x: climbs to a2 (no next), a (next = b) -> b, b1, c, c1.
    assert_eq!(
        tree.depth_first(ids[4]).collect::<Vec<_>>(),
        [4, 5, 6, 7, 8].map(JObjId)
    );
}

#[test]
fn walk_tree_reports_first_child_versus_sibling() {
    let (tree, ids) = sample();
    let mut got = Vec::new();
    tree.walk_tree(ids[0], &mut |id, kind| got.push((id.0, kind)));
    assert_eq!(
        got,
        [
            (0, 0),
            (1, 1),
            (2, 1),
            (3, 2),
            (4, 1),
            (5, 2),
            (6, 1),
            (7, 2),
            (8, 1)
        ]
    );
}

#[test]
fn add_child_appends_and_get_prev_finds_siblings() {
    let mut tree = JObjTree::new();
    let root = tree.alloc();
    let a = tree.alloc();
    let b = tree.alloc();
    let c = tree.alloc();
    tree.add_child(root, a);
    tree.add_child(root, b);
    tree.add_child(root, c);
    assert_eq!(tree.children(root).collect::<Vec<_>>(), [a, b, c]);
    assert_eq!(tree.get_prev(a), None);
    assert_eq!(tree.get_prev(b), Some(a));
    assert_eq!(tree.get_prev(c), Some(b));
    assert_eq!(tree.get_prev(root), None);
}

#[test]
#[should_panic(expected = "orphan")]
fn add_child_rejects_a_node_with_a_parent() {
    let mut tree = JObjTree::new();
    let root = tree.alloc();
    let other = tree.alloc();
    let a = tree.alloc();
    tree.add_child(root, a);
    tree.add_child(other, a);
}

#[test]
fn reparent_unlinks_and_returns_the_next_sibling() {
    let (mut tree, ids) = sample();
    let [root, a, _a1, _a2, _a2x, b, b1, c, _c1] = ids[..] else {
        panic!()
    };
    // Move b (middle child) under c.
    assert_eq!(tree.reparent(b, Some(c)), Some(c));
    assert_eq!(tree.children(root).collect::<Vec<_>>(), [a, c]);
    assert_eq!(tree.children(c).collect::<Vec<_>>(), [ids[8], b]);
    assert_eq!(tree.parent(b), Some(c));
    assert_eq!(tree.next(b), None);
    assert_eq!(tree.child(b), Some(b1), "subtree comes along");
    // Detach a (first child) with no new parent.
    assert_eq!(tree.reparent(a, None), Some(c));
    assert_eq!(tree.children(root).collect::<Vec<_>>(), [c]);
    assert_eq!(tree.parent(a), None);
    // Reparenting a root is a plain add.
    assert_eq!(tree.reparent(a, Some(root)), None);
    assert_eq!(tree.children(root).collect::<Vec<_>>(), [c, a]);
    // The walk reflects the new shape.
    let walk: Vec<u32> = tree.depth_first(root).map(|i| tree.get(i).id).collect();
    assert_eq!(walk, [0, 7, 8, 5, 6, 1, 2, 3, 4]);
}

#[test]
fn add_next_inserts_a_node_between_parent_and_sibling_chain() {
    let (mut tree, ids) = sample();
    let [root, a, _a1, _a2, _a2x, b, _b1, c, _c1] = ids[..] else {
        panic!()
    };
    let n = tree.alloc();
    tree.add_next(b, n);
    // n is now root's only child and owns the whole former chain.
    assert_eq!(tree.children(root).collect::<Vec<_>>(), [n]);
    assert_eq!(tree.children(n).collect::<Vec<_>>(), [a, b, c]);
    for id in [a, b, c] {
        assert_eq!(tree.parent(id), Some(n));
    }

    // On a root (no parent) the node itself becomes n's child.
    let mut t2 = JObjTree::new();
    let r = t2.alloc();
    let n2 = t2.alloc();
    t2.add_next(r, n2);
    assert_eq!(t2.parent(r), Some(n2));
    assert_eq!(t2.child(n2), Some(r));
    assert_eq!(t2.parent(n2), None);
}

#[test]
fn trsp_bits_bubble_up_on_add_child_and_recalc_on_reparent() {
    let mut tree = JObjTree::new();
    let root = tree.alloc();
    let mid = tree.alloc();
    let leaf = tree.alloc();
    // Bits 18..20 shifted by 10 land on ROOT_OPA/XLU/TEXEDGE.
    tree.get_mut(leaf).flags |= JOBJ_UNK_B19; // -> JOBJ_ROOT_XLU
    tree.add_child(root, mid);
    assert_eq!(tree.flags(root) & JOBJ_ROOT_MASK, 0);
    tree.add_child(mid, leaf);
    assert_eq!(tree.flags(mid) & JOBJ_ROOT_MASK, JOBJ_ROOT_XLU);
    assert_eq!(tree.flags(root) & JOBJ_ROOT_MASK, JOBJ_ROOT_XLU);
    // A child that already carries a ROOT bit passes it as-is too.
    let other = tree.alloc();
    tree.get_mut(other).flags |= JOBJ_ROOT_OPA;
    tree.add_child(root, other);
    assert_eq!(
        tree.flags(root) & JOBJ_ROOT_MASK,
        JOBJ_ROOT_XLU | JOBJ_ROOT_OPA
    );

    // Removing the leaf recomputes mid from its (now absent) children.
    tree.reparent(leaf, None);
    assert_eq!(tree.flags(mid) & JOBJ_ROOT_MASK, 0);
    // RecalcParentTrspBits walks `next`, not `parent` (jobj.c:812): root
    // keeps its bits, while mid's sibling `other`, which has no children,
    // loses the ROOT_OPA it carried itself. Kept literally.
    assert_eq!(
        tree.flags(root) & JOBJ_ROOT_MASK,
        JOBJ_ROOT_XLU | JOBJ_ROOT_OPA
    );
    assert_eq!(tree.flags(other) & JOBJ_ROOT_MASK, 0);
    tree.recalc_parent_trsp_bits(Some(root));
    assert_eq!(tree.flags(root) & JOBJ_ROOT_MASK, 0);
}

#[test]
fn instance_spec_does_not_load_children() {
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new()
            .flags(JOBJ_INSTANCE)
            .child(JointSpec::new()),
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree.child(root), None);
}

#[test]
fn reset_rst_reloads_srt_positionally_and_dirties() {
    let spec = JointSpec::new()
        .position(1.0, 1.0, 1.0)
        .child(JointSpec::new().position(2.0, 2.0, 2.0))
        .child(JointSpec::new().position(3.0, 3.0, 3.0));
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&spec);
    let a = tree.child(root).unwrap();
    let b = tree.next(a).unwrap();
    tree.setup_matrix(a);
    tree.setup_matrix(b);
    tree.set_translate_x(root, 9.0);
    tree.set_translate_x(a, 9.0);
    tree.set_translate_x(b, 9.0);
    tree.setup_matrix(a);
    tree.setup_matrix(b);

    // Desc with only one child: b is left alone.
    let short = JointSpec::new()
        .position(1.0, 1.0, 1.0)
        .rotation(0.5, 0.0, 0.0)
        .child(JointSpec::new().position(2.0, 2.0, 2.0));
    tree.reset_rst(root, &short);
    assert_eq!(tree.translation_x(root), 1.0);
    assert_eq!(tree.rotation_x(root), 0.5);
    assert_eq!(tree.translation_x(a), 2.0);
    assert_eq!(tree.translation_x(b), 9.0);
    assert!(tree.mtx_is_dirty(root) && tree.mtx_is_dirty(a) && tree.mtx_is_dirty(b));
}
