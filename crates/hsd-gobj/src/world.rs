//! The object arena and scheduler.
//!
//! [`World`] owns every GObj and GObjProc and holds all the globals the C
//! keeps in `.sbss` (`HSD_GObj_Entities`, `plinklow_gobjs`,
//! `HSD_GObjGXLinkHead`, `HSD_GObj_804D7820`, `HSD_GObj_804D7840`,
//! `HSD_GObj_804D7844`, the run-state pointers, and `HSD_GObj_804CE3E4`).
//! The intrusive doubly-linked lists of the C are kept as-is, with slab keys
//! instead of pointers, because the scheduling semantics are defined by list
//! position (where a new proc lands relative to the one currently running
//! decides whether it runs this frame). See the crate docs for the summary
//! and the per-function notes below for the line-by-line mapping.

use core::any::Any;
use core::fmt;

use crate::consts::{GXLINK_NONE, OBJ_NONE, USER_DATA_NONE};
use crate::slab::{Key, Slab};

/// Handle to an object. Stale after [`World::destroy`]; every lookup checks the
/// generation, so a stale id never aliases a later object.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct GObjId(Key);

impl GObjId {
    /// Slot index. Stable for the object's life; reused after destruction.
    pub fn index(self) -> u32 {
        self.0.index
    }
}

impl fmt::Display for GObjId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gobj#{}.{}", self.0.index, self.0.generation)
    }
}

/// Handle to a GObjProc. Stale after removal.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ProcId(Key);

impl ProcId {
    pub fn index(self) -> u32 {
        self.0.index
    }
}

impl fmt::Display for ProcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "proc#{}.{}", self.0.index, self.0.generation)
    }
}

/// Original scheduler, including owned callbacks and attached objects.
pub type World = Scheduler<Local>;
pub type GObj = Object<Local>;
pub type Proc = Process<Local>;
/// A scheduler with external dispatch tokens only. No erased objects or closures.
pub type TaggedWorld = Scheduler<Tagged>;

mod sealed {
    pub trait Sealed {}
}
#[doc(hidden)]
pub trait Storage: sealed::Sealed + Sized {
    type Callback;
    type Render;
    type Object;
    type UserRemover;
    type ObjectRemover;
    fn invoke(world: &mut Scheduler<Self>, proc: ProcId, object: GObjId);
    fn remove_obj(world: &mut Scheduler<Self>, object: GObjId);
    fn remove_user_data(world: &mut Scheduler<Self>, object: GObjId);
}
#[doc(hidden)]
pub struct Local;
impl sealed::Sealed for Local {}
impl Storage for Local {
    type Callback = Box<ProcFn>;
    type Render = Box<RenderFn>;
    type Object = Box<dyn Any>;
    type UserRemover = Box<UserDataRemoveFn>;
    type ObjectRemover = ObjRemoveFn;
    fn invoke(w: &mut World, p: ProcId, o: GObjId) {
        w.invoke(p, o);
    }
    fn remove_obj(w: &mut World, o: GObjId) {
        w.remove_obj(o);
    }
    fn remove_user_data(w: &mut World, o: GObjId) {
        w.remove_user_data(o);
    }
}
#[doc(hidden)]
pub struct Tagged;
impl sealed::Sealed for Tagged {}
/// Uninhabited: a tagged scheduler cannot contain an owned callback or object.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum Empty {}
impl Storage for Tagged {
    type Callback = Empty;
    type Render = Empty;
    type Object = Empty;
    type UserRemover = Empty;
    type ObjectRemover = Empty;
    fn invoke(_: &mut TaggedWorld, _: ProcId, _: GObjId) {
        unreachable!("tagged proc without dispatch token");
    }
    fn remove_obj(_: &mut TaggedWorld, _: GObjId) {}
    fn remove_user_data(_: &mut TaggedWorld, _: GObjId) {}
}

/// Per-frame proc callback, `HSD_GObjEvent` (forward.h:108). It receives the
/// world so it can create and destroy objects; the scheduler's re-entrancy
/// rules (deferred self-destroy, re-read of `next` after the call) are what
/// make that safe.
pub type ProcFn = dyn FnMut(&mut World, GObjId);
/// Render callback slot, `GObj_RenderFunc` (forward.h:105). Stored but never
/// invoked by this crate (no rendering); kept so `render_cb != NULL` checks
/// can be reproduced by a later layer.
pub type RenderFn = dyn FnMut(&mut World, GObjId, u32);
/// `user_data_remove_func` (gobj.h:45). Receives the world because Melee's
/// removers (e.g. `Fighter_Unload_8006DABC`) destroy other GObjs.
pub type UserDataRemoveFn = dyn FnOnce(&mut World, Box<dyn Any>);
/// Entry of the `obj_kind` remover table `HSD_GObj_804D7810` (`GObjFunc`,
/// gobj.h:49). Plain fn like the C table.
pub type ObjRemoveFn = fn(&mut World, Box<dyn Any>);

/// `HSD_GObjLibInitDataType` (gobj.h:57-63) minus the func chain, which is
/// registered through [`World::register_obj_kind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldConfig {
    /// Highest valid `p_link`. Lists are `p_link_max + 1` long.
    pub p_link_max: u8,
    /// Highest valid `gx_link` for [`World::setup_gx_link`]. The GX lists are
    /// `gx_link_max + 2` long; index `gx_link_max + 1` is the "max" list used by
    /// [`World::setup_gx_link_max`] (gobjinit.c:39-42, gobjgxlink.c:62).
    pub gx_link_max: u8,
    /// Highest valid proc priority (`s_link`). The frame loop runs
    /// `0..=gproc_pri_max` (gobj.c:101).
    pub gproc_pri_max: u8,
}

impl WorldConfig {
    /// HSD's defaults, `HSD_GObj_80408620` (gobjinit.c:6-10).
    pub const HSD_DEFAULT: WorldConfig = WorldConfig {
        p_link_max: 0x3F,
        gx_link_max: 0x3F,
        gproc_pri_max: 2,
    };
    /// Melee's configuration: HSD defaults with `gproc_pri_max = 0x18`
    /// (gm/gm_1A45.c:223-224).
    pub const MELEE: WorldConfig = WorldConfig {
        p_link_max: 0x3F,
        gx_link_max: 0x3F,
        gproc_pri_max: 0x18,
    };
}

impl Default for WorldConfig {
    fn default() -> Self {
        WorldConfig::HSD_DEFAULT
    }
}

/// The `where` argument of `CreateGObj` / `HSD_GObjPLink_8039032C`
/// (gobjplink.c:81-94, 171-184).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InsertWhere {
    /// `case 0`, `gobj_first_lower_prio` (gobjplink.c:36-43): walk from the
    /// tail while `p_priority > new`, insert after the first node with
    /// priority `<=` the new one. Equal priorities therefore go **after**
    /// existing ones: creation order within a priority. This is what
    /// `GObj_Create` uses (gobjplink.c:100).
    AfterEqualPriority,
    /// `case 1`, `gobj_first_higher_prio` (gobjplink.c:45-53): walk from the
    /// head while `p_priority < new`, insert before the first node with
    /// priority `>=` the new one. Equal priorities go **before** existing ones.
    BeforeEqualPriority,
    /// `case 2`: insert directly after `position` (gobjplink.c:88-90).
    After(GObjId),
    /// `case 3`: insert directly before `position` (gobjplink.c:91-93).
    Before(GObjId),
}

/// The base indices `HSD_GObj_80391260` (gobj.c:248-255) assigns to the four
/// HSD object kinds. In Melee they are 1..=4 because the 1-entry SObjLib table
/// is registered first (gm/gm_1A45.c:225-226, sobjlib.c:24-28).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuiltinObjKinds {
    /// `HSD_GObj_CameraKind`
    pub camera: u8,
    /// `HSD_GObj_LightKind`
    pub light: u8,
    /// `HSD_GObj_JObjKind`
    pub jobj: u8,
    /// `HSD_GObj_FogKind`
    pub fog: u8,
}

