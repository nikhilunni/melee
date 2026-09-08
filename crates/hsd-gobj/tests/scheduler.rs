//! Behavioural tests for the GObj scheduler. Each test names the C behaviour
//! it pins (file:line in the decomp's src/sysdolphin/baselib/).

use std::cell::RefCell;
use std::rc::Rc;

use hsd_gobj::consts::{class, gxlink, plink, proc_prio};
use hsd_gobj::{GObjId, InsertWhere, World, WorldConfig, GXLINK_NONE, OBJ_NONE, USER_DATA_NONE};

type Log = Rc<RefCell<Vec<String>>>;

fn log() -> Log {
    Rc::new(RefCell::new(Vec::new()))
}

fn logging(l: &Log, tag: &'static str) -> impl FnMut(&mut World, GObjId) + 'static {
    let l = l.clone();
    move |_, _| l.borrow_mut().push(tag.to_string())
}

fn plink_ids(w: &World, p_link: u8) -> Vec<GObjId> {
    w.iter_plink(p_link).collect()
}

fn melee() -> World {
    World::melee().0
}

// ---------------------------------------------------------------------------
// Creation and destruction
// ---------------------------------------------------------------------------

#[test]
fn create_initialises_fields_like_creategobj() {
    // gobjplink.c:66-80
    let mut w = melee();
    let g = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let o = w.gobj(g);
    assert_eq!(o.classifier(), class::FIGHTER);
    assert_eq!(o.p_link(), plink::FIGHTER);
    assert_eq!(o.p_priority(), 0);
    assert_eq!(o.gx_link(), GXLINK_NONE);
    assert_eq!(o.render_priority(), 0);
    assert_eq!(o.obj_kind(), OBJ_NONE);
    assert_eq!(o.user_data_kind(), USER_DATA_NONE);
    assert_eq!(o.next(), None);
    assert_eq!(o.prev(), None);
    assert_eq!(o.first_proc(), None);
    assert!(!o.has_render_cb());
    assert_eq!(w.plink_head(plink::FIGHTER), Some(g));
    assert_eq!(w.plink_tail(plink::FIGHTER), Some(g));
}

#[test]
fn equal_priority_gobjs_append_in_creation_order() {
    // gobj_first_lower_prio (gobjplink.c:36-43): fighters list is P1, P2, ...
    // head to tail, the order harness/dolphin/walk.py reads.
    let mut w = melee();
    let p1 = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let p2 = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let p3 = w.create(class::FIGHTER, plink::FIGHTER, 0);
    assert_eq!(plink_ids(&w, plink::FIGHTER), vec![p1, p2, p3]);
    assert_eq!(w.gobj(p1).prev(), None);
    assert_eq!(w.gobj(p2).prev(), Some(p1));
    assert_eq!(w.gobj(p3).next(), None);
    assert_eq!(w.plink_tail(plink::FIGHTER), Some(p3));
    // Other lists untouched.
    assert!(plink_ids(&w, plink::ITEM).is_empty());
}

#[test]
fn lower_priority_number_sorts_earlier_and_equal_goes_after() {
    // gobjplink.c:38-42: scan from tail while prio > new; insert after.
    let mut w = melee();
    let a = w.create(1, 3, 0x80);
    let b = w.create(1, 3, 0);
    let c = w.create(1, 3, 0x80);
    let d = w.create(1, 3, 0x40);
    assert_eq!(plink_ids(&w, 3), vec![b, d, a, c]);
}

#[test]
fn before_equal_priority_and_explicit_positions() {
    // gobj_first_higher_prio (gobjplink.c:45-53), cases 2 and 3 (88-93).
    let mut w = melee();
    let a = w.create(1, 3, 5);
    let b = w.create(1, 3, 5);
    let c = w.create_at(InsertWhere::BeforeEqualPriority, 1, 3, 5);
    assert_eq!(plink_ids(&w, 3), vec![c, a, b]);
    let d = w.create_at(InsertWhere::BeforeEqualPriority, 1, 3, 9);
    assert_eq!(plink_ids(&w, 3), vec![c, a, b, d]);
    let e = w.create_at(InsertWhere::After(a), 1, 3, 0);
    assert_eq!(plink_ids(&w, 3), vec![c, a, e, b, d]);
    let f = w.create_at(InsertWhere::Before(c), 1, 3, 0);
    assert_eq!(plink_ids(&w, 3), vec![f, c, a, e, b, d]);
    assert_eq!(w.plink_head(3), Some(f));
    assert_eq!(w.plink_tail(3), Some(d));
}

#[test]
fn destroy_unlinks_and_stales_the_id() {
    // gobjplink.c:116-126
    let mut w = melee();
    let a = w.create(1, plink::FIGHTER, 0);
    let b = w.create(1, plink::FIGHTER, 0);
    let c = w.create(1, plink::FIGHTER, 0);
    w.destroy(b);
    assert!(!w.contains(b));
    assert_eq!(plink_ids(&w, plink::FIGHTER), vec![a, c]);
    assert_eq!(w.gobj(a).next(), Some(c));
    assert_eq!(w.gobj(c).prev(), Some(a));
    w.destroy(a);
    assert_eq!(w.plink_head(plink::FIGHTER), Some(c));
    w.destroy(c);
    assert_eq!(w.plink_head(plink::FIGHTER), None);
    assert_eq!(w.plink_tail(plink::FIGHTER), None);
    assert_eq!(w.gobj_count(), 0);
    // A reused slot gets a new generation; the old id stays stale.
    let d = w.create(1, plink::FIGHTER, 0);
    assert_eq!(d.index(), c.index());
    assert_ne!(d, c);
    assert!(!w.contains(c));
}

#[test]
fn destroy_runs_user_data_remover_then_obj_remover_then_procs() {
    // gobjplink.c:110-115 order: user data, hsd obj, procs, gx.
    let l = log();
    let (mut w, kinds) = World::melee();
    let g = w.create(class::FIGHTER, plink::FIGHTER, 0);
    struct Fighter(u32);
    let l1 = l.clone();
    w.init_user_data(
        g,
        4,
        move |_, data| {
            let f = data.downcast::<Fighter>().unwrap();
            l1.borrow_mut().push(format!("ud{}", f.0));
        },
        Fighter(7),
    );
    w.set_obj(g, kinds.jobj, Box::new(String::from("jobj")));
    assert_eq!(w.gobj(g).obj_kind(), kinds.jobj);
    assert_eq!(
        w.gobj(g).hsd_obj::<String>().map(String::as_str),
        Some("jobj")
    );
    assert_eq!(w.user_data::<Fighter>(g).map(|f| f.0), Some(7));
    w.add_proc(g, 4, logging(&l, "proc"));
    w.setup_gx_link(g, |_, _, _| {}, gxlink::FIGHTER, 0);
    assert_eq!(w.iter_gxlink(gxlink::FIGHTER).count(), 1);

    w.destroy(g);
    assert_eq!(l.borrow().as_slice(), ["ud7"]);
    assert_eq!(w.proc_count(), 0);
    assert_eq!(w.iter_gxlink(gxlink::FIGHTER).count(), 0);
    assert_eq!(w.iter_procs(4).count(), 0);
}

#[test]
fn user_data_and_obj_slots_follow_the_c_asserts() {
    let (mut w, kinds) = World::melee();
    assert_eq!(kinds.camera, 1); // SObjLib table occupies 0 (gm_1A45.c:225)
    assert_eq!(kinds.light, 2);
    assert_eq!(kinds.jobj, 3);
    assert_eq!(kinds.fog, 4);
    let g = w.create(1, 3, 0);
    // remove on empty is a no-op (gobjuserdata.c:17-19, gobjobject.c:41)
    w.remove_user_data(g);
    w.remove_obj(g);
    w.init_user_data(g, 0, |_, _| {}, 5u8);
    assert_eq!(w.gobj(g).user_data_kind(), 0);
    *w.user_data_mut::<u8>(g).unwrap() = 6;
    assert_eq!(w.user_data::<u8>(g), Some(&6));
    assert_eq!(w.user_data::<u16>(g), None);
    w.remove_user_data(g);
    assert_eq!(w.gobj(g).user_data_kind(), USER_DATA_NONE);
    assert!(!w.gobj(g).has_user_data());
    // take_obj detaches without running the remover (gobjobject.c:26-37)
    w.set_obj(g, kinds.camera, Box::new(1i32));
    let taken = w.take_obj(g).unwrap();
    assert_eq!(*taken.downcast::<i32>().unwrap(), 1);
    assert_eq!(w.gobj(g).obj_kind(), OBJ_NONE);
    assert!(w.take_obj(g).is_none());
    // A custom kind's remover receives the object.
    static HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    fn my_remover(_: &mut World, o: Box<dyn std::any::Any>) {
        assert_eq!(*o.downcast::<i32>().unwrap(), 9);
        HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    let k = w.register_obj_kind(my_remover);
    assert_eq!(k, 5);
    w.set_obj(g, k, Box::new(9i32));
    w.remove_obj(g);
    assert_eq!(HITS.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
#[should_panic(expected = "already has user data")]
fn double_init_user_data_panics() {
    let mut w = melee();
    let g = w.create(1, 3, 0);
    w.init_user_data(g, 0, |_, _| {}, 1u8);
    w.init_user_data(g, 0, |_, _| {}, 2u8);
}

#[test]
#[should_panic(expected = "p_link 64 > p_link_max 63")]
fn p_link_out_of_range_panics() {
    let mut w = melee();
    w.create(1, 64, 0);
}

#[test]
#[should_panic(expected = "proc priority 25 > gproc_pri_max 24")]
fn proc_priority_out_of_range_panics() {
    let mut w = melee();
    let g = w.create(1, 3, 0);
    w.add_proc(g, proc_prio::MELEE_MAX + 1, |_, _| {});
}

#[test]
fn find_by_classifier_walks_the_plink_list() {
    // gobjobject.c:6-17
    let mut w = melee();
    let a = w.create(class::LIGHT, 3, 0);
    let b = w.create(class::GROUND, 3, 0);
    assert_eq!(w.find_by_classifier(3, class::GROUND), Some(b));
    assert_eq!(w.find_by_classifier(3, class::LIGHT), Some(a));
    assert_eq!(w.find_by_classifier(3, class::FOG), None);
    assert_eq!(w.find_by_classifier(4, class::LIGHT), None);
}

// ---------------------------------------------------------------------------
// Proc list ordering
// ---------------------------------------------------------------------------

#[test]
fn procs_order_by_plink_then_gobj_position_then_creation() {
    // HSD_GObjProc_8038FAA8 (gobjproc.c:12-95)
    let l = log();
    let mut w = melee();
    let f1 = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let f2 = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let stage = w.create(class::STAGE, plink::STAGE, 0);
    let item = w.create(class::ITEM, plink::ITEM, 0);

    // Register out of p_link order to show the list sorts by p_link.
    let f2a = w.add_proc(f2, 4, logging(&l, "f2a"));
    let ia = w.add_proc(item, 4, logging(&l, "ia"));
    let f1a = w.add_proc(f1, 4, logging(&l, "f1a"));
    let sa = w.add_proc(stage, 4, logging(&l, "sa"));
    let f2b = w.add_proc(f2, 4, logging(&l, "f2b"));
    let f1b = w.add_proc(f1, 4, logging(&l, "f1b"));
    let ib = w.add_proc(item, 4, logging(&l, "ib"));

    let order: Vec<_> = w.iter_procs(4).collect();
    assert_eq!(order, vec![sa, f1a, f1b, f2a, f2b, ia, ib]);

    // The owner's own chain is newest-first (gobjproc.c:88-89).
    assert_eq!(w.iter_gobj_procs(f1).collect::<Vec<_>>(), vec![f1b, f1a]);
    assert_eq!(w.proc(f1b).child(), Some(f1a));
    assert_eq!(w.proc(f1a).child(), None);

    w.run_procs();
    assert_eq!(
        l.borrow().as_slice(),
        ["sa", "f1a", "f1b", "f2a", "f2b", "ia", "ib"]
    );
}

#[test]
fn proc_lists_are_independent_per_priority_and_run_low_first() {
    // gobj.c:101: for (i = 0; i <= gproc_pri_max; i++)
    let l = log();
    let mut w = melee();
    let f = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let it = w.create(class::ITEM, plink::ITEM, 0);
    // Registration order mirrors fighter.c:897-911 / item.c:991-998 partially.
    w.add_proc(f, proc_prio::fighter::P0, logging(&l, "f0"));
    w.add_proc(f, proc_prio::fighter::UPDATE, logging(&l, "f4"));
    w.add_proc(f, proc_prio::fighter::MAP, logging(&l, "f6"));
    w.add_proc(it, proc_prio::item::P0, logging(&l, "i0"));
    w.add_proc(it, proc_prio::item::P4, logging(&l, "i4"));
    w.add_proc(it, proc_prio::item::P9, logging(&l, "i9"));
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["f0", "i0", "f4", "i4", "f6", "i9"]);
}

#[test]
fn gobj_inserted_between_others_puts_its_procs_between_theirs() {
    // The backwards walk through gobj->prev (gobjproc.c:31-51) keys the proc
    // position off the GObj's list position, not creation time.
    let mut w = melee();
    let a = w.create(1, 3, 10);
    let c = w.create(1, 3, 10);
    let pa = w.add_proc(a, 0, |_, _| {});
    let pc = w.add_proc(c, 0, |_, _| {});
    // b sorts between a and c by priority; created after both.
    let b = w.create(1, 3, 5);
    assert_eq!(plink_ids(&w, 3), vec![b, a, c]);
    let pb = w.add_proc(b, 0, |_, _| {});
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![pb, pa, pc]);
    // And a new proc on a lands after a's own procs, before c's.
    let pa2 = w.add_proc(a, 0, |_, _| {});
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![pb, pa, pa2, pc]);
}

#[test]
fn removing_procs_maintains_anchors() {
    // HSD_GObjProc_8038FC18 (gobjproc.c:104-117): anchor moves to prev if same
    // p_link, else clears; later inserts still land in the right group.
    let mut w = melee();
    let lo = w.create(1, 3, 0);
    let f1 = w.create(1, 8, 0);
    let f2 = w.create(1, 8, 0);
    let hi = w.create(1, 9, 0);
    let p_lo = w.add_proc(lo, 0, |_, _| {});
    let p_f1 = w.add_proc(f1, 0, |_, _| {});
    let p_f2 = w.add_proc(f2, 0, |_, _| {});
    let p_hi = w.add_proc(hi, 0, |_, _| {});
    assert_eq!(
        w.iter_procs(0).collect::<Vec<_>>(),
        vec![p_lo, p_f1, p_f2, p_hi]
    );

    // Remove the group's last proc: anchor falls back to p_f1.
    w.remove_proc(p_f2);
    let p_f2b = w.add_proc(f2, 0, |_, _| {});
    assert_eq!(
        w.iter_procs(0).collect::<Vec<_>>(),
        vec![p_lo, p_f1, p_f2b, p_hi]
    );

    // Remove the whole group: anchor clears; the next p_link-8 proc goes after
    // the p_link-3 anchor.
    w.remove_proc(p_f1);
    w.remove_proc(p_f2b);
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![p_lo, p_hi]);
    let p_f1c = w.add_proc(f1, 0, |_, _| {});
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![p_lo, p_f1c, p_hi]);

    // Remove the head: list head advances.
    w.remove_proc(p_lo);
    assert_eq!(w.proc_list_head(0), Some(p_f1c));
    assert_eq!(w.proc(p_f1c).prev(), None);
    assert!(!w.contains_proc(p_lo));

    // remove_all_procs empties the owner's chain.
    w.add_proc(hi, 0, |_, _| {});
    w.add_proc(hi, 1, |_, _| {});
    w.remove_all_procs(hi);
    assert_eq!(w.gobj(hi).first_proc(), None);
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![p_f1c]);
    assert_eq!(w.iter_procs(1).count(), 0);
}

