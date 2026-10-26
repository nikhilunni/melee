//! stage_info's camera limits and blast zone as the match runs. A stage's
//! creation leaves them as the assets describe (`Assets::stage_camera`,
//! `Assets::arena`); Mute City's script moves them (Ground_801C3880 ..
//! Ground_801C38AC for the camera, Ground_801C3980 .. Ground_801C39B0 for
//! the blast zone).
use melee_cm::StageCamera;
use melee_ft::fighter::life::Arena;

/// One side of a limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StageLimits {
    /// `stage_info.cam_info` and `stage_info.blast_zone`, both relative to
    /// the camera's centre.
    pub(crate) camera: StageCamera,
    /// The blast zone and camera top with the centre added back
    /// (Stage_GetBlastZone*Offset, Stage_GetCamBoundsTopOffset), as the
    /// fighters and items read them.
    pub(crate) arena: Arena,
}

impl StageLimits {
    pub(crate) fn new(assets: &crate::assets::Assets) -> Self {
        Self {
            camera: assets.stage_camera,
            arena: assets.arena,
        }
    }
    /// Ground_801C3880 / 3890 / 38A0 / 38AC.
    pub(crate) fn set_camera(&mut self, edge: Edge, value: f32) {
        let bounds = &mut self.camera.bounds;
        match edge {
            Edge::Top => bounds.top = value,
            Edge::Bottom => bounds.bottom = value,
            Edge::Left => bounds.left = value,
            Edge::Right => bounds.right = value,
        }
        self.arena.camera_top = self.camera.top();
    }
    /// Ground_801C3980 / 3990 / 39A0 / 39B0.
    pub(crate) fn set_blast(&mut self, edge: Edge, value: f32) {
        let blast = &mut self.camera.blast_zone;
        match edge {
            Edge::Top => {
                blast.top = value;
                self.arena.top = self.camera.blast_top();
            }
            Edge::Bottom => {
                blast.bottom = value;
                self.arena.bottom = self.camera.blast_bottom();
            }
            Edge::Left => {
                blast.left = value;
                self.arena.left = self.camera.blast_left();
            }
            Edge::Right => {
                blast.right = value;
                self.arena.right = self.camera.blast_right();
            }
        }
    }
}
