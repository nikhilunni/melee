//! Retail roll particles replayed with production effect requests and joint inputs.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/roll_fd_spawns.json");
#[test]
fn live_fd_roll_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("roll", 300);
}
