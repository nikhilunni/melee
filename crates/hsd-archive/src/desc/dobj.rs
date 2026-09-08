//! `HSD_DObjDesc` (dobj.h), `HSD_MObjDesc` and `HSD_Material` (mobj.h).
//!
//! ```text
//! HSD_DObjDesc, 0x10 bytes                 HSD_MObjDesc, 0x18 bytes
//!   +0x00 char*         class_name           +0x00 char*          class_name
//!   +0x04 HSD_DObjDesc* next                 +0x04 u32            rendermode
//!   +0x08 HSD_MObjDesc* mobjdesc             +0x08 HSD_TObjDesc*  texdesc     (raw)
//!   +0x0C HSD_PObjDesc* pobjdesc  (raw)      +0x0C HSD_Material*  mat
//!                                            +0x10 void*          renderdesc  (raw)
//! HSD_Material, 0x14 bytes                   +0x14 HSD_PEDesc*    pedesc      (raw)
//!   +0x00 GXColor ambient
//!   +0x04 GXColor diffuse
//!   +0x08 GXColor specular
//!   +0x0C f32     alpha
//!   +0x10 f32     shininess
//! ```
//!
//! `pobjdesc` is polygon/vertex data and `texdesc` is texture data; both
//! are render-side and kept as raw offsets. `renderdesc` is unused by
//! retail's `MObjLoad` (only `rendermode` is copied). `pedesc` is the
//! 12-byte pixel-engine blend descriptor, also render-side.

use super::{read_chain, ChainNode, Ctx, GxColor, Result, Siblings};
use crate::archive::Archive;
use crate::reader::add_offset;

/// `sizeof(HSD_DObjDesc)`.
pub const DOBJ_DESC_SIZE: u32 = 0x10;
/// `sizeof(HSD_MObjDesc)`.
pub const MOBJ_DESC_SIZE: u32 = 0x18;
/// `sizeof(HSD_Material)`.
pub const MATERIAL_SIZE: u32 = 0x14;

mod dobj_off {
    pub const CLASS_NAME: u32 = 0x00;
    pub const NEXT: u32 = 0x04;
    pub const MOBJDESC: u32 = 0x08;
    pub const POBJDESC: u32 = 0x0C;
}
const _: () = assert!(dobj_off::POBJDESC + 4 == DOBJ_DESC_SIZE);

mod mobj_off {
    pub const CLASS_NAME: u32 = 0x00;
    pub const RENDERMODE: u32 = 0x04;
    pub const TEXDESC: u32 = 0x08;
    pub const MAT: u32 = 0x0C;
    pub const RENDERDESC: u32 = 0x10;
    pub const PEDESC: u32 = 0x14;
}
const _: () = assert!(mobj_off::PEDESC + 4 == MOBJ_DESC_SIZE);

mod mat_off {
    pub const AMBIENT: u32 = 0x00;
    pub const DIFFUSE: u32 = 0x04;
    pub const SPECULAR: u32 = 0x08;
    pub const ALPHA: u32 = 0x0C;
    pub const SHININESS: u32 = 0x10;
}
const _: () = assert!(mat_off::SHININESS + 4 == MATERIAL_SIZE);

/// `HSD_Material` (mobj.h).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Material {
    pub ambient: GxColor,
    pub diffuse: GxColor,
    pub specular: GxColor,
    pub alpha: f32,
    pub shininess: f32,
}

impl Material {
    /// Read the `HSD_Material` at data offset `off`.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        Self::read_in(&Ctx::new(archive), off)
    }

    pub(crate) fn read_in(ctx: &Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        Ok(Self {
            ambient: ctx.color(add_offset(off, mat_off::AMBIENT)?)?,
            diffuse: ctx.color(add_offset(off, mat_off::DIFFUSE)?)?,
            specular: ctx.color(add_offset(off, mat_off::SPECULAR)?)?,
            alpha: r.f32(add_offset(off, mat_off::ALPHA)?)?,
            shininess: r.f32(add_offset(off, mat_off::SHININESS)?)?,
        })
    }
}

