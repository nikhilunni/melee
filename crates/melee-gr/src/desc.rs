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
/// grGroundParam's camera fields as Ground_801C0800 (0x801C0800) converts
/// them: the integers become floats (`xoris`/`lfd`/`fsubs`), the rest are read
/// as stored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraParam {
    /// +0x08 (s16): cam_vertical_tilt, which match setup installs as the
    /// camera's field of view (gm_16AE: Camera_80030730(Ground_801C20D0())).
    pub fov: f32,
    /// +0x0C (s32): cam_zoom_rate, the nearest eye distance.
    pub min_depth: f32,
    /// +0x10 (s32): cam_max_depth.
    pub max_depth: f32,
    /// +0x14 (s32): cam_pan_degrees.
    pub pan_degrees: f32,
    /// +0x18: cam_info.x24, pitch per unit of vertical offset.
    pub pitch_scale: f32,
    /// +0x1C: cam_info.x20, yaw per unit of horizontal offset.
    pub yaw_scale: f32,
    /// +0x20: cam_track_ratio.
    pub track_ratio: f32,
    /// +0x24: cam_fixed_zoom.
    pub fixed_zoom: f32,
    /// +0x28: cam_track_smooth.
    pub track_smooth: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GroundParam {
    pub map_scale: f32,
    pub camera: CameraParam,
    pub fixed_camera: bool,
    pub environment_colors: [[u8; 4]; 9],
    pub stage_param_count: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDesc {
    /// False for a null map_head entry: the map's model lives in another
    /// archive (Pokemon Stadium's forms), and every other field is empty.
    pub present: bool,
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
    pub material_scripts: Vec<Vec<hsd_archive::desc::color_animation::ColorCommand>>,
    pub kind: GrKind,
    pub parameters: GroundParam,
    pub models: Vec<ModelDesc>,
    pub position_bindings: Vec<PositionBinding>,
    pub section_counts: [usize; 6],
    pub initial_fog: [u8; 3],
    pub material_script_offsets: [u32; 4],
    /// `quake_model_set` (grDatFiles_801C6038): the screen-shake model whose
    /// root translation drives Camera_SetQuakeOffset.
    pub quake_model: Option<QuakeModel>,
}
/// A DynamicModelDesc of one joint tree and its per-kind animations
/// (Loop, Small, Medium, Large).
#[derive(Clone, Debug, PartialEq)]
pub struct QuakeModel {
    pub joint: JObjDesc,
    pub animations: Vec<AnimJoint>,
}
fn read_quake_model(archive: &Archive) -> ReadResult<Option<QuakeModel>> {
    let Some(offset) = archive.public("quake_model_set") else {
        return Ok(None);
    };
    let joint = JObjDesc::read(archive, required_link(archive, offset)?)?;
    let animations = pointer_list(archive, offset + 4)?
        .iter()
        .map(|&p| AnimJoint::read(archive, p))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(QuakeModel { joint, animations }))
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
    read_stage(archive, GrKind::Last, 4)
}

/// `grDatFiles_801C6038` (0x801C6038): Battlefield map 6 selects lights and has no fog.
pub fn read_battlefield(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Battle, 6)
}

