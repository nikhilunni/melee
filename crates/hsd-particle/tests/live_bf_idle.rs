//! Battlefield idle: production spawn/attachment inputs, retail particle outputs.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/idle_bf_spawns.json");
#[test]
fn battlefield_idle_600_particles_and_ordered_rng() {
    dust_replay::replay("idle_bf_fox", 600);
}
