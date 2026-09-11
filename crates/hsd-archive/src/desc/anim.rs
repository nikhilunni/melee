//! Animation descriptors: `HSD_AObjDesc` and `HSD_AnimJoint` (aobj.h),
//! `HSD_FObjDesc` (fobj.h), `HSD_MatAnimJoint` (mobj.h) and
//! `HSD_ShapeAnimJoint` (pobj.h).
//!
//! ```text
//! HSD_AObjDesc, 0x10 bytes              HSD_FObjDesc, 0x14 bytes
//!   +0x00 u32           flags             +0x00 HSD_FObjDesc* next
//!   +0x04 f32           end_frame         +0x04 u32           length      bytes in ad
//!   +0x08 HSD_FObjDesc* fobjdesc          +0x08 f32           startframe
//!   +0x0C u32           obj_id            +0x0C u8            type        HSD_A_J_* track id
//!                                         +0x0D u8            frac_value  HSD_A_FRAC_* | HSD_A_OP_*
//! HSD_AnimJoint, 0x14 bytes               +0x0E u8            frac_slope
//!   +0x00 HSD_AnimJoint*     child        +0x0F u8            dummy0
//!   +0x04 HSD_AnimJoint*     next         +0x10 u8*           ad          key-frame byte stream
//!   +0x08 HSD_AObjDesc*      aobjdesc
//!   +0x0C HSD_RObjAnimJoint* robj_anim  (raw)
//!   +0x10 u32                flags
//!
//! HSD_MatAnimJoint, 0x0C bytes          HSD_ShapeAnimJoint, 0x0C bytes
//!   +0x00 HSD_MatAnimJoint* child         +0x00 HSD_ShapeAnimJoint* child
//!   +0x04 HSD_MatAnimJoint* next          +0x04 HSD_ShapeAnimJoint* next
//!   +0x08 HSD_MatAnim*      matanim (raw) +0x08 HSD_ShapeAnimDObj*  shapeanimdobj (raw)
//! ```
//!
//! `HSD_FObjLoadDesc` (fobj.c) copies `startframe`, `type`, `frac_value`,
//! `frac_slope`, `ad` and `length` into an `HSD_FObj`; the interpreter
//! stops once `ad - ad_head >= length`, so `length` is the byte count of
//! the stream and [`FObjDesc::ad`] holds exactly those bytes.
//!
//! `HSD_AObjDesc.obj_id` is passed to `HSD_IDGetDataFromTable` and, failing
//! that, to `HSD_JObjLoadJoint((void*) obj_id)`, so in a file it is either
//! `0` or a relocated pointer to an `HSD_Joint`. It is kept as the raw
//! `u32` plus [`AObjDesc::obj_id_is_link`] rather than being read through,
//! since retail treats it as an id first.
//!
//! `HSD_MatAnimJoint` and `HSD_ShapeAnimJoint` mirror the joint tree so
//! `HSD_JObjAddAnimAll` can walk all four trees in lock step. Their tree
//! linkage is read; the per-node payloads (`HSD_MatAnim` chains carrying
//! texture/tev animations, `HSD_ShapeAnimDObj` chains carrying vertex
//! blend shapes) are render-side and kept as raw offsets.

use super::{read_chain, ChainNode, Ctx, Result, Siblings};
use crate::archive::Archive;
use crate::reader::add_offset;

/// `sizeof(HSD_AObjDesc)`.
pub const AOBJ_DESC_SIZE: u32 = 0x10;
/// `sizeof(HSD_FObjDesc)`.
pub const FOBJ_DESC_SIZE: u32 = 0x14;
/// `sizeof(HSD_AnimJoint)`.
pub const ANIM_JOINT_SIZE: u32 = 0x14;
/// `sizeof(HSD_MatAnimJoint)`.
pub const MAT_ANIM_JOINT_SIZE: u32 = 0x0C;
/// `sizeof(HSD_ShapeAnimJoint)`.
pub const SHAPE_ANIM_JOINT_SIZE: u32 = 0x0C;

mod aobj_off {
    pub const FLAGS: u32 = 0x00;
    pub const END_FRAME: u32 = 0x04;
    pub const FOBJDESC: u32 = 0x08;
    pub const OBJ_ID: u32 = 0x0C;
}
const _: () = assert!(aobj_off::OBJ_ID + 4 == AOBJ_DESC_SIZE);

