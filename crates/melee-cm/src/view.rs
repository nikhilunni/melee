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

/// Which screen code the console ran: the main CObj's aspect and the
/// horizontal bounds of the on-screen test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    /// Retail: the description's aspect and the CObj's scissor.
    #[default]
    Standard,
    /// "Widescreen 16:9" [Dan Salvato, mirrorbender, Achilles1515,
    /// UnclePunch] (slippi-ssbm-asm External/Widescreen). CObjLoad's
    /// perspective aspect (hook 0x8036A4A8) is scaled by 320 / 219, and
    /// Camera_80030BBC compares the window x with 100 and 540 in place of
    /// the scissor's left and right (0x80030C7C, 0x80030C88): about where
    /// the 4:3 edges fall in the wider picture, a pixel or so outside them.
    Widescreen,
    /// The same code with the aspect it had before slippi-ssbm-asm ef8e734
    /// (2019-10-29, "update fov to 320:219"): the description's times
    /// 92 / 63, the factor the tree started with (703f953). The picture is
    /// wider by 0.06%, an eighth of a pixel at the bounds, which are still
    /// 100 and 540.
    Widescreen92By63,
}

/// Overwrite CObj Values.asm: `.float 320`, `.float 219`.
const WIDESCREEN_ASPECT: (f32, f32) = (320.0, 219.0);
/// Overwrite CObj Values.asm before ef8e734: `.float 92`, `.float 63`.
const WIDESCREEN_92_BY_63_ASPECT: (f32, f32) = (92.0, 63.0);
/// Left Camera Bound.asm (`li r0, 0+100`) and Right Camera Bound.asm
/// (`li r0, 640-100`).
const WIDESCREEN_LEFT: i32 = 100;
const WIDESCREEN_RIGHT: i32 = 540;

impl Screen {
    /// The main CObj's aspect. Widescreen: `fmuls` then `fdivs` on the
    /// description's (hook 0x8036A4A8).
    pub fn aspect(self) -> f32 {
        let scaled =
            |(numerator, denominator): (f32, f32)| DESCRIPTION.aspect * numerator / denominator;
        match self {
            Self::Standard => DESCRIPTION.aspect,
            Self::Widescreen => scaled(WIDESCREEN_ASPECT),
            Self::Widescreen92By63 => scaled(WIDESCREEN_92_BY_63_ASPECT),
        }
    }

    /// The window x range Camera_80030BBC calls on screen: left inclusive,
    /// right exclusive.
    fn horizontal_bounds(self, scissor: hsd_anim::cobj::Scissor) -> (i32, i32) {
        match self {
            Self::Standard => (i32::from(scissor.left), i32::from(scissor.right)),
            Self::Widescreen | Self::Widescreen92By63 => (WIDESCREEN_LEFT, WIDESCREEN_RIGHT),
        }
    }
}

/// A point's window position, when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
    /// Inside the scissor rectangle (the screen code's bounds across).
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
            aspect: self.screen.aspect(),
            ..DESCRIPTION
        }
    }
}

/// Camera_80030BBC (0x80030BBC): project a world point with the rendered
/// camera. `None` when it has no representable window position.
pub fn to_screen(
    camera: &PerspectiveCamera,
    position: Vec3,
    screen: Screen,
) -> Option<ScreenPoint> {
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
    let (left, right) = screen.horizontal_bounds(s);
    let on_screen = x >= left && x < right && y >= i32::from(s.top) && y < i32::from(s.bottom);
    Some(ScreenPoint { x, y, on_screen })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_widescreen_aspect_is_the_descriptions_scaled_by_the_codes_floats() {
        assert_eq!(
            Screen::Standard.aspect().to_bits(),
            DESCRIPTION.aspect.to_bits()
        );
        // 1.2173333 * 320 = 389.54666, / 219 = 1.7787519 (fmuls, fdivs).
        assert_eq!(Screen::Widescreen.aspect().to_bits(), 0x3FE3_AE24);
        // 1.2173333 * 92 = 111.99466, / 63 = 1.7776932.
        assert_eq!(Screen::Widescreen92By63.aspect().to_bits(), 0x3FE3_8B73);
    }

    #[test]
    fn the_widescreen_bounds_replace_the_scissors_left_and_right() {
        let cases = [
            (Screen::Standard, (0, 640)),
            (Screen::Widescreen, (100, 540)),
            (Screen::Widescreen92By63, (100, 540)),
        ];
        for (screen, bounds) in cases {
            assert_eq!(
                screen.horizontal_bounds(DESCRIPTION.scissor),
                bounds,
                "{screen:?}"
            );
        }
    }
}

/// What `Camera_800307D0` reports: where the camera's view meets the
/// stage plane (z = 0) on the left, in the middle and on the right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundView {
    pub left: f32,
    pub center: f32,
    pub right: f32,
    /// False when the view direction is (nearly) parallel to an axis.
    pub valid: bool,
}

/// `Camera_800307D0` (0x800307D0): cast the view direction and the two
/// horizontal frustum edges (half the vertical fov times the aspect) from
/// the eye to the z = 0 plane. 0x80030828..38: two fmuls, then the double
/// 0.5 * x rounded once; 0x800308D8 / 0x80030948 / 0x800309E4 fnmsubs;
/// 0x80030904 / 0x800309A0 fmsubs and 0x80030908 / 0x800309A4 fmadds.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // fcmpo; ble: NaN takes the other branch.
pub fn ground_view(camera: &PerspectiveCamera) -> GroundView {
    use gekko_math::fma::{fmadds, fmsubs, fnmsubs};
    use gekko_math::msl::{cosf, sinf};
    /// `@2414` / `@2415`: an edge that never meets the plane.
    const FAR: f32 = 8.507_059e37;
    /// `@2412`: 1e-4, compared in double.
    const EPSILON: f64 = 1e-4;
    let half_fov = (0.5 * f64::from(
        hsd_anim::cobj::DEGREES_TO_RADIANS * camera.fov * camera.aspect,
    )) as f32;
    let eye = camera.eye;
    let forward = camera.eye_vector().unwrap_or(Vec3::new(0.0, 0.0, -1.0));
    let magnitude = |v: f32| f64::from(if v < 0.0 { -v } else { v });
    // fcmpo; ble: a NaN magnitude goes on.
    if magnitude(forward.x) <= EPSILON || magnitude(forward.z) <= EPSILON {
        return GroundView {
            left: -FAR,
            center: 0.0,
            right: FAR,
            valid: false,
        };
    }
    // 0x800308A4..BC: three fmuls by -1.0, exact negations.
    let forward = Vec3::new(-forward.x, -forward.y, -forward.z);
    let center = fnmsubs(forward.x, eye.z / forward.z, eye.x);
    let edge = |angle: f32| {
        let s = sinf(angle);
        let c = cosf(angle);
        let edge_z = fmsubs(forward.z, c, forward.x * s);
        let edge_x = fmadds(forward.x, c, forward.z * s);
        // `same_side`: fmuls, then `> 0.0` in double (@2413).
        if !(magnitude(edge_z) <= EPSILON) && !(f64::from(forward.z * edge_z) <= 0.0) {
            fnmsubs(edge_x, eye.z / edge_z, eye.x)
        } else if !(edge_x <= 0.0) {
            FAR
        } else {
            -FAR
        }
    };
    let left = edge(half_fov);
    let right = edge(-half_fov);
    GroundView {
        left,
        center,
        right,
        valid: true,
    }
}