/// `HSD_GObj` (gobj.h:28-47). Links are private; read them through the
/// getters so list invariants stay inside this module.
pub struct Object<S: Storage> {
    classifier: u16,
    p_link: u8,
    gx_link: u8,
    p_priority: u8,
    render_priority: u8,
    obj_kind: u8,
    user_data_kind: u8,
    next: Option<GObjId>,
    prev: Option<GObjId>,
    next_gx: Option<GObjId>,
    prev_gx: Option<GObjId>,
    /// `proc`: head of this object's own proc chain (linked by `child`).
    proc_head: Option<ProcId>,
    render_cb: Option<S::Render>,
    gxlink_prios: u64,
    hsd_obj: Option<S::Object>,
    user_data: Option<S::Object>,
    user_data_remove: Option<S::UserRemover>,
}

impl<S: Storage> Object<S> {
    pub fn classifier(&self) -> u16 {
        self.classifier
    }
    pub fn p_link(&self) -> u8 {
        self.p_link
    }
    pub fn p_priority(&self) -> u8 {
        self.p_priority
    }
    /// [`GXLINK_NONE`] when not on a GX list.
    pub fn gx_link(&self) -> u8 {
        self.gx_link
    }
    pub fn render_priority(&self) -> u8 {
        self.render_priority
    }
    /// [`OBJ_NONE`] when no HSD object is attached.
    pub fn obj_kind(&self) -> u8 {
        self.obj_kind
    }
    /// [`USER_DATA_NONE`] when no user data is attached.
    pub fn user_data_kind(&self) -> u8 {
        self.user_data_kind
    }
    /// Next object in the same `p_link` list (`gobj->next`).
    pub fn next(&self) -> Option<GObjId> {
        self.next
    }
    pub fn prev(&self) -> Option<GObjId> {
        self.prev
    }
    pub fn next_gx(&self) -> Option<GObjId> {
        self.next_gx
    }
    pub fn prev_gx(&self) -> Option<GObjId> {
        self.prev_gx
    }
    /// Head of the object's own proc chain (`gobj->proc`); most recently
    /// added first (gobjproc.c:88-89).
    pub fn first_proc(&self) -> Option<ProcId> {
        self.proc_head
    }
    /// `gxlink_prios` bitmask (gobj.h:42). Only read by the render pass.
    pub fn gxlink_prios(&self) -> u64 {
        self.gxlink_prios
    }
    pub fn set_gxlink_prios(&mut self, prios: u64) {
        self.gxlink_prios = prios;
    }
    pub fn has_render_cb(&self) -> bool {
        self.render_cb.is_some()
    }
    pub fn has_hsd_obj(&self) -> bool {
        self.hsd_obj.is_some()
    }
    pub fn has_user_data(&self) -> bool {
        self.user_data.is_some()
    }
}

impl<S: Storage> fmt::Debug for Object<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Object<S>")
            .field("classifier", &self.classifier)
            .field("p_link", &self.p_link)
            .field("p_priority", &self.p_priority)
            .field("gx_link", &self.gx_link)
            .field("render_priority", &self.render_priority)
            .field("obj_kind", &self.obj_kind)
            .field("user_data_kind", &self.user_data_kind)
            .field("next", &self.next)
            .field("prev", &self.prev)
            .field("proc_head", &self.proc_head)
            .finish_non_exhaustive()
    }
}

/// `HSD_GObjProc` (gobjproc.h:8-19).
pub struct Process<S: Storage> {
    /// Next proc of the same GObj (`child`).
    child: Option<ProcId>,
    /// Next/prev in the global list for this priority.
    next: Option<ProcId>,
    prev: Option<ProcId>,
    s_link: u8,
    /// `flags_1`: set by `HSD_GObj_80390C5C`, cleared by `HSD_GObj_80390C84`.
    /// A set flag skips the callback (gobj.c:109-110).
    paused: bool,
    /// `flags_2`: only ever cleared (gobj.c:72-75, gobjproc.c:159). Also
    /// skips the callback.
    flag2: bool,
    /// `flags_3` (2 bits): the frame tag the proc was last visited on, or 3
    /// for a fresh proc (gobjproc.c:160) so it always differs from the
    /// running tag (0..=2).
    frame_tag: u8,
    gobj: GObjId,
    /// `None` for tagged procs or while an owned callback is executing.
    callback: Option<S::Callback>,
    dispatch_tag: Option<usize>,
}

impl<S: Storage> Process<S> {
    /// Priority (`s_link`).
    pub fn priority(&self) -> u8 {
        self.s_link
    }
    pub fn gobj(&self) -> GObjId {
        self.gobj
    }
    /// Next proc in the global list for this priority (`next`).
    pub fn next(&self) -> Option<ProcId> {
        self.next
    }
    pub fn prev(&self) -> Option<ProcId> {
        self.prev
    }
    /// Next proc of the same GObj (`child`).
    pub fn child(&self) -> Option<ProcId> {
        self.child
    }
    /// `flags_1`
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    /// `flags_2`
    pub fn flag2(&self) -> bool {
        self.flag2
    }
    /// `flags_3`
    pub fn frame_tag(&self) -> u8 {
        self.frame_tag
    }
}

impl<S: Storage> fmt::Debug for Process<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Process<S>")
            .field("s_link", &self.s_link)
            .field("gobj", &self.gobj)
            .field("next", &self.next)
            .field("prev", &self.prev)
            .field("child", &self.child)
            .field("paused", &self.paused)
            .field("flag2", &self.flag2)
            .field("frame_tag", &self.frame_tag)
            .finish()
    }
}

/// `HSD_GObj_804CE3E4` (gobj.h:88-103): requests made against the GObj whose
/// proc is currently running, deferred until that proc returns.
#[derive(Clone, Default)]
struct Deferred {
    /// `b0`: set only while the deferred requests are being applied
    /// (gobj.c:117). While set, destroy/relink/remove happen immediately and
    /// the list functions fix up `run_next` (gobjproc.c:90-94, 101-103).
    processing: bool,
    /// `b1`: `HSD_GObjPLink_80390228(current gobj)` was requested.
    destroy: bool,
    /// `b2`: `HSD_GObjProc_8038FE24(current proc)` was requested.
    remove_proc: bool,
    /// `b3` plus `type`/`p_link`/`p_prio`/`gobj`: relink of the current gobj.
    relink: Option<(InsertWhere, u8, u8)>,
}

impl Deferred {
    /// `HSD_GObj_804CE3E4.flags != 0` (gobj.c:116). `b0` is never set outside
    /// the processing block, so this is the same as "any request pending".
    fn pending(&self) -> bool {
        self.destroy || self.remove_proc || self.relink.is_some()
    }
}

