mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/ko_fd_marth_spawns.json");
#[test]
fn ko_fd_marth_particles_480_ticks() {
    dust_replay::replay("ko_fd_marth", 480);
}
