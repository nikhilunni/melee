//! `HSD_Joint` (jobj.h), the joint descriptor that `HSD_JObjLoadJoint`
//! turns into an `HSD_JObj` tree. Called `JObjDesc` here.
//!
//! ```text
//! HSD_Joint, 0x40 bytes                (offset comments from jobj.h)
//!   +0x00 char*         class_name
//!   +0x04 u32           flags          JOBJ_* bits
//!   +0x08 HSD_Joint*    child          or an instance id when JOBJ_INSTANCE
//!   +0x0C HSD_Joint*    next
//!   +0x10 union         u              HSD_DObjDesc* / HSD_Spline* / HSD_SList* ptcl
//!   +0x14 Vec3          rotation       Euler radians
//!   +0x20 Vec3          scale
//!   +0x2C Vec3          position
//!   +0x38 MtxPtr        mtx            envelope (inverse bind) matrix, f32[3][4]
//!   +0x3C HSD_RObjDesc* robjdesc       (raw)
//! ```
//!
//! Field semantics follow `JObjLoad` (jobj.c):
//!
//! - `flags & JOBJ_INSTANCE`: `child` is **not** loaded as a sub-tree; it
//!   is the id of another joint (`HSD_JObjResolveRefs` looks it up in the
//!   id table). It is kept as [`JObjDesc::instance_of`].
//! - `flags & JOBJ_SPLINE`: `u` is an `HSD_Spline*`; `flags & JOBJ_PTCL`:
//!   `u` is an `HSD_SList*` of particle banks; otherwise `u` is the head
//!   of an `HSD_DObjDesc` chain. See [`JObjUnion`]. Retail tests spline
//!   first, then ptcl, so a joint with both bits set is a spline.
//! - `mtx`, when non-null, is copied into `jobj->envelopemtx`.
//! - `robjdesc` is loaded by `HSD_RObjLoadDesc`, which is not ported yet;
//!   the raw offset is kept.

use super::{read_chain, ChainNode, Ctx, DObjDesc, Mtx, Result, Siblings, Vec3};
use crate::archive::Archive;
use crate::reader::add_offset;

/// `sizeof(HSD_Joint)`.
pub const JOBJ_DESC_SIZE: u32 = 0x40;

/// `JOBJ_SPLINE` (jobj.h): the union holds a spline.
pub const JOBJ_SPLINE: u32 = 1 << 14;
/// `JOBJ_PTCL` (jobj.h): the union holds a particle list.
pub const JOBJ_PTCL: u32 = 1 << 5;
/// `JOBJ_INSTANCE` (jobj.h): `child` is an instance id, not a sub-tree.
pub const JOBJ_INSTANCE: u32 = 1 << 12;

mod off {
    pub const CLASS_NAME: u32 = 0x00;
    pub const FLAGS: u32 = 0x04;
    pub const CHILD: u32 = 0x08;
    pub const NEXT: u32 = 0x0C;
    pub const UNION: u32 = 0x10;
    pub const ROTATION: u32 = 0x14;
    pub const SCALE: u32 = 0x20;
    pub const POSITION: u32 = 0x2C;
    pub const MTX: u32 = 0x38;
    pub const ROBJDESC: u32 = 0x3C;
}
const _: () = assert!(off::ROTATION == off::UNION + 4);
const _: () = assert!(off::SCALE == off::ROTATION + 12);
const _: () = assert!(off::POSITION == off::SCALE + 12);
const _: () = assert!(off::MTX == off::POSITION + 12);
const _: () = assert!(off::ROBJDESC + 4 == JOBJ_DESC_SIZE);

/// What the `+0x10` union of an `HSD_Joint` holds, decided by `flags`
/// exactly as `JObjLoad` does.
#[derive(Debug, Clone, PartialEq)]
pub enum JObjUnion {
    /// Head of the `HSD_DObjDesc` chain, or `None` for a joint with no
    /// geometry.
    Dobj(Option<Box<DObjDesc>>),
    /// `JOBJ_SPLINE`: raw data offset of the `HSD_Spline`.
    Spline(Option<u32>),
    /// `JOBJ_PTCL`: raw data offset of the `HSD_SList` particle list.
    Ptcl(Option<u32>),
}

impl Default for JObjUnion {
    fn default() -> Self {
        JObjUnion::Dobj(None)
    }
}

impl JObjUnion {
    /// The display-object chain, if this union is one.
    pub fn dobj(&self) -> Option<&DObjDesc> {
        match self {
            JObjUnion::Dobj(d) => d.as_deref(),
            _ => None,
        }
    }
}