/// The object arena plus scheduler state. See the module docs.
pub struct Scheduler<S: Storage> {
    config: WorldConfig,
    gobjs: Slab<Object<S>>,
    procs: Slab<Process<S>>,
    /// `HSD_GObj_Entities[p_link]`: list heads.
    plink_head: Vec<Option<GObjId>>,
    /// `plinklow_gobjs[p_link]`: list tails.
    plink_tail: Vec<Option<GObjId>>,
    /// `HSD_GObjGXLinkHead[gx_link]`, `gx_link_max + 2` entries.
    gx_head: Vec<Option<GObjId>>,
    /// `HSD_GObj_804D7820[gx_link]`: GX list tails.
    gx_tail: Vec<Option<GObjId>>,
    /// `HSD_GObj_804D7840[s_link]`: global proc list heads, one per priority.
    proc_head: Vec<Option<ProcId>>,
    /// `HSD_GObj_804D7844[p_link + s_link * (p_link_max + 1)]`: for each
    /// (priority, p_link), the last proc in that priority's list that belongs
    /// to an object of that p_link. Insertion anchor.
    proc_anchor: Vec<Option<ProcId>>,
    /// `HSD_GObj_804D7810`: `obj_kind` remover table.
    obj_removers: Vec<S::ObjectRemover>,
    /// `HSD_GObj_804D783C`: frame tag, cycles 0,1,2.
    frame_tag: u8,
    /// `HSD_GObj_804D7834`: priority being run.
    run_pri: u8,
    /// `HSD_GObj_804D7830`: proc to visit after the current one.
    run_next: Option<ProcId>,
    /// `HSD_GObj_804D7838`: proc whose callback is running.
    cur_proc: Option<ProcId>,
    /// `HSD_GObj_804D781C`: gobj whose proc callback is running.
    cur_gobj: Option<GObjId>,
    deferred: Deferred,
    /// `*HSD_GObjLibInitData.unk_2`: bit `p_link` set pauses that list's
    /// procs (gobj.c:94-95, 109). Melee rewrites it every frame from the
    /// pause state (gm/gm_1A45.c:324-333).
    pause_mask: u64,
    /// True inside [`World::run_procs`].
    running: bool,
}

impl<S: Storage> fmt::Debug for Scheduler<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scheduler<S>")
            .field("config", &self.config)
            .field("gobjs", &self.gobjs.len())
            .field("procs", &self.procs.len())
            .field("frame_tag", &self.frame_tag)
            .field("cur_gobj", &self.cur_gobj)
            .field("cur_proc", &self.cur_proc)
            .finish_non_exhaustive()
    }
}

fn drop_obj(_: &mut World, _: Box<dyn Any>) {}

impl<S: Storage> Default for Scheduler<S> {
    fn default() -> Self {
        Self::new(WorldConfig::default())
    }
}

impl<S: Storage> Scheduler<S> {
    /// `HSD_GObj_80391304` (gobjinit.c:20-93) without the func-table walk:
    /// allocate and clear every list. No object kinds are registered; call
    /// [`World::register_builtin_obj_kinds`] or [`World::register_obj_kind`].
    pub fn new(config: WorldConfig) -> Scheduler<S> {
        let np = config.p_link_max as usize + 1;
        let ng = config.gx_link_max as usize + 2;
        let ns = config.gproc_pri_max as usize + 1;
        Scheduler {
            config,
            gobjs: Slab::default(),
            procs: Slab::default(),
            plink_head: vec![None; np],
            plink_tail: vec![None; np],
            gx_head: vec![None; ng],
            gx_tail: vec![None; ng],
            proc_head: vec![None; ns],
            proc_anchor: vec![None; ns * np],
            obj_removers: Vec::new(),
            frame_tag: 0,
            run_pri: 0,
            run_next: None,
            cur_proc: None,
            cur_gobj: None,
            deferred: Deferred::default(),
            pause_mask: 0,
            running: false,
        }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    // ------------------------------------------------------------------
    // Object kind registry (gobj.c:248-269, gobjinit.c:66-85)
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // Lookup
    // ------------------------------------------------------------------

    pub fn contains(&self, id: GObjId) -> bool {
        self.gobjs.contains(id.0)
    }

    pub fn get(&self, id: GObjId) -> Option<&Object<S>> {
        self.gobjs.get(id.0)
    }

    pub fn get_mut(&mut self, id: GObjId) -> Option<&mut Object<S>> {
        self.gobjs.get_mut(id.0)
    }

    /// Panics on a stale id; use [`World::get`] to probe.
    pub fn gobj(&self, id: GObjId) -> &Object<S> {
        self.gobjs.get(id.0).unwrap_or_else(|| panic!("stale {id}"))
    }

    pub fn gobj_mut(&mut self, id: GObjId) -> &mut Object<S> {
        self.gobjs
            .get_mut(id.0)
            .unwrap_or_else(|| panic!("stale {id}"))
    }

    pub fn contains_proc(&self, id: ProcId) -> bool {
        self.procs.contains(id.0)
    }

    pub fn get_proc(&self, id: ProcId) -> Option<&Process<S>> {
        self.procs.get(id.0)
    }

    /// Panics on a stale id; use [`World::get_proc`] to probe.
    pub fn proc(&self, id: ProcId) -> &Process<S> {
        self.procs.get(id.0).unwrap_or_else(|| panic!("stale {id}"))
    }

    fn proc_mut(&mut self, id: ProcId) -> &mut Process<S> {
        self.procs
            .get_mut(id.0)
            .unwrap_or_else(|| panic!("stale {id}"))
    }

    /// Number of live GObjs.
    pub fn gobj_count(&self) -> usize {
        self.gobjs.len()
    }

    /// Number of live procs.
    pub fn proc_count(&self) -> usize {
        self.procs.len()
    }

    /// Head of a `p_link` list, `HSD_GObj_Entities[p_link]`.
    pub fn plink_head(&self, p_link: u8) -> Option<GObjId> {
        self.plink_head[p_link as usize]
    }

    /// Tail of a `p_link` list, `plinklow_gobjs[p_link]`.
    pub fn plink_tail(&self, p_link: u8) -> Option<GObjId> {
        self.plink_tail[p_link as usize]
    }

    /// Walk a `p_link` list head to tail following `next`. This is the order
    /// `harness/dolphin/walk.py` reads (`walk_gobj_list`) and the order of
    /// `HSD_GObj_Entities->fighters` when `p_link` is
    /// [`crate::consts::plink::FIGHTER`].
    pub fn iter_plink(&self, p_link: u8) -> impl Iterator<Item = GObjId> + '_ {
        let mut cur = self.plink_head[p_link as usize];
        core::iter::from_fn(move || {
            let id = cur?;
            cur = self.gobj(id).next;
            Some(id)
        })
    }

    /// Head of a GX list, `HSD_GObjGXLinkHead[gx_link]`.
    pub fn gx_head(&self, gx_link: u8) -> Option<GObjId> {
        self.gx_head[gx_link as usize]
    }