/// grDatFiles_801C6038: Story's environment is map 3.
pub fn read_story(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Story, 3)
}
pub fn read_story_parameters(archive: &Archive) -> ReadResult<crate::story::Parameters> {
    let offset = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let mut heights = [0.0; 6];
    for (i, height) in heights.iter_mut().enumerate() {
        *height = r.f32(offset + 12 + i as u32 * 4)?;
    }
    Ok(crate::story::Parameters {
        timer_minimum: r.f32(offset)?,
        timer_range: r.f32(offset + 4)? as i32,
        group_rarity: r.f32(offset + 8)? as i32,
        heights,
    })
}
/// grPs_StageData / grDatFiles_801C6038: GrPs.dat; map 2's callback row
/// carries the environment flags.
pub fn read_stadium(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::PStadium, 2)
}
/// A form archive (GrPs1-4.dat) that grStadium_801D4548 loads mid-match
/// (grDatFiles_801C6478 reads only its map_head).
pub fn read_stadium_form(archive: &Archive) -> ReadResult<Vec<ModelDesc>> {
    let header = public(archive, "map_head")?;
    let reader = archive.reader();
    let count = count(&reader, header + 12)?;
    let base = array(archive, header + 8, count, MODEL_SIZE)?;
    (0..count)
        .map(|i| read_model(archive, base + i as u32 * MODEL_SIZE))
        .collect()
}
/// `grPStadium_YakumonoParam` (grpstadium.c:41-65).
pub fn read_stadium_parameters(archive: &Archive) -> ReadResult<crate::stadium::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let pair = |at: u32| -> ReadResult<[i32; 2]> { Ok([r.s32(p + at)?, r.s32(p + at + 4)?]) };
    Ok(crate::stadium::Parameters {
        base_frames: pair(0)?,
        form_frames: pair(8)?,
        announce_delay: r.s32(p + 0x10)?,
        sink_frames: r.s32(p + 0x14)?,
        sunk_frames: r.s32(p + 0x18)?,
        info_frames: r.s32(p + 0x20)?,
        defeat_frames: r.s32(p + 0x24)?,
        announce_frames: r.s32(p + 0x28)?,
        standings_frames: r.s32(p + 0x2C)?,
        player_camera_frames: pair(0x30)?,
        stage_camera_frames: pair(0x38)?,
        mode_weights: crate::stadium::ModeWeights {
            player_camera: r.s16(p + 0x48)?,
            match_info: r.s16(p + 0x4A)?,
            stage_camera: r.s16(p + 0x4C)?,
            picture: r.s16(p + 0x4E)?,
        },
        standings_interval: r.s16(p + 0x50)?,
    })
}
/// grOp_StageData / grDatFiles_801C6038: GrOp.dat, environment map 5.
pub fn read_pupupu(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::OldPupupu, 5)
}
pub fn read_pupupu_parameters(archive: &Archive) -> ReadResult<crate::pupupu::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    Ok(crate::pupupu::Parameters {
        background_delay: [r.s16(p)?, r.s16(p + 2)?],
        background_height: r.s16(p + 4)?,
        wind_delay: [r.s32(p + 8)?, r.s32(p + 12)?],
        wind_speed: r.f32(p + 16)?,
        right_bounds: [r.f32(p + 20)?, r.f32(p + 24)?],
        left_bounds: [r.f32(p + 28)?, r.f32(p + 32)?],
        vertical_bounds: [r.f32(p + 36)?, r.f32(p + 40)?],
        blink_delay: [r.f32(p + 44)? as i32, r.f32(p + 48)? as i32],
    })
}
/// grIz_StageData / grDatFiles_801C6038: GrIz.dat. grIz_StageCallbacks[3]
/// carries flags 0xC0000000, so map 3 selects the lights and fog.
pub fn read_izumi(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Izumi, 3)
}
/// `grIzumi_YakumonoParam` (grizumi.c:40-62), read through
/// Ground_GetYakumonoParam by grIzumi_801CBB88 (0x801CBB88).
pub fn read_izumi_parameters(archive: &Archive) -> ReadResult<crate::izumi::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |offset: u32| r.f32(p + offset);
    Ok(crate::izumi::Parameters {
        initial_heights: [f(0x0)?, f(0x8)?],
        rest_height: f(0xC)?,
        step: [f(0x18)?, f(0x1C)?],
        highest_target: f(0x20)?,
        lowest_target: f(0x24)?,
        rise_speed: f(0x28)?,
        sink_speed: f(0x2C)?,
        sink_chance_below_rest: f(0x30)?,
        rise_chance_above_rest: f(0x34)?,
        wait_frames: [f(0x3C)?, f(0x38)?],
        submerge_weight: f(0x40)?,
        stay_weight: f(0x44)?,
        step_weight: f(0x48)?,
        submerged_frames: [f(0x50)?, f(0x4C)?],
    })
}
fn read_stage(archive: &Archive, kind: GrKind, environment_map: usize) -> ReadResult<StageDesc> {
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
    // Ground_801C1E00 selects the callback row with flags_b1 (FD 4, BF 6).
    let fog = models.get(environment_map).and_then(|m| m.fog_offset);
    // HSD_FogDesc (fog.h): u32 type, fog-adjust pointer, start/end, GXColor at +0x10.
    let initial_fog = fog
        .map(|fog| reader.array::<3>(fog + 0x10))
        .transpose()?
        .unwrap_or([0; 3]);
    let scripts = public(archive, "yakumono_param")?;
    let mut material_script_offsets = [0; 4];
    // grLast's four fades; grBattle_YakumonoParam's incoming/outgoing overlays.
    let script_count = match kind {
        GrKind::Last => 4,
        GrKind::Battle => 2,
        _ => 0,
    };
    for (i, offset) in material_script_offsets
        .iter_mut()
        .enumerate()
        .take(script_count)
    {
        *offset = required_link(archive, scripts + i as u32 * 4)?;
    }
    let material_scripts = material_script_offsets[..script_count]
        .iter()
        .map(|&offset| hsd_archive::desc::color_animation::read(archive, offset))
        .collect::<Result<_, _>>()?;
    Ok(StageDesc {
        material_scripts,
        kind,
        parameters,
        models,
        position_bindings: bindings,
        section_counts,
        initial_fog,
        material_script_offsets,
        quake_model: read_quake_model(archive)?,
    })
}

