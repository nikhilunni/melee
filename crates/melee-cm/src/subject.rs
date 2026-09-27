//! CmSubject (cm/types.h, 0x6C bytes): one thing the camera frames. Fighters
//! and some items own one; the camera smooths its extents and decides each
//! tick whether it counts.
use crate::stage::StageCamera;
use hsd_types::Vec3;

/// CmSubjectState (cm/forward.h).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubjectState {
    /// Always framed.
    #[default]
    Active = 0,
    /// Never framed.
    Inactive = 1,
    /// Framed only while near the middle of the camera bounds.
    Auto = 2,
}

/// CmSubjectExtents: the framing box around `position`, in the retail field
/// order `h.x, h.y, v.x, v.y, v.z`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extents {
    /// h.x / h.y: horizontal reach to the left (negative) and right.
    pub left: f32,
    pub right: f32,
    /// v.x / v.y: vertical reach above and below (negative).
    pub top: f32,
    pub bottom: f32,
    /// v.z: the radius Camera_80030CFC and the magnifier scale use.
    pub radius: f32,
}

impl Extents {
    /// Camera_80028F5C's defaults.
    pub const DEFAULT: Extents = Extents {
        left: -1.0,
        right: 1.0,
        top: 1.0,
        bottom: -1.0,
        radius: 1.0,
    };
}

impl Default for Extents {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Frames an Auto subject waits after leaving the middle of the bounds.
const AUTO_REFRAME_DELAY: i16 = 600;
/// Extent smoothing step per tick (Camera_800293E0).
const EXTENT_STEP: f32 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub struct Subject {
    pub state: SubjectState,
    /// +0C bit 0: set by ftCo_Cliff_Cam while hanging on a ledge.
    pub on_ledge: bool,
    /// +0C bit 1: a stage override that keeps the subject unframed.
    pub force_inactive: bool,
    /// +0C bit 2: was framed on the last check.
    pub was_framed: bool,
    /// +0E: Auto subjects' reframe countdown.
    pub state_timer: i16,
    pub position: Vec3,
    /// The owner's camera bone, for on-screen and magnifier tests.
    pub bone_position: Vec3,
    pub facing: f32,
    /// The smoothed extents the camera frames.
    pub extents: Extents,
    /// The owner's current extents, approached by `extents`.
    pub target_extents: Extents,
}

impl Subject {
    /// Camera_80028F5C (0x80028F5C): a freshly linked subject.
    pub fn new(state: SubjectState) -> Self {
        Self {
            state,
            on_ledge: false,
            force_inactive: false,
            was_framed: false,
            state_timer: 0,
            position: Vec3::ZERO,
            bone_position: Vec3::ZERO,
            facing: 0.0,
            extents: Extents::DEFAULT,
            target_extents: Extents::DEFAULT,
        }
    }

    /// Camera_8002928C (0x8002928C): whether the camera frames this subject
    /// now. Auto subjects count their reframe delay down on every call, and
    /// retail calls this several times per tick, so the call count matters.
    pub fn is_framed(&mut self, stage: &StageCamera) -> bool {
        if self.state == SubjectState::Inactive || self.force_inactive {
            return false;
        }
        if self.state == SubjectState::Auto {
            if self.state_timer != 0 {
                self.state_timer -= 1;
                return false;
            }
            let (left, right) = (stage.left(), stage.right());
            let (top, bottom) = (stage.top(), stage.bottom());
            // Two ordered compares: a NaN ratio counts as inside.
            #[allow(clippy::manual_range_contains)]
            let outside_middle = |t: f32| t > 0.65 || t < 0.35;
            if outside_middle((self.position.x - left) / (right - left))
                || outside_middle((self.position.y - bottom) / (top - bottom))
            {
                if self.was_framed {
                    self.was_framed = false;
                    self.state_timer = AUTO_REFRAME_DELAY;
                }
                return false;
            }
            self.state_timer = 0;
        }
        self.was_framed = true;
        true
    }

    /// Camera_800293E0's per-subject body: step each extent toward its target
    /// by at most 0.5.
    pub(crate) fn smooth_extents(&mut self) {
        let step = |current: &mut f32, target: f32| {
            let distance = target - *current;
            if distance != 0.0 {
                if distance > EXTENT_STEP {
                    *current += EXTENT_STEP;
                } else if distance < -EXTENT_STEP {
                    *current -= EXTENT_STEP;
                } else {
                    *current = target;
                }
            }
        };
        let target = self.target_extents;
        step(&mut self.extents.left, target.left);
        step(&mut self.extents.right, target.right);
        step(&mut self.extents.top, target.top);
        step(&mut self.extents.bottom, target.bottom);
        step(&mut self.extents.radius, target.radius);
    }
}

impl Default for Subject {
    fn default() -> Self {
        Self::new(SubjectState::Active)
    }
}