    /// Walk a GX list head to tail following `next_gx`, the order
    /// `HSD_GObj_80390ED0` / `HSD_GObj_80390FC0` render in (gobj.c:168-169,
    /// 189-197).
    pub fn iter_gxlink(&self, gx_link: u8) -> impl Iterator<Item = GObjId> + '_ {
        let mut cur = self.gx_head[gx_link as usize];
        core::iter::from_fn(move || {
            let id = cur?;
            cur = self.gobj(id).next_gx;
            Some(id)
        })
    }

    /// The `gx_link` value [`World::setup_gx_link_max`] uses: `gx_link_max + 1`.
    pub fn gx_link_max_list(&self) -> u8 {
        self.config.gx_link_max + 1
    }

    /// Head of the global proc list for one priority, `HSD_GObj_804D7840[s_link]`.
    pub fn proc_list_head(&self, priority: u8) -> Option<ProcId> {
        self.proc_head[priority as usize]
    }

    /// Walk the global proc list for one priority in run order.
    pub fn iter_procs(&self, priority: u8) -> impl Iterator<Item = ProcId> + '_ {
        let mut cur = self.proc_head[priority as usize];
        core::iter::from_fn(move || {
            let id = cur?;
            cur = self.proc(id).next;
            Some(id)
        })
    }

    /// Walk one GObj's proc chain (`gobj->proc`, then `child`), most recently
    /// added first.
    pub fn iter_gobj_procs(&self, id: GObjId) -> impl Iterator<Item = ProcId> + '_ {
        let mut cur = self.gobj(id).proc_head;
        core::iter::from_fn(move || {
            let p = cur?;
            cur = self.proc(p).child;
            Some(p)
        })
    }

    /// `HSD_GObjObject_80390A3C` (gobjobject.c:6-17): first GObj in the
    /// `p_link` list with this classifier.
    pub fn find_by_classifier(&self, p_link: u8, classifier: u16) -> Option<GObjId> {
        self.iter_plink(p_link)
            .find(|&id| self.gobj(id).classifier == classifier)
    }

    // ------------------------------------------------------------------
    // Run state
    // ------------------------------------------------------------------

    /// `HSD_GObj_804D783C`. Cycles 0, 1, 2; incremented at the top of
    /// [`World::run_procs`] (gobj.c:96-99).
    pub fn frame_tag(&self) -> u8 {
        self.frame_tag
    }

    /// `HSD_GObj_804D781C`: the GObj whose proc callback is running.
    pub fn current_gobj(&self) -> Option<GObjId> {
        self.cur_gobj
    }

    /// `HSD_GObj_804D7838`: the proc whose callback is running.
    pub fn current_proc(&self) -> Option<ProcId> {
        self.cur_proc
    }

    /// `*HSD_GObjLibInitData.unk_2`. Bit `p_link` set pauses every proc
    /// whose GObj is in that list, for the next [`World::run_procs`].
    pub fn pause_mask(&self) -> u64 {
        self.pause_mask
    }

    pub fn set_pause_mask(&mut self, mask: u64) {
        self.pause_mask = mask;
    }

    // ------------------------------------------------------------------
    // p_link list (gobjplink.c)
    // ------------------------------------------------------------------

    /// `GObj_PReorder` (gobjplink.c:11-27): link `id` into its `p_link` list
    /// directly after `after`, or at the head when `after` is `None`.
    fn plink_insert_after(&mut self, id: GObjId, after: Option<GObjId>) {
        let link = self.gobj(id).p_link as usize;
        self.gobj_mut(id).prev = after;
        let next = match after {
            Some(a) => {
                let n = self.gobj(a).next;
                self.gobj_mut(a).next = Some(id);
                n
            }
            None => {
                let n = self.plink_head[link];
                self.plink_head[link] = Some(id);
                n
            }
        };
        self.gobj_mut(id).next = next;
        match next {
            Some(n) => self.gobj_mut(n).prev = Some(id),
            None => self.plink_tail[link] = Some(id),
        }
    }

    /// `HSD_GObjPLink_80390228` lines 116-125 / `HSD_GObjPLink_8039032C` lines
    /// 159-168: unlink from the `p_link` list.
    fn plink_unlink(&mut self, id: GObjId) {
        let (link, prev, next) = {
            let g = self.gobj(id);
            (g.p_link as usize, g.prev, g.next)
        };
        match prev {
            Some(p) => self.gobj_mut(p).next = next,
            None => self.plink_head[link] = next,
        }
        match next {
            Some(n) => self.gobj_mut(n).prev = prev,
            None => self.plink_tail[link] = prev,
        }
    }

    /// The `switch (where)` of `CreateGObj` / `HSD_GObjPLink_8039032C`.
    fn plink_place(&mut self, id: GObjId, where_: InsertWhere) {
        let (link, prio) = {
            let g = self.gobj(id);
            (g.p_link as usize, g.p_priority)
        };
        match where_ {
            InsertWhere::AfterEqualPriority => {
                // gobj_first_lower_prio (gobjplink.c:36-43)
                let mut cur = self.plink_tail[link];
                while let Some(c) = cur {
                    if self.gobj(c).p_priority > prio {
                        cur = self.gobj(c).prev;
                    } else {
                        break;
                    }
                }
                self.plink_insert_after(id, cur);
            }
            InsertWhere::BeforeEqualPriority => {
                // gobj_first_higher_prio (gobjplink.c:45-53)
                let mut cur = self.plink_head[link];
                while let Some(c) = cur {
                    if self.gobj(c).p_priority < prio {
                        cur = self.gobj(c).next;
                    } else {
                        break;
                    }
                }
                let after = match cur {
                    Some(c) => self.gobj(c).prev,
                    None => self.plink_tail[link],
                };
                self.plink_insert_after(id, after);
            }
            InsertWhere::After(pos) => self.plink_insert_after(id, Some(pos)),
            InsertWhere::Before(pos) => {
                let after = self.gobj(pos).prev;
                self.plink_insert_after(id, after);
            }
        }
    }

    /// `CreateGObj` (gobjplink.c:57-96). Panics if `p_link > p_link_max`
    /// (`HSD_ASSERT(0xA8, ...)`).
    pub fn create_at(
        &mut self,
        where_: InsertWhere,
        classifier: u16,
        p_link: u8,
        priority: u8,
    ) -> GObjId {
        assert!(
            p_link <= self.config.p_link_max,
            "p_link {p_link} > p_link_max {}",
            self.config.p_link_max
        );
        let id = GObjId(self.gobjs.insert(Object {
            classifier,
            p_link,
            gx_link: GXLINK_NONE,
            p_priority: priority,
            render_priority: 0,
            obj_kind: OBJ_NONE,
            user_data_kind: USER_DATA_NONE,
            next: None,
            prev: None,
            next_gx: None,
            prev_gx: None,
            proc_head: None,
            render_cb: None,
            gxlink_prios: 0,
            hsd_obj: None,
            user_data: None,
            user_data_remove: None,
        }));
        self.plink_place(id, where_);
        id
    }

    /// `GObj_Create` (gobjplink.c:98-101): `CreateGObj(0, ...)`, i.e.
    /// [`InsertWhere::AfterEqualPriority`]. Objects created with the same
    /// priority end up in creation order, head to tail.
    pub fn create(&mut self, classifier: u16, p_link: u8, priority: u8) -> GObjId {
        self.create_at(
            InsertWhere::AfterEqualPriority,
            classifier,
            p_link,
            priority,
        )
    }

    /// `HSD_GObjPLink_80390228` (gobjplink.c:103-127): destroy a GObj.
    ///
    /// If `id` is the GObj whose proc is currently running (and the deferred
    /// block is not active), the destroy is recorded and performed right after
    /// that proc returns (lines 106-109, gobj.c:116-119). Otherwise it happens
    /// now: user data remover, `obj_kind` remover, all procs, GX unlink,
    /// `p_link` unlink, free — in that order (lines 110-126).
    pub fn destroy(&mut self, id: GObjId) {
        assert!(self.contains(id), "destroy: stale {id}");
        if !self.deferred.processing && Some(id) == self.cur_gobj {
            self.deferred.destroy = true;
            return;
        }
        S::remove_user_data(self, id);
        S::remove_obj(self, id);
        self.remove_all_procs(id);
        if self.gobj(id).gx_link != GXLINK_NONE {
            self.remove_gx_link(id);
        }
        self.plink_unlink(id);
        self.gobjs.remove(id.0);
    }

    /// `HSD_GObjPLink_8039032C` (gobjplink.c:129-196): move a GObj to another
    /// `p_link` / priority / position, re-registering its procs so they land
    /// where their new list position dictates.
    ///
    /// Deferred like [`World::destroy`] when `id` is the running GObj (lines
    /// 141-148, gobj.c:121-127). Otherwise (lines 149-195): every proc is
    /// unlinked from the global lists and the chain is reversed so
    /// re-insertion runs oldest-first (restoring creation order); the GObj is
    /// moved; each proc is re-linked, and a proc whose tag equals
    /// `frame_tag - 2` (mod 3) is bumped to `frame_tag - 1` (lines 185-194).
    /// Melee never calls this directly; it is reachable only through the
    /// deferred block.
    pub fn relink(&mut self, where_: InsertWhere, id: GObjId, p_link: u8, priority: u8) {
        assert!(
            p_link <= self.config.p_link_max,
            "p_link {p_link} > p_link_max {}",
            self.config.p_link_max
        );
        assert!(self.contains(id), "relink: stale {id}");
        if !self.deferred.processing && Some(id) == self.cur_gobj {
            self.deferred.relink = Some((where_, p_link, priority));
            return;
        }
        // Unlink every proc from the global lists, reversing the chain.
        let mut cur = self.gobj(id).proc_head;
        let mut reversed: Option<ProcId> = None;
        while let Some(c) = cur {
            self.unlink_proc(c);
            let child = self.proc(c).child;
            self.proc_mut(c).child = reversed;
            reversed = Some(c);
            cur = child;
        }
        self.gobj_mut(id).proc_head = None;
        self.plink_unlink(id);
        {
            let g = self.gobj_mut(id);
            g.p_link = p_link;
            g.p_priority = priority;
        }
        self.plink_place(id, where_);
        let tag_new = if self.frame_tag == 0 {
            2
        } else {
            self.frame_tag - 1
        };
        let tag_cur = if tag_new == 0 { 2 } else { tag_new - 1 };
        let mut cur = reversed;
        while let Some(c) = cur {
            let child = self.proc(c).child;
            self.link_proc(c);
            if self.proc(c).frame_tag == tag_cur {
                self.proc_mut(c).frame_tag = tag_new;
            }
            cur = child;
        }
    }

    // ------------------------------------------------------------------
    // Procs (gobjproc.c)
    // ------------------------------------------------------------------

    fn anchor_index(&self, p_link: usize, s_link: usize) -> usize {
        p_link + s_link * (self.config.p_link_max as usize + 1)
    }

    /// `HSD_GObjProc_8038FAA8` (gobjproc.c:12-95): link a proc into the global
    /// list for its priority and onto its GObj's chain.
    ///
    /// Position in the global list (lines 28-81):
    /// 1. If an anchor exists for (`p_link`, `s_link`) — the last proc of that
    ///    priority belonging to this `p_link` — walk from this GObj backwards
    ///    through `prev` and insert after the first proc of the same priority
    ///    found in any of those GObjs' chains (lines 28-51). Chains are
    ///    newest-first, so within one GObj the new proc lands after its most
    ///    recent sibling of that priority: creation order.
    /// 2. Otherwise this proc becomes the anchor (lines 52-55) and, if no
    ///    proc was found in step 1, it goes after the anchor of the nearest
    ///    lower `p_link` (lines 59-66), or at the list head if none (71-73).
    ///
    /// The result is a global list ordered by `p_link`, then by GObj position
    /// within the `p_link` list, then by proc creation order. Lines 90-94: if
    /// the deferred block is applying a relink and the proc lands directly
    /// after the running proc, it becomes the next to visit.
    fn link_proc(&mut self, pid: ProcId) {
        let (gobj, s_link) = {
            let p = self.proc(pid);
            (p.gobj, p.s_link as usize)
        };
        let mut p_link = self.gobj(gobj).p_link as usize;
        let own_anchor = self.anchor_index(p_link, s_link);
        let mut dst: Option<ProcId> = None;

        if let Some(anchor) = self.proc_anchor[own_anchor] {
            let mut cur_gobj = Some(gobj);
            'scan: while let Some(g) = cur_gobj {
                let mut d = self.gobj(g).proc_head;
                while let Some(dp) = d {
                    if self.proc(dp).s_link as usize == s_link {
                        if anchor == dp {
                            self.proc_anchor[own_anchor] = Some(pid);
                        }
                        dst = Some(dp);
                        break 'scan;
                    }
                    d = self.proc(dp).child;
                }
                cur_gobj = self.gobj(g).prev;
            }
        } else {
            self.proc_anchor[own_anchor] = Some(pid);
        }

        if dst.is_none() {
            // while (p_link-- != 0)
            while p_link != 0 {
                p_link -= 1;
                let a = self.proc_anchor[self.anchor_index(p_link, s_link)];
                if a.is_some() {
                    dst = a;
                    break;
                }
            }
        }

        match dst {
            None => {
                let head = self.proc_head[s_link];
                self.proc_head[s_link] = Some(pid);
                let p = self.proc_mut(pid);
                p.next = head;
                p.prev = None;
            }
            Some(d) => {
                let dnext = self.proc(d).next;
                self.proc_mut(d).next = Some(pid);
                let p = self.proc_mut(pid);
                p.next = dnext;
                p.prev = Some(d);
            }
        }

        let (next, prev) = {
            let p = self.proc(pid);
            (p.next, p.prev)
        };
        if let Some(n) = next {
            self.proc_mut(n).prev = Some(pid);
        }
        let old_head = self.gobj(gobj).proc_head;
        self.proc_mut(pid).child = old_head;
        self.gobj_mut(gobj).proc_head = Some(pid);

        if self.deferred.processing
            && prev == self.cur_proc
            && next == self.run_next
            && s_link == self.run_pri as usize
        {
            self.run_next = Some(pid);
        }
    }

    /// `HSD_GObjProc_8038FC18` (gobjproc.c:97-126): unlink a proc from the
    /// global list for its priority. Fixes `run_next` while the deferred block
    /// is active (101-103) and moves the (`p_link`, `s_link`) anchor to the
    /// previous proc if that one belongs to the same `p_link`, else clears it
    /// (104-117).
    fn unlink_proc(&mut self, pid: ProcId) {
        let (gobj, s_link, prev, next) = {
            let p = self.proc(pid);
            (p.gobj, p.s_link as usize, p.prev, p.next)
        };
        let p_link = self.gobj(gobj).p_link;
        if self.deferred.processing && Some(pid) == self.run_next {
            self.run_next = next;
        }
        let a = self.anchor_index(p_link as usize, s_link);
        if self.proc_anchor[a] == Some(pid) {
            self.proc_anchor[a] = match prev {
                Some(pp) if self.gobj(self.proc(pp).gobj).p_link == p_link => Some(pp),
                _ => None,
            };
        }
        match prev {
            Some(pp) => self.proc_mut(pp).next = next,
            None => self.proc_head[s_link] = next,
        }
        if let Some(n) = next {
            self.proc_mut(n).prev = prev;
        }
    }

    /// `HSD_GObjProc_8038FCE4` (gobjproc.c:128-141): unlink from the global
    /// list and from the owner's chain.
    fn detach_proc(&mut self, pid: ProcId) {
        let gobj = self.proc(pid).gobj;
        self.unlink_proc(pid);
        let child = self.proc(pid).child;
        if self.gobj(gobj).proc_head == Some(pid) {
            self.gobj_mut(gobj).proc_head = child;
        } else {
            let mut cur = self
                .gobj(gobj)
                .proc_head
                .expect("proc chain corrupt: owner has no procs");
            while self.proc(cur).child != Some(pid) {
                cur = self
                    .proc(cur)
                    .child
                    .expect("proc chain corrupt: proc not in owner's chain");
            }
            self.proc_mut(cur).child = child;
        }
    }

    /// Register a typed external dispatcher token without capturing its owner.
    /// The owner is borrowed once by run_procs_with; list and mutation semantics
    /// are identical to ordinary HSD callbacks.
    pub fn add_tagged_proc(&mut self, gobj: GObjId, priority: u8, tag: usize) -> ProcId {
        self.add_proc_slot(gobj, priority, None, Some(tag))
    }

    fn add_proc_slot(
        &mut self,
        gobj: GObjId,
        priority: u8,
        callback: Option<S::Callback>,
        dispatch_tag: Option<usize>,
    ) -> ProcId {
        assert!(
            priority <= self.config.gproc_pri_max,
            "proc priority {priority} > gproc_pri_max {}",
            self.config.gproc_pri_max
        );
        assert!(self.contains(gobj), "add_proc: stale {gobj}");
        let pid = ProcId(self.procs.insert(Process {
            child: None,
            next: None,
            prev: None,
            s_link: priority,
            paused: false,
            flag2: false,
            frame_tag: 3,
            gobj,
            callback,
            dispatch_tag,
        }));
        self.link_proc(pid);
        pid
    }

    /// `HSD_GObjProc_8038FE24` (gobjproc.c:167-175): remove a proc. Deferred
    /// if it is the running proc and the deferred block is not active
    /// (169-170); otherwise detached and freed now.
    pub fn remove_proc(&mut self, pid: ProcId) {
        assert!(self.contains_proc(pid), "remove_proc: stale {pid}");
        if !self.deferred.processing && Some(pid) == self.cur_proc {
            self.deferred.remove_proc = true;
        } else {
            self.detach_proc(pid);
            self.procs.remove(pid.0);
        }
    }

    /// `HSD_GObjProc_8038FED4` (gobjproc.c:177-185): remove every proc of a
    /// GObj, walking the chain (newest first). The running proc, if among
    /// them, is deferred.
    pub fn remove_all_procs(&mut self, gobj: GObjId) {
        let mut cur = self.gobj(gobj).proc_head;
        while let Some(c) = cur {
            let next = self.proc(c).child;
            self.remove_proc(c);
            cur = next;
        }
    }

    /// `HSD_GObj_80390C5C` / `HSD_GObj_80390C84` (gobj.c:62-70): set or
    /// clear `flags_1` on every proc of a GObj. Set skips the callbacks.
    pub fn set_procs_paused(&mut self, gobj: GObjId, paused: bool) {
        let mut cur = self.gobj(gobj).proc_head;
        while let Some(c) = cur {
            let p = self.proc_mut(c);
            p.paused = paused;
            cur = p.child;
        }
    }

    /// `HSD_GObj_80390CAC` (gobj.c:72-75): clear `flags_2` on every proc of a
    /// GObj. Nothing in HSD or Melee sets `flags_2`; kept for completeness.
    pub fn clear_procs_flag2(&mut self, gobj: GObjId) {
        let mut cur = self.gobj(gobj).proc_head;
        while let Some(c) = cur {
            let p = self.proc_mut(c);
            p.flag2 = false;
            cur = p.child;
        }
    }

    /// `HSD_GObj_80390CD4` (gobj.c:77-85): stamp every proc of a GObj with the
    /// current frame tag so none of them runs (again) this frame. Menus use it
    /// right after creating procs from inside a proc.
    pub fn skip_procs_this_frame(&mut self, gobj: GObjId) {
        let tag = self.frame_tag;
        let mut cur = self.gobj(gobj).proc_head;
        while let Some(c) = cur {
            let p = self.proc_mut(c);
            p.frame_tag = tag;
            cur = p.child;
        }
    }

    /// `p->flags_3 = HSD_GObj_804D783C` on a single proc, the idiom in
    /// mn/mndeflicker.c:91-93 after `HSD_GObj_SetupProc` from inside a proc:
    /// the new proc will not run until next frame.
    pub fn skip_proc_this_frame(&mut self, pid: ProcId) {
        let tag = self.frame_tag;
        self.proc_mut(pid).frame_tag = tag;
    }

    /// `HSD_GObj_80390CFC` / `GObj_RunProcs` (gobj.c:87-141): one frame of
    /// proc invocation. Called once per frame by Melee's main loop
    /// (gm/gm_1A45.c:340), after the pause mask is written (324-333).
    ///
    /// Order (lines 96-140): advance the frame tag (0,1,2,0,...); for each
    /// priority 0..=`gproc_pri_max`, walk that priority's global list head to
    /// tail. A proc is visited if its tag differs from the frame's; visiting
    /// stamps the tag, then invokes the callback unless the owner's `p_link`
    /// bit is set in the pause mask or `flags_1`/`flags_2` is set (the stamp
    /// happens even when the callback is skipped). After the callback the
    /// successor is re-read from the proc (line 115), so a proc destroyed or
    /// added during the callback is respected. Requests deferred during the
    /// callback (destroy / relink / remove of the running GObj or proc) are
    /// then applied with `processing` set (116-133).
    ///
    /// Consequences that matter for frame ordering:
    /// - A proc added during a callback runs this frame iff it lands after
    ///   the current position in a priority `>=` the current one; a lower
    ///   `p_link` at the same priority lands before and waits until next frame.
    /// - A GObj destroyed by another's proc is gone immediately; its unrun
    ///   procs never run. A GObj destroying itself finishes its callback
    ///   first, and its later procs (this frame) are dropped with it.
    ///
    /// Not re-entrant: a callback must not call `run_procs`.
    pub fn run_procs(&mut self) {
        self.run_procs_with(|_, _, _| panic!("tagged proc needs an external dispatcher"));
    }

    /// Run the same scheduler with a borrowed, statically dispatched owner for
    /// tagged registrations. Ordinary callbacks remain available to engine users.
    pub fn run_procs_with(&mut self, mut dispatch: impl FnMut(&mut Scheduler<S>, GObjId, usize)) {
        assert!(!self.running, "run_procs is not re-entrant");
        self.running = true;
        let pause_mask = self.pause_mask;
        self.frame_tag += 1;
        if self.frame_tag > 2 {
            self.frame_tag = 0;
        }
        let tag = self.frame_tag;

        for pri in 0..=self.config.gproc_pri_max {
            self.run_pri = pri;
            let mut proc = self.proc_head[pri as usize];
            while let Some(pid) = proc {
                self.run_next = self.proc(pid).next;
                if self.proc(pid).frame_tag != tag {
                    self.proc_mut(pid).frame_tag = tag;
                    let gobj = self.proc(pid).gobj;
                    let p_link = self.gobj(gobj).p_link;
                    let (paused, flag2) = {
                        let p = self.proc(pid);
                        (p.paused, p.flag2)
                    };
                    if pause_mask & (1u64 << p_link) == 0 && !paused && !flag2 {
                        self.cur_gobj = Some(gobj);
                        self.cur_proc = Some(pid);
                        if let Some(tag) = self.proc(pid).dispatch_tag {
                            dispatch(self, gobj, tag);
                        } else {
                            S::invoke(self, pid, gobj);
                        }
                        // gobj.c:115. The running proc is never freed during
                        // its own callback (removal is deferred), so it is
                        // still here.
                        self.run_next = self.get_proc(pid).and_then(|p| p.next);
                        if self.deferred.pending() {
                            self.deferred.processing = true;
                            if self.deferred.destroy {
                                self.destroy(gobj);
                            } else {
                                if let Some((where_, p_link, prio)) = self.deferred.relink {
                                    self.relink(where_, gobj, p_link, prio);
                                }
                                if self.deferred.remove_proc {
                                    self.remove_proc(pid);
                                }
                            }
                            self.deferred = Deferred::default();
                        }
                        self.cur_gobj = None;
                        self.cur_proc = None;
                    }
                }
                proc = self.run_next;
            }
        }
        self.running = false;
    }

    // ------------------------------------------------------------------
    // GX link lists (gobjgxlink.c). Buckets only; nothing is rendered.
    // ------------------------------------------------------------------

    /// `GObj_GXReorder` (gobjgxlink.c:10-31): link into the GX list for
    /// `gx_link` after `after`, or at the head.
    fn gx_insert_after(&mut self, id: GObjId, after: Option<GObjId>) {
        let link = self.gobj(id).gx_link as usize;
        self.gobj_mut(id).prev_gx = after;
        let next = match after {
            Some(a) => {
                let n = self.gobj(a).next_gx;
                self.gobj_mut(a).next_gx = Some(id);
                n
            }
            None => {
                let n = self.gx_head[link];
                self.gx_head[link] = Some(id);
                n
            }
        };
        self.gobj_mut(id).next_gx = next;
        match next {
            Some(n) => self.gobj_mut(n).prev_gx = Some(id),
            None => self.gx_tail[link] = Some(id),
        }
    }

    /// Tail-first scan shared by `GObj_SetupGXLink` and `GObj_SetupGXLinkMax`
    /// (gobjgxlink.c:47-51, 65-68): insert after the last node whose
    /// `render_priority <= new`. Equal priorities go after existing ones.
    fn gx_place_after_equal(&mut self, id: GObjId) {
        let (link, prio) = {
            let g = self.gobj(id);
            (g.gx_link as usize, g.render_priority)
        };
        let mut cur = self.gx_tail[link];
        while let Some(c) = cur {
            if self.gobj(c).render_priority > prio {
                cur = self.gobj(c).prev_gx;
            } else {
                break;
            }
        }
        self.gx_insert_after(id, cur);
    }

    /// Head-first scan shared by `GObj_SetupGXLinkMaxSorted` and
    /// `HSD_GObjGXLink_80390908` (gobjgxlink.c:72-82, 94-101, 129-147):
    /// insert before the first node whose `render_priority >= new`. Equal
    /// priorities go before existing ones.
    fn gx_place_before_equal(&mut self, id: GObjId) {
        let (link, prio) = {
            let g = self.gobj(id);
            (g.gx_link as usize, g.render_priority)
        };
        let mut cur = self.gx_head[link];
        while let Some(c) = cur {
            if self.gobj(c).render_priority < prio {
                cur = self.gobj(c).next_gx;
            } else {
                break;
            }
        }
        let after = match cur {
            Some(c) => self.gobj(c).prev_gx,
            None => self.gx_tail[link],
        };
        self.gx_insert_after(id, after);
    }

    /// `HSD_GObjGXLink_8039084C` (gobjgxlink.c:104-127): unlink from the GX
    /// list and reset `gx_link`/`render_priority`. Panics if not linked
    /// (`HSD_ASSERT(415, ...)`). The render callback is kept, as in C.
    pub fn remove_gx_link(&mut self, id: GObjId) {
        let (link, prev, next) = {
            let g = self.gobj(id);
            assert!(
                g.gx_link != GXLINK_NONE,
                "remove_gx_link: {id} not on a GX list"
            );
            (g.gx_link as usize, g.prev_gx, g.next_gx)
        };
        match prev {
            Some(p) => self.gobj_mut(p).next_gx = next,
            None => self.gx_head[link] = next,
        }
        match next {
            Some(n) => self.gobj_mut(n).prev_gx = prev,
            None => self.gx_tail[link] = prev,
        }
        let g = self.gobj_mut(id);
        g.gx_link = GXLINK_NONE;
        g.render_priority = 0;
        g.prev_gx = None;
        g.next_gx = None;
    }

    /// `HSD_GObjGXLink_80390908` (gobjgxlink.c:138-148): move to another GX
    /// list/priority; equal priorities go before existing ones. Panics if
    /// `gx_link > gx_link_max` or the object is not currently linked.
    pub fn change_gx_link(&mut self, id: GObjId, gx_link: u8, priority: u8) {
        assert!(
            gx_link <= self.config.gx_link_max,
            "gx_link {gx_link} > gx_link_max {}",
            self.config.gx_link_max
        );
        self.remove_gx_link(id);
        {
            let g = self.gobj_mut(id);
            g.gx_link = gx_link;
            g.render_priority = priority;
        }
        self.gx_place_before_equal(id);
    }

    /// `HSD_GObjGXLink_803909D8` (gobjgxlink.c:150-163): take `other`'s GX
    /// list and priority and sit directly before it.
    pub fn adopt_gx_link(&mut self, id: GObjId, other: GObjId) {
        let (link, prio, other_prev) = {
            let o = self.gobj(other);
            (o.gx_link, o.render_priority, o.prev_gx)
        };
        self.remove_gx_link(id);
        {
            let g = self.gobj_mut(id);
            g.gx_link = link;
            g.render_priority = prio;
        }
        self.gx_insert_after(id, other_prev);
    }

    // ------------------------------------------------------------------
    // HSD object slot (gobjobject.c)
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // User data (gobjuserdata.c)
    // ------------------------------------------------------------------
}

