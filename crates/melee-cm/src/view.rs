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
}

/// Overwrite CObj Values.asm: `.float 320`, `.float 219`.
const WIDESCREEN_ASPECT_NUMERATOR: f32 = 320.0;
const WIDESCREEN_ASPECT_DENOMINATOR: f32 = 219.0;
/// Left Camera Bound.asm (`li r0, 0+100`) and Right Camera Bound.asm
/// (`li r0, 640-100`).
const WIDESCREEN_LEFT: i32 = 100;
const WIDESCREEN_RIGHT: i32 = 540;

impl Screen {
    /// The main CObj's aspect. Widescreen: `fmuls` then `fdivs` on the
    /// description's (hook 0x8036A4A8).
    pub fn aspect(self) -> f32 {
        match self {
            Self::Standard => DESCRIPTION.aspect,
            Self::Widescreen => {
                DESCRIPTION.aspect * WIDESCREEN_ASPECT_NUMERATOR / WIDESCREEN_ASPECT_DENOMINATOR
            }
        }
    }

    /// The window x range Camera_80030BBC calls on screen: left inclusive,
    /// right exclusive.
    fn horizontal_bounds(self, scissor: hsd_anim::cobj::Scissor) -> (i32, i32) {
        match self {
            Self::Standard => (i32::from(scissor.left), i32::from(scissor.right)),
            Self::Widescreen => (WIDESCREEN_LEFT, WIDESCREEN_RIGHT),
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
    fn the_widescreen_aspect_is_the_descriptions_times_320_over_219() {
        assert_eq!(
            Screen::Standard.aspect().to_bits(),
            DESCRIPTION.aspect.to_bits()
        );
        // 1.2173333 * 320 = 389.54666, / 219 = 1.7787519 (fmuls, fdivs).
        assert_eq!(Screen::Widescreen.aspect().to_bits(), 0x3FE3_AE24);
    }

    #[test]
    fn the_widescreen_bounds_replace_the_scissors_left_and_right() {
        let cases = [
            (Screen::Standard, (0, 640)),
            (Screen::Widescreen, (100, 540)),
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
