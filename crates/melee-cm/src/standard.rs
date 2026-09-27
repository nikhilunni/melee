//! The standard gameplay camera, Camera_8002B3D4 and the functions it calls.
//! Every fused multiply-add below is cited from `asm.py <symbol> --fused`.
use crate::camera::{GameCamera, Transform};
use crate::params::{DESCRIPTION, TRACKING_WEIGHTS, TUNING, WORLD_FORWARD};
use crate::stage::StageCamera;
use crate::subject::Subject;
use gekko_math::fma::{fmadds, fmsubs, fnmsubs};
use gekko_math::msl::{sqrtf, tanf};
use hsd_anim::cobj::DEGREES_TO_RADIANS;
use hsd_types::Vec3;
use melee_lb::trigf::atan2f;
use melee_lb::vector::{normalize, rotate_about, Axis};

/// CameraBounds: the framed rectangle and eye distance for one transform.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bounds {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub subjects: i32,
    pub depth: f32,
}

/// Camera_80029124's result bits (CAM_BOUNDS_OUTSIDE_*).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutsideBounds(u8);

impl OutsideBounds {
    const TOP: Self = Self(1);
    const BOTTOM: Self = Self(2);
    const LEFT: Self = Self(4);
    const RIGHT: Self = Self(8);
    fn empty() -> Self {
        Self(0)
    }
    fn is_empty(self) -> bool {
        self.0 == 0
    }
    fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOrAssign for OutsideBounds {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Half the default frame (Camera_8002958C with no subjects).
const EMPTY_FRAME_HALF_SIZE: f32 = 40.0;
/// Camera_8002958C: the bottom margin grows from 10 to 400 units as the eye
/// moves from 80 to 5000 units away.
const NEAR_DEPTH: f32 = 80.0;
const FAR_DEPTH: f32 = 5000.0;
const DEPTH_RANGE: f32 = 4920.0;
const BOTTOM_MARGIN_GROWTH: f32 = 390.0;
const BOTTOM_MARGIN: f32 = 10.0;
/// Camera_80029AAC's rate clamp and the spread used without subjects.
const MAX_LERP: f32 = 1.0;
const MIN_LERP: f32 = 0.0001;
const NO_SUBJECT_SPREAD: f32 = 99999.0;
/// Camera_8002A768: a frustum corner must point this far down to meet z = 0.
const CORNER_MIN_DOWNWARD: f32 = -0.001;

/// Camera_80029124 (0x80029124) with a distance of 0: which bounds the point
/// is outside of. The bottom bound is raised to the stage floor.
fn outside_bounds(position: Vec3, stage: &StageCamera) -> OutsideBounds {
    let mut flags = OutsideBounds::empty();
    let bottom = stage.framing_floor();
    if position.x < stage.left() {
        flags |= OutsideBounds::LEFT;
    }
    if position.x > stage.right() {
        flags |= OutsideBounds::RIGHT;
    }
    if position.y > stage.top() {
        flags |= OutsideBounds::TOP;
    }
    if position.y < bottom {
        flags |= OutsideBounds::BOTTOM;
    }
    flags
}

/// Camera_8002958C's clamp of a point into the camera bounds.
fn clamp_to_bounds(position: &mut Vec3, stage: &StageCamera) {
    let flags = outside_bounds(*position, stage);
    if flags.is_empty() {
        return;
    }
    if flags.contains(OutsideBounds::LEFT) {
        position.x = stage.left();
    }
    if flags.contains(OutsideBounds::RIGHT) {
        position.x = stage.right();
    }
    if flags.contains(OutsideBounds::TOP) {
        position.y = stage.top();
    }
    if flags.contains(OutsideBounds::BOTTOM) {
        position.y = stage.framing_floor();
    }
}

/// Camera_8002958C (0x8002958C): the rectangle framing every counted
/// subject, each widened by its extents times the tracking ratio.
pub(crate) fn frame_subjects(
    subjects: &mut [&mut Subject],
    transform: &Transform,
    stage: &StageCamera,
) -> Bounds {
    let mut count = 0;
    for subject in subjects.iter_mut() {
        if subject.is_framed(stage) {
            count += 1;
        }
    }
    let (mut x_min, mut y_min) = (f32::MAX, f32::MAX);
    let (mut x_max, mut y_max) = (-f32::MAX, -f32::MAX);
    if count != 0 {
        let weight = TRACKING_WEIGHTS.get(count as usize).copied().unwrap_or(1.0);
        let multiplier = weight * stage.track_ratio;
        count = 0;
        for subject in subjects.iter_mut() {
            if !subject.is_framed(stage) {
                continue;
            }
            count += 1;
            let mut base = subject.position;
            clamp_to_bounds(&mut base, stage);
            let ext = subject.extents;
            // retail 0x80029718 / 0x800297CC / 0x80029880 / 0x80029934:
            // fmadds per extent; the test point keeps the other axis from
            // the previous test, clamped or not.
            let mut test = subject.position;
            test.x = fmadds(ext.left, multiplier, base.x);
            clamp_to_bounds(&mut test, stage);
            x_min = if test.x < x_min { test.x } else { x_min };
            x_max = if test.x > x_max { test.x } else { x_max };
            test.x = fmadds(ext.right, multiplier, base.x);
            clamp_to_bounds(&mut test, stage);
            x_min = if test.x < x_min { test.x } else { x_min };
            x_max = if test.x > x_max { test.x } else { x_max };
            test.y = fmadds(ext.bottom, multiplier, base.y);
            clamp_to_bounds(&mut test, stage);
            y_min = if test.y < y_min { test.y } else { y_min };
            y_max = if test.y > y_max { test.y } else { y_max };
            test.y = fmadds(ext.top, multiplier, base.y);
            clamp_to_bounds(&mut test, stage);
            y_min = if test.y < y_min { test.y } else { y_min };
            y_max = if test.y > y_max { test.y } else { y_max };
        }
    }
    if count == 0 {
        let (cx, cy) = (stage.offset_x, stage.offset_y);
        x_min = cx - EMPTY_FRAME_HALF_SIZE;
        y_min = cy - EMPTY_FRAME_HALF_SIZE;
        x_max = EMPTY_FRAME_HALF_SIZE + cx;
        y_max = EMPTY_FRAME_HALF_SIZE + cy;
    }
    let depth = if transform.position.z < 0.0 {
        -transform.position.z
    } else {
        transform.position.z
    };
    let depth_factor = if depth < NEAR_DEPTH {
        0.0
    } else if depth > FAR_DEPTH {
        1.0
    } else {
        (depth - NEAR_DEPTH) / DEPTH_RANGE
    };
    Bounds {
        x_min,
        // retail 0x80029A64: fmadds.
        y_min: y_min - fmadds(BOTTOM_MARGIN_GROWTH, depth_factor, BOTTOM_MARGIN),
        x_max,
        y_max,
        subjects: count,
        depth,
    }
}

/// The larger side of the framed rectangle.
fn spread(bounds: &Bounds) -> f32 {
    let dx = bounds.x_max - bounds.x_min;
    let dy = bounds.y_max - bounds.y_min;
    if dx > dy {
        dx
    } else {
        dy
    }
}

/// Camera_80029AAC (0x80029AAC): move the interest toward its target at a
/// rate that grows with the spread and shrinks with the zoom.
pub(crate) fn follow_interest(bounds: &Bounds, transform: &mut Transform, speed: f32, zoom: f32) {
    let spread = if bounds.subjects != 0 {
        spread(bounds)
    } else {
        NO_SUBJECT_SPREAD
    };
    let offset_x = transform.target_interest.x - transform.interest.x;
    let offset_y = transform.target_interest.y - transform.interest.y;
    let t = &TUNING;
    let rate = if spread > t.follow_spread_max {
        t.follow_rate_wide
    } else if spread < t.follow_spread_min {
        t.follow_rate_narrow
    } else {
        let ratio = (spread - t.follow_spread_min) / (t.follow_spread_max - t.follow_spread_min);
        // retail 0x80029B50: fmadds.
        fmadds(
            t.follow_rate_wide - t.follow_rate_narrow,
            ratio,
            t.follow_rate_narrow,
        )
    };
    let zoom_scale = if zoom > MIN_LERP { 1.0 / zoom } else { 1000.0 };
    let lerp = (zoom_scale * (rate * speed)).clamp(MIN_LERP, MAX_LERP);
    // retail 0x80029BAC / 0x80029BB8: fmadds.
    transform.interest.x = fmadds(offset_x, lerp, transform.interest.x);
    transform.interest.y = fmadds(offset_y, lerp, transform.interest.y);
}

/// Camera_80029BC4 (0x80029BC4): the eye distance that fits the rectangle,
/// clamped to the stage's depth range. Stored in `bounds.depth`.
pub(crate) fn fit_depth(bounds: &mut Bounds, transform: &Transform, stage: &StageCamera) {
    let vertical = (bounds.y_max - bounds.y_min) / tanf(DEGREES_TO_RADIANS * transform.target_fov);
    let horizontal = (bounds.x_max - bounds.x_min)
        / (DESCRIPTION.aspect * tanf(DEGREES_TO_RADIANS * transform.target_fov));
    let mut depth = if horizontal > vertical {
        horizontal
    } else {
        vertical
    };
    if depth < stage.min_depth {
        depth = stage.min_depth;
    }
    if depth > stage.max_depth {
        depth = stage.max_depth;
    }
    bounds.depth = depth;
}

/// Camera_80029C88 (0x80029C88): move the eye toward its target.
pub(crate) fn follow_position(transform: &mut Transform, speed: f32) {
    let target = transform.target_position;
    let position = transform.position;
    let (dx, dy, dz) = (
        target.x - position.x,
        target.y - position.y,
        target.z - position.z,
    );
    let mut scale = TUNING.position_follow_rate * speed;
    if scale > 1.0 {
        scale = 1.0;
    }
    // retail 0x80029CD4 / 0x80029CE0 / 0x80029CEC: fmadds.
    transform.position.x = fmadds(dx, scale, position.x);
    transform.position.y = fmadds(dy, scale, position.y);
    transform.position.z = fmadds(dz, scale, position.z);
}

/// Camera_80029CF8 (0x80029CF8): aim the target interest and eye so the
/// rectangle fills the view, pitched and yawed toward the stage centre.
pub(crate) fn aim(bounds: &Bounds, transform: &mut Transform, stage: &StageCamera) {
    let t = &TUNING;
    let spread = spread(bounds);
    let low_bias = if spread > t.bias_spread_max {
        t.low_bias_wide
    } else if spread < t.bias_spread_min {
        t.low_bias_narrow
    } else {
        let ratio = (spread - t.bias_spread_min) / (t.bias_spread_max - t.bias_spread_min);
        // retail 0x80029DB0: fmadds.
        fmadds(
            t.low_bias_wide - t.low_bias_narrow,
            ratio,
            t.low_bias_narrow,
        )
    };
    let centre_y = stage.offset_y;
    let height_sum = (bounds.y_min - centre_y) + (bounds.y_max - centre_y);
    // retail 0x80029DCC: fmadds.
    let framed_y = fmadds(height_sum, 0.5 - low_bias, centre_y);
    let mut pitch = -(DEGREES_TO_RADIANS * ((framed_y + t.pitch_height_bias) * stage.pitch_scale));
    let pitch_max = DEGREES_TO_RADIANS * t.pitch_max_degrees;
    if pitch > pitch_max {
        pitch = pitch_max;
    }
    let pitch_min = DEGREES_TO_RADIANS * t.pitch_min_degrees;
    if pitch < pitch_min {
        pitch = pitch_min;
    }
    pitch += stage.pan_radians();
    let fov = DEGREES_TO_RADIANS * transform.fov;
    // retail 0x80029E48 fmadds / 0x80029E7C fmsubs.
    let fov_up = fmadds(0.5, fov, pitch);
    let fov_down = fmsubs(0.5, fov, pitch);
    let tan_up = tanf(fov_up);
    let tan_down = tanf(fov_down);
    let distance_y = (bounds.y_max - bounds.y_min) / (tan_up + tan_down);
    let offset_y = distance_y * tanf(pitch);
    // retail 0x80029EE0: fnmsubs.
    transform.target_interest.y = offset_y + fnmsubs(distance_y, tan_up, bounds.y_max);

    let centre_x = 0.5 * (bounds.x_min + bounds.x_max);
    let mut yaw = -(DEGREES_TO_RADIANS * ((centre_x - stage.offset_x) * stage.yaw_scale));
    let yaw_max = DEGREES_TO_RADIANS * t.yaw_max_degrees;
    if yaw > yaw_max {
        yaw = yaw_max;
    }
    let yaw_min = DEGREES_TO_RADIANS * t.yaw_min_degrees;
    if yaw < yaw_min {
        yaw = yaw_min;
    }
    // retail 0x80029F70 fmsubs / 0x80029FA4 fmadds.
    let fov_right = fmsubs(0.5, DEGREES_TO_RADIANS * transform.fov, yaw);
    let fov_left = fmadds(0.5, DEGREES_TO_RADIANS * transform.fov, yaw);
    let tan_right = DESCRIPTION.aspect * tanf(fov_right);
    let tan_left = DESCRIPTION.aspect * tanf(fov_left);
    let distance_x = (bounds.x_max - bounds.x_min) / (tan_right + tan_left);
    let offset_x = DESCRIPTION.aspect * (distance_x * tanf(yaw));
    // retail 0x8002A01C: fnmsubs.
    transform.target_interest.x = fnmsubs(distance_x, tan_right, bounds.x_max) - offset_x;
    transform.target_interest.z = 0.0;
    let mut depth = if distance_y > distance_x {
        distance_y
    } else {
        distance_x
    };
    if depth < stage.min_depth {
        depth = stage.min_depth;
    }
    if depth > stage.max_depth {
        depth = stage.max_depth;
    }
    transform.target_position.x = transform.target_interest.x + offset_x;
    transform.target_position.y = transform.target_interest.y - offset_y;
    transform.target_position.z = transform.target_interest.z + depth;
}

/// Where a view ray through a frustum corner meets the z = 0 plane.
fn frustum_corner(
    transform: &Transform,
    pitch_offset: f32,
    yaw_offset: f32,
    pitch: f32,
    yaw: f32,
) -> Option<Vec3> {
    let mut corner = rotate_about(WORLD_FORWARD, Axis::X, pitch_offset);
    corner = rotate_about(corner, Axis::Y, yaw_offset);
    corner.x *= DESCRIPTION.aspect;
    corner = normalize(corner);
    corner = rotate_about(corner, Axis::X, pitch);
    corner = rotate_about(corner, Axis::Y, yaw);
    if corner.z < CORNER_MIN_DOWNWARD {
        let scale = -transform.target_position.z / corner.z;
        let p = transform.target_position;
        Some(Vec3::new(
            corner.x * scale + p.x,
            corner.y * scale + p.y,
            corner.z * scale + p.z,
        ))
    } else {
        None
    }
}

/// Camera_8002A768 (0x8002A768) with arg1 = 0: shift the target eye and
/// interest so the view's footprint on z = 0 stays inside the camera bounds.
pub(crate) fn keep_view_in_bounds(transform: &mut Transform, stage: &StageCamera) {
    let half_fov = 0.5 * (DEGREES_TO_RADIANS * transform.target_fov);
    let view = normalize(Vec3::new(
        transform.target_interest.x - transform.target_position.x,
        transform.target_interest.y - transform.target_position.y,
        transform.target_interest.z - transform.target_position.z,
    ));
    let pitch = atan2f(view.y, -view.z);
    let yaw = atan2f(-view.x, -view.z);
    let top_left = frustum_corner(transform, half_fov, half_fov, pitch, yaw);
    let top_right = frustum_corner(transform, half_fov, -half_fov, pitch, yaw);
    let bottom_right = frustum_corner(transform, -half_fov, half_fov, pitch, yaw);
    let bottom_left = frustum_corner(transform, -half_fov, -half_fov, pitch, yaw);

    let (mut left, mut bottom, mut right, mut top) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut hit = OutsideBounds::empty();
    // Each corner is tested against the two sides it can cross, in retail order.
    let mut past_left = |x: f32, hit: &mut OutsideBounds| {
        if x < stage.left() {
            let overlap = stage.left() - x;
            if overlap > left {
                left = overlap;
                *hit |= OutsideBounds::LEFT;
            }
        }
    };
    if let Some(c) = top_left {
        past_left(c.x, &mut hit);
    }
    let mut above_top = |y: f32, hit: &mut OutsideBounds| {
        if y > stage.top() {
            let overlap = stage.top() - y;
            if overlap < top {
                top = overlap;
                *hit |= OutsideBounds::TOP;
            }
        }
    };
    if let Some(c) = top_left {
        above_top(c.y, &mut hit);
    }
    let mut past_right = |x: f32, hit: &mut OutsideBounds| {
        if x > stage.right() {
            let overlap = stage.right() - x;
            if overlap < right {
                right = overlap;
                *hit |= OutsideBounds::RIGHT;
            }
        }
    };
    if let Some(c) = top_right {
        past_right(c.x, &mut hit);
        above_top(c.y, &mut hit);
    }
    let mut below_bottom = |y: f32, hit: &mut OutsideBounds| {
        if y < stage.bottom() {
            let overlap = stage.bottom() - y;
            if overlap > bottom {
                bottom = overlap;
                *hit |= OutsideBounds::BOTTOM;
            }
        }
    };
    if let Some(c) = bottom_right {
        past_left(c.x, &mut hit);
        below_bottom(c.y, &mut hit);
    }
    if let Some(c) = bottom_left {
        past_right(c.x, &mut hit);
        below_bottom(c.y, &mut hit);
    }
    if hit.is_empty() {
        return;
    }
    let mut correction = Vec3::ZERO;
    if hit.contains(OutsideBounds::LEFT) && hit.contains(OutsideBounds::RIGHT) {
        correction.x = 0.5 * (left + right);
    } else if hit.contains(OutsideBounds::LEFT) {
        correction.x = left;
    } else if hit.contains(OutsideBounds::RIGHT) {
        correction.x = right;
    }
    if hit.contains(OutsideBounds::TOP) && hit.contains(OutsideBounds::BOTTOM) {
        correction.y = 0.5 * (bottom + top);
    } else if hit.contains(OutsideBounds::TOP) {
        correction.y = top;
    } else if hit.contains(OutsideBounds::BOTTOM) {
        correction.y = bottom;
    }
    // The C scales the correction by 1.0 (exact) before both lbVector_Adds.
    let add = |v: &mut Vec3| {
        v.x += correction.x;
        v.y += correction.y;
        v.z += correction.z;
    };
    add(&mut transform.target_position);
    add(&mut transform.target_interest);
}

/// Camera_8002B3D4's `update_transform`: frame, blend the field of view, fit
/// the depth and aim.
fn update_transform(
    subjects: &mut [&mut Subject],
    transform: &mut Transform,
    stage: &StageCamera,
    lock_depth: bool,
) -> Bounds {
    let mut bounds = frame_subjects(subjects, transform, stage);
    // cm_803BCCA0.x40, which match setup set to the stage's field of view.
    transform.target_fov = stage.fov;
    // retail 0x8002B43C: fmadds.
    transform.fov = fmadds(
        transform.target_fov - transform.fov,
        TUNING.fov_follow_rate,
        transform.fov,
    );
    fit_depth(&mut bounds, transform, stage);
    assert!(
        !lock_depth,
        "Camera_80030AF8: the player-depth lock (x399_b2) is not ported"
    );
    aim(&bounds, transform, stage);
    keep_view_in_bounds(transform, stage);
    bounds
}

impl GameCamera {
    /// Camera_8002B3D4 (0x8002B3D4): one standard-mode tick.
    pub fn update_standard(&mut self, subjects: &mut [&mut Subject], stage: &StageCamera) {
        // Camera_80030DF8.
        self.translation = hsd_types::Vec2::ZERO;
        for subject in subjects.iter_mut() {
            if subject.is_framed(stage) {
                subject.smooth_extents();
            }
        }
        // Camera_8002B0E0 adjusts the zoom only in single-player modes.
        assert!(
            !self.single_player_zoom,
            "Camera_8002B0E0: single-player zoom is not ported"
        );
        let bounds = update_transform(subjects, &mut self.transform, stage, self.lock_depth);
        let bounds_copy =
            update_transform(subjects, &mut self.transform_copy, stage, self.lock_depth);
        if self.zoom == 1.0 {
            let t = &self.transform;
            let dx = t.target_position.x - t.target_interest.x;
            let dy = t.target_position.y - t.target_interest.y;
            let dz = t.target_position.z - t.target_interest.z;
            // retail 0x8002B590..0x8002B5A4: (dx^2 + dy^2), then dz^2 + that.
            self.zoom_distance = sqrtf(dz * dz + (dx * dx + dy * dy));
        } else {
            unimplemented!("Camera_8002B1F8: zoomed single-player framing");
        }
        follow_interest(&bounds, &mut self.transform, stage.track_smooth, self.zoom);
        follow_position(&mut self.transform, stage.track_smooth);
        follow_interest(
            &bounds_copy,
            &mut self.transform_copy,
            stage.track_smooth,
            self.zoom,
        );
        follow_position(&mut self.transform_copy, stage.track_smooth);
        // A stopped Loop quake clears `quake.looping`; its animation owner
        // destroys the model (Camera_UpdateQuakes' HSD_GObjPLink_80390228).
        let _ = self.quake.count_down();
        self.translation = self.quake.apply(&bounds, &self.transform, self.zoom, stage);
        self.update_average_bounds_width(stage);
    }

    /// Camera_8002F3AC (0x8002F3AC), at match setup: one update, then jump
    /// straight to the targets.
    pub fn snap_standard(&mut self, subjects: &mut [&mut Subject], stage: &StageCamera) {
        self.update_standard(subjects, stage);
        for t in [&mut self.transform, &mut self.transform_copy] {
            t.position = t.target_position;
            t.interest = t.target_interest;
            t.fov = t.target_fov;
        }
    }

    /// Camera_8002B3D4's `update_avg_bounds_width`.
    fn update_average_bounds_width(&mut self, stage: &StageCamera) {
        if self.bounds_width_samples > AVERAGE_WINDOW {
            self.bounds_width_sum = self.bounds_width_average;
            self.bounds_width_samples = 1;
        }
        let left = stage.left();
        self.bounds_width_sum += stage.right() - left;
        self.bounds_width_samples += 1;
        self.bounds_width_average = self.bounds_width_sum / f32::from(self.bounds_width_samples);
    }
}

/// Samples after which the running bounds width restarts from its average.
const AVERAGE_WINDOW: i16 = 1000;