impl World {
    /// Melee's world as set up by `gm_801A4BD4` (gm/gm_1A45.c:223-229):
    /// [`WorldConfig::MELEE`], the 1-entry SObjLib kind table at index 0
    /// (sobjlib.c:24-28), then the four HSD kinds at 1..=4.
    pub fn melee() -> (World, BuiltinObjKinds) {
        let mut w = World::new(WorldConfig::MELEE);
        let sobj = w.register_obj_kind(drop_obj);
        debug_assert_eq!(sobj, 0);
        let kinds = w.register_builtin_obj_kinds();
        (w, kinds)
    }
    /// `HSD_GObj_803912A8` for one entry: append a remover to the `obj_kind`
    /// table and return its index. In C the table is a chain flattened at
    /// init; the indices come out the same.
    pub fn register_obj_kind(&mut self, remover: ObjRemoveFn) -> u8 {
        let kind = u8::try_from(self.obj_removers.len()).expect("obj_kind table full");
        assert!(kind != OBJ_NONE, "obj_kind table full");
        self.obj_removers.push(remover);
        kind
    }
    /// `HSD_GObj_80391260` (gobj.c:248-255): register camera, light, jobj,
    /// fog in that order. The C removers drop a reference count; here the
    /// boxed object is simply dropped.
    pub fn register_builtin_obj_kinds(&mut self) -> BuiltinObjKinds {
        BuiltinObjKinds {
            camera: self.register_obj_kind(drop_obj),
            light: self.register_obj_kind(drop_obj),
            jobj: self.register_obj_kind(drop_obj),
            fog: self.register_obj_kind(drop_obj),
        }
    }
    /// `HSD_GObj_SetupProc` (gobjproc.c:148-165): create a proc with the
    /// given priority and link it. Panics if `priority > gproc_pri_max`
    /// (`HSD_ASSERT(216, ...)`). The new proc's tag is 3 (line 160), so it
    /// runs the first time the loop reaches it — this frame if it lands after
    /// the current position, otherwise next frame.
    pub fn add_proc<F>(&mut self, gobj: GObjId, priority: u8, callback: F) -> ProcId
    where
        F: FnMut(&mut World, GObjId) + 'static,
    {
        self.add_proc_slot(gobj, priority, Some(Box::new(callback)), None)
    }
    /// `proc->on_invoke(proc->gobj)` (gobj.c:114). The callback is moved out
    /// of the slot for the duration so it can borrow the world mutably.
    fn invoke(&mut self, pid: ProcId, gobj: GObjId) {
        let Some(mut cb) = self.proc_mut(pid).callback.take() else {
            return;
        };
        cb(self, gobj);
        if let Some(p) = self.procs.get_mut(pid.0) {
            if p.callback.is_none() {
                p.callback = Some(cb);
            }
        }
    }
    /// `GObj_SetupGXLink` (gobjgxlink.c:36-53). Panics if
    /// `gx_link > gx_link_max` (`HSD_ASSERT(167, ...)`). Does not unlink first;
    /// the C assumes the object is not yet on a GX list.
    pub fn setup_gx_link<F>(&mut self, id: GObjId, render_cb: F, gx_link: u8, priority: u8)
    where
        F: FnMut(&mut World, GObjId, u32) + 'static,
    {
        assert!(
            gx_link <= self.config.gx_link_max,
            "gx_link {gx_link} > gx_link_max {}",
            self.config.gx_link_max
        );
        {
            let g = self.gobj_mut(id);
            g.render_cb = Some(Box::new(render_cb));
            g.gx_link = gx_link;
            g.render_priority = priority;
        }
        self.gx_place_after_equal(id);
    }
    /// `GObj_SetupGXLinkMax` (gobjgxlink.c:55-70): like
    /// [`World::setup_gx_link`] on the extra list `gx_link_max + 1`.
    pub fn setup_gx_link_max<F>(&mut self, id: GObjId, render_cb: F, priority: u8)
    where
        F: FnMut(&mut World, GObjId, u32) + 'static,
    {
        let link = self.gx_link_max_list();
        {
            let g = self.gobj_mut(id);
            g.render_cb = Some(Box::new(render_cb));
            g.gx_link = link;
            g.render_priority = priority;
        }
        self.gx_place_after_equal(id);
    }
    /// `GObj_SetupGXLinkMaxSorted` (gobjgxlink.c:84-102): the `max` list, but
    /// equal priorities go before existing ones.
    pub fn setup_gx_link_max_sorted<F>(&mut self, id: GObjId, render_cb: F, priority: u8)
    where
        F: FnMut(&mut World, GObjId, u32) + 'static,
    {
        let link = self.gx_link_max_list();
        {
            let g = self.gobj_mut(id);
            g.render_cb = Some(Box::new(render_cb));
            g.gx_link = link;
            g.render_priority = priority;
        }
        self.gx_place_before_equal(id);
    }
    /// Take the render callback out (for a render pass that needs `&mut
    /// World`). Put it back with [`World::restore_render_cb`].
    pub fn take_render_cb(&mut self, id: GObjId) -> Option<Box<RenderFn>> {
        self.gobj_mut(id).render_cb.take()
    }
    pub fn restore_render_cb(&mut self, id: GObjId, cb: Box<RenderFn>) {
        if let Some(g) = self.get_mut(id) {
            g.render_cb = Some(cb);
        }
    }
    /// `HSD_GObjObject_80390A70` (gobjobject.c:19-24): attach an HSD object.
    /// Panics if one is already attached (`HSD_ASSERT(42, ...)`) or `kind` is
    /// not registered.
    pub fn set_obj(&mut self, id: GObjId, kind: u8, obj: Box<dyn Any>) {
        assert!(
            (kind as usize) < self.obj_removers.len(),
            "obj_kind {kind} not registered"
        );
        let g = self.gobj_mut(id);
        assert!(
            g.obj_kind == OBJ_NONE,
            "set_obj: {id} already has an object"
        );
        g.obj_kind = kind;
        g.hsd_obj = Some(obj);
    }
    /// `HSD_GObjObject_80390ADC` (gobjobject.c:26-37): detach and return the
    /// HSD object without running its remover.
    pub fn take_obj(&mut self, id: GObjId) -> Option<Box<dyn Any>> {
        let g = self.gobj_mut(id);
        if g.obj_kind != OBJ_NONE {
            g.obj_kind = OBJ_NONE;
            g.hsd_obj.take()
        } else {
            None
        }
    }
    /// `HSD_GObjObject_80390B0C` (gobjobject.c:39-46): run the kind's remover
    /// on the HSD object and clear the slot. No-op when nothing is attached.
    pub fn remove_obj(&mut self, id: GObjId) {
        let g = self.gobj_mut(id);
        if g.obj_kind == OBJ_NONE {
            return;
        }
        let kind = g.obj_kind;
        g.obj_kind = OBJ_NONE;
        let obj = g.hsd_obj.take();
        let remover = self.obj_removers[kind as usize];
        if let Some(obj) = obj {
            remover(self, obj);
        }
    }
    /// `GObj_InitUserData` (gobjuserdata.c:6-13). Panics if user data is
    /// already attached (`HSD_ASSERT(40, ...)`). `kind` is Melee's tag (4 for
    /// fighters, ft/fighter.c:862); `remover` runs at destroy time with the
    /// world and the boxed data.
    pub fn init_user_data<T, F>(&mut self, id: GObjId, kind: u8, remover: F, data: T)
    where
        T: Any,
        F: FnOnce(&mut World, Box<dyn Any>) + 'static,
    {
        let g = self.gobj_mut(id);
        assert!(
            g.user_data_kind == USER_DATA_NONE,
            "init_user_data: {id} already has user data"
        );
        g.user_data_kind = kind;
        g.user_data = Some(Box::new(data));
        g.user_data_remove = Some(Box::new(remover));
    }
    /// `GObj_RemoveUserData` (gobjuserdata.c:15-25): run the remover and clear
    /// the slot. No-op when nothing is attached. Panics if data is attached
    /// without a remover (`HSD_ASSERT(99, ...)`).
    pub fn remove_user_data(&mut self, id: GObjId) {
        let g = self.gobj_mut(id);
        if g.user_data_kind == USER_DATA_NONE {
            return;
        }
        let remover = g
            .user_data_remove
            .take()
            .unwrap_or_else(|| panic!("remove_user_data: {id} has no remover"));
        g.user_data_kind = USER_DATA_NONE;
        let data = g.user_data.take();
        if let Some(data) = data {
            remover(self, data);
        }
    }
    /// Typed view of a GObj's user data.
    pub fn user_data<T: Any>(&self, id: GObjId) -> Option<&T> {
        self.get(id)?.user_data::<T>()
    }
    pub fn user_data_mut<T: Any>(&mut self, id: GObjId) -> Option<&mut T> {
        self.get_mut(id)?.user_data_mut::<T>()
    }
}
impl GObj {
    /// Typed view of `user_data` (`HSD_GObjGetUserData`, gobj.h:147-150).
    pub fn user_data<T: Any>(&self) -> Option<&T> {
        self.user_data.as_deref()?.downcast_ref()
    }
    pub fn user_data_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.user_data.as_deref_mut()?.downcast_mut()
    }
    /// Typed view of `hsd_obj` (`HSD_GObjGetHSDObj`, gobj.h:152-155).
    pub fn hsd_obj<T: Any>(&self) -> Option<&T> {
        self.hsd_obj.as_deref()?.downcast_ref()
    }
    pub fn hsd_obj_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.hsd_obj.as_deref_mut()?.downcast_mut()
    }
}

