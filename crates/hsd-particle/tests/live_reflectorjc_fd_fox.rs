//! Reflector model particles, replayed against retail.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/reflectorjc_fd_fox_spawns.json");
#[test]
fn live_reflectorjc_fd_fox_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("reflectorjc_fd_fox", 300);
}
