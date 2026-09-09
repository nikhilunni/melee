//! Typed, owned-tree readers for HSD descriptor structs as laid out on disc
//! inside a `.dat` archive.
//!
//! Retail overlays these C structs directly on the relocated data section
//! and the `*LoadDesc` functions (`JObjLoad`, `HSD_DObjLoadDesc`,
//! `HSD_AObjLoadDesc`, `HSD_FObjLoadDesc`, ...) walk them by pointer. This
//! module does the same walk over the un-relocated bytes, following each
//! pointer field through [`Archive::link`], and produces owned Rust values
//! so that nothing above `hsd-archive` needs to know a single offset.
//!
//! # Structs covered
//!
//! | Rust type | C struct | header | size |
//! |---|---|---|---|
//! | [`JObjDesc`] | `HSD_Joint` | `sysdolphin/baselib/jobj.h` | `0x40` |
//! | [`DObjDesc`] | `HSD_DObjDesc` | `sysdolphin/baselib/dobj.h` | `0x10` |
//! | [`MObjDesc`] | `HSD_MObjDesc` | `sysdolphin/baselib/mobj.h` | `0x18` |
//! | [`Material`] | `HSD_Material` | `sysdolphin/baselib/mobj.h` | `0x14` |
//! | [`AObjDesc`] | `HSD_AObjDesc` | `sysdolphin/baselib/aobj.h` | `0x10` |
//! | [`FObjDesc`] | `HSD_FObjDesc` | `sysdolphin/baselib/fobj.h` | `0x14` |
//! | [`AnimJoint`] | `HSD_AnimJoint` | `sysdolphin/baselib/aobj.h` | `0x14` |
//! | [`MatAnimJoint`] | `HSD_MatAnimJoint` | `sysdolphin/baselib/mobj.h` | `0x0C` |
//! | [`ShapeAnimJoint`] | `HSD_ShapeAnimJoint` | `sysdolphin/baselib/pobj.h` | `0x0C` |
//! | [`FigaTree`] | `FigaTree` | `melee/lb/lbanim.h` | `0x14` |
//! | [`FigaTrack`] | `FigaTrack` | `melee/lb/lbanim.h` | `0x0C` |
//!
//! The decomp names the joint descriptor `HSD_Joint`; the Rust type is
//! called `JObjDesc` to line up with the other `*Desc` names, and every
//! doc comment cites the C name so the header can be found.
//!
//! None of these headers carry an `ASSERT_SIZE`, so the sizes above are
//! derived from the field lists (all pointers are 4 bytes, `Vec3` is 12,
//! `GXColor` is 4) and pinned by `const` assertions in each submodule: the
//! last field's offset plus its width must equal the declared size.
//!
//! # Linkage
//!
//! Tree structs keep the C `child` / `next` linkage literally as
//! `Option<Box<Self>>` rather than flattening siblings into a `Vec`.
//! Reasons:
//!
//! - `JObjLoad`, `HSD_JObjAddAnimAll` and `HSD_JObjResolveRefsAll` walk a
//!   joint tree and its animation trees *in lock step* through `child` and
//!   `next`. Keeping the same shape lets the `hsd-gobj` port be a direct
//!   transliteration, and keeps sibling order exactly as stored.
//! - A public root may itself have a `next` sibling (`JObjLoad` loads
//!   `joint->next` for the root too). A `children: Vec` model would have
//!   nowhere to put it.
//!
//! [`JObjDesc::children`] and the other `children()`/`siblings()` iterators
//! are provided for callers who just want to visit nodes in order.
//!
//! Types with a `next` chain implement `Drop` by hand so that dropping a
//! long sibling list does not recurse once per node. The `child` axis is
//! bounded by [`MAX_DEPTH`], so recursion along it is safe.
//!
//! # Pointer policy
//!
//! Every pointer field is resolved with [`Archive::link`]: a slot named by
//! the relocation table is a link to the offset it holds (even offset 0);
//! a slot that is *not* relocated and holds `0` is a null pointer; a slot
//! that is not relocated and holds a non-zero value is reported as
//! [`DescError::UnrelocatedPointer`], because retail would dereference it
//! as garbage.
//!
//! Fields documented as "raw offset" (`pobjdesc`, `texdesc`, `renderdesc`,
//! `pedesc`, `robjdesc`, `robj_anim`, `matanim`, `shapeanimdobj`, particle
//! and spline unions) are resolved the same way but not read through;
//! they are kept as `Option<u32>` data offsets for later modules.
//!
//! # Safety against malformed files
//!
//! Reading never panics. Every read is bounds-checked, pointer cycles are
//! detected by tracking the set of struct offsets on the current
//! traversal path ([`DescError::Cycle`]), `child` nesting is capped at
//! [`MAX_DEPTH`] and the total number of structs visited in one call at
//! [`MAX_NODES`]. Shared sub-trees (two pointers to one struct, which is a
//! DAG rather than a cycle) are allowed and are read into independent
//! owned copies.

