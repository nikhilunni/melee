//! Library half of the simulator: pieces the binary and its tests share.
//!
//! The binary lives in `main.rs`. This crate root exists so integration tests
//! under `tests/` can reach the schema-coverage machinery without going
//! through the CLI.

pub mod bones;
pub mod camera;
pub mod schema;

pub mod assets;
pub mod frame;
pub mod gobj;
pub mod initial_state;
pub mod inputs;
pub mod replay;
pub mod replay_batch;
pub mod replay_cost;
mod replay_game;
pub mod replay_lag_frames;
pub mod replay_online;
mod replay_observed;
pub mod replay_retail_pads;
pub mod replay_screen_codes;
pub mod replay_stage_codes;
pub mod scenario;
pub mod scratch;
pub mod search;
pub mod trace;
pub mod trace_after_map;
pub mod trace_items;
pub mod triage;

pub mod scene_stage;
