//! Dash dust fields and ordered RNG, with only documented display caches excluded.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/dash_fd_spawns.rs"]
mod spawns;
#[test]
fn live_fd_dash_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("dash");
}
