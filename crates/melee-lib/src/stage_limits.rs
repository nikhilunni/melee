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
    /// grPura_80211D00's tail (0x80211D88..): each camera limit, made
    /// relative to Stage_UnkSetVec3TCam_Offset's centre, times 0.8 (fsubs,
    /// then fmuls), in top, bottom, left, right order. The creation's, so
    /// the assets hold the result.
    pub(crate) fn shrink_pura_camera(&mut self) {
        const SHRINK: f32 = 0.8;
        let (x, y) = (self.camera.offset_x, self.camera.offset_y);
        self.set_camera(Edge::Top, SHRINK * (self.camera.top() - y));
        self.set_camera(Edge::Bottom, SHRINK * (self.camera.bottom() - y));
        self.set_camera(Edge::Left, SHRINK * (self.camera.left() - x));
        self.set_camera(Edge::Right, SHRINK * (self.camera.right() - x));
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
