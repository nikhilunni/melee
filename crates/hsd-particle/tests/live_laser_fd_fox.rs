//! Retail blaster muzzle, laser hit and HUD particles from production inputs.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/laser_fd_fox_spawns.json");

#[test]
fn live_laser_fd_fox_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("laser_fd_fox", 300);
}
