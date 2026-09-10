//! Joint objects: the `HSD_JObj` scene-graph runtime from
//! `src/sysdolphin/baselib/jobj.c` and `jobj.h`, without rendering.
//!
//! # Layout
//!
//! A skeleton is a [`JObjTree`]: an arena of [`JObj`] nodes addressed by
//! [`JObjId`]. Every node keeps the C's three links (`parent`, `child`,
//! `next`) as ids, so traversal order is the C's: a parent points at its
//! first child and siblings chain through `next`. Nothing is ever freed
//! from the arena; the C's reference counting (`HSD_JObjUnref`,
//! `HSD_JObjRemove`, `HSD_JObjRemoveAll`, `JObjReleaseChild`) and the
//! `HSD_IDInsertToTable` id table are not ported. Melee never unlinks a
//! fighter bone at runtime.
//!
//! # Matrix composition (pinned)
//!
//! `HSD_JObjMakeMatrix` (`jobj.c:138-196`) builds the world matrix as
//!
//! 1. `HSD_JObjSetupMatrix(parent)` so the parent's `mtx` is current;
//! 2. the accumulated scale `scl`: with `JOBJ_CLASSICAL_SCALE` it is a copy
//!    of the parent's `scl` (or none); otherwise it is `scale * parent.scl`
//!    componentwise (or `scale` alone) (`jobj.c:143-166`);
//! 3. the local matrix: [`crate::mtx::hsd_mtx_srt_quat`] under
//!    `JOBJ_USE_QUATERNION`, else [`crate::mtx::hsd_mtx_srt`] with
//!    `rotate.{x,y,z}` read as Euler angles; both receive the parent's `scl`
//!    as the compensation vector when the parent has one (`jobj.c:167-183`);
//! 4. `PSMTXConcat(parent->mtx, jobj->mtx, jobj->mtx)` when there is a
//!    parent (`jobj.c:184-186`), i.e. `world = parent_world * local` through
//!    [`crate::mtx::mtx_concat`].
//!
//! `HSD_JObjSetupMatrixSub` (`jobj.c:1385-1443`) calls that, clears
//! `JOBJ_MTX_DIRTY`, and, unless `JOBJ_USER_DEF_MTX` is set, runs the IK /
//! RObj post-passes (deferred, see below) and clears the flag again.
//!
//! # Dirty propagation
//!
//! `HSD_JObjMtxIsDirty` (`jobj.h:558`) is `MTX_DIRTY && !USER_DEF_MTX`.
//! The `HSD_JObjSetMtxDirty` macro (`jobj.h:583`) calls
//! `HSD_JObjSetMtxDirtySub` (`jobj.c:1445`) only when the node is not
//! already dirty; the sub sets bit 6 and recurses into children that lack
//! `JOBJ_MTX_INDEP_PARENT` and are not already dirty, stopping at
//! `JOBJ_INSTANCE` nodes. The SRT setters (`jobj.h:615-1130`) mark dirty
//! unless `JOBJ_MTX_INDEP_SRT` is set. `HSD_JObjSetFlags`/`ClearFlags`
//! (`jobj.c:993`, `1019`) mark dirty when `JOBJ_CLASSICAL_SCALE` toggles.
//! `HSD_JObjCheckDepend` (`jobj.c:30`) re-dirties a clean node whose parent
//! is dirty (or that is an IK joint) before its animation step.
//!
//! # Animation
//!
//! `HSD_JObjAnimAll` (`jobj.c:558`) resets the end-callback counters
//! ([`AObjEndCallback`]), walks the tree in child-then-next order running
//! `HSD_JObjAnim` (`jobj.c:531`) on each node, then would invoke the
//! callbacks; the counters are returned instead (Melee registers none).
//! `HSD_JObjAnim` calls `HSD_JObjCheckDepend`, steps the joint's `AObj`
//! through `JObjUpdateFunc` (`jobj.c:351`), then the RObj list (deferred)
//! and the DObj list. `JObjUpdateFunc` is [`JObjTree::update_func`].
//!
//! # Deferred, with what is needed
//!
//! - **RObj** (`robj.c`): `HSD_JObj::robj` is not stored. Needed by jobj:
//!   `HSD_RObjGetByType` for `REFTYPE_IKHINT` (bone length, `rotate_x`),
//!   `REFTYPE_JOBJ` subtype 1/3 and `REFTYPE_LIMIT` subtype 5/6;
//!   `HSD_RObjUpdateAll`, which drives `JObjUpdateFunc` with the matrix
//!   column ids `0x32..0x35` (a `Vec3` payload) and the re-decomposition ids
//!   `0x36..0x39` (null payload) - both are implemented here and reachable
//!   through [`ObjData`]; `HSD_RObjGetGlobalPosition`; `resolveIKJoint1`
//!   (`jobj.c:1104`), `resolveIKJoint2` (`jobj.c:1257`) and the
//!   `JOBJ_EFFECTOR` arm of `HSD_JObjSetupMatrixSub` (`jobj.c:1404-1430`),
//!   which are the only consumers of `JOBJ_JOINT1`/`JOINT2`/`EFFECTOR`;
//!   `HSD_JObjCheckDepend`'s `robj != NULL` term; the `JOBJ_JOINT1` branch
//!   of `HSD_A_J_ROTX` (`jobj.c:379-384`); and the `RObj*Anim*` list
//!   functions plus `HSD_RObjLoadDesc`/`ResolveRefsAll`. Every one of those
//!   paths is a no-op here, which is exactly the C's behaviour for a joint
//!   whose `robj` is `NULL`.
//! - **Spline** (`spline.c`): the `HSD_A_J_PATH` track (`jobj.c:362-377`)
//!   and the `aobj->hsd_obj` translation override in `HSD_JObjMakeMatrix`
//!   (`jobj.c:187-195`) need `splArcLengthPoint` and the AObj's `obj_id`
//!   resolution. Linear archive paths and their in-tree AObj references are
//!   supported; unresolved runtime references report [`JObjEvent::Path`].
//!   Cubic spline families remain unported.
//! - **Rendering**: `HSD_JObjDispAll`, `HSD_JObjMakePositionMtx`,
//!   `HSD_JObjDispSub`, envelope skinning (`envelopemtx` is stored, unused),
//!   billboard flags (stored, unused), the `ptcl` union member.
//! - **Class system**: `hsdJObj` method dispatch (`make_mtx` is always
//!   `HSD_JObjMakeMatrix`), `HSD_JObjSetDefaultClass`, allocation,
//!   `HSD_JObjSetCurrent`/`GetCurrent`.
//! - **User callbacks**: `ufc_callbacks` (no registration API is exported;
//!   Melee registers none), `dptcl_callback` (Melee registers
//!   `efLib_Cb_DPtcl`), `jsound_callback`, `ptcltgt_callback`. Their
//!   invocations are recorded in [`JObjTree::events`] for the caller.

use crate::aobj::{AObj, AObjDesc, AObjEndCallback};
use crate::dobj::DObj;
use crate::mobj::MatAnim;
use crate::mtx::{self, InverseTrig};
use crate::quat::Quaternion;
use gekko_math::msl::fabsf;
use hsd_types::{Mtx, Vec3};

// ---------------------------------------------------------------------------
// jobj.h constants
// ---------------------------------------------------------------------------

/// `HSD_A_J_*` track ids (`jobj.h:19-51`).
pub const HSD_A_J_ROTX: u8 = 1;
pub const HSD_A_J_ROTY: u8 = 2;
pub const HSD_A_J_ROTZ: u8 = 3;
pub const HSD_A_J_PATH: u8 = 4;
pub const HSD_A_J_TRAX: u8 = 5;
pub const HSD_A_J_TRAY: u8 = 6;
pub const HSD_A_J_TRAZ: u8 = 7;
pub const HSD_A_J_SCAX: u8 = 8;
pub const HSD_A_J_SCAY: u8 = 9;
pub const HSD_A_J_SCAZ: u8 = 10;
pub const HSD_A_J_NODE: u8 = 11;
pub const HSD_A_J_BRANCH: u8 = 12;
pub const HSD_A_J_SETBYTE0: u8 = 20;
pub const HSD_A_J_SETBYTE9: u8 = 29;
pub const HSD_A_J_SETFLOAT0: u8 = 30;
pub const HSD_A_J_SETFLOAT9: u8 = 39;
/// Unnamed ids `JObjUpdateFunc` handles (`jobj.c:470-526`).
pub const HSD_A_J_DPTCL: u8 = 0x28;
pub const HSD_A_J_JSOUND: u8 = 0x29;
pub const HSD_A_J_PTCLTGT: u8 = 0x2A;
pub const HSD_A_J_MTX_COL0: u8 = 0x32;
pub const HSD_A_J_MTX_COL1: u8 = 0x33;
pub const HSD_A_J_MTX_COL2: u8 = 0x34;
pub const HSD_A_J_MTX_COL3: u8 = 0x35;
pub const HSD_A_J_MTX_SRT: u8 = 0x36;
pub const HSD_A_J_MTX_ROT: u8 = 0x37;
pub const HSD_A_J_MTX_TRA: u8 = 0x38;
pub const HSD_A_J_MTX_SCA: u8 = 0x39;

