//! The main camera as retail renders the current tick, read from the
//! simulation's camera without advancing it.
use crate::Match;

/// `Camera_8002A4AC` (0x8002A4AC) through `Camera_8002AF68`: the main CObj's
/// eye and basis (`C_MTXLookAt` rows) and its perspective.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCamera {
    pub eye: [f32; 3],
    /// View-space x, y and z axes in world space; `toward_eye` points from
    /// the interest back toward the eye.
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub toward_eye: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    /// Retail viewport aspect; a consumer may substitute its own.
    pub aspect: f32,
}
impl ViewCamera {
    pub(super) fn capture(game: &Match) -> Self {
        let state = game.engine.state();
        let camera = state.camera.render_camera(&state.assets.stage_camera);
        let view = camera.view_matrix().0;
        let row = |r: usize| [view[r][0], view[r][1], view[r][2]];
        Self {
            eye: [camera.eye.x, camera.eye.y, camera.eye.z],
            right: row(0),
            up: row(1),
            toward_eye: row(2),
            fov: camera.fov,
            near: camera.near,
            far: camera.far,
            aspect: camera.aspect,
        }
    }
}