fn read_model(archive: &Archive, offset: u32) -> ReadResult<ModelDesc> {
    let reader = archive.reader();
    // An unused entry holds an unrelocated -1 joint pointer, which the
    // loader leaves null (grDatFiles_801C6330 skips such archives).
    if archive.link(offset)?.is_none() && reader.u32(offset)? == u32::MAX {
        return Ok(ModelDesc {
            present: false,
            joint: JObjDesc::default(),
            animations: Vec::new(),
            animation_loops: Vec::new(),
            joint_mappings: Vec::new(),
            material_animation_offsets: Vec::new(),
            shape_animation_offsets: Vec::new(),
            light_list_offset: None,
            fog_offset: None,
        });
    }
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
        present: true,
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
            .position(|m| m.present && m.joint.offset == joint)
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
        camera: CameraParam {
            fov: f32::from(r.s16(p + 0x8)?),
            min_depth: r.s32(p + 0xC)? as f32,
            max_depth: r.s32(p + 0x10)? as f32,
            pan_degrees: r.s32(p + 0x14)? as f32,
            pitch_scale: r.f32(p + 0x18)?,
            yaw_scale: r.f32(p + 0x1C)?,
            track_ratio: r.f32(p + 0x20)?,
            fixed_zoom: r.f32(p + 0x24)?,
            track_smooth: r.f32(p + 0x28)?,
        },
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
        desc.kind,
    ))
}

/// Ground_801C24F8: FD's StageParam row (gr/types.h, stride 0x64).
/// +14 selects rule 6 (all characters unlocked), +16 is the percent threshold.
pub fn read_fd_music(archive: &Archive) -> ReadResult<crate::music::MusicParameters> {
    read_music(archive, 32)
}

/// `Ground_801C24F8` (0x801C24F8): select a StageParam row by stage-select ID.
pub fn read_music(archive: &Archive, stage_id: i32) -> ReadResult<crate::music::MusicParameters> {
    let root = public(archive, "grGroundParam")?;
    let r = archive.reader();
    let count = r.u32(root + 0xB4)?;
    let table = archive
        .link(root + 0xB0)?
        .ok_or_else(|| error("missing StageParam"))?;
    let mut selected = None;
    for i in 0..count {
        let row = table + i * 0x64;
        if r.s32(row)? == stage_id {
            selected = Some(row);
            break;
        }
    }
    let row = selected.ok_or_else(|| error("missing stage music parameters"))?;
    let rule = match r.s16(row + 0x14)? {
        0 => crate::music::MusicRule::Primary,
        6 => crate::music::MusicRule::AllCharactersUnlocked,
        rule => return Err(error(format!("unsupported stage music unlock rule {rule}"))),
    };
    Ok(crate::music::MusicParameters {
        rule,
        primary: r.s32(row + 4)?,
        alternate: r.s32(row + 8)?,
        alternate_chance: r.s16(row + 0x16)?,
        sudden_death: r.s32(row + 0xC)?,
    })
}

