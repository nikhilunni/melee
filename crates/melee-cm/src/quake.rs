//! Screen shake: Camera_RequestQuake, Camera_UpdateQuakes, Camera_ApplyQuake.
//!
//! A request starts one of the stage's `quake_model_set` animations
//! (grLib_801C9CEC); its proc at s_link 1 writes the root joint's x/y
//! translation to [`Quake::offset`] every tick. The camera then scales that
//! offset into a screen-space translation of the rendered eye and interest.
use crate::camera::Transform;
use crate::params::{DESCRIPTION, TUNING, VIEWPORT_HEIGHT, VIEWPORT_WIDTH};
use crate::stage::StageCamera;
use crate::standard::Bounds;
use gekko_math::fma::fmadds;
use gekko_math::msl::tanf;
use hsd_anim::cobj::DEGREES_TO_RADIANS;
use hsd_types::Vec2;

/// CmQuakeKind (cm/forward.h).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuakeKind {
    Loop = 1,
    Small = 2,
    Medium = 3,
    Large = 4,
}

impl QuakeKind {
    /// The `quake_model_set` animation index grLib_801C9CEC plays.
    pub fn animation_index(self) -> usize {
        self as usize - 1
    }
}

/// Frames a Loop request keeps its countdown; the others last 22.
const LOOP_FRAMES: i32 = 10;
const ONE_SHOT_FRAMES: i32 = 22;
/// Camera_ApplyQuake scales the model's offset by ten.
const OFFSET_SCALE: f32 = 10.0;
/// Below this depth range the quake uses the middle depth factor.
const MIN_DEPTH_RANGE: f32 = 0.001;

#[derive(Clone, Debug, PartialEq)]
pub struct Quake {
    /// quake_frames_left, indexed by CmQuakeKind (index 0 unused).
    pub frames_left: [i32; 5],
    /// quake_offset: the playing animation's root translation this tick.
    pub offset: Vec2,
    /// quake_scale (Camera_SetQuakeScale).
    pub scale: f32,
    /// quake_gobj: a looping animation is playing.
    pub looping: bool,
}

impl Default for Quake {
    fn default() -> Self {
        Self {
            frames_left: [0; 5],
            offset: Vec2::ZERO,
            scale: 1.0,
            looping: false,
        }
    }
}

impl Quake {
    /// Camera_RequestQuake (0x80030E44 area): restart the countdown. Returns
    /// whether the caller must start a new animation of this kind (a Loop
    /// request only starts one when none is playing). The per-tick request
    /// log (quakes[0]) has no reader and is not kept.
    pub fn request(&mut self, kind: QuakeKind) -> bool {
        match kind {
            QuakeKind::Loop => {
                let start = !self.looping;
                self.looping = true;
                self.frames_left[kind as usize] = LOOP_FRAMES;
                start
            }
            _ => {
                self.frames_left[kind as usize] = ONE_SHOT_FRAMES;
                true
            }
        }
    }

    /// Camera_UpdateQuakes (0x8002A268 area): count every active kind down;
    /// a looping animation stops once its countdown ends. Returns whether
    /// the looping animation must be destroyed now.
    pub(crate) fn count_down(&mut self) -> bool {
        let mut any = false;
        for frames in &mut self.frames_left {
            if *frames != 0 {
                *frames -= 1;
                any = true;
            }
        }
        let stop = any && self.looping && self.frames_left[QuakeKind::Loop as usize] == 0;
        if stop {
            self.looping = false;
        }
        stop
    }

    /// Camera_ApplyQuake (0x8002A0C0): the render translation for this
    /// tick's offset, which is then consumed.
    pub(crate) fn apply(
        &mut self,
        bounds: &Bounds,
        transform: &Transform,
        zoom: f32,
        stage: &StageCamera,
    ) -> Vec2 {
        let mut input_x = (self.offset.x * self.scale) * OFFSET_SCALE;
        let mut input_y = (self.offset.y * self.scale) * OFFSET_SCALE;
        // gm_8016B41C (single-player scenes) would scale by cm_803BCCA0.xE8.
        input_x *= zoom;
        input_y *= zoom;
        let half_view_height = bounds.depth * tanf(0.5 * (DEGREES_TO_RADIANS * transform.fov));
        let scale_x = DESCRIPTION.aspect * (half_view_height / (0.5 * VIEWPORT_WIDTH as f32));
        let scale_y = half_view_height / (0.5 * VIEWPORT_HEIGHT as f32);
        let depth_range = stage.max_depth - stage.min_depth;
        let depth_ratio = if depth_range < MIN_DEPTH_RANGE {
            0.5
        } else {
            (bounds.depth - stage.min_depth) / depth_range
        };
        let t = &TUNING;
        // retail 0x8002A230 / 0x8002A234: fmadds.
        let factor_y = fmadds(depth_ratio, t.quake_far_y - t.quake_near_y, t.quake_near_y);
        let factor_x = fmadds(depth_ratio, t.quake_far_x - t.quake_near_x, t.quake_near_x);
        self.offset = Vec2::ZERO;
        Vec2::new(
            factor_x * (input_x * scale_x),
            factor_y * (input_y * scale_y),
        )
    }
}
