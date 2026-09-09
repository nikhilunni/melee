//! Melee-owned archive layouts from gr/types.h. All pointer arithmetic stays here.
use hsd_archive::desc::{AnimJoint, JObjDesc};
use hsd_archive::{Archive, Reader};
use melee_mp::{desc::read_public_coll_data, CollMap};
use melee_types::GrKind;

pub type ReadResult<T> = Result<T, Box<dyn std::error::Error>>;
const MODEL_SIZE: u32 = 0x34;
const JOINT_MAPPING_SIZE: u32 = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointMapping {
    pub joint_index: i16,
    pub target_index: i16,
    /// Meaning depends on the stage; retained without reinterpretation.
    pub extra: i16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionBinding {
    pub model_id: usize,
    pub joint_index: i16,
    pub stage_position: i16,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GroundParam {
    pub map_scale: f32,
    pub fixed_camera: bool,
    pub environment_colors: [[u8; 4]; 9],
    pub stage_param_count: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDesc {
    pub joint: JObjDesc,
    pub animations: Vec<AnimJoint>,
    pub animation_loops: Vec<bool>,
    pub joint_mappings: Vec<JointMapping>,
    /// Mat/shape descriptors remain archive offsets until their interpreters exist.
    pub material_animation_offsets: Vec<u32>,
    pub shape_animation_offsets: Vec<u32>,
    pub light_list_offset: Option<u32>,
    pub fog_offset: Option<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StageDesc {
    pub parameters: GroundParam,
    pub models: Vec<ModelDesc>,
    pub position_bindings: Vec<PositionBinding>,
    pub section_counts: [usize; 6],
    pub initial_fog: [u8; 3],
    pub material_script_offsets: [u32; 4],
}

fn error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into()).into()
}
fn public(archive: &Archive, name: &str) -> ReadResult<u32> {
    archive
        .public(name)
        .ok_or_else(|| error(format!("missing public {name}")))
}
fn link(archive: &Archive, slot: u32) -> ReadResult<Option<u32>> {
    let result = archive.link(slot)?;
    if result.is_none() && archive.reader().u32(slot)? != 0 {
        return Err(error(format!("unrelocated pointer at {slot:#x}")));
    }
    Ok(result)
}
fn required_link(archive: &Archive, slot: u32) -> ReadResult<u32> {
    link(archive, slot)?.ok_or_else(|| error(format!("null pointer at {slot:#x}")))
}
fn count(reader: &Reader<'_>, slot: u32) -> ReadResult<usize> {
    let n = reader.s32(slot)?;
    usize::try_from(n).map_err(|_| error(format!("negative count {n} at {slot:#x}")))
}
fn array(archive: &Archive, slot: u32, n: usize, stride: u32) -> ReadResult<u32> {
    if n == 0 {
        return Ok(0);
    }
    let base = required_link(archive, slot)?;
    let bytes = u32::try_from(n)?
        .checked_mul(stride)
        .ok_or_else(|| error("array size overflow"))?;
    archive.reader().slice(base, bytes)?;
    Ok(base)
}
fn pointer_list(archive: &Archive, slot: u32) -> ReadResult<Vec<u32>> {
    let Some(mut cursor) = link(archive, slot)? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    // Finite bound also rejects a table missing its terminator.
    for _ in 0..archive.data().len() / 4 {
        match link(archive, cursor)? {
            None => return Ok(result),
            Some(target) => result.push(target),
        }
        cursor = cursor
            .checked_add(4)
            .ok_or_else(|| error("pointer list overflow"))?;
    }
    Err(error("unterminated pointer list"))
}

/// `grDatFiles_801C6038` (grdatfiles.c), retail 0x801C6038: resolve stage publics.
pub fn read_final_destination(archive: &Archive) -> ReadResult<StageDesc> {
    let header = public(archive, "map_head")?;
    let reader = archive.reader();
    reader.slice(header, 0x30)?;
    let mut section_counts = [0; 6];
    for (i, n) in section_counts.iter_mut().enumerate() {
        *n = count(&reader, header + i as u32 * 8 + 4)?;
    }
    let base = array(archive, header + 8, section_counts[1], MODEL_SIZE)?;
    let models = (0..section_counts[1])
        .map(|i| read_model(archive, base + i as u32 * MODEL_SIZE))
        .collect::<ReadResult<Vec<_>>>()?;
    let bindings = read_position_bindings(archive, header, section_counts[0], &models)?;
    let parameters = read_parameters(archive)?;
    // Ground_801C1E00 selects the callback row with flags_b1: FD map 4.
    let fog = models
        .get(4)
        .and_then(|m| m.fog_offset)
        .ok_or_else(|| error("FD map 4 has no fog"))?;
    // HSD_FogDesc (fog.h): u32 type, fog-adjust pointer, start/end, GXColor at +0x10.
    let initial_fog = reader.array::<3>(fog + 0x10)?;
    let scripts = public(archive, "yakumono_param")?;
    let mut material_script_offsets = [0; 4];
    for (i, offset) in material_script_offsets.iter_mut().enumerate() {
        *offset = required_link(archive, scripts + i as u32 * 4)?;
    }
    Ok(StageDesc {
        parameters,
        models,
        position_bindings: bindings,
        section_counts,
        initial_fog,
        material_script_offsets,
    })
}

fn read_model(archive: &Archive, offset: u32) -> ReadResult<ModelDesc> {
    let reader = archive.reader();
    let joint = JObjDesc::read(archive, required_link(archive, offset)?)?;
    let animation_offsets = pointer_list(archive, offset + 4)?;
    let animations = animation_offsets
        .iter()
        .map(|&p| AnimJoint::read(archive, p))
        .collect::<Result<Vec<_>, _>>()?;
    let flags = link(archive, offset + 0x28)?;
    let animation_loops = (0..animations.len())
        .map(|i| match flags {
            Some(p) => Ok(reader.u8(p + i as u32)? != 0),
            None => Ok(false),
        })
        .collect::<ReadResult<Vec<_>>>()?;
    let n = count(&reader, offset + 0x24)?;
    let base = array(archive, offset + 0x20, n, JOINT_MAPPING_SIZE)?;
    let joint_mappings = (0..n)
        .map(|i| {
            let p = base + i as u32 * JOINT_MAPPING_SIZE;
            Ok(JointMapping {
                joint_index: reader.s16(p)?,
                target_index: reader.s16(p + 2)?,
                extra: reader.s16(p + 4)?,
            })
        })
        .collect::<ReadResult<Vec<_>>>()?;
    Ok(ModelDesc {
        joint,
        animations,
        animation_loops,
        joint_mappings,
        material_animation_offsets: pointer_list(archive, offset + 8)?,
        shape_animation_offsets: pointer_list(archive, offset + 12)?,
        light_list_offset: link(archive, offset + 0x18)?,
        fog_offset: link(archive, offset + 0x1c)?,
    })
}

/// `Ground_801C34AC`, retail 0x801C34AC: map_head's first table binds named
/// stage positions to depth-first joints. Its pair pointer can target offset zero.
fn read_position_bindings(
    archive: &Archive,
    header: u32,
    n: usize,
    models: &[ModelDesc],
) -> ReadResult<Vec<PositionBinding>> {
    let r = archive.reader();
    let base = array(archive, header, n, 12)?;
    let mut result = Vec::new();
    for i in 0..n {
        let p = base + i as u32 * 12;
        let joint = required_link(archive, p)?;
        let model_id = models
            .iter()
            .position(|m| m.joint.offset == joint)
            .ok_or_else(|| error("unbound position model"))?;
        let pairs = count(&r, p + 8)?;
        let pair_base = array(archive, p + 4, pairs, 4)?;
        for j in 0..pairs {
            let q = pair_base + j as u32 * 4;
            let joint_index = r.s16(q)?;
            if joint_index < 0 || joint_index as usize >= models[model_id].joint.descendants().len()
            {
                return Err(error("position joint outside model"));
            }
            result.push(PositionBinding {
                model_id,
                joint_index,
                stage_position: r.s16(q + 2)?,
            });
        }
    }
    Ok(result)
}
/// `Ground_801C0498`, retail 0x801C0498: scale is the first parameter float.
fn read_parameters(archive: &Archive) -> ReadResult<GroundParam> {
    let p = public(archive, "grGroundParam")?;
    let r = archive.reader();
    r.slice(p, 0xDC)?;
    let mut colors = [[0; 4]; 9];
    for (i, c) in colors.iter_mut().enumerate() {
        *c = r.array(p + 0xB8 + i as u32 * 4)?;
    }
    Ok(GroundParam {
        map_scale: r.f32(p)?,
        fixed_camera: r.s32(p + 0x4C)? != 0,
        environment_colors: colors,
        stage_param_count: count(&r, p + 0xB4)?,
    })
}
/// `mpLibLoad` from ground.c: collision is an independent public, not in map_head.
pub fn load_collision(archive: &Archive, desc: &StageDesc) -> ReadResult<CollMap> {
    Ok(CollMap::load(
        read_public_coll_data(archive)?,
        desc.parameters.map_scale,
        GrKind::Last,
    ))
}

/// Ground_801C24F8: FD's StageParam row (gr/types.h, stride 0x64).
/// +14 selects rule 6 (all characters unlocked), +16 is the percent threshold.
pub fn read_fd_music(archive: &Archive) -> ReadResult<crate::music::MusicParameters> {
    let root = public(archive, "grGroundParam")?;
    let r = archive.reader();
    let count = r.u32(root + 0xB4)?;
    let table = archive
        .link(root + 0xB0)?
        .ok_or_else(|| error("missing StageParam"))?;
    const FINAL_DESTINATION_STAGE: i32 = 32; // St_Kind_Last, stage-select numbering.
    let mut selected = None;
    for i in 0..count {
        let row = table + i * 0x64;
        if r.s32(row)? == FINAL_DESTINATION_STAGE {
            selected = Some(row);
            break;
        }
    }
    let row = selected.ok_or_else(|| error("missing Final Destination StageParam"))?;
    if r.s16(row + 0x14)? != 6 {
        return Err(error("unsupported FD music unlock rule"));
    }
    Ok(crate::music::MusicParameters {
        primary: r.s32(row + 4)?,
        alternate: r.s32(row + 8)?,
        alternate_chance: r.s16(row + 0x16)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_array_count_is_rejected() {
        let bytes = (-1_i32).to_be_bytes();
        assert!(count(&Reader::new(&bytes), 0).is_err());
    }
}
