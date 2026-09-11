//! HSD GObj scheduler: object lists, per-frame proc ordering, link/priority
//! queues. Port of `src/sysdolphin/baselib/gobj*.c` from the decomp.
//!
//! See CLAUDE.md for the porting rules that apply to every crate. This crate
//! has no float math; what it must reproduce exactly is *ordering*: which
//! GObj sits where in each `p_link` list, and in which order proc callbacks
//! run within a frame. Those orders decide the frame-phase sequence the
//! oracle compares.
//!
//! # Model
//!
//! A [`World`] owns every [`GObj`] and [`Proc`] in generational slabs keyed by
//! [`GObjId`] / [`ProcId`]. The C's intrusive doubly-linked lists are kept
//! verbatim with slab keys instead of pointers, because the scheduling rules
//! are defined in terms of list position. Procs are boxed
//! `FnMut(&mut World, GObjId)` closures rather than an enum of typed
//! callbacks: this is a layer-1 crate and cannot name Melee's callbacks, a
//! closure can carry per-proc state, and passing `&mut World` lets a callback
//! create and destroy objects the way the C callbacks do (the scheduler's
//! deferral rules make that safe). `user_data` and `hsd_obj` are
//! `Box<dyn Any>` slots with typed accessors.
//!
//! # Semantics pinned down (decomp line citations)
//!
//! **GObj creation order.** `GObj_Create` is `CreateGObj(0, ...)`
//! (gobjplink.c:98-101); `where = 0` scans from the list *tail* while
//! `p_priority > new` and inserts after the first node with priority `<=`
//! new (gobjplink.c:36-43). Equal priorities therefore append: fighters, all
//! created at priority 0 (ft/fighter.c:852), sit in `HSD_GObj_Entities[8]` in
//! creation order head to tail, which is what `harness/dolphin/walk.py`
//! follows via `next`.
//!
//! **Proc list order.** One global doubly-linked list per priority
//! (`HSD_GObj_804D7840[s_link]`). `HSD_GObjProc_8038FAA8` (gobjproc.c:12-95)
//! inserts a new proc after the most recent same-priority proc found by
//! walking from its GObj backwards through `prev`, or after the anchor of the
//! nearest lower `p_link`, or at the head. Net order: ascending `p_link`,
//! then GObj position within its `p_link` list, then proc creation order.
//! `HSD_GObj_804D7844[p_link + s_link*(p_link_max+1)]` caches the last proc of
//! each (`p_link`, `s_link`) group as the insertion anchor.
//!
//! **Per-frame invocation.** `HSD_GObj_80390CFC` (gobj.c:88-141), called once
//! per frame from Melee's main loop (gm/gm_1A45.c:340): frame tag cycles
//! 0,1,2; priorities run `0..=gproc_pri_max` (Melee sets 0x18,
//! gm/gm_1A45.c:224), each list head to tail. A proc is visited when its tag
//! differs from the frame's (new procs start at 3, gobjproc.c:160); visiting
//! stamps the tag and then invokes unless the owner's `p_link` bit is set in
//! the pause mask (`*HSD_GObjLibInitData.unk_2`, rewritten each frame at
//! gm/gm_1A45.c:324-333) or `flags_1`/`flags_2` is set. The successor is
//! re-read from the proc after the callback returns (gobj.c:115).
//!
//! **New procs this frame?** Yes iff the proc lands after the current
//! position in a priority `>=` the one running. A fighter (p_link 8) proc at
//! priority 4 spawning an item (p_link 9) with procs at 0,1,4,5,9,... runs
//! the item's 4 (lands after the fighter group, same list) and 5..16 this
//! frame, but its 0 and 1 wait until next frame (those lists already ran).
//!
//! **Mid-iteration destroy.** Destroying the GObj whose proc is running is
//! deferred (`HSD_GObj_804CE3E4.b1`, gobjplink.c:106-109) and applied after
//! the callback (gobj.c:116-119); removing the running proc is deferred the
//! same way (`b2`, gobjproc.c:169-170). Destroying *another* GObj is
//! immediate; its unrun procs are unlinked and never run, and the re-read of
//! `next` keeps the walk valid. While deferred requests are being applied
//! (`b0`, gobj.c:117) the unlink function steps `run_next` past freed procs
//! (gobjproc.c:101-103) and the link function makes a proc reinserted
//! directly after the running one the next to visit (gobjproc.c:90-94).
//!
//! **Ambiguities.** (1) `HSD_GObjPLink_8039032C` bumps a re-linked proc's tag
//! from `frame_tag-2` to `frame_tag-1` (gobjplink.c:185-194); in steady state
//! no proc carries `frame_tag-2` (every proc is stamped every frame, paused
//! or not), so this is a no-op we transcribe literally. (2) `flags_2` is only
//! ever cleared, never set, anywhere in HSD or Melee. (3) The `obj_kind`
//! removers (`HSD_JObjRemoveAll` etc.) are reference-count drops in C; here
//! the boxed object is dropped, and `hsd-anim` may register its own kind with
//! a real remover if it needs a hook. (4) Callbacks are moved out of their
//! slot while running; the C guarantees the running proc is never freed
//! during its own callback (deferral), which is what makes the move safe.
//!
//! # C function → Rust
//!
//! | C (retail address) | Rust |
//! |---|---|
//! | `HSD_GObj_803912E0` (gobjinit.c:12) defaults | [`WorldConfig::HSD_DEFAULT`] |
//! | `HSD_GObj_80391304` (gobjinit.c:20) init | [`World::new`], [`World::melee`] |
//! | `HSD_GObj_803912A8` (gobj.c:257) add kind table | [`World::register_obj_kind`] |
//! | `HSD_GObj_80391260` (gobj.c:248) builtin kinds | [`World::register_builtin_obj_kinds`] |
//! | `CreateGObj` 0x8038FFB8 | [`World::create_at`] |
//! | `GObj_Create` 0x803901F0 | [`World::create`] |
//! | `GObj_PReorder` 0x8038FF5C | `World::plink_insert_after` (private) |
//! | `HSD_GObjPLink_80390228` destroy | [`World::destroy`] |
//! | `HSD_GObjPLink_8039032C` relink | [`World::relink`] |
//! | `HSD_GObj_SetupProc` 0x8038FD54 | [`World::add_proc`] |
//! | `HSD_GObjProc_8038FAA8` link proc | `World::link_proc` (private) |
//! | `HSD_GObjProc_8038FC18` unlink proc | `World::unlink_proc` (private) |
//! | `HSD_GObjProc_8038FCE4` detach proc | `World::detach_proc` (private) |
//! | `HSD_GObjProc_8038FE24` remove proc | [`World::remove_proc`] |
//! | `HSD_GObjProc_8038FED4` remove all procs | [`World::remove_all_procs`] |
//! | `HSD_GObj_80390CFC` run procs | [`World::run_procs`] |
//! | `HSD_GObj_80390C5C` / `80390C84` flags_1 | [`World::set_procs_paused`] |
//! | `HSD_GObj_80390CAC` flags_2 = 0 | [`World::clear_procs_flag2`] |
//! | `HSD_GObj_80390CD4` flags_3 = tag | [`World::skip_procs_this_frame`] |
//! | `p->flags_3 = HSD_GObj_804D783C` (mn idiom) | [`World::skip_proc_this_frame`] |
//! | `GObj_GXReorder` 0x8039063C | `World::gx_insert_after` (private) |
//! | `GObj_SetupGXLink` 0x8039069C | [`World::setup_gx_link`] |
//! | `GObj_SetupGXLinkMax` 0x8039075C | [`World::setup_gx_link_max`] |
//! | `GObj_SetupGXLinkMaxSorted` 0x803907C8 | [`World::setup_gx_link_max_sorted`] |
//! | `HSD_GObjGXLink_8039084C` unlink GX | [`World::remove_gx_link`] |
//! | `HSD_GObjGXLink_80390908` change GX | [`World::change_gx_link`] |
//! | `HSD_GObjGXLink_803909D8` adopt GX | [`World::adopt_gx_link`] |
//! | `HSD_GObj_80390ED0`, `HSD_GObj_80390FC0` render passes | not ported (iteration order via [`World::iter_gxlink`]) |
//! | `HSD_GObjObject_80390A3C` find by classifier | [`World::find_by_classifier`] |
//! | `HSD_GObjObject_80390A70` set obj | [`World::set_obj`] |
//! | `HSD_GObjObject_80390ADC` take obj | [`World::take_obj`] |
//! | `HSD_GObjObject_80390B0C` remove obj | [`World::remove_obj`] |
//! | `GObj_InitUserData` 0x80390B68 | [`World::init_user_data`] |
//! | `GObj_RemoveUserData` 0x80390BE4 | [`World::remove_user_data`] |
//! | `HSD_GObjGetUserData` / `HSD_GObjGetHSDObj` (gobj.h:147-155) | [`GObj::user_data`], [`GObj::hsd_obj`] |
//! | `HSD_GObjGetNext` (gobj.h:162) | [`GObj::next`], [`World::iter_plink`] |
//! | `HSD_GObj_Entities[p]` / `plinklow_gobjs[p]` | [`World::plink_head`] / [`World::plink_tail`] |
//! | `HSD_GObj_804D7840[s]` | [`World::proc_list_head`], [`World::iter_procs`] |
//! | `HSD_GObj_804D783C` / `804D781C` / `804D7838` | [`World::frame_tag`] / [`World::current_gobj`] / [`World::current_proc`] |
//! | `*HSD_GObjLibInitData.unk_2` | [`World::set_pause_mask`] |
//! | `HSD_GObj_JObjCallback` / `LObjCallback` / `FogCallback` | not ported (render) |

#![forbid(unsafe_code)]

pub mod consts;
mod slab;
mod world;

pub use consts::{GXLINK_NONE, OBJ_NONE, USER_DATA_NONE};
pub use world::{
    BuiltinObjKinds, GObj, GObjId, InsertWhere, ObjRemoveFn, Proc, ProcFn, ProcId, RenderFn,
    TaggedWorld, UserDataRemoveFn, World, WorldConfig,
};