/// `HSD_Joint` (jobj.h): one node of a joint descriptor tree.
///
/// `child` and `next` mirror the C pointers; see the module docs of
/// [`super`] for why they are not flattened.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JObjDesc {
    /// Data offset this struct was read from. Retail also uses this
    /// address as the joint's id (`jobj->id = (u32) joint`).
    pub offset: u32,
    pub class_name: Option<String>,
    /// `JOBJ_*` bits.
    pub flags: u32,
    /// First child, unless `flags & JOBJ_INSTANCE`.
    pub child: Option<Box<JObjDesc>>,
    /// Data offset of the joint this one instances (`flags & JOBJ_INSTANCE`
    /// only). `JObjLoad` skips `child` for such joints and
    /// `HSD_JObjResolveRefs` later looks the id up, so it is not read
    /// through here.
    pub instance_of: Option<u32>,
    /// Next sibling.
    pub next: Option<Box<JObjDesc>>,
    /// The `u` union, typed by `flags`.
    pub u: JObjUnion,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub position: Vec3,
    /// Envelope matrix, when the joint has one.
    pub mtx: Option<Mtx>,
    /// Raw data offset of the `HSD_RObjDesc` chain, if any.
    pub robjdesc: Option<u32>,
}

impl JObjDesc {
    /// Read the joint tree rooted at data offset `off`, including `next`
    /// siblings of the root (as `HSD_JObjLoadJoint` does).
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let mut ctx = Ctx::new(archive);
        let head = read_chain::<Self>(&mut ctx, Some(off))?;
        // `read_chain` returns `Some` whenever it was handed `Some`.
        Ok(*head.unwrap_or_default())
    }

    /// Direct children in `child`/`next` order.
    pub fn children(&self) -> Siblings<'_, JObjDesc> {
        Siblings::new(self.child.as_deref(), |j| j.next.as_deref())
    }

    /// This joint and every `next` sibling after it, in order.
    pub fn siblings(&self) -> Siblings<'_, JObjDesc> {
        Siblings::new(Some(self), |j| j.next.as_deref())
    }

    /// Every joint in the sub-tree rooted here (siblings of `self`
    /// excluded), in the depth-first order `JObjLoad` allocates them:
    /// a node, then its children's sub-trees in order.
    pub fn descendants(&self) -> Vec<&JObjDesc> {
        let mut out = Vec::new();
        let mut stack: Vec<&JObjDesc> = vec![self];
        while let Some(j) = stack.pop() {
            out.push(j);
            // Push children in reverse so the first child pops first.
            let kids: Vec<&JObjDesc> = j.children().collect();
            stack.extend(kids.into_iter().rev());
        }
        out
    }

    /// `flags & JOBJ_INSTANCE`.
    pub fn is_instance(&self) -> bool {
        self.flags & JOBJ_INSTANCE != 0
    }
}

impl ChainNode for JObjDesc {
    const WHAT: &'static str = "HSD_Joint";
    const NEXT: u32 = off::NEXT;

    fn read_node(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        let flags = r.u32(add_offset(off, off::FLAGS)?)?;

        let child_link = ctx.link("HSD_Joint.child", off, off::CHILD)?;
        let (child, instance_of) = if flags & JOBJ_INSTANCE != 0 {
            (None, child_link)
        } else {
            let child = match child_link {
                Some(c) => {
                    ctx.descend("HSD_Joint", c)?;
                    let out = read_chain::<Self>(ctx, Some(c));
                    ctx.ascend();
                    out?
                }
                None => None,
            };
            (child, None)
        };

        let union_link = ctx.link("HSD_Joint.u", off, off::UNION)?;
        let u = if flags & JOBJ_SPLINE != 0 {
            JObjUnion::Spline(union_link)
        } else if flags & JOBJ_PTCL != 0 {
            JObjUnion::Ptcl(union_link)
        } else {
            JObjUnion::Dobj(read_chain::<DObjDesc>(ctx, union_link)?)
        };

        let mtx = match ctx.link("HSD_Joint.mtx", off, off::MTX)? {
            Some(m) => {
                let mut out: Mtx = [[0.0; 4]; 3];
                for (i, row) in out.iter_mut().enumerate() {
                    for (j, cell) in row.iter_mut().enumerate() {
                        *cell = r.f32(add_offset(m, ((i * 4 + j) * 4) as u32)?)?;
                    }
                }
                Some(out)
            }
            None => None,
        };

        Ok(Self {
            offset: off,
            class_name: ctx.class_name("HSD_Joint.class_name", off, off::CLASS_NAME)?,
            flags,
            child,
            instance_of,
            next: None,
            u,
            rotation: ctx.vec3(add_offset(off, off::ROTATION)?)?,
            scale: ctx.vec3(add_offset(off, off::SCALE)?)?,
            position: ctx.vec3(add_offset(off, off::POSITION)?)?,
            mtx,
            robjdesc: ctx.link("HSD_Joint.robjdesc", off, off::ROBJDESC)?,
        })
    }

    fn set_next(&mut self, next: Option<Box<Self>>) {
        self.next = next;
    }
}

impl Drop for JObjDesc {
    /// Unlink the `next` chain iteratively so a long sibling list does
    /// not recurse once per node. `child` drops recursively, bounded by
    /// [`super::MAX_DEPTH`].
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut node) = next {
            next = node.next.take();
        }
    }
}