// ---------------------------------------------------------------------------
// Per-frame invocation
// ---------------------------------------------------------------------------

#[test]
fn frame_tag_cycles_and_procs_run_once_per_frame() {
    // gobj.c:96-99, 106-107
    let l = log();
    let mut w = melee();
    assert_eq!(w.frame_tag(), 0);
    let g = w.create(1, 3, 0);
    let p = w.add_proc(g, 0, logging(&l, "p"));
    assert_eq!(w.proc(p).frame_tag(), 3);
    for expect in [1u8, 2, 0, 1] {
        w.run_procs();
        assert_eq!(w.frame_tag(), expect);
        assert_eq!(w.proc(p).frame_tag(), expect);
    }
    assert_eq!(l.borrow().len(), 4);
    assert_eq!(w.current_gobj(), None);
    assert_eq!(w.current_proc(), None);
}

#[test]
fn current_gobj_and_proc_are_set_during_invoke() {
    let mut w = melee();
    let g = w.create(1, 3, 0);
    let seen = Rc::new(RefCell::new(None));
    let s = seen.clone();
    let p = w.add_proc(g, 0, move |w, id| {
        *s.borrow_mut() = Some((w.current_gobj(), w.current_proc(), id));
    });
    w.run_procs();
    assert_eq!(*seen.borrow(), Some((Some(g), Some(p), g)));
}

