//! The rendered camera: Camera_8002A4AC's CObj setup and the screen tests
//! that read it (Camera_80030BBC, Camera_80030CD8).
use crate::camera::{GameCamera, Mode, Transform};
use crate::params::DESCRIPTION;
use crate::stage::StageCamera;
use gekko_math::msl::fctiwz;
use hsd_anim::cobj::PerspectiveCamera;
use hsd_types::Vec3;

/// Screen positions beyond this do not fit an s32 (Camera_80030BBC).
const SCREEN_LIMIT: f32 = 2.147_483_6e9;

/// A point's window position, when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
    /// Inside the scissor rectangle.
    pub on_screen: bool,
}

impl GameCamera {
    /// Camera_8002A4AC (0x8002A4AC) for the standard mode, through
    /// Camera_8002AF68: the main CObj as this tick's render sees it.
    pub fn render_camera(&self, stage: &StageCamera) -> PerspectiveCamera {
        let Mode::Standard = self.mode;
        self.render_transform(&self.transform, stage)
    }

    /// Camera_8002A4AC's second Camera_8002AF68 call: cm_804D6464, the CObj
    /// that follows `transform_copy`. Camera_800310B8 hands it to the screen
    /// KO, which places the fighter through its inverse viewing matrix.
    pub fn render_copy_camera(&self, stage: &StageCamera) -> PerspectiveCamera {
        let Mode::Standard = self.mode;
        self.render_transform(&self.transform_copy, stage)
    }

    /// Camera_8002AF68 (0x8002AF68): one transform, translated and with the eye
    /// kept above the stage's minimum height.
    fn render_transform(&self, t: &Transform, stage: &StageCamera) -> PerspectiveCamera {
        let mut interest = t.interest;
        interest.x += self.translation.x;
        interest.y += self.translation.y;
        let mut eye = t.position;
        eye.x += self.translation.x;
        eye.y += self.translation.y;
        if eye.y < stage.min_eye_height {
            eye.y = stage.min_eye_height;
        }
        PerspectiveCamera {
            eye,
            interest,
            fov: t.fov,
            near: self.near,
            far: self.far,
            ..DESCRIPTION
        }
    }
}

/// Camera_80030BBC (0x80030BBC): project a world point with the rendered
/// camera. `None` when it has no representable window position.
pub fn to_screen(camera: &PerspectiveCamera, position: Vec3) -> Option<ScreenPoint> {
    let point = melee_lb::vector::world_to_screen(camera, position);
    if point.x > SCREEN_LIMIT
        || point.x < -SCREEN_LIMIT
        || point.y > SCREEN_LIMIT
        || point.y < -SCREEN_LIMIT
    {
        return None;
    }
    let (x, y) = (fctiwz(point.x), fctiwz(point.y));
    let s = camera.scissor;
    let on_screen = x >= i32::from(s.left)
        && x < i32::from(s.right)
        && y >= i32::from(s.top)
        && y < i32::from(s.bottom);
    Some(ScreenPoint { x, y, on_screen })
}
