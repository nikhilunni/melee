//! Melee-owned archive layouts from gr/types.h. All pointer arithmetic stays here.
use hsd_archive::desc::{AnimJoint, JObjDesc};
use hsd_archive::{Archive, Reader};
use melee_mp::{desc::read_public_coll_data, CollMap};
use melee_types::GrKind;

pub type ReadResult<T> = Result<T, Box<dyn std::error::Error>>;
const MODEL_SIZE: u32 = 0x34;
const JOINT_MAPPING_SIZE: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// +0x50..+0x64: the fixed mode's eye, field of view and angles
    /// (Ground_801C0800 -> Ground_801C392C).
    pub fixed_view: melee_cm::FixedView,
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
    pub splines: Vec<hsd_archive::desc::spline::Spline>,
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
/// grFs_StageData / grDatFiles_801C6038: GrFs.dat. grFs_StageCallbacks[6]
/// carries flags 0xC0000000, so map 6 selects the lights and fog.
pub fn read_fourside(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Fourside, 6)
}
/// `grFourside_YakumonoParam` (grfourside.c:28-50) as the asm reads it.
pub fn read_fourside_parameters(archive: &Archive) -> ReadResult<crate::fourside::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let w = |offset: u32| r.s32(p + offset);
    let f = |offset: u32| r.f32(p + offset);
    Ok(crate::fourside::Parameters {
        heli_wait: w(0x0)?,
        heli_wait_add: w(0x4)?,
        crane_wait: w(0xC)?,
        crane_wait_add: w(0x10)?,
        crane_iron_wait: w(0x14)?,
        crane_iron_wait_add: w(0x18)?,
        crane_iron_up: [f(0x1C)?, f(0x20)?],
        crane_iron_down: [f(0x24)?, f(0x28)?],
        crane_iron_speed: f(0x2C)?,
        crane_iron_stop_acceleration: f(0x30)?,
        ufo_wait: w(0x34)?,
        ufo_camera_offset: f(0x38)?,
        ufo_stay_time: w(0x3C)?,
        ufo_stay_time_add: w(0x40)?,
        ufo_challenge: r.s16(p + 0x44)?,
        ufo_standing_odds: r.s16(p + 0x46)?,
        ufo_challenged_odds: r.s16(p + 0x48)?,
    })
}
/// grSh_StageData / grDatFiles_801C6038: GrSh.dat. grSh_StageCallbacks[2]
/// carries flags 0xC0000000, so map 2 selects the lights and fog.
pub fn read_shrine(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Shrine, 2)
}
/// grI1_StageData / grDatFiles_801C6038: GrI1.dat. grI1_StageCallbacks[3]
/// carries flags 0xC0000000, so map 3 selects the lights and fog.
pub fn read_inishie1(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Inishie1, 3)
}
/// `grInishie1_YakumonoParam` (grinishie1.c:117-136) as the asm reads it.
pub fn read_inishie1_parameters(archive: &Archive) -> ReadResult<crate::inishie1::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    Ok(crate::inishie1::Parameters {
        hatena1_wait: [r.f32(p)?, r.f32(p + 0x4)?],
        hatena2_wait: [r.f32(p + 0x8)?, r.f32(p + 0xC)?],
        all_hatena_cooldown: r.f32(p + 0x10)?,
        all_hatena_window: r.s16(p + 0x14)?,
        hatena_flicker: r.s16(p + 0x16)?,
        return_flicker: r.u16(p + 0x18)?,
        respawn_wait: [r.s16(p + 0x1A)?, r.s16(p + 0x1C)?],
        lift_max_offset: r.f32(p + 0x20)?,
        lift_acceleration: r.f32(p + 0x24)?,
        lift_max_speed: r.f32(p + 0x28)?,
        lift_damping_both: r.f32(p + 0x2C)?,
        lift_damping_one: r.f32(p + 0x30)?,
        lift_friction: r.f32(p + 0x34)?,
        lift_return_speed: r.f32(p + 0x38)?,
        lift_free_weight: r.f32(p + 0x3C)?,
        lift_standing_weight: r.f32(p + 0x40)?,
        lift_fall_acceleration: r.f32(p + 0x44)?,
        lift_fall_speed: r.f32(p + 0x48)?,
        lift_away_ticks: r.f32(p + 0x4C)?,
        lift_settle_speed: r.f32(p + 0x50)?,
    })
}
/// grOt_StageData / grDatFiles_801C6038: GrOt.dat. grOt_803E27E0[5]
/// carries flags 0xC0000000, so map 5 selects the lights and fog.
pub fn read_onett(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Onett, 5)
}
/// `grOnett_StageParam` (gronett.c:64-91) as the asm reads it.
pub fn read_onett_parameters(archive: &Archive) -> ReadResult<crate::onett::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    // The car timers are floats the asm converts (fctiwz) where it reads them.
    let i = |at: u32| -> ReadResult<i32> { Ok(gekko_math::msl::fctiwz(r.f32(p + at)?)) };
    Ok(crate::onett::Parameters {
        awning_initial: f(0x0)?,
        max_velocity: f(0x4)?,
        vel_threshold: f(0x8)?,
        pos_threshold: f(0xC)?,
        damping: f(0x10)?,
        spring_force: f(0x14)?,
        spring_constant: f(0x18)?,
        max_displacement: f(0x1C)?,
        awning_delta: f(0x20)?,
        smoke_pause: [f(0x24)?, f(0x28)?],
        smoke_puff: [f(0x2C)?, f(0x30)?],
        roof_crack: f(0x34)?,
        roof_break: f(0x38)?,
        roof_wait: [f(0x3C)?, f(0x40)?],
        car_wait: i(0x44)?,
        car_announce: i(0x48)?,
        car_delay_middle: i(0x4C)?,
        car_delay_last: i(0x50)?,
        car_speed_base: f(0x54)?,
        car_speed_range: f(0x58)?,
        car_spin: f(0x5C)?,
        warning_ticks: i(0x60)?,
        horn_lead: f(0x64)?,
    })
}
/// grMc_StageData / grDatFiles_801C6038: GrMc.dat. grMc_StageCallbacks[0x1E]
/// carries flags 0xC0000000, so map 0x1E selects the lights and fog.
pub fn read_mute_city(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::MuteCity, 0x1E)
}
/// grPu_StageData / grDatFiles_801C6038: GrPu.dat. stage_callbacks[4]
/// carries flags 0x40000000 (the fog); row 27's 0x80000000 is the lights.
pub fn read_pura(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Pura, 4)
}
/// grBb_StageData / grDatFiles_801C6038: GrBb.dat. grBb_StageCallbacks[1]
/// carries flags 0x40000000 (the fog); row 0x23's 0x80000000 is the lights.
pub fn read_big_blue(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::BigBlue, 1)
}
/// `grMc_YakumonoParam` (grmutecity.c:303-317): the cars' floats.
pub fn read_mute_city_parameters(archive: &Archive) -> ReadResult<crate::mutecity::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    Ok(crate::mutecity::Parameters {
        smoke_damage: f(0x2C)?,
        explode_damage: f(0x30)?,
        top_speed: f(0x34)?,
        leader_top_speed: f(0x38)?,
        trailer_top_speed: f(0x3C)?,
        acceleration: f(0x40)?,
        lane_step: f(0x44)?,
        lane_centering: f(0x48)?,
        braking: f(0x4C)?,
    })
}
/// A stage line hit (lbColl_80008D30_arg1, lb/forward.h:94) at a
/// relocated pointer slot.
pub fn read_line_hazard(archive: &Archive, slot: u32) -> ReadResult<melee_types::combat::LineHazard> {
    let p = required_link(archive, slot)?;
    let r = archive.reader();
    let element = r.u32(p + 0x18)? as i32;
    Ok(melee_types::combat::LineHazard {
        damage: r.u32(p + 0x4)?,
        angle: r.s32(p + 0x8)?,
        growth: r.u32(p + 0xC)?,
        weight_knockback: r.u32(p + 0x10)?,
        base_knockback: r.u32(p + 0x14)?,
        element: melee_types::HitElement::try_from(element)
            .map_err(|_| error(format!("line hazard element {element}")))?,
        sound_severity: r.s32(p + 0x1C)?,
        sound_kind: r.s32(p + 0x20)?,
    })
}
/// grMuteCity_801F2BBC's two hits: yakumono_param x8 (the track's end
/// lines 0x31 and 0x35) and xC (lines 0x32..0x34).
pub fn read_mute_city_hazards(
    archive: &Archive,
) -> ReadResult<[melee_types::combat::LineHazard; 2]> {
    let p = public(archive, "yakumono_param")?;
    Ok([read_line_hazard(archive, p + 0x8)?, read_line_hazard(archive, p + 0xC)?])
}
/// The HSD_Spline a model bone carries (`jobj->u.spline`, JOBJ_SPLINE).
pub fn bone_spline(
    archive: &Archive,
    model: &ModelDesc,
    bone: usize,
) -> ReadResult<hsd_archive::desc::spline::Spline> {
    let joints = model.joint.descendants();
    let joint = joints.get(bone).ok_or_else(|| error("spline bone outside model"))?;
    match joint.u {
        hsd_archive::desc::jobj::JObjUnion::Spline(Some(offset)) => {
            Ok(hsd_archive::desc::spline::Spline::read(archive, offset)?)
        }
        _ => Err(error("bone without a spline")),
    }
}
/// grGb_StageData / grDatFiles_801C6038: GrGb.dat. grGb_StageCallbacks[2]
/// carries flags 0xC0000000, so map 2 selects the lights and fog.
pub fn read_greatbay(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::GreatBay, 2)
}
/// `grGb_StageAttr` (grgreatbay.c:153-190) as the asm reads it.
pub fn read_greatbay_parameters(archive: &Archive) -> ReadResult<crate::greatbay::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    let h = |at: u32| r.s16(p + at);
    let mut items = [(0i16, 0i16); 10];
    for (i, item) in items.iter_mut().enumerate() {
        let at = 0x7C + i as u32 * 4;
        *item = (h(at)?, h(at + 2)?);
    }
    Ok(crate::greatbay::Parameters {
        moon_fall_wait: [h(0)?, h(2)?],
        float_landing_rate: f(0x4)?,
        float_slant: [f(0x8)?, f(0xC)?, f(0x10)?, f(0x14)?, f(0x18)?],
        float_slide: [f(0x1C)?, f(0x20)?, f(0x24)?, f(0x28)?, f(0x2C)?],
        float_down: [f(0x30)?, f(0x34)?, f(0x38)?, f(0x3C)?, f(0x40)?],
        turtle_wait: [h(0x44)?, h(0x46)?],
        turtle_rebirth: [h(0x48)?, h(0x4A)?],
        turtle_x: f(0x4C)?,
        turtle_y: f(0x50)?,
        turtle_x_offset_init: f(0x54)?,
        turtle_x_lr_offset: [f(0x58)?, f(0x5C)?],
        turtle_x_fb_offset: [f(0x60)?, f(0x64)?],
        turtle_scale: f(0x68)?,
        turtle_ud_scale: f(0x6C)?,
        turtle_dir_prob: [h(0x70)?, h(0x72)?, h(0x74)?, h(0x76)?],
        turtle_item_prob: f(0x78)?,
        turtle_items: items,
    })
}
/// grIm_StageData / grDatFiles_801C6038: GrIm.dat. stage_callbacks[8]
/// carries flags 0xC0000000, so map 8 selects the lights and fog.
pub fn read_icemt(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Icemt, 8)
}
/// `grIceMt_YakumonoParam` (gricemt.c:63-125) as the Versus code reads
/// it: the speeds as `((f32*)(param + 4))[idx]` (grIceMt_801FA364).
pub fn read_icemt_parameters(archive: &Archive) -> ReadResult<crate::icemt::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let mut speeds = [0.0; 12];
    for (i, speed) in speeds.iter_mut().enumerate() {
        *speed = r.f32(p + 4 + 4 * i as u32)?;
    }
    Ok(crate::icemt::Parameters {
        segment_cooldown: r.s16(p)?,
        speed_cooldown: r.s16(p + 0x2)?,
        speeds,
        ease_frames: r.s16(p + 0x34)?,
        hold: [r.s16(p + 0x36)?, r.s16(p + 0x38)?],
        segment_gap: r.f32(p + 0x40)?,
    })
}
/// grI2_StageData / grDatFiles_801C6038: GrI2.dat. grI2_StageCallbacks[2]
/// carries flags 0xC0000000, so map 2 selects the lights and fog.
pub fn read_inishie2(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Inishie2, 2)
}
/// `grInishie2_YakumonoParam` (grinishie2.c:166-181) as the asm reads it.
pub fn read_inishie2_parameters(archive: &Archive) -> ReadResult<crate::inishie2::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let vec = |at: u32| -> ReadResult<hsd_types::Vec3> {
        Ok(hsd_types::Vec3::new(r.f32(at)?, r.f32(at + 4)?, r.f32(at + 8)?))
    };
    Ok(crate::inishie2::Parameters {
        lift_waits: [[r.s16(p)?, r.s16(p + 0x2)?], [r.s16(p + 0x4)?, r.s16(p + 0x6)?]],
        birdo_wait: [r.s16(p + 0x8)?, r.s16(p + 0xA)?],
        carpet_wait: [r.s16(p + 0xC)?, r.s16(p + 0xE)?],
        lift_delays: [r.s16(p + 0x10)?, r.s16(p + 0x12)?],
        birdo_spawn: [vec(p + 0x14)?, vec(p + 0x20)?],
        birdo_platform_height: r.f32(p + 0x2C)?,
        carpet_spawn: [vec(p + 0x30)?, vec(p + 0x3C)?],
        carpet_turn_odds: [r.s16(p + 0x48)?, r.s16(p + 0x4A)?],
    })
}
/// grCs_StageData / grDatFiles_801C6038: GrCs.dat. grCs_StageCallbacks[3]
/// carries flags 0xC0000000, so map 3 selects the lights and fog.
pub fn read_castle(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Castle, 3)
}
/// `grCastle_YakumonoParam` (grcastle.c) as the asm reads it.
pub fn read_castle_parameters(archive: &Archive) -> ReadResult<crate::castle::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    let h = |at: u32| -> ReadResult<i32> { Ok(i32::from(r.s16(p + at)?)) };
    let four = |at: u32| -> ReadResult<[f32; 4]> { Ok([f(at)?, f(at + 4)?, f(at + 8)?, f(at + 12)?]) };
    let mut flights = [crate::castle::Flight {
        cannon_delay: 0,
        sound: 0.0,
        cannon_rotation: hsd_types::Vec3::ZERO,
    }; 9];
    for (i, flight) in flights.iter_mut().enumerate() {
        let at = 0x5C + i as u32 * 0x14;
        *flight = crate::castle::Flight {
            cannon_delay: h(at)?,
            sound: f(at + 4)?,
            cannon_rotation: hsd_types::Vec3::new(f(at + 8)?, f(at + 0xC)?, f(at + 0x10)?),
        };
    }
    Ok(crate::castle::Parameters {
        switch_wait: [h(0x0)?, h(0x2)?],
        switch_freeze: h(0x4)?,
        switch_repeat_divisor: h(0x6)?,
        first_bill_wait: [h(0x8)?, h(0xA)?],
        next_bill_wait: [h(0xC)?, h(0xE)?],
        platform_top: f(0x10)?,
        platform_bottom: f(0x14)?,
        platform_starts_low: f(0x18)?,
        rise: [f(0x20)?, f(0x24)?, f(0x28)?],
        sink: [f(0x2C)?, f(0x30)?, f(0x34)?],
        rise_delay: f(0x38)?,
        sink_delay: f(0x3C)?,
        blocks_time: [h(0x40)?, h(0x42)?],
        blocks_blink: h(0x44)?,
        block_grow: f(0x48)?,
        block_shrink: f(0x4C)?,
        item_launch: f(0x50)?,
        switch_linger: h(0x54)?,
        bill_fuse: h(0x58)?,
        flights,
        cannon_lift: f(0x110)?,
        bill_extents: four(0x118)?,
        cannon_waits: [h(0x12C)?, h(0x12E)?, h(0x130)?, h(0x132)?],
        explosion_extents: four(0x134)?,
    })
}
/// `grGreens_YakumonoParam` (grgreens.c) as the asm reads it.
pub fn read_greens_parameters(archive: &Archive) -> ReadResult<crate::greens::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    let i = |at: u32| r.s32(p + at);
    Ok(crate::greens::Parameters {
        block_wait: [i(0x0)?, i(0x4)?],
        column_weights: [i(0x18)?, i(0x14)?, i(0x10)?, i(0xC)?, i(0x8)?],
        empty_column_weight: i(0x1C)?,
        bomb_odds: i(0x20)?,
        block_radius: i(0x24)?,
        bomb_radius: i(0x28)?,
        fall_speed_max: f(0x2C)?,
        fall_acceleration: f(0x30)?,
        idle_wait: [i(0x34)?, i(0x38)?],
        wind_speed: f(0x3C)?,
        wind_left: f(0x40)?,
        wind_right: f(0x44)?,
        wind_top: f(0x48)?,
        wind_bottom: f(0x4C)?,
        inhale_loops: f(0x50)?,
        blow_loops: f(0x54)?,
        blow_turn_loops: f(0x58)?,
        apple_count: [i(0x5C)?, i(0x60)?],
        apple_start: i(0x64)?,
        apple_interval: i(0x68)?,
        apple_height: [f(0x6C)?, f(0x70)?],
        apple_kept: f(0x74)?,
        apple_heal: f(0x78)?,
    })
}
/// grGr_StageData: GrGr.dat. grGr_callbacks[6] carries 0xC0000000.
pub fn read_greens(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Greens, 6)
}
/// grYt_StageData / grDatFiles_801C6038: GrYt.dat. grYt_StageCallbacks[1]
/// carries flags 0xC0000000, so map 1 selects the lights and fog.
pub fn read_yorster(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Yorster, 1)
}
/// `YorsterParams` (gryorster.c:57-66) as the asm reads it.
pub fn read_yorster_parameters(archive: &Archive) -> ReadResult<crate::yorster::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    Ok(crate::yorster::Parameters {
        bump_speed: r.f32(p)?,
        bump_full: r.f32(p + 0x4)?,
        damage_threshold: r.f32(p + 0x8)?,
        damage_full: r.f32(p + 0xC)?,
        bump_freeze: r.s32(p + 0x10)?,
        hit_freeze: r.s32(p + 0x14)?,
        flip_delay: r.s32(p + 0x18)?,
        spin_end_frame: r.s32(p + 0x1C)?,
    })
}
/// grOy_StageData / grDatFiles_801C6038: GrOy.dat. grOy_StageCallbacks[4]
/// carries flags 0xC0000000, so map 4 selects the lights and fog.
pub fn read_old_yoshi(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::OldYoshi, 4)
}
/// `yakumono_param` as groldyoshi.c reads it (grOldYoshi_8020EC10,
/// grOldYoshi_8020F088): +0x18 with `lha`.
pub fn read_old_yoshi_parameters(archive: &Archive) -> ReadResult<crate::oldyoshi::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    Ok(crate::oldyoshi::Parameters {
        vanish_after: r.s16(p)?,
        return_after: r.s16(p + 0x2)?,
        maximum_speed: r.f32(p + 0x4)?,
        acceleration: r.f32(p + 0x8)?,
        sink_depth: r.f32(p + 0xC)?,
        disable_frame: r.s16(p + 0x10)?,
        enable_frame: r.s16(p + 0x12)?,
        guest_wait: [r.s16(p + 0x16)?, r.s16(p + 0x14)?],
        guest_height: r.s16(p + 0x18)?,
    })
}
/// grOk_StageData / grDatFiles_801C6038: GrOk.dat. stage_callbacks[3]
/// carries flags 0xC0000000, so map 3 selects the lights and fog.
pub fn read_old_kongo(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::OldKongo, 3)
}
/// `grOldKongo_YakumonoParam` (groldkongo.c:27-61) as the asm reads it.
pub fn read_old_kongo_parameters(archive: &Archive) -> ReadResult<crate::oldkongo::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |offset: u32| r.f32(p + offset);
    let w = |offset: u32| r.s32(p + offset);
    let mut aim_weights = [0; 8];
    for (i, weight) in aim_weights.iter_mut().enumerate() {
        *weight = r.s16(p + 0x2C + 2 * i as u32)?;
    }
    Ok(crate::oldkongo::Parameters {
        bird_wait: [i32::from(r.s16(p + 0x2)?), i32::from(r.s16(p)?)],
        bird_height_range: f(0x4)?,
        shot_delay: [f(0x8)?, f(0xC)?],
        catch_radius: f(0x10)?,
        turn_wait: [f(0x18)?, f(0x14)?],
        turn_acceleration: f(0x1C)?,
        turn_speed: f(0x20)?,
        turn_spin: [f(0x28)?, f(0x24)?],
        aim_weights,
        stop_interval: [w(0x40)?, w(0x3C)?],
        move_acceleration: f(0x44)?,
        move_speed: f(0x48)?,
        drift_frames: [w(0x50)?, w(0x4C)?],
        shot_damage: w(0x54)?,
        shot_angle: w(0x58)?,
        shot_growth: w(0x5C)?,
        shot_set_knockback: w(0x60)?,
        shot_base_knockback: w(0x64)?,
        shot_element: w(0x68)?,
        catch_material: w(0x6C)?,
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
/// grGd_StageData / grDatFiles_801C6038: GrGd.dat. grGd_StageCallbacks[2]
/// (the main stage) carries flags 0xC0000000.
pub fn read_garden(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Garden, 2)
}
/// `grGarden_YakumonoParam` (grgarden.c:26-35), read through
/// Ground_GetYakumonoParam by grGarden_80202B70 (0x80202B70).
pub fn read_garden_parameters(archive: &Archive) -> ReadResult<crate::garden::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    Ok(crate::garden::Parameters {
        water_level: r.f32(p)?,
        current: r.f32(p + 0x4)?,
        cranky_rest_loops: r.s32(p + 0x8)?,
        cranky_busy_loops: r.s32(p + 0xC)?,
        klaptrap_wait_minimum: r.s32(p + 0x10)?,
        klaptrap_wait_range: r.s32(p + 0x14)?,
        klaptrap_x: [r.f32(p + 0x18)?, r.f32(p + 0x1C)?],
    })
}
/// grKg_StageData / grDatFiles_801C6038: GrKg.dat. grKg_StageCallbacks[10]
/// (the main stage) carries flags 0xC0000000.
pub fn read_kongo(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Kongo, 10)
}
/// `grKongo_YakumonoParam` (grkongo.static.h), read through
/// Ground_GetYakumonoParam by grKongo_801D523C (0x801D523C).
pub fn read_kongo_parameters(archive: &Archive) -> ReadResult<crate::kongo::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |offset: u32| r.f32(p + offset);
    let pair = |offset: u32| -> ReadResult<[f32; 2]> { Ok([f(offset)?, f(offset + 4)?]) };
    let ints = |offset: u32| -> ReadResult<[i32; 2]> { Ok([r.s32(offset + p)?, r.s32(p + offset + 4)?]) };
    let mut direction_weights = [0; 8];
    for (i, w) in direction_weights.iter_mut().enumerate() {
        *w = r.s16(p + 0x44 + i as u32 * 2)?;
    }
    let mut launch_hit = [0; 6];
    for (i, w) in launch_hit.iter_mut().enumerate() {
        *w = r.u32(p + 0x6C + i as u32 * 4)?;
    }
    Ok(crate::kongo::Parameters {
        event_wait: pair(0x0)?,
        klap_weight: f(0x8)?,
        box_weight: f(0xC)?,
        object_weight: f(0x10)?,
        path_speed: f(0x14)?,
        log_share: f(0x18)?,
        repeat_chance: f(0x1C)?,
        hold_frames: pair(0x20)?,
        catch_radius: f(0x28)?,
        rest_frames: pair(0x2C)?,
        spin_acceleration: f(0x34)?,
        spin_speed: f(0x38)?,
        spin_frames: pair(0x3C)?,
        direction_weights,
        dim_frames: ints(0x54)?,
        rate_step: f(0x5C)?,
        rate_top: f(0x60)?,
        bright_frames: ints(0x64)?,
        launch_hit,
        catch_material: r.u32(p + 0x84)?,
        log_landings: f(0x88)?,
        log_frames: f(0x8C)?,
        plank_limit: f(0x90)?,
        plank_restore: f(0x94)?,
        plank_speed_limit: f(0x98)?,
        plank_slack: f(0x9C)?,
        plank_pull: f(0xA0)?,
        plank_kick: f(0xA4)?,
        end_tilt: f(0xA8)?,
        end_angle_limit: f(0xAC)?,
        end_jolt: f(0xB0)?,
        swing_damping: f(0xB4)?,
        swing_limit: f(0xB8)?,
    })
}
/// grFz_StageData / grDatFiles_801C6038: GrFz.dat. grFz_StageCallbacks[2]
/// (the controller) carries flags 0xC0000000.
pub fn read_flatzone(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Flatzone, 2)
}
/// `grFlatzone_YakumonoParam` (grflatzone.c:20-37), read through
/// Ground_GetYakumonoParam by grFlatzone_80216E78 (0x80216E78).
pub fn read_flatzone_parameters(archive: &Archive) -> ReadResult<crate::flatzone::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let w = |offset: u32| r.s32(p + offset);
    Ok(crate::flatzone::Parameters {
        event_wait: [w(0x4)?, w(0x0)?],
        variant_wait: [w(0xC)?, w(0x8)?],
        tools_frames: w(0x10)?,
        tools_interval: w(0x14)?,
        tools_right: w(0x18)?,
        tools_left: w(0x1C)?,
        oil_walk_in: w(0x20)?,
        oil_right_x: r.f32(p + 0x24)?,
        oil_left_x: r.f32(p + 0x28)?,
        oil_step_frames: w(0x2C)?,
        oil_steps: [w(0x30)?, w(0x34)?],
        oil_step: r.f32(p + 0x38)?,
        oil_frames: w(0x3C)?,
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
        splines: if kind == GrKind::Kongo {
            hsd_archive::desc::spline::Spline::read_stage_bank(archive)?
        } else {
            Vec::new()
        },
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
        fixed_view: melee_cm::FixedView {
            position: hsd_types::Vec3::new(r.f32(p + 0x50)?, r.f32(p + 0x54)?, r.f32(p + 0x58)?),
            fov: r.f32(p + 0x5C)?,
            vertical_angle: r.f32(p + 0x60)?,
            horizontal_angle: r.f32(p + 0x64)?,
        },
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
        3 => crate::music::MusicRule::CharacterUnlocked(crate::music::MARTH_UNLOCK_BIT),
        4 => crate::music::MusicRule::CharacterUnlocked(crate::music::YOUNG_LINK_UNLOCK_BIT),
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

/// The random item spawner's stage data for one StageParam row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemRates {
    /// `stage_info.xA0` (Ground_801C28CC): per item kind below
    /// It_Kind_L_Gun_Ray, the ground parameters' weight (+0x6A) times the
    /// row's (+0x1A).
    pub weights: [i32; ITEM_RATE_KINDS],
    /// Ground_801C2AE8: `(0.01f * ground +0x68) * (0.01f * row +0x18)`.
    pub interval_scale: f32,
}
/// Ground_801C28CC's loop bound.
pub const ITEM_RATE_KINDS: usize = 35;
/// Ground_801C2AE8's percent (retail 801C2B3C: `@842`).
const PERCENT: f32 = 0.01;

/// `Ground_801C28CC` (0x801C28CC) and `Ground_801C2AE8` (0x801C2AE8): the
/// item weights and spawn interval scale of the StageParam row for
/// `stage_id` (a StKind).
pub fn read_item_rates(archive: &Archive, stage_id: i32) -> ReadResult<ItemRates> {
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
    let row = selected.ok_or_else(|| error("missing stage item parameters"))?;
    let mut weights = [0; ITEM_RATE_KINDS];
    for (j, weight) in weights.iter_mut().enumerate() {
        let offset = 2 * j as u32;
        *weight =
            i32::from(r.s16(root + 0x6A + offset)?) * i32::from(r.s16(row + 0x1A + offset)?);
    }
    // retail 801C2B5C..801C2B64: two fmuls, then their product.
    let ground = PERCENT * f32::from(r.s16(root + 0x68)?);
    let stage = PERCENT * f32::from(r.s16(row + 0x18)?);
    Ok(ItemRates {
        weights,
        interval_scale: ground * stage,
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

/// grVe_StageData / grDatFiles_801C6038: GrVe.dat. grVe_StageCallbacks[5]
/// carries flags 0xC0000000, so map 5 selects the lights and fog.
pub fn read_venom(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Venom, 5)
}
/// `grVenom_YakumonoParam` (grvenom.c:38-50).
pub fn read_venom_parameters(archive: &Archive) -> ReadResult<crate::venom::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    Ok(crate::venom::Parameters {
        first_wait: [f(0x0)?, f(0x4)?],
        wait: [f(0x8)?, f(0xC)?],
        wolfen_chance: f(0x10)?,
        maneuver_wait: f(0x2C)?,
        ship_scale: f(0x34)?,
    })
}
/// grZe_StageData / grDatFiles_801C6038: GrZe.dat. grZe_callbacks[6]
/// carries flags 0xC0000000, so map 6 selects the lights and fog.
pub fn read_zebes(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Zebes, 6)
}
/// `grZe_YakumonoParam` (grzebes.c:83-119) as the asm reads it.
pub fn read_zebes_parameters(archive: &Archive) -> ReadResult<crate::zebes::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let f = |at: u32| r.f32(p + at);
    let mut acid_levels = [crate::zebes::AcidLevel::default(); crate::zebes::ACID_LEVELS];
    for (index, level) in acid_levels.iter_mut().enumerate() {
        let at = p + 0xA0 + 8 * index as u32;
        *level = crate::zebes::AcidLevel {
            base: r.s16(at)?,
            delay_min: r.s16(at + 2)?,
            delay_max: r.s16(at + 4)?,
            level: r.s16(at + 6)?,
        };
    }
    Ok(crate::zebes::Parameters {
        island_wait: [f(0x0)?, f(0x4)?],
        pulse_wait: [f(0x8)?, f(0xC)?],
        regrowth_step: r.s32(p + 0x10)?,
        acid_hit: required_link(archive, p + 0x2C)?,
        plate_max_offset: f(0x30)?,
        plate_max_speed: f(0x34)?,
        plate_spring: f(0x38)?,
        plate_damping: f(0x3C)?,
        plate_rest_speed: f(0x40)?,
        plate_rest_offset: f(0x44)?,
        plate_heal_interval: f(0x48)?,
        plate_break_damage: f(0x4C)?,
        plate_wait: [f(0x50)?, f(0x54)?],
        regrowth_size: [f(0x58)?, f(0x5C)?],
        bubble_wait: [f(0x60)?, f(0x64)?],
        growth: f(0x68)?,
        attract_far: f(0x6C)?,
        attract_near: f(0x70)?,
        link: f(0x74)?,
        attraction: f(0x78)?,
        max_pull: f(0x80)?,
        sink: f(0x84)?,
        max_speed: f(0x88)?,
        acid_top: f(0x8C)?,
        acid_bottom: f(0x90)?,
        acid_spread: f(0x94)?,
        acid_max_speed: f(0x98)?,
        acid_acceleration: f(0x9C)?,
        acid_levels,
    })
}
/// grKr_StageData / grDatFiles_801C6038: GrKr.dat. grKr_StageCallbacks[3]
/// carries flags 0xC0000000, so map 3 selects the lights and fog.
pub fn read_kraid(archive: &Archive) -> ReadResult<StageDesc> {
    read_stage(archive, GrKind::Kraid, 3)
}
/// `grKraid_YakumonoParam` (grkraid.c:14-23) as the asm reads it.
pub fn read_kraid_parameters(archive: &Archive) -> ReadResult<crate::kraid::Parameters> {
    let p = public(archive, "yakumono_param")?;
    let r = archive.reader();
    let mut kraid_pos_x = [0.0; 6];
    for (i, x) in kraid_pos_x.iter_mut().enumerate() {
        *x = r.f32(p + 0x1C + 4 * i as u32)?;
    }
    Ok(crate::kraid::Parameters {
        map_time_min: r.s32(p)?,
        map_time_max: r.s32(p + 0x4)?,
        map_time_acl: r.s32(p + 0x8)?,
        map_rot_spd_min: r.f32(p + 0xC)?,
        map_rot_spd_max: r.f32(p + 0x10)?,
        kraid_wait_time: r.s32(p + 0x14)?,
        kraid_wait_time_add: r.s32(p + 0x18)?,
        kraid_pos_x,
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
