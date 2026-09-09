//! State-callback replays; m4_gate separately verifies the complete scene/RNG.
mod fighter_support;
use fighter_support::replay::replay_state_callbacks;

#[test]
fn squat_fox_state_callbacks() {
    replay_state_callbacks("squat", "ledger");
}
#[test]
fn turn_fox_state_callbacks() {
    replay_state_callbacks("turn", "ledger");
}
#[test]
fn walk_fox_state_callbacks() {
    replay_state_callbacks("walk", "ledger");
}
