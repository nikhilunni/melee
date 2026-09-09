mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/grab_fd_marth_spawns.json");

#[test]
fn grab_fd_marth_catch_startup_particles_127_ticks() {
    dust_replay::replay_prefix("grab_fd_marth", 300, 127);
}

#[test]
fn grab_fd_marth_particles_300_ticks() {
    dust_replay::replay("grab_fd_marth", 300);
}