/// `Ground_801C466C` (0x801C466C), ground.c:2665-2670, and
/// `lb_80011AC4` (0x80011AC4): the selected map's Melee LightList table.
pub fn read_static_lights(
    archive: &Archive,
    model: &ModelDesc,
) -> ReadResult<Vec<hsd_archive::desc::light::LightDesc>> {
    let Some(offset) = model.light_list_offset else {
        return Ok(Vec::new());
    };
    hsd_archive::visual::read_lights(archive, offset)?
        .into_iter()
        .map(|light| {
            if light.has_animation_set {
                unimplemented!("ground.c:2723-2747: animated light descriptors");
            }
            Ok(light.descriptor)
        })
        .collect()
}

/// `HSD_FogDesc` (fog.h): GX fog type, then start and end depth and color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogDesc {
    /// `GXFogType`.
    pub kind: u32,
    pub start: f32,
    pub end: f32,
    pub color: [u8; 4],
}
/// The fog a map row carries (`ModelDesc::fog_offset`).
pub fn read_fog(archive: &Archive, offset: u32) -> ReadResult<FogDesc> {
    let reader = archive.reader();
    Ok(FogDesc {
        kind: reader.u32(offset)?,
        start: reader.f32(offset + 8)?,
        end: reader.f32(offset + 12)?,
        color: reader.array(offset + 0x10)?,
    })
}

/// One `LightOverrideEntry` of map_head's fourth table (`UnkStageDat.unk18`,
/// 8 bytes: the light descriptor, then bits a, b, c of one byte).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightOverride {
    pub descriptor: Option<u32>,
    /// Bit a: `LOBJ_SPECULAR` (0x8).
    pub specular: bool,
    /// Bit b: `LOBJ_DIFFUSE` (0x4).
    pub diffuse: bool,
    /// Bit c: light flag 0x400.
    pub flag_400: bool,
}
/// `Ground_801C20E0` (0x801C20E0) reads these light overrides.
pub fn read_light_overrides(archive: &Archive) -> ReadResult<Vec<LightOverride>> {
    let header = public(archive, "map_head")?;
    let reader = archive.reader();
    let n = count(&reader, header + 0x1C)?;
    let base = array(archive, header + 0x18, n, 8)?;
    (0..n as u32)
        .map(|i| {
            let entry = base + i * 8;
            let bits = reader.u8(entry + 4)?;
            Ok(LightOverride {
                descriptor: archive.link(entry)?,
                specular: bits & 0x80 != 0,
                diffuse: bits & 0x40 != 0,
                flag_400: bits & 0x20 != 0,
            })
        })
        .collect()
}
/// `Ground_801C43C4` (0x801C43C4): map_head's fifth table (`unk20`, 8-byte
/// `GroundShadowEntry`) pairs each light animation with its loop bit.
pub fn read_light_animation_loops(archive: &Archive) -> ReadResult<Vec<(Option<u32>, bool)>> {
    let header = public(archive, "map_head")?;
    let reader = archive.reader();
    let n = count(&reader, header + 0x24)?;
    let base = array(archive, header + 0x20, n, 8)?;
    (0..n as u32)
        .map(|i| {
            let entry = base + i * 8;
            Ok((archive.link(entry)?, reader.u8(entry + 4)? & 0x80 != 0))
        })
        .collect()
}

/// grAnime_801C7C1C (0x801C7C1C): animation arrays contain consecutive
/// HSD_AnimJoint records, indexed by Ground's descendant number.
pub fn animation_subtree(
    archive: &Archive,
    model: &ModelDesc,
    animation: usize,
    bone: u32,
) -> ReadResult<AnimJoint> {
    Ok(AnimJoint::read(
        archive,
        model.animations[animation].offset + bone * hsd_archive::desc::ANIM_JOINT_SIZE,
    )?)
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
