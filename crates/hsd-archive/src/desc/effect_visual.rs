//! EF_EffectDesc (ef/types.h): lifetime and authored model/animation trees.
use super::{AnimJoint, JObjDesc, MatAnimJoint, Result, ShapeAnimJoint};
use crate::Archive;
#[derive(Clone, Debug)]
pub struct EffectVisual {
    pub lifetime: f32,
    pub model: JObjDesc,
    pub animation: Option<AnimJoint>,
    pub material: Option<MatAnimJoint>,
    pub shape: Option<ShapeAnimJoint>,
}
impl EffectVisual {
    pub fn read(archive: &Archive, table: u32, index: u32) -> Result<Self> {
        if index as usize >= super::MAX_NODES {
            return Err(super::DescError::TooManyNodes {
                what: "EF_EffectDesc",
                offset: table,
            });
        }
        let offset = crate::add_offset(table, 8 + index * 20)?;
        archive.reader().slice(offset, 20)?;
        let model = archive
            .link(offset + 4)?
            .ok_or(super::DescError::NullPointer {
                field: "EF_EffectDesc.model",
                at: offset + 4,
            })?;
        Ok(Self {
            lifetime: archive.reader().f32(offset)?,
            model: JObjDesc::read(archive, model)?,
            animation: archive
                .link(offset + 8)?
                .map(|p| AnimJoint::read(archive, p))
                .transpose()?,
            material: archive
                .link(offset + 12)?
                .map(|p| MatAnimJoint::read(archive, p))
                .transpose()?,
            shape: archive
                .link(offset + 16)?
                .map(|p| ShapeAnimJoint::read(archive, p))
                .transpose()?,
        })
    }
}
