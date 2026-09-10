//! Concrete item engine; retail `it/item.c` and `it/types.h`.
pub mod desc;
mod engine;
mod logic;
mod spawn;
pub use engine::*;
pub use logic::*;
pub use spawn::*;
