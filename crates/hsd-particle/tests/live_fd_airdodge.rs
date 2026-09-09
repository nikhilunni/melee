//! Retail airdodge particles replayed with production effect requests and joint inputs.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/airdodge_fd_spawns.json");
#[test]
fn live_fd_airdodge_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("airdodge_fd_fox", 300);
}