use core::fmt;
use std::collections::HashSet;

use crate::archive::Archive;
use crate::error::Error;
use crate::reader::{add_offset, Reader};

pub mod anim;
pub mod dobj;
pub mod figatree;
pub mod jobj;
pub mod light;
pub mod spline;

pub use anim::{
    AObjDesc, AnimJoint, FObjDesc, MatAnimJoint, ShapeAnimJoint, ANIM_JOINT_SIZE, AOBJ_DESC_SIZE,
    FOBJ_DESC_SIZE, MAT_ANIM_JOINT_SIZE, SHAPE_ANIM_JOINT_SIZE,
};
pub use dobj::{DObjDesc, MObjDesc, Material, DOBJ_DESC_SIZE, MATERIAL_SIZE, MOBJ_DESC_SIZE};
pub use figatree::{FigaTrack, FigaTree, FIGATRACK_SIZE, FIGATREE_SIZE};
pub use jobj::{JObjDesc, JObjUnion, JOBJ_DESC_SIZE};

/// Maximum `child` nesting depth accepted by any reader in this module.
///
/// Retail skeletons are a few dozen levels deep at most; this only exists
/// so a hostile file cannot exhaust the stack.
pub const MAX_DEPTH: usize = 256;

/// Maximum number of descriptor structs one `read` call will visit.
///
/// Bounds the work (and memory) a file can demand through shared
/// sub-trees, which are copied once per reference.
pub const MAX_NODES: usize = 1 << 20;

/// Three floats, as `Vec3` in `dolphin/mtx.h`. Bit copies of the file
/// contents; no arithmetic is performed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `Mtx` from `dolphin/mtx.h`: `f32[3][4]`, three rows of four, stored
/// row-major.
pub type Mtx = [[f32; 4]; 3];

/// `GXColor` from `dolphin/gx/GXStruct.h`: four bytes `r, g, b, a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GxColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Errors from descriptor reads. Wraps the crate's [`Error`] for plain
/// bounds failures and adds the structural faults only a tree walk can
/// detect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescError {
    /// A bounds or overflow failure from the underlying reader.
    Archive(Error),
    InvalidSpline {
        offset: u32,
        reason: &'static str,
    },
    /// The archive has no public symbol with this name.
    MissingSymbol {
        name: String,
    },
    /// A pointer field holds a non-zero value but its slot is not in the
    /// relocation table, so it cannot be a valid link.
    UnrelocatedPointer {
        /// `Struct.field` the slot belongs to.
        field: &'static str,
        /// Data offset of the slot.
        at: u32,
        /// The value found there.
        value: u32,
    },
    /// A pointer field that retail dereferences unconditionally is null.
    NullPointer {
        /// `Struct.field` the slot belongs to.
        field: &'static str,
        /// Data offset of the slot.
        at: u32,
    },
    /// A struct at `offset` was reached again while it was still on the
    /// traversal path (a `child`/`next` loop).
    Cycle {
        what: &'static str,
        offset: u32,
    },
    /// `child` nesting exceeded [`MAX_DEPTH`].
    DepthExceeded {
        what: &'static str,
        offset: u32,
    },
    /// More than [`MAX_NODES`] structs were visited in one read.
    TooManyNodes {
        what: &'static str,
        offset: u32,
    },
    /// A byte stream (`FObjDesc.ad`, `FigaTrack.ad_head`) runs past the
    /// end of the data section.
    TruncatedStream {
        /// `Struct.field` naming the stream.
        field: &'static str,
        /// Offset of the stream's first byte.
        offset: u32,
        /// Declared length in bytes.
        len: u32,
        /// Size of the data section.
        available: usize,
    },
    /// A `FigaTree.nodes` entry is negative and not the `-1` terminator.
    BadFigaTreeNode {
        offset: u32,
        value: i8,
    },
}

