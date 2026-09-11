//! Melee LightList tables and the first HSD light animation. Binary pointers
//! stay here; consumers receive typed descriptors and encoded animation tracks.
use super::{invalid, Result};
use crate::{
    desc::{light::LightDesc, AObjDesc},
    Archive,
};

pub struct Light {
    pub descriptor: LightDesc,
    pub has_animation_set: bool,
    pub color_animation: Option<AObjDesc>,
    pub position_animation: Option<AObjDesc>,
    pub interest_animation: Option<AObjDesc>,
}

pub fn read_lights(archive: &Archive, offset: u32) -> Result<Vec<Light>> {
    let mut result = Vec::new();
    let mut cursor = offset;
    while let Some(entry) = archive.link(cursor)? {
        if result.len() == 8 {
            return Err(invalid(cursor, "light set exceeds eight slots"));
        }
        let descriptor = archive
            .link(entry)?
            .ok_or_else(|| invalid(entry, "missing light descriptor"))?;
        let descriptor = LightDesc::read(archive, descriptor)
            .map_err(|_| invalid(entry, "invalid light descriptor"))?;
        if descriptor.next_offset.is_some() {
            return Err(invalid(entry, "chained light descriptors unsupported"));
        }
        let animation_set = archive.link(crate::add_offset(entry, 4)?)?;
        let animation = animation_set
            .map(|table| archive.link(table))
            .transpose()?
            .flatten();
        let mut light = Light {
            descriptor,
            has_animation_set: animation_set.is_some(),
            color_animation: None,
            position_animation: None,
            interest_animation: None,
        };
        if let Some(animation) = animation {
            if archive.link(animation)?.is_some() {
                return Err(invalid(animation, "chained light animations unsupported"));
            }
            light.color_animation = read_aobj(archive, crate::add_offset(animation, 4)?)?;
            light.position_animation =
                read_world_animation(archive, crate::add_offset(animation, 8)?)?;
            light.interest_animation =
                read_world_animation(archive, crate::add_offset(animation, 12)?)?;
        }
        result.push(light);
        cursor = crate::add_offset(cursor, 4)?;
    }
    Ok(result)
}
fn read_aobj(archive: &Archive, slot: u32) -> Result<Option<AObjDesc>> {
    archive
        .link(slot)?
        .map(|offset| {
            AObjDesc::read(archive, offset).map_err(|_| invalid(offset, "invalid light animation"))
        })
        .transpose()
}
fn read_world_animation(archive: &Archive, slot: u32) -> Result<Option<AObjDesc>> {
    let Some(offset) = archive.link(slot)? else {
        return Ok(None);
    };
    if archive.link(crate::add_offset(offset, 4)?)?.is_some() {
        return Err(invalid(offset, "light animation constraints unsupported"));
    }
    read_aobj(archive, offset)
}
