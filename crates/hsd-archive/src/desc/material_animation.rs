//! HSD_MatAnim and HSD_TexAnim payloads. Image and palette tables retain
//! archive offsets; decoding their immutable pixels is a presentation choice.
use super::{AObjDesc, Ctx, DescError, Result, MAX_NODES};
use crate::{add_offset, Archive};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct TextureAnimation {
    pub id: u32,
    pub animation: Option<AObjDesc>,
    pub images: Vec<Option<u32>>,
    pub palettes: Vec<Option<u32>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialAnimation {
    pub animation: Option<AObjDesc>,
    pub textures: Vec<TextureAnimation>,
    pub render_animation: Option<u32>,
}
impl MaterialAnimation {
    pub fn read_chain(archive: &Archive, head: Option<u32>) -> Result<Vec<Self>> {
        read_chain(archive, head, "HSD_MatAnim", |ctx, offset| {
            ctx.reader().slice(offset, 16)?;
            Ok(Self {
                animation: animation(ctx, offset, 4)?,
                textures: TextureAnimation::read_chain(
                    archive,
                    ctx.link("MatAnim.texanim", offset, 8)?,
                )?,
                render_animation: ctx.link("MatAnim.renderanim", offset, 12)?,
            })
        })
    }
}
impl TextureAnimation {
    pub fn read_chain(archive: &Archive, head: Option<u32>) -> Result<Vec<Self>> {
        read_chain(archive, head, "HSD_TexAnim", |ctx, offset| {
            let r = ctx.reader();
            r.slice(offset, 24)?;
            Ok(Self {
                id: r.u32(add_offset(offset, 4)?)?,
                animation: animation(ctx, offset, 8)?,
                images: table(ctx, offset, 12, r.u16(add_offset(offset, 20)?)?)?,
                palettes: table(ctx, offset, 16, r.u16(add_offset(offset, 22)?)?)?,
            })
        })
    }
}
fn animation(ctx: &Ctx<'_>, base: u32, slot: u32) -> Result<Option<AObjDesc>> {
    ctx.link("material animation.aobj", base, slot)?
        .map(|p| AObjDesc::read(ctx.archive, p))
        .transpose()
}
fn table(ctx: &Ctx<'_>, base: u32, slot: u32, count: u16) -> Result<Vec<Option<u32>>> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let offset =
        ctx.link("texture animation.table", base, slot)?
            .ok_or(DescError::NullPointer {
                field: "texture animation.table",
                at: add_offset(base, slot)?,
            })?;
    ctx.reader().slice(offset, u32::from(count) * 4)?;
    (0..u32::from(count))
        .map(|i| ctx.link("texture animation.table entry", offset, i * 4))
        .collect()
}
fn read_chain<T>(
    archive: &Archive,
    mut next: Option<u32>,
    what: &'static str,
    read: impl Fn(&Ctx<'_>, u32) -> Result<T>,
) -> Result<Vec<T>> {
    let ctx = Ctx::new(archive);
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    while let Some(offset) = next {
        if !seen.insert(offset) {
            return Err(DescError::Cycle { what, offset });
        }
        if seen.len() > MAX_NODES {
            return Err(DescError::TooManyNodes { what, offset });
        }
        result.push(read(&ctx, offset)?);
        next = ctx.link(what, offset, 0)?;
    }
    Ok(result)
}