impl From<Error> for DescError {
    fn from(e: Error) -> Self {
        DescError::Archive(e)
    }
}

impl fmt::Display for DescError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescError::Archive(e) => write!(f, "{e}"),
            DescError::InvalidSpline { offset, reason } => write!(f, "invalid spline at {offset:#x}: {reason}"),
            DescError::MissingSymbol { name } => {
                write!(f, "archive exports no public symbol {name:?}")
            }
            DescError::UnrelocatedPointer { field, at, value } => write!(
                f,
                "{field} at data offset {at:#x} holds {value:#x} but is not in the relocation table"
            ),
            DescError::NullPointer { field, at } => {
                write!(f, "{field} at data offset {at:#x} is null but is required")
            }
            DescError::Cycle { what, offset } => {
                write!(f, "{what} at data offset {offset:#x} links back into itself")
            }
            DescError::DepthExceeded { what, offset } => write!(
                f,
                "{what} at data offset {offset:#x} exceeds the child depth limit of {MAX_DEPTH}"
            ),
            DescError::TooManyNodes { what, offset } => write!(
                f,
                "{what} at data offset {offset:#x} exceeds the limit of {MAX_NODES} structs per read"
            ),
            DescError::TruncatedStream {
                field,
                offset,
                len,
                available,
            } => write!(
                f,
                "{field} stream at data offset {offset:#x} of {len:#x} bytes runs past the data section of {available:#x} bytes"
            ),
            DescError::BadFigaTreeNode { offset, value } => write!(
                f,
                "FigaTree.nodes entry at data offset {offset:#x} is {value}, expected a track count or -1"
            ),
        }
    }
}

impl std::error::Error for DescError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DescError::Archive(e) => Some(e),
            _ => None,
        }
    }
}

/// Result alias for this module.
pub type Result<T> = core::result::Result<T, DescError>;

/// Per-read traversal state: the archive, the offsets currently on the
/// traversal path, the `child` depth and the node budget.
pub(crate) struct Ctx<'a> {
    archive: &'a Archive,
    path: HashSet<u32>,
    depth: usize,
    nodes: usize,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            path: HashSet::new(),
            depth: 0,
            nodes: 0,
        }
    }

    pub(crate) fn reader(&self) -> Reader<'a> {
        self.archive.reader()
    }

    /// Resolve the pointer field at `base + field` per the module's
    /// pointer policy.
    pub(crate) fn link(&self, name: &'static str, base: u32, field: u32) -> Result<Option<u32>> {
        let at = add_offset(base, field)?;
        if let Some(target) = self.archive.link(at)? {
            return Ok(Some(target));
        }
        let value = self.reader().u32(at)?;
        if value != 0 {
            return Err(DescError::UnrelocatedPointer {
                field: name,
                at,
                value,
            });
        }
        Ok(None)
    }

    /// The NUL-terminated `class_name` string a slot points at, if any.
    pub(crate) fn class_name(
        &self,
        name: &'static str,
        base: u32,
        field: u32,
    ) -> Result<Option<String>> {
        match self.link(name, base, field)? {
            Some(off) => Ok(Some(self.reader().cstr(off)?.to_owned())),
            None => Ok(None),
        }
    }

    /// `len` bytes at `offset`, or [`DescError::TruncatedStream`].
    pub(crate) fn stream(&self, field: &'static str, offset: u32, len: u32) -> Result<Vec<u8>> {
        let reader = self.reader();
        reader
            .slice(offset, len)
            .map(<[u8]>::to_vec)
            .map_err(|_| DescError::TruncatedStream {
                field,
                offset,
                len,
                available: reader.len(),
            })
    }

    /// Three big-endian floats at `offset`.
    pub(crate) fn vec3(&self, offset: u32) -> Result<Vec3> {
        let r = self.reader();
        Ok(Vec3 {
            x: r.f32(offset)?,
            y: r.f32(add_offset(offset, 4)?)?,
            z: r.f32(add_offset(offset, 8)?)?,
        })
    }

    /// A `GXColor` at `offset`.
    pub(crate) fn color(&self, offset: u32) -> Result<GxColor> {
        let [r, g, b, a] = self.reader().array::<4>(offset)?;
        Ok(GxColor { r, g, b, a })
    }

    /// Mark a struct as entered: checks the node budget and the path for
    /// cycles. Must be paired with [`Ctx::leave`].
    pub(crate) fn enter(&mut self, what: &'static str, offset: u32) -> Result<()> {
        if self.nodes >= MAX_NODES {
            return Err(DescError::TooManyNodes { what, offset });
        }
        if !self.path.insert(offset) {
            return Err(DescError::Cycle { what, offset });
        }
        self.nodes += 1;
        Ok(())
    }

    pub(crate) fn leave(&mut self, offset: u32) {
        self.path.remove(&offset);
    }

    /// Step one level down the `child` axis. Must be paired with
    /// [`Ctx::ascend`].
    pub(crate) fn descend(&mut self, what: &'static str, offset: u32) -> Result<()> {
        if self.depth >= MAX_DEPTH {
            return Err(DescError::DepthExceeded { what, offset });
        }
        self.depth += 1;
        Ok(())
    }

    pub(crate) fn ascend(&mut self) {
        self.depth -= 1;
    }
}

