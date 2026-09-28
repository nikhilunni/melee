//! A player's transformation (Zelda and Sheik): the scene owns both of the
//! player's fighters, so it performs ftCommon_8007EFC8 when the form in
//! play asks from its accessory4 (Fighter_8006C80C, s_link 9).
use crate::initial_state::InitialState;
use crate::scene_fighter::with_fighter;
use anyhow::Result;
use melee_ft::fighter::{
    transform::{transfer, TransferContext},
    RetailTrig,
};

/// Hand fighter `index`'s player to its partner if the fighter asked this
/// proc. Returns true when the partner took over.
pub(super) fn perform(state: &mut InitialState, index: usize) -> Result<bool> {
    if !state.fighters[index].0.core.take_transformation_request() {
        return Ok(false);
    }
    let slot = state.fighters[index].0.player.id;
    let partner = state
        .fighters
        .iter()
        .enumerate()
        .position(|(other, f)| other != index && f.0.player.id == slot)
        .expect("a transforming player has a partner");
    let (src, dst) = pair(&mut state.fighters, index, partner);
    let src_assets = &state.assets.fighters[index];
    let dst_assets = &state.assets.fighters[partner];
    with_fighter!(src, |src| with_fighter!(dst, |dst| {
        transfer(
            src,
            dst,
            src_assets,
            dst_assets,
            TransferContext {
                map: &state.map,
                counter: &mut state.spawn_counter,
                stage_camera: &state.assets.stage_camera,
            },
        )
    }))
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    // Each Fighter_ChangeMotionState (the form in play going to sleep,
    // then its partner's arrival) flushed that fighter's efAsync queue.
    for fighter in [index, partner] {
        with_fighter!(&mut state.fighters[fighter], |f| state
            .effects
            .flush::<RetailTrig>(
                melee_ef::EffectTiming::Immediate,
                fighter,
                &mut f.core,
                &state.assets.common_particle_bank,
                &state.assets.particle_bank,
                &mut state.particles,
                &mut state.rng,
            ))?;
    }
    Ok(true)
}

/// Two distinct fighters of the list, mutably.
fn pair<T>(fighters: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);
    if a < b {
        let (left, right) = fighters.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = fighters.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}
