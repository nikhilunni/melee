//! HSD scene graph and animation: JObj/DObj/AObj/FObj matrix and keyframe evaluation, no rendering.
//!
//! See CLAUDE.md for the porting rules that apply to every crate.
//! Module ownership during parallel work: `mtx` and `quat` are one unit,
//! `aobj` and `fobj` are another, `jobj` comes after both.

pub mod aobj;
pub mod cobj;
pub mod dobj;
pub mod fobj;
pub mod jobj;
pub mod load;
pub mod material_playback;
pub mod mobj;
pub mod mtx;
pub mod orientation;
pub mod quat;
pub mod tobj;

pub mod spline;