mod fobj_off {
    pub const NEXT: u32 = 0x00;
    pub const LENGTH: u32 = 0x04;
    pub const STARTFRAME: u32 = 0x08;
    pub const TYPE: u32 = 0x0C;
    pub const FRAC_VALUE: u32 = 0x0D;
    pub const FRAC_SLOPE: u32 = 0x0E;
    pub const DUMMY0: u32 = 0x0F;
    pub const AD: u32 = 0x10;
}
const _: () = assert!(fobj_off::DUMMY0 + 1 == fobj_off::AD);
const _: () = assert!(fobj_off::AD + 4 == FOBJ_DESC_SIZE);

mod aj_off {
    pub const CHILD: u32 = 0x00;
    pub const NEXT: u32 = 0x04;
    pub const AOBJDESC: u32 = 0x08;
    pub const ROBJ_ANIM: u32 = 0x0C;
    pub const FLAGS: u32 = 0x10;
}
const _: () = assert!(aj_off::FLAGS + 4 == ANIM_JOINT_SIZE);

mod tree3_off {
    pub const CHILD: u32 = 0x00;
    pub const NEXT: u32 = 0x04;
    pub const PAYLOAD: u32 = 0x08;
}
const _: () = assert!(tree3_off::PAYLOAD + 4 == MAT_ANIM_JOINT_SIZE);
const _: () = assert!(tree3_off::PAYLOAD + 4 == SHAPE_ANIM_JOINT_SIZE);

/// `HSD_FObjDesc` (fobj.h): one animation track.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FObjDesc {
    /// Data offset this struct was read from.
    pub offset: u32,
    pub next: Option<Box<FObjDesc>>,
    /// Byte length of the key-frame stream.
    pub length: u32,
    pub startframe: f32,
    /// Track id: `HSD_A_J_*` for joints, `HSD_A_M_*` for materials, and so
    /// on, depending on the object the `AObjDesc` animates.
    pub type_: u8,
    /// Value fraction/format bits (`HSD_A_FRAC_*`).
    pub frac_value: u8,
    /// Slope fraction/format bits (`HSD_A_FRAC_*`).
    pub frac_slope: u8,
    /// The `dummy0` padding byte, kept so a round trip is exact.
    pub dummy0: u8,
    /// Data offset of the key-frame stream, if any.
    pub ad_offset: Option<u32>,
    /// The `length` bytes of key-frame data at `ad_offset`. Empty when
    /// `ad` is null.
    pub ad: Vec<u8>,
}

impl FObjDesc {
    /// Read the `HSD_FObjDesc` chain headed at data offset `off`.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let mut ctx = Ctx::new(archive);
        let head = read_chain::<Self>(&mut ctx, Some(off))?;
        // `read_chain` returns `Some` whenever it was handed `Some`.
        Ok(*head.unwrap_or_default())
    }

    /// This track and every `next` after it, in chain order.
    pub fn siblings(&self) -> Siblings<'_, FObjDesc> {
        Siblings::new(Some(self), |f| f.next.as_deref())
    }
}

impl ChainNode for FObjDesc {
    const WHAT: &'static str = "HSD_FObjDesc";
    const NEXT: u32 = fobj_off::NEXT;

    fn read_node(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        let length = r.u32(add_offset(off, fobj_off::LENGTH)?)?;
        let ad_offset = ctx.link("HSD_FObjDesc.ad", off, fobj_off::AD)?;
        let ad = match ad_offset {
            Some(a) => ctx.stream("HSD_FObjDesc.ad", a, length)?,
            None => Vec::new(),
        };
        Ok(Self {
            offset: off,
            next: None,
            length,
            startframe: r.f32(add_offset(off, fobj_off::STARTFRAME)?)?,
            type_: r.u8(add_offset(off, fobj_off::TYPE)?)?,
            frac_value: r.u8(add_offset(off, fobj_off::FRAC_VALUE)?)?,
            frac_slope: r.u8(add_offset(off, fobj_off::FRAC_SLOPE)?)?,
            dummy0: r.u8(add_offset(off, fobj_off::DUMMY0)?)?,
            ad_offset,
            ad,
        })
    }

    fn set_next(&mut self, next: Option<Box<Self>>) {
        self.next = next;
    }
}

impl Drop for FObjDesc {
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut node) = next {
            next = node.next.take();
        }
    }
}