#[test]
fn pause_mask_and_flags_skip_callback_but_still_stamp_tag() {
    // gobj.c:106-110: the tag is stamped before the pause check.
    let l = log();
    let mut w = melee();
    let f = w.create(1, plink::FIGHTER, 0);
    let s = w.create(1, plink::STAGE, 0);
    let pf = w.add_proc(f, 0, logging(&l, "f"));
    let ps = w.add_proc(s, 0, logging(&l, "s"));

    w.set_pause_mask(1 << plink::FIGHTER);
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["s"]);
    assert_eq!(w.proc(pf).frame_tag(), w.frame_tag());
    assert_eq!(w.proc(ps).frame_tag(), w.frame_tag());

    w.set_pause_mask(0);
    w.set_procs_paused(s, true);
    assert!(w.proc(ps).is_paused());
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["s", "f"]);
    w.set_procs_paused(s, false);
    w.run_procs();
    // Stage (p_link 5) precedes fighter (p_link 8) in the priority-0 list.
    assert_eq!(l.borrow().as_slice(), ["s", "f", "s", "f"]);
}

#[test]
fn skip_procs_this_frame_defers_to_next_frame() {
    // HSD_GObj_80390CD4 (gobj.c:77-85) and the mndeflicker idiom.
    let l = log();
    let mut w = melee();
    let g = w.create(1, 3, 0);
    w.add_proc(g, 0, logging(&l, "a"));
    w.skip_procs_this_frame(g);
    // Tag now equals the *current* tag (0), but run_procs advances to 1 first,
    // so it still runs this frame: the idiom only works from inside a proc.
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["a"]);

    // From inside a proc: a new proc at a higher priority would run this
    // frame; stamping it holds it to next frame.
    let l2 = log();
    let mut w = melee();
    let g = w.create(1, 3, 0);
    let l3 = l2.clone();
    let mut spawned = false;
    w.add_proc(g, 0, move |w, id| {
        l3.borrow_mut().push("spawner".into());
        if !spawned {
            spawned = true;
            let l4 = l3.clone();
            let p = w.add_proc(id, 1, move |_, _| l4.borrow_mut().push("late".into()));
            w.skip_proc_this_frame(p);
        }
    });
    w.run_procs();
    assert_eq!(l2.borrow().as_slice(), ["spawner"]);
    // Next frame: priority 0 (spawner) runs before priority 1 (late).
    w.run_procs();
    assert_eq!(l2.borrow().as_slice(), ["spawner", "spawner", "late"]);
}

