//! Concrete item engine; retail `it/item.c` and `it/types.h`.
pub mod bone_motion;
pub mod desc;
mod engine;
mod reflection;
pub use reflection::*;
pub mod hurt;
mod link;
pub use link::*;
mod logic;
mod map;
pub use map::AirContact;
pub mod pose;
mod spawn;
pub use engine::*;
pub use logic::*;
pub use spawn::*;
