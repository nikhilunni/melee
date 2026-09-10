mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/uair_fd_fox_spawns.json");

#[test]
fn uair_fd_fox_particles_300_ticks() {
    dust_replay::replay("uair_fd_fox", 300);
}