#[test]
fn newly_created_procs_run_this_frame_iff_after_current_position() {
    // gobj.c:115 re-reads next; new procs carry tag 3 (gobjproc.c:160).
    let l = log();
    let mut w = melee();
    let fighter = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let stage = w.create(class::STAGE, plink::STAGE, 0);
    w.add_proc(stage, 4, logging(&l, "stage4"));
    let l1 = l.clone();
    let mut spawned = false;
    w.add_proc(fighter, 4, move |w, _| {
        l1.borrow_mut().push("fighter4".into());
        if spawned {
            return;
        }
        spawned = true;
        // Spawn an item (p_link 9 > 8) with procs like item.c:991-998.
        let item = w.create(class::ITEM, plink::ITEM, 0);
        for pri in [0u8, 1, 4, 5, 9] {
            let l2 = l1.clone();
            w.add_proc(item, pri, move |_, _| {
                l2.borrow_mut().push(format!("item{pri}"))
            });
        }
        // Spawn a lower p_link object (3) with a priority-4 proc: it lands
        // before the fighter in the priority-4 list and waits a frame.
        let light = w.create(class::LIGHT, plink::LIGHT, 0);
        let l3 = l1.clone();
        w.add_proc(light, 4, move |_, _| l3.borrow_mut().push("light4".into()));
        // Same gobj, same priority: lands right after this proc, runs now.
        let l4 = l1.clone();
        w.add_proc(w.current_gobj().unwrap(), 4, move |_, _| {
            l4.borrow_mut().push("fighter4b".into())
        });
    });
    w.add_proc(fighter, 6, logging(&l, "fighter6"));

    w.run_procs();
    assert_eq!(
        l.borrow().as_slice(),
        [
            "stage4",
            "fighter4",
            "fighter4b",
            "item4",
            "item5",
            "fighter6",
            "item9"
        ]
    );
    l.borrow_mut().clear();
    w.run_procs();
    assert_eq!(
        l.borrow().as_slice(),
        [
            "item0",
            "item1",
            "light4",
            "stage4",
            "fighter4",
            "fighter4b",
            "item4",
            "item5",
            "fighter6",
            "item9"
        ]
    );
}