/// `HSD_AObjDesc` (aobj.h): an animation object with its track list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AObjDesc {
    /// Data offset this struct was read from.
    pub offset: u32,
    /// `AOBJ_*` bits (`AOBJ_LOOP`, `AOBJ_NO_ANIM`, ...).
    pub flags: u32,
    pub end_frame: f32,
    /// Head of the track chain.
    pub fobj: Option<Box<FObjDesc>>,
    /// Raw `obj_id`: `0`, or the data offset of the `HSD_Joint` the
    /// animation targets when [`Self::obj_id_is_link`].
    pub obj_id: u32,
    /// `true` if the `obj_id` slot is in the relocation table.
    pub obj_id_is_link: bool,
}

impl AObjDesc {
    /// Read the `HSD_AObjDesc` at data offset `off`.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let mut ctx = Ctx::new(archive);
        ctx.enter("HSD_AObjDesc", off)?;
        let out = Self::read_in(&mut ctx, off);
        ctx.leave(off);
        out
    }

    pub(crate) fn read_in(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        let fobj_head = ctx.link("HSD_AObjDesc.fobjdesc", off, aobj_off::FOBJDESC)?;
        let fobj = read_chain::<FObjDesc>(ctx, fobj_head)?;
        let obj_id_at = add_offset(off, aobj_off::OBJ_ID)?;
        Ok(Self {
            offset: off,
            flags: r.u32(add_offset(off, aobj_off::FLAGS)?)?,
            end_frame: r.f32(add_offset(off, aobj_off::END_FRAME)?)?,
            fobj,
            obj_id: r.u32(obj_id_at)?,
            obj_id_is_link: ctx.archive.is_relocated_offset(obj_id_at),
        })
    }

    /// Tracks in chain order.
    pub fn tracks(&self) -> Siblings<'_, FObjDesc> {
        Siblings::new(self.fobj.as_deref(), |f| f.next.as_deref())
    }
}

/// `HSD_AnimJoint` (aobj.h): the joint-animation tree that parallels an
/// `HSD_Joint` tree node for node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimJoint {
    /// Data offset this struct was read from.
    pub offset: u32,
    pub child: Option<Box<AnimJoint>>,
    pub next: Option<Box<AnimJoint>>,
    pub aobjdesc: Option<Box<AObjDesc>>,
    /// Raw data offset of the `HSD_RObjAnimJoint` chain, if any.
    pub robj_anim: Option<u32>,
    pub flags: u32,
}

impl AnimJoint {
    /// Read the `HSD_AnimJoint` tree rooted at data offset `off`, siblings
    /// of the root included.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let mut ctx = Ctx::new(archive);
        let head = read_chain::<Self>(&mut ctx, Some(off))?;
        // `read_chain` returns `Some` whenever it was handed `Some`.
        Ok(*head.unwrap_or_default())
    }

    /// Direct children in `child`/`next` order.
    pub fn children(&self) -> Siblings<'_, AnimJoint> {
        Siblings::new(self.child.as_deref(), |a| a.next.as_deref())
    }

    /// This node and every `next` sibling after it, in order.
    pub fn siblings(&self) -> Siblings<'_, AnimJoint> {
        Siblings::new(Some(self), |a| a.next.as_deref())
    }
}

impl ChainNode for AnimJoint {
    const WHAT: &'static str = "HSD_AnimJoint";
    const NEXT: u32 = aj_off::NEXT;

    fn read_node(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        let child = match ctx.link("HSD_AnimJoint.child", off, aj_off::CHILD)? {
            Some(c) => {
                ctx.descend("HSD_AnimJoint", c)?;
                let out = read_chain::<Self>(ctx, Some(c));
                ctx.ascend();
                out?
            }
            None => None,
        };
        let aobjdesc = match ctx.link("HSD_AnimJoint.aobjdesc", off, aj_off::AOBJDESC)? {
            Some(a) => {
                ctx.enter("HSD_AObjDesc", a)?;
                let out = AObjDesc::read_in(ctx, a);
                ctx.leave(a);
                Some(Box::new(out?))
            }
            None => None,
        };
        Ok(Self {
            offset: off,
            child,
            next: None,
            aobjdesc,
            robj_anim: ctx.link("HSD_AnimJoint.robj_anim", off, aj_off::ROBJ_ANIM)?,
            flags: r.u32(add_offset(off, aj_off::FLAGS)?)?,
        })
    }

    fn set_next(&mut self, next: Option<Box<Self>>) {
        self.next = next;
    }
}

impl Drop for AnimJoint {
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut node) = next {
            next = node.next.take();
        }
    }
}

