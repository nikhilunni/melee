//! All fighter scheduler procs, starting at the owned savestate boundary.
mod fighter_support;
#[test]
fn start_fox_600() {
    fighter_support::replay::replay("start", 600, false);
}
