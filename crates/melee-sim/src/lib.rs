//! Library half of the simulator: pieces the binary and its tests share.
//!
//! The binary lives in `main.rs`. This crate root exists so integration tests
//! under `tests/` can reach the schema-coverage machinery without going
//! through the CLI.

pub mod bones;
pub mod schema;

pub mod assets;
pub mod frame;
pub mod initial_state;
pub mod inputs;
pub mod replay;
pub mod scenario;
pub mod trace;
pub mod trace_items;

mod countdown;

mod scene_fighter;
mod scene_items;

pub mod scene_stage;