/// `HSD_MatAnimJoint` (mobj.h): material-animation tree paralleling the
/// joint tree. The `matanim` payload is a raw offset (see module docs).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MatAnimJoint {
    /// Data offset this struct was read from.
    pub offset: u32,
    pub child: Option<Box<MatAnimJoint>>,
    pub next: Option<Box<MatAnimJoint>>,
    /// Raw data offset of the `HSD_MatAnim` chain (one per `DObj`), if any.
    pub matanim: Option<u32>,
}

/// `HSD_ShapeAnimJoint` (pobj.h): shape-animation tree paralleling the
/// joint tree. The `shapeanimdobj` payload is a raw offset (see module
/// docs).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapeAnimJoint {
    /// Data offset this struct was read from.
    pub offset: u32,
    pub child: Option<Box<ShapeAnimJoint>>,
    pub next: Option<Box<ShapeAnimJoint>>,
    /// Raw data offset of the `HSD_ShapeAnimDObj` chain, if any.
    pub shapeanimdobj: Option<u32>,
}

macro_rules! tree3 {
    ($ty:ident, $what:literal, $payload:ident, $payload_name:literal) => {
        impl $ty {
            #[doc = concat!("Read the `", $what, "` tree rooted at data offset `off`, siblings of the root included.")]
            pub fn read(archive: &Archive, off: u32) -> Result<Self> {
                let mut ctx = Ctx::new(archive);
                let head = read_chain::<Self>(&mut ctx, Some(off))?;
                // `read_chain` returns `Some` whenever it was handed `Some`.
                Ok(*head.unwrap_or_default())
            }

            /// Direct children in `child`/`next` order.
            pub fn children(&self) -> Siblings<'_, $ty> {
                Siblings::new(self.child.as_deref(), |a| a.next.as_deref())
            }

            /// This node and every `next` sibling after it, in order.
            pub fn siblings(&self) -> Siblings<'_, $ty> {
                Siblings::new(Some(self), |a| a.next.as_deref())
            }
        }

        impl ChainNode for $ty {
            const WHAT: &'static str = $what;
            const NEXT: u32 = tree3_off::NEXT;

            fn read_node(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
                let child = match ctx.link(concat!($what, ".child"), off, tree3_off::CHILD)? {
                    Some(c) => {
                        ctx.descend($what, c)?;
                        let out = read_chain::<Self>(ctx, Some(c));
                        ctx.ascend();
                        out?
                    }
                    None => None,
                };
                Ok(Self {
                    offset: off,
                    child,
                    next: None,
                    $payload: ctx.link($payload_name, off, tree3_off::PAYLOAD)?,
                })
            }

            fn set_next(&mut self, next: Option<Box<Self>>) {
                self.next = next;
            }
        }

        impl Drop for $ty {
            fn drop(&mut self) {
                let mut next = self.next.take();
                while let Some(mut node) = next {
                    next = node.next.take();
                }
            }
        }
    };
}

tree3!(
    MatAnimJoint,
    "HSD_MatAnimJoint",
    matanim,
    "HSD_MatAnimJoint.matanim"
);
tree3!(
    ShapeAnimJoint,
    "HSD_ShapeAnimJoint",
    shapeanimdobj,
    "HSD_ShapeAnimJoint.shapeanimdobj"
);

impl ShapeAnimJoint {
    /// Empty parallel shape trees are common; only a linked AObj deforms vertices.
    pub fn has_animation(&self, archive: &Archive) -> Result<bool> {
        for node in self.siblings() {
            let mut dobj = node.shapeanimdobj;
            let mut seen = std::collections::BTreeSet::new();
            while let Some(offset) = dobj {
                if !seen.insert(offset) {
                    return Err(super::DescError::Cycle {
                        what: "HSD_ShapeAnimDObj",
                        offset,
                    });
                }
                archive.reader().slice(offset, 8)?;
                let mut shape = archive.link(offset + 4)?;
                let mut shapes = std::collections::BTreeSet::new();
                while let Some(offset) = shape {
                    if !shapes.insert(offset) {
                        return Err(super::DescError::Cycle {
                            what: "HSD_ShapeAnim",
                            offset,
                        });
                    }
                    archive.reader().slice(offset, 8)?;
                    if archive.link(offset + 4)?.is_some() {
                        return Ok(true);
                    }
                    shape = archive.link(offset)?;
                }
                dobj = archive.link(offset)?;
            }
            if let Some(child) = &node.child {
                if child.has_animation(archive)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
