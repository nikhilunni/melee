//! Typed light/world-position descriptors (`lobj.h`, `wobj.h`).
use super::{Ctx, Result, Vec3};
use crate::Archive;

#[derive(Clone, Debug, PartialEq)]
pub struct WorldPositionDesc {
    pub class_name: Option<String>,
    pub position: Vec3,
    pub constraints_offset: Option<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LightDesc {
    pub class_name: Option<String>,
    pub next_offset: Option<u32>,
    pub flags: u16,
    pub attenuation_flags: u16,
    pub color: [u8; 4],
    pub position: Option<WorldPositionDesc>,
    pub interest: Option<WorldPositionDesc>,
    /// Directional lights store a single shininess float; other kinds retain
    /// the attenuation descriptor offset for their owning runtime reader.
    pub shininess: Option<f32>,
    pub attenuation_offset: Option<u32>,
    pub point_attenuation: Option<PointAttenuation>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointAttenuation {
    pub reference_brightness: f32,
    pub reference_distance: f32,
    pub distance_function: u32,
}
impl LightDesc {
    /// `HSD_LObjLoadDesc` (lobj.c), retail 0x803672DC; `LObjLoad`, 0x80366EA8.
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let ctx = Ctx::new(archive);
        let r = ctx.reader();
        r.slice(offset, 0x1C)?;
        let flags = r.u16(offset + 8)?;
        let position = |slot| -> Result<_> {
            ctx.link("HSD_LightDesc.wobj", offset, slot)?
                .map(|p| {
                    r.slice(p, 0x14)?;
                    Ok(WorldPositionDesc {
                        class_name: ctx.class_name("HSD_WObjDesc.class_name", p, 0)?,
                        position: ctx.vec3(p + 4)?,
                        constraints_offset: ctx.link("HSD_WObjDesc.robj", p, 0x10)?,
                    })
                })
                .transpose()
        };
        let attenuation = ctx.link("HSD_LightDesc.u", offset, 0x18)?;
        Ok(Self {
            class_name: ctx.class_name("HSD_LightDesc.class_name", offset, 0)?,
            next_offset: ctx.link("HSD_LightDesc.next", offset, 4)?,
            flags,
            attenuation_flags: r.u16(offset + 0xA)?,
            color: r.array(offset + 0xC)?,
            position: position(0x10)?,
            interest: position(0x14)?,
            shininess: if flags & 3 == 1 {
                attenuation.map(|p| r.f32(p)).transpose()?
            } else {
                None
            },
            attenuation_offset: attenuation,
            point_attenuation: if flags & 3 == 2 && r.u16(offset + 0xA)? == 0 {
                attenuation
                    .map(|p| -> Result<_> {
                        Ok(PointAttenuation {
                            reference_brightness: r.f32(p)?,
                            reference_distance: r.f32(p + 4)?,
                            distance_function: r.u32(p + 8)?,
                        })
                    })
                    .transpose()?
            } else {
                None
            },
        })
    }
}
