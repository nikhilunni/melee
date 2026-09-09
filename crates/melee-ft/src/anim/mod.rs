//! Fighter animation playback (`ft/ftanim.c`): attaching a motion's FigaTree to
//! the skeleton, blending, per-tick frame stepping, and the idle-animation
//! choice (`ft/ftwaitanim.c`). Owned by Milestone 3 task T7.

pub mod attach;
pub mod blend;
pub mod playback;
pub mod root_motion;
pub mod wait_choice;

pub use playback::{FighterAnimation, Motion, MotionFlags, PartAnimation};
pub use wait_choice::{choose_wait_animation, WaitChoice, WaitEntry};
