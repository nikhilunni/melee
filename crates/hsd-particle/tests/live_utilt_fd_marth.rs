mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/utilt_fd_marth_spawns.json");

#[test]
fn utilt_fd_marth_particles_300_ticks() {
    dust_replay::replay("utilt_fd_marth", 300);
}
