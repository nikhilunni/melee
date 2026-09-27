//! The camera's static `.data` tables (cm/camera.c), transcribed from the
//! retail DOL.
use hsd_anim::cobj::{PerspectiveCamera, Scissor, Viewport};
use hsd_types::Vec3;

/// `cm_803BCCA0` (0x803BCCA0), the standard mode's tuning. Only the fields
/// a versus match reads are named; the free and pause camera rows
/// (+0x64..+0xD0) are not ported.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    /// +0x08: added to the framed height before the pitch scale.
    pub pitch_height_bias: f32,
    /// +0x0C / +0x10: pitch clamp, degrees.
    pub pitch_max_degrees: f32,
    pub pitch_min_degrees: f32,
    /// +0x14 / +0x18: yaw clamp, degrees.
    pub yaw_max_degrees: f32,
    pub yaw_min_degrees: f32,
    /// +0x1C / +0x20: how far below the framed centre the interest sits,
    /// for a narrow and a wide spread, blended across +0x24..+0x28.
    pub low_bias_narrow: f32,
    pub low_bias_wide: f32,
    pub bias_spread_min: f32,
    pub bias_spread_max: f32,
    /// +0x2C / +0x30: interest follow rate for a narrow and a wide spread,
    /// blended across +0x34..+0x38.
    pub follow_rate_narrow: f32,
    pub follow_rate_wide: f32,
    pub follow_spread_min: f32,
    pub follow_spread_max: f32,
    /// +0x3C: eye follow rate.
    pub position_follow_rate: f32,
    /// +0x40: the standard field of view, degrees. Match setup replaces it
    /// with the stage's (Camera_80030730); see [`crate::StageCamera::fov`].
    pub fov: f32,
    /// +0x44: field-of-view follow rate.
    pub fov_follow_rate: f32,
    /// +0x54 / +0x58: quake scale (y, x) at the nearest depth.
    pub quake_near_y: f32,
    pub quake_near_x: f32,
    /// +0x5C / +0x60: quake scale (y, x) at the farthest depth.
    pub quake_far_y: f32,
    pub quake_far_x: f32,
}

pub const TUNING: Tuning = Tuning {
    pitch_height_bias: -30.0,
    pitch_max_degrees: 5.0,
    pitch_min_degrees: -7.0,
    yaw_max_degrees: 17.5,
    yaw_min_degrees: -17.5,
    low_bias_narrow: 0.0,
    low_bias_wide: f32::from_bits(0x3D8B_AC71), // 0.0682
    bias_spread_min: 60.0,
    bias_spread_max: 120.0,
    follow_rate_narrow: 0.05,
    follow_rate_wide: 0.1,
    follow_spread_min: 120.0,
    follow_spread_max: 900.0,
    position_follow_rate: 0.15,
    fov: 38.0,
    fov_follow_rate: 0.1,
    quake_near_y: 1.0,
    quake_near_x: 1.0,
    quake_far_y: 0.6,
    quake_far_x: 0.6,
};

/// `cm_803BCB9C`: the framing margin multiplier by subject count (index 0
/// unused); five or more subjects use 1.0.
pub const TRACKING_WEIGHTS: [f32; 5] = [0.0, 1.5, 1.32, 1.16, 1.0];

/// `cm_WorldForward` (0x803B73B8).
pub const WORLD_FORWARD: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: -1.0,
};

/// `cm_803BCB64`, the main camera's HSD_CameraDescPerspective, with the
/// eye (`cm_803BCB3C`) and interest (`cm_803BCB50`) WObj positions.
pub const DESCRIPTION: PerspectiveCamera = PerspectiveCamera {
    viewport: Viewport {
        xmin: 0.0,
        xmax: 640.0,
        ymin: 0.0,
        ymax: 480.0,
    },
    scissor: Scissor {
        left: 0,
        right: 640,
        top: 0,
        bottom: 480,
    },
    eye: Vec3 {
        x: 0.0,
        y: 40.241_425,
        z: 300.241,
    },
    interest: Vec3 {
        x: 0.0,
        y: 10.0,
        z: 0.0,
    },
    roll: 0.0,
    near: 0.1,
    far: 16384.0,
    fov: 30.0,
    aspect: 1.217_333_3,
};

/// The description's viewport as the s16 rectangle ApplyQuake reads.
pub const VIEWPORT_WIDTH: i32 = 640;
pub const VIEWPORT_HEIGHT: i32 = 480;
