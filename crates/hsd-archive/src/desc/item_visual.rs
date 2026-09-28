//! Article model and item-state visual descriptors from it/types.h.
use super::{AnimJoint, JObjDesc, MatAnimJoint, Result};
use crate::Archive;
#[derive(Clone, Debug, Default)]
pub struct ItemVisual {
    pub model: JObjDesc,
    pub attachment_bone: usize,
    pub states: Vec<ItemVisualState>,
}
#[derive(Clone, Debug)]
pub struct ItemVisualState {
    pub joint: Option<AnimJoint>,
    pub material: Option<MatAnimJoint>,
    pub shape: Option<super::ShapeAnimJoint>,
}
impl ItemVisual {
    pub fn read(archive: &Archive, model: u32, states: u32, count: usize) -> Result<Self> {
        let reader = archive.reader();
        reader.slice(model, 16)?;
        // Item_80267978 (item.c:738-745): an article without a model gets a
        // bare identity JObj (Toad's spores).
        let joint = archive.link(model)?;
        if count > super::MAX_NODES {
            return Err(super::DescError::TooManyNodes {
                what: "ItemStateDesc",
                offset: states,
            });
        }
        reader.slice(states, count as u32 * 16)?;
        let mut animations = Vec::with_capacity(count);
        for index in 0..count {
            let offset = crate::add_offset(states, index as u32 * 16)?;
            animations.push(ItemVisualState {
                joint: archive
                    .link(offset)?
                    .map(|p| AnimJoint::read(archive, p))
                    .transpose()?,
                material: archive
                    .link(offset + 4)?
                    .map(|p| MatAnimJoint::read(archive, p))
                    .transpose()?,
                shape: archive
                    .link(offset + 8)?
                    .map(|p| super::ShapeAnimJoint::read(archive, p))
                    .transpose()?,
            });
        }
        Ok(Self {
            model: match joint {
                Some(joint) => JObjDesc::read(archive, joint)?,
                None => JObjDesc::default(),
            },
            attachment_bone: reader.u32(model + 8)? as usize,
            states: animations,
        })
    }
}
