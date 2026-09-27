//! The gameplay camera at the savestate boundary: `game_camera`
//! (0x80452C68), the CmSubject list (cm_804D6468) and the magnifier's
//! per-player state (ifMagnify_804A1DE0), read from saved MEM1.
use super::{float, vector, word};
use anyhow::{bail, ensure, Result};
use hsd_types::Vec2;
use melee_cm::{Extents, GameCamera, Subject, SubjectState, Transform};

const GAME_CAMERA: u32 = 0x8045_2C68;
const CAMERA_BYTES: usize = 0x39C;
const SUBJECT_LIST_HEAD: u32 = 0x804D_6468;
const SUBJECT_BYTES: usize = 0x6C;
const MAGNIFY: u32 = 0x804A_1DE0;
/// ifMagnify.player[slot].state (the bitfield byte), stride 0x10.
const MAGNIFY_PLAYER_STATE: u32 = 0x14 + 0xC;
/// ifMagnifyPlayer.state.is_offscreen, the first (most significant) bit.
const MAGNIFIED: u8 = 0x80;

fn half(raw: &[u8], at: usize) -> i16 {
    i16::from_be_bytes(raw[at..at + 2].try_into().unwrap())
}
fn transform(raw: &[u8], at: usize) -> Transform {
    Transform {
        interest: vector(raw, at),
        target_interest: vector(raw, at + 0xC),
        position: vector(raw, at + 0x18),
        target_position: vector(raw, at + 0x24),
        fov: float(raw, at + 0x30),
        target_fov: float(raw, at + 0x34),
    }
}
fn extents(raw: &[u8], at: usize) -> Extents {
    Extents {
        left: float(raw, at),
        right: float(raw, at + 4),
        top: float(raw, at + 8),
        bottom: float(raw, at + 0xC),
        radius: float(raw, at + 0x10),
    }
}

/// struct Camera (cm/types.h, 0x39C bytes), standard mode only.
pub fn decode_camera(raw: &[u8]) -> Result<GameCamera> {
    ensure!(raw.len() == CAMERA_BYTES, "Camera dump length");
    ensure!(word(raw, 4) == 0, "only the standard camera mode is ported");
    let mut camera = GameCamera::new();
    camera.near = float(raw, 0xC);
    camera.far = float(raw, 0x10);
    camera.transform = transform(raw, 0x14);
    camera.transform_copy = transform(raw, 0x4C);
    camera.translation = Vec2::new(float(raw, 0x84), float(raw, 0x88));
    for (kind, frames) in camera.quake.frames_left.iter_mut().enumerate() {
        *frames = word(raw, 0x8C + 4 * kind) as i32;
    }
    camera.quake.looping = word(raw, 0xA0) != 0;
    camera.quake.offset = Vec2::new(float(raw, 0xA4), float(raw, 0xA8));
    camera.quake.scale = float(raw, 0xAC);
    camera.bounds_width_average = float(raw, 0x2B0);
    camera.bounds_width_sum = float(raw, 0x2B4);
    camera.bounds_width_samples = half(raw, 0x2B8);
    camera.zoom_hold = half(raw, 0x2BA);
    camera.zoom = float(raw, 0x2BC);
    camera.zoom_distance = float(raw, 0x2C0);
    // +0x399 bit 2 (MSB-first bitfield).
    camera.lock_depth = raw[0x399] & 0x20 != 0;
    Ok(camera)
}

/// CmSubject (cm/types.h, 0x6C bytes).
pub fn decode_subject(raw: &[u8]) -> Result<Subject> {
    ensure!(raw.len() == SUBJECT_BYTES, "CmSubject dump length");
    let state = match word(raw, 8) {
        0 => SubjectState::Active,
        1 => SubjectState::Inactive,
        2 => SubjectState::Auto,
        other => bail!("CmSubjectState {other}"),
    };
    Ok(Subject {
        state,
        on_ledge: raw[0xC] & 0x80 != 0,
        force_inactive: raw[0xC] & 0x40 != 0,
        was_framed: raw[0xC] & 0x20 != 0,
        state_timer: half(raw, 0xE),
        position: vector(raw, 0x10),
        bone_position: vector(raw, 0x1C),
        facing: float(raw, 0x28),
        extents: extents(raw, 0x2C),
        target_extents: extents(raw, 0x40),
    })
}

/// The ifMagnify player state byte of `slot`, from the 0xF0-byte struct.
pub fn magnified(magnify: &[u8], slot: usize) -> bool {
    magnify[MAGNIFY_PLAYER_STATE as usize + 0x10 * slot] & MAGNIFIED != 0
}

/// The saved camera and its subjects in list order (newest first).
pub(crate) fn restore(saved: &super::SavedPose) -> Result<(GameCamera, Vec<Subject>)> {
    let camera = decode_camera(saved.bytes(GAME_CAMERA, CAMERA_BYTES))?;
    let mut subjects = Vec::new();
    let mut address = word(saved.bytes(SUBJECT_LIST_HEAD, 4), 0);
    while address != 0 {
        ensure!(subjects.len() < 64, "unterminated CmSubject list");
        let raw = saved.bytes(address, SUBJECT_BYTES);
        subjects.push(decode_subject(raw)?);
        address = word(raw, 4);
    }
    Ok((camera, subjects))
}

/// Whether the magnifier drew `slot` on the last display pass.
pub(crate) fn restore_magnified(saved: &super::SavedPose, slot: usize) -> bool {
    magnified(saved.bytes(MAGNIFY, 0xF0), slot)
}
