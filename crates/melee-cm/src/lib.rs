//! Gameplay camera (decomp src/melee/cm): the subjects it frames, the
//! standard mode's per-tick tracking, screen shake and the rendered CObj.
//!
//! See CLAUDE.md for the porting rules that apply to every crate.
mod camera;
pub mod params;
pub mod quake;
pub mod stage;
mod standard;
pub mod subject;
pub mod view;

pub use camera::{GameCamera, Mode, Transform};
pub use quake::{Quake, QuakeKind};
pub use stage::{Rect, StageCamera};
pub use subject::{Extents, Subject, SubjectState};
pub use view::{to_screen, Screen, ScreenPoint};
