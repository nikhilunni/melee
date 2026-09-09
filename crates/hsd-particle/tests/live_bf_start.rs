//! Battlefield start: production spawn/attachment inputs, retail particle outputs.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/start_bf_spawns.json");
#[test]
fn battlefield_start_600_particles_and_ordered_rng() {
    dust_replay::replay("start_bf_fox", 600);
}
