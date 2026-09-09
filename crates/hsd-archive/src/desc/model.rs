//! Melee's sc/types.h DynamicModelDesc pointer table, kept at the archive edge.
use super::{AnimJoint, DescError, JObjDesc, Result};
use crate::{reader::add_offset, Archive};

/// Read one joint tree and its selected joint animation from a public
/// DynamicModelDesc** table. Material/shape tracks do not control the
/// countdown completion predicate lb_8000B09C, which tests joint AObjs only.
pub fn read_dynamic_model_animation(
    archive: &Archive,
    symbol: &str,
    model: u32,
    animation: u32,
) -> Result<(JObjDesc, AnimJoint)> {
    let table = archive
        .public(symbol)
        .ok_or_else(|| DescError::MissingSymbol {
            name: symbol.into(),
        })?;
    let element = |base, index: u32| -> Result<u32> {
        let bytes = index.checked_mul(4).ok_or(crate::Error::OffsetOverflow {
            offset: index,
            add: index,
        })?;
        Ok(add_offset(base, bytes)?)
    };
    let link = |offset, field| -> Result<u32> {
        archive
            .link(offset)?
            .ok_or(DescError::NullPointer { field, at: offset })
    };
    let model = link(element(table, model)?, "DynamicModelDesc")?;
    let joint = link(model, "DynamicModelDesc.joint")?;
    let animations = link(add_offset(model, 4)?, "DynamicModelDesc.anims")?;
    let animation = link(
        element(animations, animation)?,
        "DynamicModelDesc.animation",
    )?;
    Ok((
        JObjDesc::read(archive, joint)?,
        AnimJoint::read(archive, animation)?,
    ))
}