impl Clone for Object<Tagged> {
    fn clone(&self) -> Self {
        Self {
            classifier: self.classifier,
            p_link: self.p_link,
            gx_link: self.gx_link,
            p_priority: self.p_priority,
            render_priority: self.render_priority,
            obj_kind: self.obj_kind,
            user_data_kind: self.user_data_kind,
            next: self.next,
            prev: self.prev,
            next_gx: self.next_gx,
            prev_gx: self.prev_gx,
            proc_head: self.proc_head,
            render_cb: self.render_cb,
            gxlink_prios: self.gxlink_prios,
            hsd_obj: self.hsd_obj,
            user_data: self.user_data,
            user_data_remove: self.user_data_remove,
        }
    }
}

impl Clone for Process<Tagged> {
    fn clone(&self) -> Self {
        Self {
            child: self.child,
            next: self.next,
            prev: self.prev,
            s_link: self.s_link,
            paused: self.paused,
            flag2: self.flag2,
            frame_tag: self.frame_tag,
            gobj: self.gobj,
            callback: self.callback,
            dispatch_tag: self.dispatch_tag,
        }
    }
}

impl Clone for Scheduler<Tagged> {
    fn clone(&self) -> Self {
        Self {
            config: self.config,
            gobjs: self.gobjs.clone(),
            procs: self.procs.clone(),
            plink_head: self.plink_head.clone(),
            plink_tail: self.plink_tail.clone(),
            gx_head: self.gx_head.clone(),
            gx_tail: self.gx_tail.clone(),
            proc_head: self.proc_head.clone(),
            proc_anchor: self.proc_anchor.clone(),
            obj_removers: self.obj_removers.clone(),
            frame_tag: self.frame_tag,
            run_pri: self.run_pri,
            run_next: self.run_next,
            cur_proc: self.cur_proc,
            cur_gobj: self.cur_gobj,
            deferred: self.deferred.clone(),
            pause_mask: self.pause_mask,
            running: self.running,
        }
    }
}
