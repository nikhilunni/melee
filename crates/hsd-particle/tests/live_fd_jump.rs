//! Jump and landing dust, using requests and joint inputs from the production port.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/jump_fd_spawns.json");
#[test]
fn live_fd_jump_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("jump_fd_fox", 300);
}