/// `HSD_MObjDesc` (mobj.h): a material descriptor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MObjDesc {
    /// Data offset this struct was read from.
    pub offset: u32,
    pub class_name: Option<String>,
    /// `RENDER_*` bits (mobj.h).
    pub rendermode: u32,
    /// Raw data offset of the `HSD_TObjDesc` chain, if any.
    pub texdesc: Option<u32>,
    pub mat: Option<Material>,
    /// Raw data offset of `renderdesc`, if any. Retail never reads it.
    pub renderdesc: Option<u32>,
    /// Raw data offset of the `HSD_PEDesc`, if any.
    pub pedesc: Option<u32>,
}

impl MObjDesc {
    /// Read the `HSD_MObjDesc` at data offset `off`.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let mut ctx = Ctx::new(archive);
        ctx.enter("HSD_MObjDesc", off)?;
        let out = Self::read_in(&mut ctx, off);
        ctx.leave(off);
        out
    }

    pub(crate) fn read_in(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        let mat = match ctx.link("HSD_MObjDesc.mat", off, mobj_off::MAT)? {
            Some(m) => Some(Material::read_in(ctx, m)?),
            None => None,
        };
        Ok(Self {
            offset: off,
            class_name: ctx.class_name("HSD_MObjDesc.class_name", off, mobj_off::CLASS_NAME)?,
            rendermode: r.u32(add_offset(off, mobj_off::RENDERMODE)?)?,
            texdesc: ctx.link("HSD_MObjDesc.texdesc", off, mobj_off::TEXDESC)?,
            mat,
            renderdesc: ctx.link("HSD_MObjDesc.renderdesc", off, mobj_off::RENDERDESC)?,
            pedesc: ctx.link("HSD_MObjDesc.pedesc", off, mobj_off::PEDESC)?,
        })
    }
}

/// `HSD_DObjDesc` (dobj.h): one display object in a joint's `next` chain.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DObjDesc {
    /// Data offset this struct was read from.
    pub offset: u32,
    pub class_name: Option<String>,
    /// The following display object on the same joint.
    pub next: Option<Box<DObjDesc>>,
    pub mobj: Option<Box<MObjDesc>>,
    /// Raw data offset of the `HSD_PObjDesc` chain (render data), if any.
    pub pobjdesc: Option<u32>,
}

impl DObjDesc {
    /// Read the `HSD_DObjDesc` chain headed at data offset `off`.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let mut ctx = Ctx::new(archive);
        Self::read_head(&mut ctx, off)
    }

    /// Read a chain whose head is known to be present.
    pub(crate) fn read_head(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let head = read_chain::<Self>(ctx, Some(off))?;
        // `read_chain` returns `Some` whenever it was handed `Some`.
        Ok(*head.unwrap_or_default())
    }

    /// This node and every `next` after it, in chain order.
    pub fn siblings(&self) -> Siblings<'_, DObjDesc> {
        Siblings::new(Some(self), |d| d.next.as_deref())
    }
}

impl ChainNode for DObjDesc {
    const WHAT: &'static str = "HSD_DObjDesc";
    const NEXT: u32 = dobj_off::NEXT;

    fn read_node(ctx: &mut Ctx<'_>, off: u32) -> Result<Self> {
        let mobj = match ctx.link("HSD_DObjDesc.mobjdesc", off, dobj_off::MOBJDESC)? {
            Some(m) => {
                ctx.enter("HSD_MObjDesc", m)?;
                let out = MObjDesc::read_in(ctx, m);
                ctx.leave(m);
                Some(Box::new(out?))
            }
            None => None,
        };
        Ok(Self {
            offset: off,
            class_name: ctx.class_name("HSD_DObjDesc.class_name", off, dobj_off::CLASS_NAME)?,
            next: None,
            mobj,
            pobjdesc: ctx.link("HSD_DObjDesc.pobjdesc", off, dobj_off::POBJDESC)?,
        })
    }

    fn set_next(&mut self, next: Option<Box<Self>>) {
        self.next = next;
    }
}

impl Drop for DObjDesc {
    /// Unlink the `next` chain iteratively so a long chain does not
    /// recurse once per node.
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut node) = next {
            next = node.next.take();
        }
    }
}