/// A struct with a C `next` pointer, read as a singly linked chain.
pub(crate) trait ChainNode: Sized {
    /// C struct name, for error messages.
    const WHAT: &'static str;
    /// Data offset of the `next` field.
    const NEXT: u32;
    /// Read every field except `next` (recursing into `child` and other
    /// owned links). The node has already been `enter`ed.
    fn read_node(ctx: &mut Ctx<'_>, offset: u32) -> Result<Self>;
    /// Attach the already-read tail.
    fn set_next(&mut self, next: Option<Box<Self>>);
}

/// Read the chain headed at `head`, following `next` iteratively so a long
/// sibling list neither recurses nor escapes cycle detection. All nodes of
/// the chain stay on the traversal path until the whole chain is read, so
/// a `child` that points back at any sibling is caught as a cycle.
pub(crate) fn read_chain<T: ChainNode>(
    ctx: &mut Ctx<'_>,
    head: Option<u32>,
) -> Result<Option<Box<T>>> {
    let mut offsets: Vec<u32> = Vec::new();
    let mut nodes: Vec<T> = Vec::new();
    let mut cursor = head;
    let result: Result<()> = (|| {
        while let Some(off) = cursor {
            ctx.enter(T::WHAT, off)?;
            offsets.push(off);
            nodes.push(T::read_node(ctx, off)?);
            cursor = ctx.link(T::WHAT, off, T::NEXT)?;
        }
        Ok(())
    })();
    for off in &offsets {
        ctx.leave(*off);
    }
    result?;
    let mut tail: Option<Box<T>> = None;
    for mut node in nodes.into_iter().rev() {
        node.set_next(tail);
        tail = Some(Box::new(node));
    }
    Ok(tail)
}

/// Iterator over a `next` chain by reference.
pub struct Siblings<'a, T> {
    cursor: Option<&'a T>,
    next: fn(&'a T) -> Option<&'a T>,
}

impl<'a, T> Siblings<'a, T> {
    pub(crate) fn new(head: Option<&'a T>, next: fn(&'a T) -> Option<&'a T>) -> Self {
        Self { cursor: head, next }
    }
}

impl<'a, T> Iterator for Siblings<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let cur = self.cursor?;
        self.cursor = (self.next)(cur);
        Some(cur)
    }
}

/// Data offset of the public symbol `name`, or [`DescError::MissingSymbol`].
fn public_offset(archive: &Archive, name: &str) -> Result<u32> {
    archive
        .public(name)
        .ok_or_else(|| DescError::MissingSymbol {
            name: name.to_owned(),
        })
}

/// Read the `HSD_Joint` tree rooted at the public symbol `name`, as
/// `HSD_JObjLoadJoint(HSD_ArchiveGetPublicAddress(archive, name))` would.
pub fn read_public_jobj(archive: &Archive, name: &str) -> Result<JObjDesc> {
    JObjDesc::read(archive, public_offset(archive, name)?)
}

/// Read the `HSD_AnimJoint` tree rooted at the public symbol `name`.
pub fn read_public_animjoint(archive: &Archive, name: &str) -> Result<AnimJoint> {
    AnimJoint::read(archive, public_offset(archive, name)?)
}

/// Read the `FigaTree` at the public symbol `name` (fighter motion files
/// export one per action, e.g. `PlyFox5K_Share_ACTION_Wait1_figatree`).
pub fn read_public_figatree(archive: &Archive, name: &str) -> Result<FigaTree> {
    FigaTree::read(archive, public_offset(archive, name)?)
}
