//! Concrete item engine; retail `it/item.c` and `it/types.h`.
pub mod desc;
mod engine;
mod reflection;
pub use reflection::*;
pub mod hurt;
mod logic;
mod map;
pub use map::AirContact;
mod spawn;
pub use engine::*;
pub use logic::*;
pub use spawn::*;
