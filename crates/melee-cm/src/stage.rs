//! The stage's camera description, `stage_info.cam_info` and the blast zone
//! (gr/stage.c getters), as Ground_801C0800 / Ground_801C39C0 /
//! Ground_801C3BB4 fill them from grGroundParam and the stage markers.

/// An axis-aligned rectangle relative to the camera offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageCamera {
    /// cam_info.cam_bounds, relative to `offset`.
    pub bounds: Rect,
    /// cam_x_offset / cam_y_offset (marker 0x94).
    pub offset_x: f32,
    pub offset_y: f32,
    /// cam_vertical_tilt (grGroundParam +0x8): match setup copies it over the
    /// tuning table's field of view (Camera_80030730), so it is the standard
    /// mode's field of view in degrees.
    pub fov: f32,
    /// cam_pan_degrees (grGroundParam +0x14).
    pub pan_degrees: f32,
    /// cam_info.x20 (grGroundParam +0x1C): yaw per unit of horizontal offset.
    pub yaw_scale: f32,
    /// cam_info.x24 (grGroundParam +0x18): pitch per unit of vertical offset.
    pub pitch_scale: f32,
    /// cam_track_ratio (grGroundParam +0x20).
    pub track_ratio: f32,
    /// cam_fixed_zoom (grGroundParam +0x24).
    pub fixed_zoom: f32,
    /// cam_track_smooth (grGroundParam +0x28).
    pub track_smooth: f32,
    /// cam_zoom_rate (grGroundParam +0xC): the nearest eye distance.
    pub min_depth: f32,
    /// cam_max_depth (grGroundParam +0x10).
    pub max_depth: f32,
    /// stage_info.blast_zone, relative to `offset`.
    pub blast_zone: Rect,
    /// stage_info.x724 (Ground_801C4368): the rising floor Brinstar's acid
    /// moves; -10000 elsewhere.
    pub floor: f32,
    /// Camera_8002AF68's stage-specific lowest eye height (Castle, Corneria,
    /// Zebes, Garden, Kinoko Route, Home-Run); `-F32_MAX` elsewhere.
    pub min_eye_height: f32,
}

/// `Stage_GetCamPanAngleRadians`'s literal 0.0174532923847F.
const DEGREES_TO_RADIANS: f32 = 0.017_453_292;

impl StageCamera {
    /// Ground_801BFFB0's defaults (ground.c:245-260) before a stage installs
    /// its own. Ground_801BFFB0 does not set the track ratio, fixed zoom and
    /// smoothing; this fixture value uses the neutral 1.0 for them.
    pub const GROUND_DEFAULTS: StageCamera = StageCamera {
        bounds: Rect {
            left: -170.0,
            right: 170.0,
            top: 120.0,
            bottom: -60.0,
        },
        offset_x: 0.0,
        offset_y: 0.0,
        fov: 30.0,
        pan_degrees: -10.0,
        yaw_scale: 0.2,
        pitch_scale: 0.2,
        track_ratio: 1.0,
        fixed_zoom: 1.0,
        track_smooth: 1.0,
        min_depth: 82.0,
        max_depth: 1000.0,
        blast_zone: Rect {
            left: -99999.0,
            right: 99999.0,
            top: 99999.0,
            bottom: -99999.0,
        },
        floor: -10000.0,
        min_eye_height: -f32::MAX,
    };

    /// Stage_GetCamBoundsLeftOffset and its three siblings.
    pub fn left(&self) -> f32 {
        self.bounds.left + self.offset_x
    }
    pub fn right(&self) -> f32 {
        self.bounds.right + self.offset_x
    }
    pub fn top(&self) -> f32 {
        self.bounds.top + self.offset_y
    }
    pub fn bottom(&self) -> f32 {
        self.bounds.bottom + self.offset_y
    }
    /// Stage_GetBlastZone*Offset.
    pub fn blast_left(&self) -> f32 {
        self.blast_zone.left + self.offset_x
    }
    pub fn blast_right(&self) -> f32 {
        self.blast_zone.right + self.offset_x
    }
    pub fn blast_top(&self) -> f32 {
        self.blast_zone.top + self.offset_y
    }
    pub fn blast_bottom(&self) -> f32 {
        self.blast_zone.bottom + self.offset_y
    }
    /// Stage_GetCamPanAngleRadians.
    pub fn pan_radians(&self) -> f32 {
        DEGREES_TO_RADIANS * self.pan_degrees
    }
    /// Ground_801C4368 + 1.0 in double, then the higher of it and the
    /// bottom bound: the lowest point a subject may frame.
    pub(crate) fn framing_floor(&self) -> f32 {
        let floor = (f64::from(self.floor) + 1.0) as f32;
        if self.bottom() > floor {
            self.bottom()
        } else {
            floor
        }
    }
}
