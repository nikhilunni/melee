//! `game_camera` (0x80452C68, struct Camera in cm/types.h): the state the
//! gameplay camera carries between ticks.
use crate::params::DESCRIPTION;
use crate::quake::Quake;
use hsd_types::{Vec2, Vec3};

/// CameraTransformState: where the camera looks and the targets it eases to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub interest: Vec3,
    pub target_interest: Vec3,
    pub position: Vec3,
    pub target_position: Vec3,
    /// Vertical field of view, degrees.
    pub fov: f32,
    pub target_fov: f32,
}

impl Transform {
    /// Camera_Init: both positions from the description's WObjs.
    pub const INITIAL: Transform = Transform {
        interest: DESCRIPTION.interest,
        target_interest: DESCRIPTION.interest,
        position: DESCRIPTION.eye,
        target_position: DESCRIPTION.eye,
        fov: DESCRIPTION.fov,
        target_fov: DESCRIPTION.fov,
    };
}

/// CameraType. The standard and fixed modes are ported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Standard = 0,
    /// CAMERA_FIXED (Camera_SetModeToFixed).
    Fixed = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameCamera {
    pub mode: Mode,
    pub near: f32,
    pub far: f32,
    pub transform: Transform,
    /// transform_copy: a second transform run through the same update. Only
    /// the unused second CObj (cm_804D6464) reads it.
    pub transform_copy: Transform,
    /// +0x84: the quake offset applied to the rendered eye and interest.
    pub translation: Vec2,
    pub quake: Quake,
    /// +0x2B0 / +0x2B4 / +0x2B8: running average of the bounds width
    /// (Camera_80030E10).
    pub bounds_width_average: f32,
    pub bounds_width_sum: f32,
    pub bounds_width_samples: i16,
    /// +0x2BA: single-player zoom hold timer.
    pub zoom_hold: i16,
    /// +0x2BC: single-player zoom; 1.0 is unzoomed.
    pub zoom: f32,
    /// +0x2C0: the unzoomed eye distance.
    pub zoom_distance: f32,
    /// gm_8016B41C: the scene allows the single-player zoom (Camera_8002B0E0).
    pub single_player_zoom: bool,
    /// +0x399 bit 2 (Camera_80030AF8): freeze framing while player 1 leaves
    /// the z plane.
    pub lock_depth: bool,
    /// The screen code installed (`Screen::Widescreen`); retail otherwise.
    /// Only the rendered CObj and the on-screen test read it: the framing
    /// uses the description's aspect (cm_803BCB64) whatever the CObj holds.
    pub screen: crate::view::Screen,
}

impl GameCamera {
    /// Camera_Init (0x80028E00 area): the state before the first update.
    pub fn new() -> Self {
        Self {
            mode: Mode::Standard,
            near: DESCRIPTION.near,
            far: DESCRIPTION.far,
            transform: Transform::INITIAL,
            transform_copy: Transform::INITIAL,
            translation: Vec2::ZERO,
            quake: Quake::default(),
            bounds_width_average: 0.0,
            bounds_width_sum: 0.0,
            bounds_width_samples: 0,
            zoom_hold: 0,
            zoom: 1.0,
            zoom_distance: -1.0,
            single_player_zoom: false,
            lock_depth: false,
            screen: crate::view::Screen::Standard,
        }
    }

    /// Ground_EnableMatchCamera (0x801C5800): the fixed mode where the
    /// stage's grGroundParam sets `x4C_fixed_cam`, else the standard one.
    pub fn enable_match_camera(&mut self, fixed: bool) {
        self.mode = if fixed { Mode::Fixed } else { Mode::Standard };
    }

    /// The mode's callback (cm_803BCB18): Camera_8002B3D4 or
    /// Camera_8002DDC4.
    pub fn update(&mut self, subjects: &mut [Option<&mut crate::Subject>], stage: &crate::StageCamera) {
        match self.mode {
            Mode::Standard => self.update_standard(subjects, stage),
            Mode::Fixed => self.update_fixed(stage),
        }
    }

    /// Camera_8002F3AC (0x8002F3AC): the mode's callback, then every
    /// transform at its targets.
    pub fn snap(&mut self, subjects: &mut [Option<&mut crate::Subject>], stage: &crate::StageCamera) {
        self.update(subjects, stage);
        for t in [&mut self.transform, &mut self.transform_copy] {
            t.position = t.target_position;
            t.interest = t.target_interest;
            t.fov = t.target_fov;
        }
    }

    /// Camera_80031144: the zoom, 1.0 outside single-player zooming.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Camera_80030E10: the average camera-bounds width, 10000 before the
    /// first sample.
    pub fn average_bounds_width(&self) -> f32 {
        if self.bounds_width_samples < 1 {
            return 10000.0;
        }
        self.bounds_width_average
    }
}

impl Default for GameCamera {
    fn default() -> Self {
        Self::new()
    }
}
