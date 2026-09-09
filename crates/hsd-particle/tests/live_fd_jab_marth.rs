//! Retail jab particles: production effect inputs, every simulation field/draw.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/jab_fd_marth_spawns.json");
#[test]
fn live_fd_jab_marth_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("jab_fd_marth", 300);
}
