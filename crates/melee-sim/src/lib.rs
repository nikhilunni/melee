//! Library half of the simulator: pieces the binary and its tests share.
//!
//! The binary lives in `main.rs`. This crate root exists so integration tests
//! under `tests/` can reach the schema-coverage machinery without going
//! through the CLI.

pub mod schema;