#[test]
fn self_destroy_is_deferred_until_callback_returns() {
    // gobjplink.c:106-109, gobj.c:116-119
    let l = log();
    let mut w = melee();
    let a = w.create(1, 3, 0);
    let b = w.create(1, 3, 0);
    let l1 = l.clone();
    w.add_proc(a, 0, move |w, id| {
        w.destroy(id);
        // Still alive inside the callback.
        assert!(w.contains(id));
        l1.borrow_mut().push("a0".into());
    });
    // A later proc of a at the same priority is dropped with it and never runs.
    w.add_proc(a, 0, logging(&l, "a0-later"));
    w.add_proc(a, 1, logging(&l, "a1"));
    w.add_proc(b, 0, logging(&l, "b0"));
    w.add_proc(b, 1, logging(&l, "b1"));
    w.init_user_data(
        a,
        0,
        {
            let l = l.clone();
            move |_, _| l.borrow_mut().push("a-ud".into())
        },
        (),
    );

    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["a0", "a-ud", "b0", "b1"]);
    assert!(!w.contains(a));
    assert_eq!(plink_ids(&w, 3), vec![b]);
    assert_eq!(w.proc_count(), 2);
    w.run_procs();
    assert_eq!(
        l.borrow().as_slice(),
        ["a0", "a-ud", "b0", "b1", "b0", "b1"]
    );
}

