//! Illusion trail from production fighter/item scheduling, replayed against retail.
mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
const SPAWN_FIXTURE: &str = include_str!("data/airillusion_fd_fox_spawns.json");

#[test]
fn live_airillusion_fd_fox_300_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay("airillusion_fd_fox", 300);
}