/// `TYPE_JOBJ` (`fobj.h:30`): the `obj_type` `JObjSortAnim` hoists to the
/// front of the track list. Numerically `HSD_A_J_BRANCH`.
pub const TYPE_JOBJ: u8 = 12;

/// `JOBJ_*` flags (`jobj.h:53-96`).
pub const JOBJ_BILLBOARD_FIELD: u32 = 0xE00;
pub const JOBJ_BILLBOARD: u32 = 0x200;
pub const JOBJ_VBILLBOARD: u32 = 0x400;
pub const JOBJ_HBILLBOARD: u32 = 0x600;
pub const JOBJ_RBILLBOARD: u32 = 0x800;
pub const JOBJ_PBILLBOARD: u32 = 0x2000;
pub const JOBJ_SKELETON: u32 = 1 << 0;
pub const JOBJ_SKELETON_ROOT: u32 = 1 << 1;
pub const JOBJ_ENVELOPE_MODEL: u32 = 1 << 2;
pub const JOBJ_CLASSICAL_SCALE: u32 = 1 << 3;
pub const JOBJ_HIDDEN: u32 = 1 << 4;
pub const JOBJ_PTCL: u32 = 1 << 5;
pub const JOBJ_MTX_DIRTY: u32 = 1 << 6;
pub const JOBJ_LIGHTING: u32 = 1 << 7;
pub const JOBJ_TEXGEN: u32 = 1 << 8;
pub const JOBJ_INSTANCE: u32 = 1 << 12;
pub const JOBJ_SPLINE: u32 = 1 << 14;
pub const JOBJ_FLIP_IK: u32 = 1 << 15;
pub const JOBJ_SPECULAR: u32 = 1 << 16;
pub const JOBJ_USE_QUATERNION: u32 = 1 << 17;
pub const JOBJ_UNK_B18: u32 = 1 << 18;
pub const JOBJ_UNK_B19: u32 = 1 << 19;
pub const JOBJ_UNK_B20: u32 = 1 << 20;
pub const JOBJ_NULL_OBJ: u32 = 0 << 21;
pub const JOBJ_JOINT1: u32 = 1 << 21;
pub const JOBJ_JOINT2: u32 = 2 << 21;
pub const JOBJ_JOINT: u32 = 3 << 21;
pub const JOBJ_EFFECTOR: u32 = 3 << 21;
pub const JOBJ_USER_DEF_MTX: u32 = 1 << 23;
pub const JOBJ_MTX_INDEP_PARENT: u32 = 1 << 24;
pub const JOBJ_MTX_INDEP_SRT: u32 = 1 << 25;
pub const JOBJ_UNK_B26: u32 = 1 << 26;
pub const JOBJ_UNK_B27: u32 = 1 << 27;
pub const JOBJ_ROOT_OPA: u32 = 1 << 28;
pub const JOBJ_ROOT_XLU: u32 = 1 << 29;
pub const JOBJ_ROOT_TEXEDGE: u32 = 1 << 30;
pub const JOBJ_ROOT_MASK: u32 = JOBJ_ROOT_OPA | JOBJ_ROOT_TEXEDGE | JOBJ_ROOT_XLU;

/// The `flags` argument `HSD_JObjReqAnimAll`/`RemoveAnimAll` pass: every
/// animation kind (`jobj.c:228`, `271`).
pub const JOBJ_ALL_ANIM: u32 = 0x7FF;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Index of a [`JObj`] in its [`JObjTree`] arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JObjId(pub usize);

/// `HSD_JObj` (`jobj.h:98-118`) minus `object` (class header), `u.ptcl`,
/// and `robj` (deferred). Linear splines and their animation references are owned.
#[derive(Debug, Clone, PartialEq)]
pub struct JObj {
    /// `flags`: the `JOBJ_*` bits.
    pub flags: u32,
    /// `next`: next sibling.
    pub next: Option<JObjId>,
    /// `parent`.
    pub parent: Option<JObjId>,
    /// `child`: first child. Under `JOBJ_INSTANCE` the C points this at
    /// another tree's node; the builder leaves it `None`.
    pub child: Option<JObjId>,
    /// `rotate`: a `Quaternion` in the C. Without `JOBJ_USE_QUATERNION`
    /// only `x`, `y`, `z` are read, as Euler angles (`jobj.c:181`).
    pub rotate: Quaternion,
    /// `scale`.
    pub scale: Vec3,
    /// `translate`.
    pub translate: Vec3,
    /// `mtx`: world matrix, valid when not dirty.
    pub mtx: Mtx,
    /// `scl`: accumulated scale, allocated on demand by `HSD_JObjMakeMatrix`.
    pub scl: Option<Vec3>,
    /// `envelopemtx`: the joint's inverse bind matrix; stored, unused here.
    pub envelopemtx: Option<Mtx>,
    /// `aobj`: the joint animation.
    pub aobj: Option<AObj>,
    /// `u.dobj`: the display-object list, head first (see `dobj.rs`). Only
    /// meaningful when neither `JOBJ_PTCL` nor `JOBJ_SPLINE` is set.
    pub dobj: Vec<DObj>,
    pub spline: Option<hsd_archive::desc::spline::LinearSpline>,
    pub path_reference: Option<JObjId>,
    /// `id`: in the C the `HSD_Joint*` the node was loaded from, used as
    /// the id-table key. Free for the caller here.
    pub id: u32,
}

impl JObj {
    /// `JObjInit` (`jobj.c:1466`): zeroed by `hsdNew`, then
    /// `flags = JOBJ_MTX_DIRTY` and unit scale. `rotate.w` stays `0`.
    pub fn init() -> JObj {
        JObj {
            flags: JOBJ_MTX_DIRTY,
            next: None,
            parent: None,
            child: None,
            rotate: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            scale: Vec3::new(1.0, 1.0, 1.0),
            translate: Vec3::ZERO,
            mtx: Mtx::ZERO,
            scl: None,
            envelopemtx: None,
            aobj: None,
            dobj: Vec::new(),
            spline: None,
            path_reference: None,
            id: 0,
        }
    }

    /// `union_type_ptcl` (`jobj.h:92`).
    pub fn union_type_ptcl(&self) -> bool {
        self.flags & JOBJ_PTCL != 0
    }

    /// `union_type_spline` (`jobj.h:93`).
    pub fn union_type_spline(&self) -> bool {
        self.flags & JOBJ_SPLINE != 0
    }

    /// `union_type_dobj` (`jobj.h:94`).
    pub fn union_type_dobj(&self) -> bool {
        self.flags & (JOBJ_PTCL | JOBJ_SPLINE) == 0
    }

    /// `HSD_JObjMtxIsDirty` (`jobj.h:558`): dirty and not user-defined.
    pub fn mtx_is_dirty(&self) -> bool {
        self.flags & JOBJ_USER_DEF_MTX == 0 && self.flags & JOBJ_MTX_DIRTY != 0
    }
}

impl Default for JObj {
    fn default() -> JObj {
        JObj::init()
    }
}

/// `HSD_ObjData` (`fobj.h:64`): the union `JObjUpdateFunc` receives.
/// `FObj` tracks always deliver `Float`; `HSD_RObjUpdateAll` delivers
/// `Vec` for the column ids and `Null` for the re-decomposition ids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjData {
    Float(f32),
    Vec(Vec3),
    Null,
}

impl ObjData {
    /// `val->fv`. For `Vec` that is the first word, `p.x`.
    pub fn fv(self) -> f32 {
        match self {
            ObjData::Float(v) => v,
            ObjData::Vec(p) => p.x,
            ObjData::Null => 0.0,
        }
    }

    /// `val->iv`: the same word read as `s32`.
    pub fn iv(self) -> i32 {
        self.fv().to_bits() as i32
    }
}

/// A callback invocation `JObjUpdateFunc` would have made
/// (`jobj.c:434-487`), plus the deferred PATH track. Drained by the caller
/// from [`JObjTree::events`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JObjEvent {
    /// `HSD_A_J_SETBYTE0..9`: `cb(jobj, type, val->iv)` for each `ufc_callbacks` entry.
    SetByte { jobj: JObjId, ty: u8, value: i32 },
    /// `HSD_A_J_SETFLOAT0..9`: `cb(jobj, type, val->fv)`.
    SetFloat { jobj: JObjId, ty: u8, value: f32 },
    /// Type `0x28`: `dptcl_callback(0, lo, hi, jobj)`.
    DPtcl { jobj: JObjId, lo: i32, hi: i32 },
    /// Type `0x29`: `jsound_callback(val->iv)`.
    JSound(i32),
    /// Type `0x2A`: `ptcltgt_callback(jobj, val->iv)`.
    PtclTgt { jobj: JObjId, value: i32 },
    /// `HSD_A_J_PATH` after its `[0, 1]` clamp; the spline evaluation that
    /// would set the translation is deferred.
    Path { jobj: JObjId, t: f32 },
}