#[test]
fn destroying_another_gobj_mid_iteration_is_immediate() {
    // gobjplink.c:110-126 runs now; gobj.c:115 re-reads next so the walk
    // skips the freed procs.
    let l = log();
    let mut w = melee();
    let a = w.create(1, 3, 0);
    let b = w.create(1, 3, 0);
    let c = w.create(1, 3, 0);
    let l1 = l.clone();
    w.add_proc(a, 0, move |w, _| {
        l1.borrow_mut().push("a0".into());
        w.destroy(b);
        assert!(!w.contains(b));
    });
    w.add_proc(b, 0, logging(&l, "b0"));
    w.add_proc(b, 0, logging(&l, "b0b"));
    w.add_proc(b, 1, logging(&l, "b1"));
    w.add_proc(c, 0, logging(&l, "c0"));
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["a0", "c0"]);
    assert_eq!(plink_ids(&w, 3), vec![a, c]);
    assert_eq!(w.proc_count(), 2);
}

#[test]
fn destroying_the_next_gobj_from_a_previous_priority_group() {
    // Item destroyed by a fighter proc while the item's proc is run_next.
    let l = log();
    let mut w = melee();
    let f = w.create(class::FIGHTER, plink::FIGHTER, 0);
    let it = w.create(class::ITEM, plink::ITEM, 0);
    let l1 = l.clone();
    w.add_proc(f, 4, move |w, _| {
        l1.borrow_mut().push("f4".into());
        let victim = w.plink_head(plink::ITEM).unwrap();
        w.destroy(victim);
    });
    w.add_proc(it, 4, logging(&l, "i4"));
    w.add_proc(it, 9, logging(&l, "i9"));
    w.add_proc(f, 9, logging(&l, "f9"));
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["f4", "f9"]);
    assert!(!w.contains(it));
}

#[test]
fn self_remove_proc_is_deferred_and_other_removals_are_immediate() {
    // gobjproc.c:167-175
    let l = log();
    let mut w = melee();
    let g = w.create(1, 3, 0);
    let l1 = l.clone();
    let counter = Rc::new(RefCell::new(0));
    let cnt = counter.clone();
    let other_slot: Rc<RefCell<Option<hsd_gobj::ProcId>>> = Rc::new(RefCell::new(None));
    let os = other_slot.clone();
    let p_self = w.add_proc(g, 0, move |w, _| {
        *cnt.borrow_mut() += 1;
        l1.borrow_mut().push("self".into());
        let me = w.current_proc().unwrap();
        w.remove_proc(me);
        assert!(w.contains_proc(me), "self removal is deferred");
        if let Some(o) = os.borrow_mut().take() {
            w.remove_proc(o);
            assert!(!w.contains_proc(o), "other removal is immediate");
        }
    });
    let p_other = w.add_proc(g, 0, logging(&l, "other"));
    *other_slot.borrow_mut() = Some(p_other);
    w.add_proc(g, 1, logging(&l, "p1"));

    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["self", "p1"]);
    assert!(!w.contains_proc(p_self));
    assert!(!w.contains_proc(p_other));
    assert_eq!(w.iter_gobj_procs(g).count(), 1);
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["self", "p1", "p1"]);
    assert_eq!(*counter.borrow(), 1);
}

#[test]
fn mndeflicker_idiom_replace_current_proc() {
    // mn/mndeflicker.c:91-93: remove current, add a new one at the same
    // priority, stamp it so it starts next frame.
    let l = log();
    let mut w = melee();
    let g = w.create(1, 3, 0);
    let l1 = l.clone();
    w.add_proc(g, 0, move |w, id| {
        l1.borrow_mut().push("old".into());
        w.remove_proc(w.current_proc().unwrap());
        let l2 = l1.clone();
        let p = w.add_proc(id, 0, move |_, _| l2.borrow_mut().push("new".into()));
        w.skip_proc_this_frame(p);
    });
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["old"]);
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["old", "new"]);
    assert_eq!(w.proc_count(), 1);
}

#[test]
fn remover_destroying_other_gobjs_during_deferred_block() {
    // Fighter_Unload-style: a user-data remover that destroys other GObjs
    // while the deferred block (b0) is active. gobjproc.c:101-103 keeps
    // run_next valid when the victim's proc is the next to visit.
    let l = log();
    let mut w = melee();
    let owner = w.create(1, 3, 0);
    let child = w.create(1, 3, 0);
    let after = w.create(1, 3, 0);
    let l1 = l.clone();
    w.add_proc(owner, 0, move |w, id| {
        l1.borrow_mut().push("owner".into());
        w.destroy(id);
    });
    w.add_proc(child, 0, logging(&l, "child"));
    w.add_proc(after, 0, logging(&l, "after"));
    let l2 = l.clone();
    w.init_user_data(
        owner,
        0,
        move |w, _| {
            l2.borrow_mut().push("unload".into());
            w.destroy(child);
        },
        (),
    );
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["owner", "unload", "after"]);
    assert_eq!(plink_ids(&w, 3), vec![after]);
}

