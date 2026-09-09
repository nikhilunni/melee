//! Tick-boundary cached matrices and SRT. No dirty-bone exclusions: the
//! tick capture does not include per-joint flags.
mod fighter_support;
#[test]
#[ignore = "bone oracle not yet matched: Euler rotate[3] carries an uninitialised retail stack word; idle tick-0 tail matrix and part-animation import differ; see dynamics/README.md"]
fn start_fox_bones_130() {
    fighter_support::replay::replay("start", 130, true);
}
#[test]
#[ignore = "bone oracle not yet matched: Euler rotate[3] carries an uninitialised retail stack word; idle tick-0 tail matrix and part-animation import differ; see dynamics/README.md"]
fn idle_fox_bones_8() {
    fighter_support::replay::replay("idle", 8, true);
}