/// `HSD_AnimJoint` (`aobj.h:57`) as an owned tree: `children` is the
/// `child`/`next` chain in order. `robj_anim` is deferred with the RObj.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimJoint {
    pub aobjdesc: Option<AObjDesc>,
    /// Bit 0 selects `JOBJ_CLASSICAL_SCALE` (`jobj.c:309`).
    pub flags: u32,
    pub children: Vec<AnimJoint>,
}

/// `HSD_MatAnimJoint` (`mobj.h:136`) as an owned tree; `matanim` is the
/// per-DObj list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MatAnimJoint {
    pub matanim: Vec<MatAnim>,
    pub children: Vec<MatAnimJoint>,
}

/// `HSD_Joint` (`jobj.h:120-134`) as an owned tree for building skeletons
/// in tests and, later, from `hsd-archive` descs. `children` is the
/// `child`/`next` chain in order. `robjdesc` and `class_name` are omitted.
#[derive(Debug, Clone, PartialEq)]
pub struct JointSpec {
    pub flags: u32,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub position: Vec3,
    /// `mtx`: the inverse bind matrix for envelope models.
    pub mtx: Option<Mtx>,
    /// `u.dobjdesc`, already loaded.
    pub dobj: Vec<DObj>,
    /// Value for [`JObj::id`].
    pub id: u32,
    pub children: Vec<JointSpec>,
}

impl Default for JointSpec {
    fn default() -> JointSpec {
        JointSpec::new()
    }
}

impl JointSpec {
    /// Identity joint: no flags, zero rotation and position, unit scale.
    pub fn new() -> JointSpec {
        JointSpec {
            flags: 0,
            rotation: Vec3::ZERO,
            scale: Vec3::new(1.0, 1.0, 1.0),
            position: Vec3::ZERO,
            mtx: None,
            dobj: Vec::new(),
            id: 0,
            children: Vec::new(),
        }
    }

    pub fn flags(mut self, flags: u32) -> JointSpec {
        self.flags = flags;
        self
    }

    pub fn rotation(mut self, x: f32, y: f32, z: f32) -> JointSpec {
        self.rotation = Vec3::new(x, y, z);
        self
    }

    pub fn scale(mut self, x: f32, y: f32, z: f32) -> JointSpec {
        self.scale = Vec3::new(x, y, z);
        self
    }

    pub fn position(mut self, x: f32, y: f32, z: f32) -> JointSpec {
        self.position = Vec3::new(x, y, z);
        self
    }

    pub fn envelope_mtx(mut self, mtx: Mtx) -> JointSpec {
        self.mtx = Some(mtx);
        self
    }

    pub fn dobj(mut self, dobj: Vec<DObj>) -> JointSpec {
        self.dobj = dobj;
        self
    }

    pub fn id(mut self, id: u32) -> JointSpec {
        self.id = id;
        self
    }

    /// Append a child (becomes the last `next` sibling).
    pub fn child(mut self, child: JointSpec) -> JointSpec {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<JointSpec>) -> JointSpec {
        self.children = children;
        self
    }
}

// ---------------------------------------------------------------------------
// The arena
// ---------------------------------------------------------------------------

/// A skeleton: an arena of [`JObj`] nodes and the callback events its
/// animation produced.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JObjTree {
    /// Resolved external positions for REFTYPE_JOBJ subtype 1 constraints.
    position_constraints: std::collections::BTreeMap<JObjId, Option<Vec3>>,
    nodes: Vec<JObj>,
    /// Inactive joint track buffers, reserved once by fixed-skeleton owners.
    spare_tracks: Vec<Vec<crate::fobj::FObj>>,
    /// Callback invocations recorded by [`JObjTree::update_func`], oldest
    /// first. The caller drains them.
    pub events: Vec<JObjEvent>,
}

/// Depth-first walk in the order Melee's `ftParts` bone-indexing loops use
/// (`ftparts.c:405-441`, `:470-497`, `:964-987`). See
/// [`JObjTree::depth_first`].
pub struct DepthFirst<'a> {
    tree: &'a JObjTree,
    cur: Option<JObjId>,
}

impl<'a> Iterator for DepthFirst<'a> {
    type Item = JObjId;

    fn next(&mut self) -> Option<JObjId> {
        let id = self.cur?;
        self.cur = self.tree.next_depth_first(id);
        Some(id)
    }
}

impl JObjTree {
    /// Successor in `depth_first` order, without retaining a tree borrow.
    /// Callers may update a node between steps when its links stay unchanged.
    pub fn next_depth_first(&self, id: JObjId) -> Option<JObjId> {
        let node = &self.nodes[id.0];
        if node.flags & JOBJ_INSTANCE == 0 && node.child.is_some() {
            // Descend the left side of the tree.
            node.child
        } else if node.next.is_some() {
            // Visit bottom nodes from left to right.
            node.next
        } else {
            // Go back up the tree until we can continue to the right.
            let mut j = id;
            loop {
                match self.nodes[j.0].parent {
                    None => break None,
                    Some(p) => {
                        if let Some(pn) = self.nodes[p.0].next {
                            break Some(pn);
                        }
                        j = p;
                    }
                }
            }
        }
    }
}

impl JObjTree {
    pub fn new() -> JObjTree {
        JObjTree::default()
    }

    /// Prepare an inactive constraint slot before animation updates begin.
    pub fn reserve_position_constraint(&mut self, id: JObjId) {
        self.position_constraints.entry(id).or_insert(None);
    }

    /// lb_8000C1C0 / HSD_RObjUpdateAll: the scene resolves external joint ownership.
    pub fn set_position_constraint(&mut self, id: JObjId, position: Option<Vec3>) {
        if let Some(slot) = self.position_constraints.get_mut(&id) {
            *slot = position;
        } else if position.is_some() {
            self.position_constraints.insert(id, position);
        }
        self.set_mtx_dirty_sub(id);
    }

    /// Number of nodes allocated.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// `HSD_JObjAlloc` (`jobj.c:1045`) -> `hsdNew` -> `JObjInit`: a fresh,
    /// dirty, unit-scale, unlinked joint.
    pub fn alloc(&mut self) -> JObjId {
        self.nodes.push(JObj::init());
        JObjId(self.nodes.len() - 1)
    }

    pub fn get(&self, id: JObjId) -> &JObj {
        &self.nodes[id.0]
    }

    pub fn get_mut(&mut self, id: JObjId) -> &mut JObj {
        &mut self.nodes[id.0]
    }

    /// All ids in allocation order.
    pub fn ids(&self) -> impl Iterator<Item = JObjId> {
        (0..self.nodes.len()).map(JObjId)
    }

    /// `HSD_JObjGetChild` (`jobj.h:172`).
    pub fn child(&self, id: JObjId) -> Option<JObjId> {
        self.nodes[id.0].child
    }

    /// `HSD_JObjGetNext` (`jobj.h:181`).
    pub fn next(&self, id: JObjId) -> Option<JObjId> {
        self.nodes[id.0].next
    }

    /// `HSD_JObjGetParent` (`jobj.h:190`).
    pub fn parent(&self, id: JObjId) -> Option<JObjId> {
        self.nodes[id.0].parent
    }