#[test]
fn relink_moves_gobj_and_reorders_its_procs() {
    // HSD_GObjPLink_8039032C (gobjplink.c:129-196), direct call.
    let l = log();
    let mut w = melee();
    let a = w.create(1, 3, 0);
    let b = w.create(1, 9, 0);
    let pa1 = w.add_proc(a, 0, logging(&l, "a1"));
    let pa2 = w.add_proc(a, 0, logging(&l, "a2"));
    let pa3 = w.add_proc(a, 1, logging(&l, "a3"));
    let pb = w.add_proc(b, 0, logging(&l, "b"));
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![pa1, pa2, pb]);

    w.relink(InsertWhere::AfterEqualPriority, a, 9, 0);
    assert_eq!(w.gobj(a).p_link(), 9);
    assert_eq!(plink_ids(&w, 3), Vec::<GObjId>::new());
    assert_eq!(plink_ids(&w, 9), vec![b, a]);
    // Same proc ids, now after b's, still in creation order; chain restored
    // newest-first.
    assert_eq!(w.iter_procs(0).collect::<Vec<_>>(), vec![pb, pa1, pa2]);
    assert_eq!(w.iter_procs(1).collect::<Vec<_>>(), vec![pa3]);
    assert_eq!(
        w.iter_gobj_procs(a).collect::<Vec<_>>(),
        vec![pa3, pa2, pa1]
    );
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["b", "a1", "a2", "a3"]);
}

#[test]
fn self_relink_is_deferred_and_makes_reinserted_proc_run_next() {
    // Deferred relink (gobjplink.c:141-148, gobj.c:121-127) plus the
    // run_next fix-up in the link function (gobjproc.c:90-94): the gobj's
    // own unrun procs at this priority still run this frame.
    let l = log();
    let mut w = melee();
    let a = w.create(1, 3, 0);
    let b = w.create(1, 9, 0);
    let l1 = l.clone();
    w.add_proc(a, 0, move |w, id| {
        l1.borrow_mut().push("a1".into());
        if w.gobj(id).p_link() == 3 {
            w.relink(InsertWhere::AfterEqualPriority, id, 9, 0);
            assert_eq!(w.gobj(id).p_link(), 3, "deferred");
        }
    });
    w.add_proc(a, 0, logging(&l, "a2"));
    w.add_proc(b, 0, logging(&l, "b"));
    w.run_procs();
    assert_eq!(w.gobj(a).p_link(), 9);
    assert_eq!(plink_ids(&w, 9), vec![b, a]);
    // a1 ran; then a was moved after b; b runs; then a2 (tag still unrun).
    assert_eq!(l.borrow().as_slice(), ["a1", "b", "a2"]);
    l.borrow_mut().clear();
    w.run_procs();
    assert_eq!(l.borrow().as_slice(), ["b", "a1", "a2"]);
}

#[test]
fn hsd_default_config_has_three_priorities() {
    // gobjinit.c:6-10
    let mut w = World::new(WorldConfig::HSD_DEFAULT);
    assert_eq!(w.config().gproc_pri_max, 2);
    let g = w.create(1, 0, 0);
    w.add_proc(g, 2, |_, _| {});
    assert_eq!(w.gx_link_max_list(), 0x40);
}

// ---------------------------------------------------------------------------
// GX link buckets
// ---------------------------------------------------------------------------

