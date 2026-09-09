//! Jump and landing dust, using requests and joint inputs from the production port.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/jump_fd_spawns.rs"]
mod spawns;
#[test]
fn live_fd_jump_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("jump", 300);
}