    /// The `child` / `next` chain of `id`, in order.
    pub fn children(&self, id: JObjId) -> impl Iterator<Item = JObjId> + '_ {
        let mut cur = self.nodes[id.0].child;
        std::iter::from_fn(move || {
            let c = cur?;
            cur = self.nodes[c.0].next;
            Some(c)
        })
    }

    /// The `ftParts` traversal from `root`: a node, then (unless it is a
    /// `JOBJ_INSTANCE`) its first child, else its next sibling, else the
    /// next sibling of the nearest ancestor that has one. Note that the C
    /// climbs through `root`'s own ancestors too, so on a subtree root with
    /// a parent the walk continues into the parent's later siblings.
    pub fn depth_first(&self, root: JObjId) -> DepthFirst<'_> {
        DepthFirst {
            tree: self,
            cur: Some(root),
        }
    }

    /// The `index`-th node of [`JObjTree::depth_first`], the way
    /// `fp->parts[index].joint` is filled (`ftparts.c:405-441`).
    pub fn bone(&self, root: JObjId, index: usize) -> Option<JObjId> {
        self.depth_first(root).nth(index)
    }

    // -- loading ------------------------------------------------------------

    /// `HSD_JObjLoadJoint` (`jobj.c:667`): `JObjLoadJointSub(joint, NULL)`
    /// then `HSD_JObjResolveRefsAll` (which only resolves `JOBJ_INSTANCE`
    /// children and RObj/DObj references, all deferred).
    ///
    /// Node allocation order follows the C: a joint is allocated, then its
    /// child chain (recursively), then its next chain, so ids are in
    /// pre-order for a freshly loaded tree.
    pub fn load_joint(&mut self, joint: &JointSpec) -> JObjId {
        self.load_joint_chain(std::slice::from_ref(joint), None)
            .expect("non-empty chain")
    }

    /// `JObjLoadJointSub` + `JObjLoad` (`jobj.c:610-665`) over a `next`
    /// chain given as a slice. Returns the id of the first joint.
    fn load_joint_chain(&mut self, joints: &[JointSpec], parent: Option<JObjId>) -> Option<JObjId> {
        let (joint, rest) = joints.split_first()?;
        let id = self.alloc();
        // JObjLoad (jobj.c:629)
        if joint.flags & JOBJ_INSTANCE == 0 {
            let child = self.load_joint_chain(&joint.children, Some(id));
            self.nodes[id.0].child = child;
        }
        let next = self.load_joint_chain(rest, parent);
        let node = &mut self.nodes[id.0];
        node.next = next;
        node.parent = parent;
        node.flags |= joint.flags;
        if node.union_type_dobj() {
            // HSD_DObjLoadDesc(joint->u.dobjdesc)
            node.dobj = joint.dobj.clone();
        }
        // spline / ptcl union members and HSD_RObjLoadDesc: deferred.
        node.rotate.x = joint.rotation.x;
        node.rotate.y = joint.rotation.y;
        node.rotate.z = joint.rotation.z;
        node.scale = joint.scale;
        node.translate = joint.position;
        node.mtx = Mtx::IDENTITY;
        node.scl = None;
        node.envelopemtx = joint.mtx;
        node.id = joint.id;
        Some(id)
    }

    /// `JObjResetRST` (`jobj.c:57`): reload SRT from the joint desc and
    /// mark dirty.
    pub fn reset_rst_one(&mut self, id: JObjId, joint: &JointSpec) {
        let node = &mut self.nodes[id.0];
        node.rotate.x = joint.rotation.x;
        node.rotate.y = joint.rotation.y;
        node.rotate.z = joint.rotation.z;
        node.scale = joint.scale;
        node.translate = joint.position;
        if node.flags & JOBJ_MTX_INDEP_SRT == 0 {
            self.set_mtx_dirty(id);
        }
    }

    /// `HSD_JObjResetRST` (`jobj.c:72`): [`JObjTree::reset_rst_one`] down
    /// the tree, pairing children with the desc's children positionally;
    /// once the desc chain runs out the remaining children are untouched.
    pub fn reset_rst(&mut self, id: JObjId, joint: &JointSpec) {
        self.reset_rst_one(id, joint);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut child_jobj = self.nodes[id.0].child;
            let mut child_joint = joint.children.iter();
            while let Some(c) = child_jobj {
                if let Some(j) = child_joint.next() {
                    self.reset_rst(c, j);
                }
                child_jobj = self.nodes[c.0].next;
            }
        }
    }

    // -- tree walking -------------------------------------------------------

    /// `HSD_JObjWalkTree` (`jobj.c:111`): `cb(id, kind)` with kind `0` for
    /// the root, `1` for a first child, `2` for a later sibling
    /// (`HSD_JObjWalkTree0`, `jobj.c:89`). Stops at `JOBJ_INSTANCE` nodes.
    pub fn walk_tree(&self, id: JObjId, cb: &mut impl FnMut(JObjId, u32)) {
        cb(id, 0);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            for c in self.children(id) {
                self.walk_tree0(c, cb);
            }
        }
    }

    fn walk_tree0(&self, id: JObjId, cb: &mut impl FnMut(JObjId, u32)) {
        let parent = self.nodes[id.0]
            .parent
            .expect("HSD_ASSERT(0xAE, jobj->parent)");
        let kind = if self.nodes[parent.0].child == Some(id) {
            1
        } else {
            2
        };
        cb(id, kind);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            for c in self.children(id) {
                self.walk_tree0(c, cb);
            }
        }
    }

    // -- links --------------------------------------------------------------

    /// `HSD_JObjGetPrev` (`jobj.c:909`): the previous sibling, or `None`
    /// for a root or a first child. Panics if the tree is inconsistent, as
    /// the C's `HSD_Panic` does.
    pub fn get_prev(&self, id: JObjId) -> Option<JObjId> {
        let parent = self.nodes[id.0].parent?;
        if self.nodes[parent.0].child == Some(id) {
            return None;
        }
        let mut cur = self.nodes[parent.0].child;
        while let Some(c) = cur {
            if self.nodes[c.0].next == Some(id) {
                return Some(c);
            }
            cur = self.nodes[c.0].next;
        }
        panic!("can not find specified jobj. maybe jobj tree is broken.");
    }

    /// `RecalcParentTrspBits` (`jobj.c:799`): recompute each node's
    /// `JOBJ_ROOT_*` bits from its children, walking `next` (literally: the
    /// C advances along `jobj->next`, not `parent`) until a node loses no
    /// bits.
    pub fn recalc_parent_trsp_bits(&mut self, jobj: Option<JObjId>) {
        let mut cur = jobj;
        while let Some(j) = cur {
            let mut flags = !JOBJ_ROOT_MASK;
            for c in self.children(j) {
                let cf = self.nodes[c.0].flags;
                flags |= (cf | (cf << 10)) & JOBJ_ROOT_MASK;
            }
            if self.nodes[j.0].flags & !flags == 0 {
                break;
            }
            self.nodes[j.0].flags &= flags;
            cur = self.nodes[j.0].next;
        }
    }

    /// `UpdateParentTrspBits` (`jobj.c:816`): OR the child's `ROOT_*` bits
    /// (its own and its bits 18..20 shifted up) into every ancestor until
    /// one already has them.
    fn update_parent_trsp_bits(&mut self, jobj: Option<JObjId>, child: JObjId) {
        let cf = self.nodes[child.0].flags;
        let flags = (cf | (cf << 10)) & JOBJ_ROOT_MASK;
        let mut cur = jobj;
        while let Some(j) = cur {
            if flags & !self.nodes[j.0].flags == 0 {
                break;
            }
            self.nodes[j.0].flags |= flags;
            cur = self.nodes[j.0].parent;
        }
    }

    /// `HSD_JObjAddChild` (`jobj.c:828`): append `child` (which must be an
    /// orphan without siblings) to `jobj`'s child chain.
    pub fn add_child(&mut self, jobj: JObjId, child: JObjId) {
        assert!(
            self.nodes[child.0].parent.is_none(),
            "child should be a orphan."
        );
        assert!(
            self.nodes[child.0].next.is_none(),
            "child should not have siblings"
        );
        match self.nodes[jobj.0].child {
            None => self.nodes[jobj.0].child = Some(child),
            Some(first) => {
                assert!(self.nodes[jobj.0].flags & JOBJ_INSTANCE == 0);
                let mut last = first;
                while let Some(n) = self.nodes[last.0].next {
                    assert!(last != child);
                    last = n;
                }
                self.nodes[last.0].next = Some(child);
            }
        }
        self.nodes[child.0].parent = Some(jobj);
        self.update_parent_trsp_bits(Some(jobj), child);
    }

    /// `HSD_JObjReparent` (`jobj.c:854`): unlink `jobj` from its parent and
    /// siblings, then `add_child(parent, jobj)` if `parent` is `Some`.
    /// Returns the sibling that followed `jobj`.
    pub fn reparent(&mut self, jobj: JObjId, parent: Option<JObjId>) -> Option<JObjId> {
        let next = self.nodes[jobj.0].next;
        if let Some(p) = self.nodes[jobj.0].parent {
            if self.nodes[p.0].child == Some(jobj) {
                self.nodes[p.0].child = next;
            } else {
                let prev = self.get_prev(jobj).expect("HSD_ASSERT(0x56F, prev)");
                self.nodes[prev.0].next = next;
            }
            self.recalc_parent_trsp_bits(Some(p));
            self.nodes[jobj.0].parent = None;
        }
        self.nodes[jobj.0].next = None;
        if let Some(p) = parent {
            self.add_child(p, jobj);
        }
        next
    }

    /// `HSD_JObjAddNext` (`jobj.c:878`): insert `next` between `jobj`'s
    /// parent and `jobj`'s whole sibling chain, which becomes `next`'s
    /// children (appended after any it already has).
    pub fn add_next(&mut self, jobj: JObjId, next: JObjId) {
        let parent = self.nodes[jobj.0].parent;
        let cur = match parent {
            Some(p) => {
                let c = self.nodes[p.0].child;
                self.nodes[p.0].child = None;
                self.nodes[jobj.0].flags &= !JOBJ_ROOT_MASK;
                c
            }
            None => Some(jobj),
        };
        self.reparent(next, parent);
        match self.nodes[next.0].child {
            Some(mut c) => {
                while let Some(n) = self.nodes[c.0].next {
                    c = n;
                }
                self.nodes[c.0].next = cur;
            }
            None => self.nodes[next.0].child = cur,
        }
        let mut cur = cur;
        while let Some(c) = cur {
            self.nodes[c.0].parent = Some(next);
            self.update_parent_trsp_bits(Some(next), c);
            cur = self.nodes[c.0].next;
        }
    }

    /// `HSD_JObjGetDObj` (`jobj.c:931`): the DObj list, or `None` for a
    /// ptcl / spline joint.
    pub fn dobj(&self, id: JObjId) -> Option<&[DObj]> {
        let node = &self.nodes[id.0];
        if node.union_type_dobj() {
            Some(&node.dobj)
        } else {
            None
        }
    }

    /// `HSD_JObjAddDObj` (`jobj.c:939`): prepend to the list; ignored for
    /// ptcl / spline joints.
    pub fn add_dobj(&mut self, id: JObjId, dobj: DObj) {
        let node = &mut self.nodes[id.0];
        if node.union_type_dobj() {
            node.dobj.insert(0, dobj);
        }
    }

    // -- flags --------------------------------------------------------------

    /// `HSD_JObjGetFlags` (`jobj.c:985`).
    pub fn flags(&self, id: JObjId) -> u32 {
        self.nodes[id.0].flags
    }

    /// `HSD_JObjSetFlags` (`jobj.c:993`): marks dirty first if
    /// `JOBJ_CLASSICAL_SCALE` would change.
    pub fn set_flags(&mut self, id: JObjId, flags: u32) {
        if (self.nodes[id.0].flags ^ flags) & JOBJ_CLASSICAL_SCALE != 0 {
            self.set_mtx_dirty(id);
        }
        self.nodes[id.0].flags |= flags;
    }

    /// `HSD_JObjSetFlagsAll` (`jobj.c:1006`): down the tree, stopping at
    /// `JOBJ_INSTANCE` nodes.
    pub fn set_flags_all(&mut self, id: JObjId, flags: u32) {
        self.set_flags(id, flags);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut i = self.nodes[id.0].child;
            while let Some(c) = i {
                self.set_flags_all(c, flags);
                i = self.nodes[c.0].next;
            }
        }
    }

    /// `HSD_JObjClearFlags` (`jobj.c:1019`). Uses the same
    /// `(jobj->flags ^ flags) & JOBJ_CLASSICAL_SCALE` test as `SetFlags`,
    /// so clearing a *set* `JOBJ_CLASSICAL_SCALE` does not mark dirty while
    /// clearing an already-clear one does. Kept literally.
    pub fn clear_flags(&mut self, id: JObjId, flags: u32) {
        if (self.nodes[id.0].flags ^ flags) & JOBJ_CLASSICAL_SCALE != 0 {
            self.set_mtx_dirty(id);
        }
        self.nodes[id.0].flags &= !flags;
    }

    /// `HSD_JObjClearFlagsAll` (`jobj.c:1032`).
    pub fn clear_flags_all(&mut self, id: JObjId, flags: u32) {
        self.clear_flags(id, flags);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut i = self.nodes[id.0].child;
            while let Some(c) = i {
                self.clear_flags_all(c, flags);
                i = self.nodes[c.0].next;
            }
        }
    }

    // -- dirty tracking -----------------------------------------------------

    /// `HSD_JObjMtxIsDirty` (`jobj.h:558`).
    pub fn mtx_is_dirty(&self, id: JObjId) -> bool {
        self.nodes[id.0].mtx_is_dirty()
    }

    /// The `HSD_JObjSetMtxDirty` macro (`jobj.h:583`): run the sub only
    /// when the node is not already dirty.
    pub fn set_mtx_dirty(&mut self, id: JObjId) {
        if !self.mtx_is_dirty(id) {
            self.set_mtx_dirty_sub(id);
        }
    }

    /// `HSD_JObjSetMtxDirtySub` (`jobj.c:1445`): set bit 6 and recurse into
    /// children without `JOBJ_MTX_INDEP_PARENT` that are not already dirty.
    pub fn set_mtx_dirty_sub(&mut self, id: JObjId) {
        self.nodes[id.0].flags |= 0x40;
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut child = self.nodes[id.0].child;
            while let Some(c) = child {
                if self.nodes[c.0].flags & JOBJ_MTX_INDEP_PARENT == 0 && !self.mtx_is_dirty(c) {
                    self.set_mtx_dirty_sub(c);
                }
                child = self.nodes[c.0].next;
            }
        }
    }

    /// `HSD_JObjCheckDepend` (`jobj.c:30`): before a node animates, mark it
    /// dirty if its parent is dirty (user-defined matrices only when they
    /// are not `JOBJ_MTX_INDEP_PARENT`), or if it is an IK joint. The
    /// `robj != NULL` term is deferred with the RObj.
    pub fn check_depend(&mut self, id: JObjId) {
        if self.mtx_is_dirty(id) {
            return;
        }
        let flags = self.nodes[id.0].flags;
        let parent = self.nodes[id.0].parent;
        if flags & JOBJ_USER_DEF_MTX != 0 {
            if flags & JOBJ_MTX_INDEP_PARENT == 0 && parent.is_some_and(|p| self.mtx_is_dirty(p)) {
                self.nodes[id.0].flags |= JOBJ_MTX_DIRTY;
            }
        } else if parent.is_some_and(|p| self.nodes[p.0].flags & JOBJ_MTX_DIRTY != 0)
            || (flags & JOBJ_EFFECTOR) == JOBJ_JOINT1
            || (flags & JOBJ_EFFECTOR) == JOBJ_JOINT2
            || (flags & JOBJ_EFFECTOR) == JOBJ_EFFECTOR
        {
            self.nodes[id.0].flags |= JOBJ_MTX_DIRTY;
        }
    }

    // -- matrices -----------------------------------------------------------

    /// `HSD_JObjMakeMatrix` (`jobj.c:138`): the composition pinned in the
    /// module docs, including the resolved spline reference's translation
    /// override (`jobj.c:187-195`).
    pub fn make_matrix(&mut self, id: JObjId) {
        let parent = self.nodes[id.0].parent;
        self.setup_matrix_opt(parent);
        let parent_scl = parent.and_then(|p| self.nodes[p.0].scl);

        let node = &mut self.nodes[id.0];
        if node.flags & JOBJ_CLASSICAL_SCALE != 0 {
            // Copy the parent's accumulated scale, or drop ours.
            node.scl = parent_scl;
        } else {
            node.scl = Some(match parent_scl {
                Some(ps) => Vec3::new(
                    node.scale.x * ps.x,
                    node.scale.y * ps.y,
                    node.scale.z * ps.z,
                ),
                None => node.scale,
            });
        }

        // has_scl(jobj->parent) ? jobj->parent->scl : NULL
        let scl = parent_scl.as_ref();
        if node.flags & JOBJ_USE_QUATERNION != 0 {
            mtx::hsd_mtx_srt_quat(
                &mut node.mtx,
                &node.scale,
                &node.rotate,
                &node.translate,
                scl,
            );
        } else {
            let euler = Vec3::new(node.rotate.x, node.rotate.y, node.rotate.z);
            mtx::hsd_mtx_srt(&mut node.mtx, &node.scale, &euler, &node.translate, scl);
        }

        if let Some(p) = parent {
            // PSMTXConcat(jobj->parent->mtx, jobj->mtx, jobj->mtx)
            let pm = self.nodes[p.0].mtx;
            let m = self.nodes[id.0].mtx;
            mtx::mtx_concat(&pm, &m, &mut self.nodes[id.0].mtx);
        }
        if let Some(reference) = self.nodes[id.0]
            .aobj
            .as_ref()
            .and(self.nodes[id.0].path_reference)
        {
            self.setup_matrix(reference);
            let mut position = Vec3::ZERO;
            mtx::mtx_mult_vec(
                &self.nodes[reference.0].mtx,
                &self.nodes[id.0].translate,
                &mut position,
            );
            self.nodes[id.0].mtx.0[0][3] = position.x;
            self.nodes[id.0].mtx.0[1][3] = position.y;
            self.nodes[id.0].mtx.0[2][3] = position.z;
        }
    }

    /// `HSD_JObjSetupMatrixSub` (`jobj.c:1385`): `make_mtx`, clear dirty,
    /// then unless `JOBJ_USER_DEF_MTX` the IK / RObj post-pass (deferred; a
    /// joint without an RObj does nothing there) and clear dirty again.
    pub fn setup_matrix_sub(&mut self, id: JObjId) {
        self.make_matrix(id);
        self.nodes[id.0].flags &= !JOBJ_MTX_DIRTY;
        if self.nodes[id.0].flags & JOBJ_USER_DEF_MTX == 0 {
            match self.nodes[id.0].flags & JOBJ_JOINT {
                JOBJ_JOINT1 => {
                    // resolveIKJoint1 (jobj.c:1104): needs REFTYPE_IKHINT RObj. Deferred.
                }
                JOBJ_JOINT2 => {
                    // resolveIKJoint2 (jobj.c:1257): needs REFTYPE_IKHINT RObj. Deferred.
                }
                JOBJ_EFFECTOR => {
                    // jobj.c:1404-1430: only acts when the parent has a
                    // REFTYPE_IKHINT RObj. Deferred; no-op without one.
                }
                _ => {
                    // jobj.c:1432-1438: HSD_RObjUpdateAll when robj != NULL. Deferred.
                }
            }
            if let Some(position) = self.position_constraints.get(&id).copied().flatten() {
                // HSD_RObjGetGlobalPosition: accumulate from +0 then divide by
                // the single target count. No fused instructions in retail.
                let world = Vec3::new(0.0 + position.x, 0.0 + position.y, 0.0 + position.z);
                self.nodes[id.0].mtx.0[0][3] = world.x;
                self.nodes[id.0].mtx.0[1][3] = world.y;
                self.nodes[id.0].mtx.0[2][3] = world.z;
                // JObjUpdateFunc types 0x35/0x38: retain the constrained world
                // matrix and recover local translation with the audited kernel.
                let mut local = self.nodes[id.0].mtx;
                if let Some(parent) = self.nodes[id.0].parent {
                    mtx::hsd_mtx_inverse_concat(
                        &self.nodes[parent.0].mtx,
                        &self.nodes[id.0].mtx,
                        &mut local,
                    );
                }
                self.nodes[id.0].translate = Vec3::new(local.0[0][3], local.0[1][3], local.0[2][3]);
            }
            self.nodes[id.0].flags &= !JOBJ_MTX_DIRTY;
        }
    }

    /// `HSD_JObjSetupMatrix` (`jobj.h:571`): rebuild `mtx` only when dirty.
    pub fn setup_matrix(&mut self, id: JObjId) {
        if self.mtx_is_dirty(id) {
            self.setup_matrix_sub(id);
        }
    }

    /// `HSD_JObjSetupMatrix` with the C's `NULL` acceptance.
    fn setup_matrix_opt(&mut self, id: Option<JObjId>) {
        if let Some(id) = id {
            self.setup_matrix(id);
        }
    }

    /// `HSD_JObjGetMtxPtr` (`jobj.h:1141`): set up the matrix and return
    /// it. This is what `lbColl` reads for hurtboxes and `ftAnim` inverts.
    pub fn get_mtx(&mut self, id: JObjId) -> &Mtx {
        self.setup_matrix(id);
        &self.nodes[id.0].mtx
    }

    /// `HSD_JObjCopyMtx` (`jobj.h:1168`): overwrite `mtx` without touching
    /// the flags.
    pub fn copy_mtx(&mut self, id: JObjId, m: &Mtx) {
        mtx::mtx_copy(m, &mut self.nodes[id.0].mtx);
    }

    // -- SRT accessors (jobj.h:615-1130) ------------------------------------

    /// The tail every SRT setter shares: `if (!(flags & JOBJ_MTX_INDEP_SRT))
    /// HSD_JObjSetMtxDirty(jobj);`.
    fn srt_changed(&mut self, id: JObjId) {
        if self.nodes[id.0].flags & JOBJ_MTX_INDEP_SRT == 0 {
            self.set_mtx_dirty(id);
        }
    }

    /// `HSD_JObjSetRotation` (`jobj.h:615`).
    pub fn set_rotation(&mut self, id: JObjId, rotate: &Quaternion) {
        self.nodes[id.0].rotate = *rotate;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetRotationX` (`jobj.h:637`). Asserts `!JOBJ_USE_QUATERNION`.
    pub fn set_rotation_x(&mut self, id: JObjId, x: f32) {
        debug_assert!(self.nodes[id.0].flags & JOBJ_USE_QUATERNION == 0);
        self.nodes[id.0].rotate.x = x;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetRotationY` (`jobj.h:658`).
    pub fn set_rotation_y(&mut self, id: JObjId, y: f32) {
        debug_assert!(self.nodes[id.0].flags & JOBJ_USE_QUATERNION == 0);
        self.nodes[id.0].rotate.y = y;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetRotationZ` (`jobj.h:679`).
    pub fn set_rotation_z(&mut self, id: JObjId, z: f32) {
        debug_assert!(self.nodes[id.0].flags & JOBJ_USE_QUATERNION == 0);
        self.nodes[id.0].rotate.z = z;
        self.srt_changed(id);
    }

    /// `HSD_JObjGetRotation` (`jobj.h:697`).
    pub fn rotation(&self, id: JObjId) -> Quaternion {
        self.nodes[id.0].rotate
    }

    /// `HSD_JObjGetRotationX` (`jobj.h:713`).
    pub fn rotation_x(&self, id: JObjId) -> f32 {
        self.nodes[id.0].rotate.x
    }

    /// `HSD_JObjGetRotationY` (`jobj.h:728`).
    pub fn rotation_y(&self, id: JObjId) -> f32 {
        self.nodes[id.0].rotate.y
    }

    /// `HSD_JObjGetRotationZ` (`jobj.h:743`).
    pub fn rotation_z(&self, id: JObjId) -> f32 {
        self.nodes[id.0].rotate.z
    }

    /// `HSD_JObjSetScale` (`jobj.h:758`).
    pub fn set_scale(&mut self, id: JObjId, scale: &Vec3) {
        self.nodes[id.0].scale = *scale;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetScaleX` (`jobj.h:774`).
    pub fn set_scale_x(&mut self, id: JObjId, x: f32) {
        self.nodes[id.0].scale.x = x;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetScaleY` (`jobj.h:789`).
    pub fn set_scale_y(&mut self, id: JObjId, y: f32) {
        self.nodes[id.0].scale.y = y;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetScaleZ` (`jobj.h:804`).
    pub fn set_scale_z(&mut self, id: JObjId, z: f32) {
        self.nodes[id.0].scale.z = z;
        self.srt_changed(id);
    }

    /// `HSD_JObjGetScale` (`jobj.h:821`).
    pub fn scale(&self, id: JObjId) -> Vec3 {
        self.nodes[id.0].scale
    }

    /// `HSD_JObjGetScaleX` (`jobj.h:873`).
    pub fn scale_x(&self, id: JObjId) -> f32 {
        self.nodes[id.0].scale.x
    }

    /// `HSD_JObjGetScaleY` (`jobj.h:886`).
    pub fn scale_y(&self, id: JObjId) -> f32 {
        self.nodes[id.0].scale.y
    }

    /// `HSD_JObjGetScaleZ` (`jobj.h:899`).
    pub fn scale_z(&self, id: JObjId) -> f32 {
        self.nodes[id.0].scale.z
    }

    /// `HSD_JObjSetTranslate` (`jobj.h:914`).
    pub fn set_translate(&mut self, id: JObjId, translate: &Vec3) {
        self.nodes[id.0].translate = *translate;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetTranslateX` (`jobj.h:930`).
    pub fn set_translate_x(&mut self, id: JObjId, x: f32) {
        self.nodes[id.0].translate.x = x;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetTranslateY` (`jobj.h:945`).
    pub fn set_translate_y(&mut self, id: JObjId, y: f32) {
        self.nodes[id.0].translate.y = y;
        self.srt_changed(id);
    }

    /// `HSD_JObjSetTranslateZ` (`jobj.h:960`).
    pub fn set_translate_z(&mut self, id: JObjId, z: f32) {
        self.nodes[id.0].translate.z = z;
        self.srt_changed(id);
    }

    /// `HSD_JObjGetTranslation` (`jobj.h:977`).
    pub fn translation(&self, id: JObjId) -> Vec3 {
        self.nodes[id.0].translate
    }

    /// `HSD_JObjGetTranslationX` (`jobj.h:991`).
    pub fn translation_x(&self, id: JObjId) -> f32 {
        self.nodes[id.0].translate.x
    }

    /// `HSD_JObjGetTranslationY` (`jobj.h:1004`).
    pub fn translation_y(&self, id: JObjId) -> f32 {
        self.nodes[id.0].translate.y
    }

    /// `HSD_JObjGetTranslationZ` (`jobj.h:1017`).
    pub fn translation_z(&self, id: JObjId) -> f32 {
        self.nodes[id.0].translate.z
    }

    /// `HSD_JObjAddRotationX` (`jobj.h:1027`).
    pub fn add_rotation_x(&mut self, id: JObjId, x: f32) {
        self.nodes[id.0].rotate.x += x;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddRotationY` (`jobj.h:1039`).
    pub fn add_rotation_y(&mut self, id: JObjId, y: f32) {
        self.nodes[id.0].rotate.y += y;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddRotationZ` (`jobj.h:1051`).
    pub fn add_rotation_z(&mut self, id: JObjId, z: f32) {
        self.nodes[id.0].rotate.z += z;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddScaleX` (`jobj.h:1063`).
    pub fn add_scale_x(&mut self, id: JObjId, x: f32) {
        self.nodes[id.0].scale.x += x;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddScaleY` (`jobj.h:1075`).
    pub fn add_scale_y(&mut self, id: JObjId, y: f32) {
        self.nodes[id.0].scale.y += y;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddScaleZ` (`jobj.h:1087`).
    pub fn add_scale_z(&mut self, id: JObjId, z: f32) {
        self.nodes[id.0].scale.z += z;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddTranslationX` (`jobj.h:1100`).
    pub fn add_translation_x(&mut self, id: JObjId, x: f32) {
        self.nodes[id.0].translate.x += x;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddTranslationY` (`jobj.h:1112`).
    pub fn add_translation_y(&mut self, id: JObjId, y: f32) {
        self.nodes[id.0].translate.y += y;
        self.srt_changed(id);
    }

    /// `HSD_JObjAddTranslationZ` (`jobj.h:1124`).
    pub fn add_translation_z(&mut self, id: JObjId, z: f32) {
        self.nodes[id.0].translate.z += z;
        self.srt_changed(id);
    }

    // -- animation ----------------------------------------------------------

    /// `HSD_JObjRemoveAnimByFlags` (`jobj.c:198`): bit 0 drops the joint
    /// AObj; the DObj list gets the same flags. RObj deferred.
    pub fn remove_anim_by_flags(&mut self, id: JObjId, flags: u32) {
        if flags & 1 != 0 {
            if let Some(mut aobj) = self.nodes[id.0].aobj.take() {
                if let Some(spare) = self.spare_tracks.get_mut(id.0) {
                    aobj.fobj.clear();
                    *spare = aobj.fobj;
                }
            }
        }
        if self.nodes[id.0].union_type_dobj() {
            DObj::remove_anim_all_by_flags(&mut self.nodes[id.0].dobj, flags);
        }
    }

    /// Allocate track storage for a fixed skeleton before simulation begins.
    /// `per_joint` comes from the caller's archive format's track-count bound.
    pub fn reserve_animation_tracks(&mut self, per_joint: usize) {
        self.spare_tracks.resize_with(self.nodes.len(), Vec::new);
        for (joint, spare) in self.nodes.iter_mut().zip(&mut self.spare_tracks) {
            let tracks = joint.aobj.as_mut().map_or(spare, |a| &mut a.fobj);
            tracks.reserve(per_joint.saturating_sub(tracks.len()));
        }
        // FObjInterpretAnim emits at most one value per track per evaluation.
        self.events.reserve(self.nodes.len() * per_joint);
    }

    /// Recycle the replaced joint animation's storage, retaining archive owners
    /// until the caller clears it. Unreserved loader callers get an empty Vec.
    pub fn take_animation_tracks(&mut self, id: JObjId) -> Vec<crate::fobj::FObj> {
        if let Some(aobj) = self.nodes[id.0].aobj.take() {
            aobj.fobj
        } else {
            self.spare_tracks
                .get_mut(id.0)
                .map(std::mem::take)
                .unwrap_or_default()
        }
    }

    /// `HSD_JObjRemoveAnimAllByFlags` (`jobj.c:212`).
    pub fn remove_anim_all_by_flags(&mut self, id: JObjId, flags: u32) {
        self.remove_anim_by_flags(id, flags);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut child = self.nodes[id.0].child;
            while let Some(c) = child {
                self.remove_anim_all_by_flags(c, flags);
                child = self.nodes[c.0].next;
            }
        }
    }

    /// `HSD_JObjRemoveAnim` (`jobj.c:226`).
    pub fn remove_anim(&mut self, id: JObjId) {
        self.remove_anim_by_flags(id, JOBJ_ALL_ANIM);
    }

    /// `HSD_JObjRemoveAnimAll` (`jobj.c:231`).
    pub fn remove_anim_all(&mut self, id: JObjId) {
        self.remove_anim_all_by_flags(id, JOBJ_ALL_ANIM);
    }

    /// `HSD_JObjReqAnimByFlags` (`jobj.c:236`): bit 0 restarts the joint
    /// AObj at `frame`; the DObj list gets `(frame, flags)`. RObj deferred.
    pub fn req_anim_by_flags(&mut self, id: JObjId, flags: u32, frame: f32) {
        if flags & 1 != 0 {
            if let Some(aobj) = self.nodes[id.0].aobj.as_mut() {
                aobj.req_anim(frame);
            }
        }
        if self.nodes[id.0].union_type_dobj() {
            DObj::req_anim_all_by_flags(&mut self.nodes[id.0].dobj, frame, flags);
        }
    }

    /// `HSD_JObjReqAnimAllByFlags` (`jobj.c:255`).
    pub fn req_anim_all_by_flags(&mut self, id: JObjId, flags: u32, frame: f32) {
        self.req_anim_by_flags(id, flags, frame);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut child = self.nodes[id.0].child;
            while let Some(c) = child {
                self.req_anim_all_by_flags(c, flags, frame);
                child = self.nodes[c.0].next;
            }
        }
    }

    /// `HSD_JObjReqAnimAll` (`jobj.c:269`).
    pub fn req_anim_all(&mut self, id: JObjId, frame: f32) {
        self.req_anim_all_by_flags(id, JOBJ_ALL_ANIM, frame);
    }

    /// `HSD_JObjReqAnim` (`jobj.c:274`).
    pub fn req_anim(&mut self, id: JObjId, frame: f32) {
        self.req_anim_by_flags(id, JOBJ_ALL_ANIM, frame);
    }

    /// `HSD_JObjAddAnim` (`jobj.c:298`): a `Some` anim joint replaces the
    /// AObj (sorted by [`jobj_sort_anim`]) and sets or clears
    /// `JOBJ_CLASSICAL_SCALE` from its bit 0; the DObj list gets the mat
    /// anims. `robj_anim` and shape anims are deferred.
    pub fn add_anim(
        &mut self,
        id: JObjId,
        an_joint: Option<&AnimJoint>,
        mat_joint: Option<&MatAnimJoint>,
    ) {
        if let Some(aj) = an_joint {
            let mut aobj = aj.aobjdesc.as_ref().map(AObj::load_desc);
            if let Some(a) = aobj.as_mut() {
                jobj_sort_anim(a);
            }
            self.nodes[id.0].aobj = aobj;
            // Archive attachment is the construction boundary for engine
            // animations. Reserve one callback per track in the loaded tree;
            // scene callers drain this buffer between animation evaluations.
            let event_bound: usize = self
                .nodes
                .iter()
                .filter_map(|node| node.aobj.as_ref())
                .map(|aobj| aobj.fobj.len())
                .sum();
            self.events
                .reserve(event_bound.saturating_sub(self.events.len()));
            if aj.flags & 1 != 0 {
                self.set_flags(id, JOBJ_CLASSICAL_SCALE);
            } else {
                self.clear_flags(id, JOBJ_CLASSICAL_SCALE);
            }
        }
        if self.nodes[id.0].union_type_dobj() {
            let matanim: &[MatAnim] = mat_joint.map_or(&[], |m| m.matanim.as_slice());
            DObj::add_anim_all(&mut self.nodes[id.0].dobj, matanim);
        }
    }

    /// Attach an initialization-loaded joint animation using the skeleton's
    /// reserved FObj storage. Same reset, sorting and flags as add_anim; part
    /// animation owners can repeat this without importing tracks in the tick.
    pub fn add_prepared_joint_anim(&mut self, id: JObjId, joint: &AnimJoint, prepared: &AObj) {
        let mut tracks = self.take_animation_tracks(id);
        assert!(
            tracks.capacity() >= prepared.fobj.len(),
            "unreserved part tracks"
        );
        tracks.clear();
        tracks.extend(prepared.fobj.iter().cloned());
        let mut aobj = AObj {
            flags: prepared.flags,
            curr_frame: prepared.curr_frame,
            rewind_frame: prepared.rewind_frame,
            end_frame: prepared.end_frame,
            framerate: prepared.framerate,
            fobj: tracks,
        };
        jobj_sort_anim(&mut aobj);
        self.nodes[id.0].aobj = Some(aobj);
        if joint.flags & 1 != 0 {
            self.set_flags(id, JOBJ_CLASSICAL_SCALE);
        } else {
            self.clear_flags(id, JOBJ_CLASSICAL_SCALE);
        }
        if self.nodes[id.0].union_type_dobj() {
            DObj::add_anim_all(&mut self.nodes[id.0].dobj, &[]);
        }
    }

    /// `HSD_JObjAddAnimAll` (`jobj.c:323`): [`JObjTree::add_anim`] down the
    /// tree, pairing children positionally with the anim trees' children;
    /// once an anim chain runs out the remaining joints get `None`.
    pub fn add_anim_all(
        &mut self,
        id: JObjId,
        ajoint: Option<&AnimJoint>,
        mjoint: Option<&MatAnimJoint>,
    ) {
        self.add_anim(id, ajoint, mjoint);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut jp = self.nodes[id.0].child;
            let mut aj = ajoint.map(|a| a.children.iter());
            let mut mj = mjoint.map(|m| m.children.iter());
            while let Some(j) = jp {
                let a = aj.as_mut().and_then(|it| it.next());
                let m = mj.as_mut().and_then(|it| it.next());
                self.add_anim_all(j, a, m);
                jp = self.nodes[j.0].next;
            }
        }
    }

    /// `JObjUpdateFunc` (`jobj.c:351`): apply one track value to the joint.
    ///
    /// `T` is only used by the `0x36`/`0x37` re-decomposition ids
    /// (`HSD_MtxGetRotation`), which an RObj expression delivers; FObj
    /// tracks never do.
    pub fn update_func<T: InverseTrig>(&mut self, id: JObjId, ty: u8, val: ObjData) {
        match ty {
            HSD_A_J_PATH => {
                let mut fv = val.fv();
                if f64::from(fv) < 0.0 {
                    fv = 0.0;
                }
                if 1.0 < f64::from(fv) {
                    fv = 1.0;
                }
                // splArcLengthPoint(&p, aobj->hsd_obj->u.spline, fv) then
                // HSD_JObjSetTranslate{X,Y,Z}: deferred (spline).
                if let Some(reference) = self.nodes[id.0].path_reference {
                    let spline = self.nodes[reference.0]
                        .spline
                        .as_ref()
                        .expect("PATH reference must hold a spline");
                    let position = crate::spline::linear_point(spline, fv);
                    self.set_translate(id, &position);
                } else {
                    self.events.push(JObjEvent::Path { jobj: id, t: fv });
                }
            }
            HSD_A_J_ROTX => {
                // JOBJ_JOINT1: robj->u.ik_hint.rotate_x = fv. Deferred (RObj).
                self.set_rotation_x(id, val.fv());
            }
            HSD_A_J_ROTY => self.set_rotation_y(id, val.fv()),
            HSD_A_J_ROTZ => self.set_rotation_z(id, val.fv()),
            HSD_A_J_TRAX => self.set_translate_x(id, val.fv()),
            HSD_A_J_TRAY => self.set_translate_y(id, val.fv()),
            HSD_A_J_TRAZ => self.set_translate_z(id, val.fv()),
            HSD_A_J_SCAX => self.set_scale_x(id, clamp_scale(val.fv())),
            HSD_A_J_SCAY => self.set_scale_y(id, clamp_scale(val.fv())),
            HSD_A_J_SCAZ => self.set_scale_z(id, clamp_scale(val.fv())),
            HSD_A_J_BRANCH => {
                if val.fv() > 0.5 {
                    self.clear_flags_all(id, JOBJ_HIDDEN);
                } else {
                    self.set_flags_all(id, JOBJ_HIDDEN);
                }
            }
            HSD_A_J_NODE => {
                if val.fv() > 0.5 {
                    self.clear_flags(id, JOBJ_HIDDEN);
                } else {
                    self.set_flags(id, JOBJ_HIDDEN);
                }
            }
            HSD_A_J_SETBYTE0..=HSD_A_J_SETBYTE9 => {
                self.events.push(JObjEvent::SetByte {
                    jobj: id,
                    ty,
                    value: val.iv(),
                });
            }
            HSD_A_J_SETFLOAT0..=HSD_A_J_SETFLOAT9 => {
                self.events.push(JObjEvent::SetFloat {
                    jobj: id,
                    ty,
                    value: val.fv(),
                });
            }
            HSD_A_J_DPTCL => {
                let iv = val.iv();
                let lo = iv & 0x3F;
                let hi = (iv >> 6) & 0xFF_FFFF;
                self.events.push(JObjEvent::DPtcl { jobj: id, lo, hi });
            }
            HSD_A_J_JSOUND => self.events.push(JObjEvent::JSound(val.iv())),
            HSD_A_J_PTCLTGT => self.events.push(JObjEvent::PtclTgt {
                jobj: id,
                value: val.iv(),
            }),
            HSD_A_J_MTX_COL0..=HSD_A_J_MTX_COL3 => {
                // Only HSD_RObjUpdateAll sends these, with a Vec3 payload.
                if let ObjData::Vec(p) = val {
                    let col = usize::from(ty - HSD_A_J_MTX_COL0);
                    let m = &mut self.nodes[id.0].mtx.0;
                    m[0][col] = p.x;
                    m[1][col] = p.y;
                    m[2][col] = p.z;
                }
            }
            HSD_A_J_MTX_SRT..=HSD_A_J_MTX_SCA => {
                let mut m = Mtx::ZERO;
                match self.nodes[id.0].parent {
                    Some(p) => mtx::hsd_mtx_inverse_concat(
                        &self.nodes[p.0].mtx,
                        &self.nodes[id.0].mtx,
                        &mut m,
                    ),
                    None => mtx::mtx_copy(&self.nodes[id.0].mtx, &mut m),
                }
                let node = &mut self.nodes[id.0];
                if ty == HSD_A_J_MTX_SRT || ty == HSD_A_J_MTX_TRA {
                    mtx::hsd_mtx_get_translate(&m, &mut node.translate);
                }
                if ty == HSD_A_J_MTX_SRT || ty == HSD_A_J_MTX_ROT {
                    // HSD_MtxGetRotation(mtx, (Vec3*) &jobj->rotate)
                    let mut euler = Vec3::new(node.rotate.x, node.rotate.y, node.rotate.z);
                    mtx::hsd_mtx_get_rotation::<T>(&m, &mut euler);
                    node.rotate.x = euler.x;
                    node.rotate.y = euler.y;
                    node.rotate.z = euler.z;
                }
                if ty == HSD_A_J_MTX_SRT || ty == HSD_A_J_MTX_SCA {
                    mtx::hsd_mtx_get_scale(&m, &mut node.scale);
                }
            }
            _ => {}
        }
    }

    /// `HSD_JObjAnim` (`jobj.c:531`): `HSD_JObjCheckDepend`, one
    /// `HSD_AObjInterpretAnim` step through `JObjUpdateFunc`, then the RObj
    /// list (deferred) and `HSD_DObjAnimAll`. `cb` accumulates the
    /// end-callback counters.
    pub fn anim<T: InverseTrig>(&mut self, id: JObjId, cb: &mut AObjEndCallback) {
        self.check_depend(id);
        // The AObj is taken out for the step so the update callback can
        // borrow the tree. JObjUpdateFunc never touches jobj->aobj itself
        // (the HSD_A_J_PATH read of aobj->hsd_obj is deferred).
        if let Some(mut aobj) = self.nodes[id.0].aobj.take() {
            aobj.interpret_anim(
                &mut |ty, fv| self.update_func::<T>(id, ty, ObjData::Float(fv)),
                cb,
            );
            self.nodes[id.0].aobj = Some(aobj);
        }
        if self.nodes[id.0].union_type_dobj() {
            DObj::anim_all(&mut self.nodes[id.0].dobj, cb);
        }
    }

    /// `JObjAnimAll` (`jobj.c:543`): [`JObjTree::anim`] on the node, then
    /// on each child chain in order, stopping at `JOBJ_INSTANCE`.
    pub fn jobj_anim_all<T: InverseTrig>(&mut self, id: JObjId, cb: &mut AObjEndCallback) {
        self.anim::<T>(id, cb);
        if self.nodes[id.0].flags & JOBJ_INSTANCE == 0 {
            let mut child = self.nodes[id.0].child;
            while let Some(c) = child {
                self.jobj_anim_all::<T>(c, cb);
                child = self.nodes[c.0].next;
            }
        }
    }

    /// `HSD_JObjAnimAll` (`jobj.c:558`): `HSD_AObjInitEndCallBack`, the
    /// tree walk, then `HSD_AObjInvokeCallBacks`. The counters are
    /// returned; [`AObjEndCallback::should_invoke`] is the invoke test.
    pub fn anim_all<T: InverseTrig>(&mut self, id: JObjId) -> AObjEndCallback {
        let mut cb = AObjEndCallback::default();
        cb.init();
        self.jobj_anim_all::<T>(id, &mut cb);
        cb
    }
}

/// The `HSD_A_J_SCA*` floor (`jobj.c:403`): `if (fabsf_bitwise(fv) < 1e-3F)
/// fv = 1e-3F`.
fn clamp_scale(fv: f32) -> f32 {
    if fabsf(fv) < 1e-3 {
        1e-3
    } else {
        fv
    }
}

/// `JObjSortAnim` (`jobj.c:279`): move the first track whose `obj_type` is
/// `TYPE_JOBJ` (`12`, the BRANCH id) to the front of the list so it is
/// evaluated before the SRT tracks.
pub fn jobj_sort_anim(aobj: &mut AObj) {
    if let Some(i) = aobj.fobj.iter().position(|f| f.obj_type == TYPE_JOBJ) {
        let f = aobj.fobj.remove(i);
        aobj.fobj.insert(0, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_matches_jobj_init() {
        let j = JObj::init();
        assert_eq!(j.flags, JOBJ_MTX_DIRTY);
        assert_eq!(j.scale, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(j.rotate, Quaternion::new(0.0, 0.0, 0.0, 0.0));
        assert!(j.mtx_is_dirty());
        assert!(j.union_type_dobj());
    }

    #[test]
    fn obj_data_words() {
        assert_eq!(ObjData::Float(1.0).iv(), 0x3F80_0000);
        assert_eq!(ObjData::Vec(Vec3::new(2.0, 0.0, 0.0)).fv(), 2.0);
        assert_eq!(ObjData::Null.iv(), 0);
    }

    #[test]
    fn clamp_scale_floor() {
        assert_eq!(clamp_scale(0.0), 1e-3);
        assert_eq!(clamp_scale(-0.0005), 1e-3);
        assert_eq!(clamp_scale(-0.5), -0.5);
        assert_eq!(clamp_scale(1e-3), 1e-3);
    }
}