#[test]
fn gx_link_lists_order_by_render_priority_with_equal_after() {
    // GObj_SetupGXLink (gobjgxlink.c:36-53)
    let mut w = melee();
    let a = w.create(1, 8, 0);
    let b = w.create(1, 8, 0);
    let c = w.create(1, 8, 0);
    let d = w.create(1, 8, 0);
    w.setup_gx_link(a, |_, _, _| {}, gxlink::FIGHTER, 1);
    w.setup_gx_link(b, |_, _, _| {}, gxlink::FIGHTER, 0);
    w.setup_gx_link(c, |_, _, _| {}, gxlink::FIGHTER, 1);
    w.setup_gx_link(d, |_, _, _| {}, gxlink::FIGHTER, 2);
    assert_eq!(
        w.iter_gxlink(gxlink::FIGHTER).collect::<Vec<_>>(),
        vec![b, a, c, d]
    );
    assert!(w.gobj(a).has_render_cb());
    assert_eq!(w.gobj(a).gx_link(), gxlink::FIGHTER);
    assert_eq!(w.gobj(d).render_priority(), 2);
    assert_eq!(w.gobj(b).prev_gx(), None);
    assert_eq!(w.gobj(b).next_gx(), Some(a));

    // Unlink resets fields (gobjgxlink.c:104-127).
    w.remove_gx_link(a);
    assert_eq!(
        w.iter_gxlink(gxlink::FIGHTER).collect::<Vec<_>>(),
        vec![b, c, d]
    );
    assert_eq!(w.gobj(a).gx_link(), GXLINK_NONE);
    assert_eq!(w.gobj(a).render_priority(), 0);
    assert!(w.gobj(a).has_render_cb(), "render_cb is kept");

    // Change: equal goes before (gobjgxlink.c:138-148).
    w.change_gx_link(c, gxlink::FIGHTER, 0);
    assert_eq!(
        w.iter_gxlink(gxlink::FIGHTER).collect::<Vec<_>>(),
        vec![c, b, d]
    );
    // Adopt: sit directly before `other` with its link/prio (150-163).
    w.setup_gx_link(a, |_, _, _| {}, gxlink::ITEM, 0);
    w.adopt_gx_link(a, d);
    assert_eq!(
        w.iter_gxlink(gxlink::FIGHTER).collect::<Vec<_>>(),
        vec![c, b, a, d]
    );
    assert_eq!(w.iter_gxlink(gxlink::ITEM).count(), 0);
    assert_eq!(w.gobj(a).render_priority(), 2);
}

#[test]
fn gx_link_max_lists() {
    // gobjgxlink.c:55-102
    let mut w = melee();
    let max = w.gx_link_max_list();
    assert_eq!(max, 0x40);
    let a = w.create(1, 8, 0);
    let b = w.create(1, 8, 0);
    let c = w.create(1, 8, 0);
    w.setup_gx_link_max(a, |_, _, _| {}, 5);
    w.setup_gx_link_max(b, |_, _, _| {}, 5);
    w.setup_gx_link_max_sorted(c, |_, _, _| {}, 5);
    assert_eq!(w.iter_gxlink(max).collect::<Vec<_>>(), vec![c, a, b]);
    assert_eq!(w.gobj(a).gx_link(), max);
    // Destroy unlinks from the GX list too (gobjplink.c:113-115).
    w.destroy(a);
    assert_eq!(w.iter_gxlink(max).collect::<Vec<_>>(), vec![c, b]);
}

#[test]
#[should_panic(expected = "gx_link 64 > gx_link_max 63")]
fn gx_link_out_of_range_panics() {
    let mut w = melee();
    let a = w.create(1, 8, 0);
    w.setup_gx_link(a, |_, _, _| {}, 64, 0);
}

#[test]
fn render_cb_can_be_taken_and_restored() {
    let mut w = melee();
    let a = w.create(1, 8, 0);
    let hits = Rc::new(RefCell::new(0));
    let h = hits.clone();
    w.setup_gx_link(
        a,
        move |_, _, code| *h.borrow_mut() += code,
        gxlink::FIGHTER,
        0,
    );
    let mut cb = w.take_render_cb(a).unwrap();
    assert!(!w.gobj(a).has_render_cb());
    cb(&mut w, a, 7);
    w.restore_render_cb(a, cb);
    assert!(w.gobj(a).has_render_cb());
    assert_eq!(*hits.borrow(), 7);
}

// ---------------------------------------------------------------------------
// Fighter-list walk as the harness sees it
// ---------------------------------------------------------------------------

#[test]
fn fighter_list_walk_matches_harness_expectations() {
    // harness/dolphin/walk.py: fighters = HSD_GObj_Entities[8] followed by
    // next; user_data is the Fighter, skipped when absent.
    struct Fighter {
        port: u8,
    }
    let mut w = melee();
    let ports = [0u8, 1, 3];
    let mut ids = Vec::new();
    for &port in &ports {
        let g = w.create(class::FIGHTER, plink::FIGHTER, 0);
        w.setup_gx_link(g, |_, _, _| {}, gxlink::FIGHTER, 0);
        w.init_user_data(g, 4, |_, _| {}, Fighter { port });
        ids.push(g);
    }
    // A fighter mid-construction (no user data yet) still appears in the walk.
    let half = w.create(class::FIGHTER, plink::FIGHTER, 0);
    assert_eq!(
        plink_ids(&w, plink::FIGHTER),
        vec![ids[0], ids[1], ids[2], half]
    );
    let walked: Vec<u8> = w
        .iter_plink(plink::FIGHTER)
        .filter_map(|g| w.user_data::<Fighter>(g).map(|f| f.port))
        .collect();
    assert_eq!(walked, ports);
    // Removing the middle fighter keeps the rest in order.
    w.destroy(ids[1]);
    assert_eq!(plink_ids(&w, plink::FIGHTER), vec![ids[0], ids[2], half]);
}
