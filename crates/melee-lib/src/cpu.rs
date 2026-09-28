//! The scene side of the CPU proc: the view melee-cpu thinks with.
use crate::{frame::MAX_FIGHTERS, initial_state::InitialState};
use melee_ft::fighter::Fighter;

/// Fighter_8006ABA0 -> ftCo_800B3900 for fighter `index`, which the CPU
/// drives (see `Fighter::cpu_driven`).
pub(crate) fn think(state: &mut InitialState, index: usize) {
    // melee-cpu's item target choices read these; no item views exist yet.
    if !state.items.is_empty() {
        unimplemented!("ftCo_800A5F4C and friends: CPU item views");
    }
    // gm_8016C75C: the player's KO total, which nothing counts yet. A KO
    // starts with a fighter's death states, so none may be in them.
    if state
        .fighters
        .iter()
        .any(|f| matches!(f.0.state_data, melee_ft::fighter::MotionData::Life(_)))
    {
        unimplemented!("gm_8016C75C: KO totals for the CPU");
    }
    let count = state.fighters.len();
    let deadzone = state.assets.fighters[index]
        .input
        .thresholds
        .horizontal_stick_deadzone;
    let (before, rest) = state.fighters.split_at_mut(index);
    let (own, after) = rest.split_first_mut().expect("the thinking fighter");
    let mut others: [Option<&Fighter>; MAX_FIGHTERS] = [None; MAX_FIGHTERS];
    for (slot, fighter) in others.iter_mut().zip(before.iter()) {
        *slot = Some(&fighter.0);
    }
    for (slot, fighter) in others[index + 1..].iter_mut().zip(after.iter()) {
        *slot = Some(&fighter.0);
    }
    let mut scene = melee_cpu::Scene {
        fighters: &others[..count],
        own: index,
        map: &mut state.map,
        arena: &state.assets.arena,
        items: &[],
        data: &state.assets.cpu,
        player_kills: 0,
        horizontal_deadzone: deadzone,
    };
    melee_cpu::think(&mut own.0, &mut scene, &mut state.rng);
}
