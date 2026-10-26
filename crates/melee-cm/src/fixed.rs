//! The fixed mode (CAMERA_FIXED, Camera_8002DDC4): a stage whose
//! grGroundParam sets `x4C_fixed_cam` (Ground_EnableMatchCamera) looks from
//! one place at one point, whatever its subjects do.
use crate::camera::{GameCamera, Transform};
use crate::standard::Bounds;
use crate::StageCamera;
use gekko_math::{fma::fmadds, msl::sqrtf};
use hsd_types::Vec3;
use melee_lb::vector::{rotate_about, Axis};

/// `cm_803BCCA0` +0x74, +0x78 and +0x7C: the fixed mode's interest, eye and
/// field-of-view follow rates.
const INTEREST_RATE: f32 = 0.1;
const POSITION_RATE: f32 = 0.1;
const FOV_RATE: f32 = 0.1;
/// `Stage_80224CAC`'s 0.0174532923847F.
const DEGREES_TO_RADIANS: f32 = 0.017_453_292;
/// `Stage_80224CAC`'s look direction before its rotation.
const LOOK: Vec3 = Vec3::new(0.0, 0.0, -100.0);

/// `stage_info.cam_info`'s fixed view (Ground_801C392C, from grGroundParam
/// +0x50..+0x64).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FixedView {
    pub position: Vec3,
    pub fov: f32,
    /// Degrees about x and about y.
    pub vertical_angle: f32,
    pub horizontal_angle: f32,
}

impl FixedView {
    /// Stage_80224CAC (0x80224CAC): where the look from the fixed position,
    /// turned by the view's angles (lbVector_ApplyEulerRotation), meets the
    /// z = 0 plane (fneg, fdivs, two fmadds).
    pub fn interest(&self) -> Vec3 {
        let x = DEGREES_TO_RADIANS * self.vertical_angle;
        let y = DEGREES_TO_RADIANS * self.horizontal_angle;
        let look = rotate_about(rotate_about(rotate_about(LOOK, Axis::X, x), Axis::Y, y), Axis::Z, 0.0);
        let p = self.position;
        let t = p.z / -look.z;
        // retail 0x80224D50 / 0x80224D5C.
        Vec3::new(fmadds(look.x, t, p.x), fmadds(look.y, t, p.y), 0.0)
    }
}

/// `a + (b - a) * rate` as Camera_8002DDC4 eases (fsubs, then fmadds).
fn ease(current: f32, target: f32, rate: f32) -> f32 {
    fmadds(target - current, rate, current)
}

fn ease_vec(current: &mut Vec3, target: Vec3, rate: f32) {
    current.x = ease(current.x, target.x, rate);
    current.y = ease(current.y, target.y, rate);
    current.z = ease(current.z, target.z, rate);
}

impl GameCamera {
    /// Camera_8002DDC4 (0x8002DDC4): one fixed-mode tick. Both transforms
    /// ease towards the stage's fixed interest, field of view and eye; the
    /// quake follows at the transform's depth.
    pub fn update_fixed(&mut self, stage: &StageCamera) {
        // Camera_80030DF8.
        self.translation = hsd_types::Vec2::ZERO;
        let view = &stage.fixed;
        let interest = view.interest();
        // retail 0x8002DE18..0x8002DE44, then 0x8002DE6C..0x8002DE90.
        self.transform.target_interest = interest;
        ease_vec(&mut self.transform.interest, interest, INTEREST_RATE);
        self.transform_copy.target_interest = interest;
        ease_vec(&mut self.transform_copy.interest, interest, INTEREST_RATE);
        for t in [&mut self.transform, &mut self.transform_copy] {
            t.target_fov = view.fov;
            t.fov = ease(t.fov, t.target_fov, FOV_RATE);
        }
        for t in [&mut self.transform, &mut self.transform_copy] {
            t.target_position = view.position;
            ease_vec(&mut t.position, view.position, POSITION_RATE);
        }
        let bounds = Bounds {
            depth: depth(&self.transform),
            ..Bounds::default()
        };
        // Camera_UpdateQuakes, then Camera_ApplyQuake.
        let _ = self.quake.count_down();
        self.translation = self.quake.apply(&bounds, &self.transform, self.zoom, stage);
    }
}

/// `set_bounds_z`: the length of target interest minus target eye
/// (lbVector_Diff, three fmuls summed x+y then z, sqrtf).
fn depth(t: &Transform) -> f32 {
    let dx = t.target_interest.x - t.target_position.x;
    let dy = t.target_interest.y - t.target_position.y;
    let dz = t.target_interest.z - t.target_position.z;
    // retail 0x8002DF9C..0x8002DFB0.
    sqrtf(dz * dz + (dx * dx + dy * dy))
}
