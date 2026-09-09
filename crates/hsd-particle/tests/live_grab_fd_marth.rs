mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
// Only production requests from the verified ticks 0..=126 are available.
const SPAWN_FIXTURE: &str = include_str!("data/grab_fd_marth_startup_spawns.json");

#[test]
fn grab_fd_marth_catch_startup_particles_127_ticks() {
    dust_replay::replay_prefix("grab_fd_marth", 300, 127);
}

#[test]
#[ignore = "gameplay stops at tick 127; capture/throw/missed-tech effects and remaining external spawn fixture are unported"]
fn grab_fd_marth_particles_300_ticks() {
    dust_replay::replay("grab_fd_marth", 300);
}
