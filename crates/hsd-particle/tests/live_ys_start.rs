//! Yoshi's Story start: production puff inputs, retail particle outputs.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/start_ys_spawns.json");
#[test]
fn story_start_600_particles_and_ordered_rng() {
    dust_replay::replay("start_ys_fox", 600);
}
