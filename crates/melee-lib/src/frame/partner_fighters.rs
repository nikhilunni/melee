//! A player's two fighters that reach into each other (Popo and Nana): the
//! scene owns both, so it hands a fighter its partner around each proc
//! (`CharacterCallbacks::OBSERVE_PARTNER` / `ACT_ON_PARTNER`).
use crate::initial_state::InitialState;
use crate::scene_fighter::with_fighter;
use anyhow::Result;
use melee_ft::fighter::RetailTrig;

/// The other fighter of `index`'s player, if it has one.
fn partner_of(state: &InitialState, index: usize) -> Option<usize> {
    let player = state.fighters[index].0.player.id;
    (0..state.fighters.len())
        .find(|&other| other != index && state.fighters[other].0.player.id == player)
}

/// Before one of `index`'s procs: the fighter reads its partner.
pub(super) fn observe(state: &mut InitialState, index: usize) {
    let Some(observe) = state.fighters[index].0.character.table().observe_partner else {
        return;
    };
    let Some(partner) = partner_of(state, index) else {
        return;
    };
    let (fighter, partner) = pair(&mut state.fighters, index, partner);
    with_fighter!(fighter, |f| with_fighter!(partner, |p| observe(f, p)));
}

/// After one of `index`'s procs: what it left for its partner. A fighter
/// whose motion this changed flushes its efAsync queue, as its
/// Fighter_ChangeMotionState did inside the proc (fighter.c:951).
pub(super) fn act(state: &mut InitialState, index: usize) -> Result<()> {
    let Some(act) = state.fighters[index].0.character.table().act_on_partner else {
        return Ok(());
    };
    let Some(other) = partner_of(state, index) else {
        return Ok(());
    };
    let (fighter, partner) = pair(&mut state.fighters, index, other);
    let changed = with_fighter!(fighter, |f| with_fighter!(partner, |p| act(
        f,
        p,
        &state.assets.fighters[index],
        &state.assets.fighters[other],
        &mut state.map
    )))
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    for (member, moved) in [(index, changed.fighter), (other, changed.partner)] {
        if !moved {
            continue;
        }
        with_fighter!(&mut state.fighters[member], |f| state
            .effects
            .flush::<RetailTrig>(
                melee_ef::EffectTiming::Immediate,
                member,
                &mut f.core,
                &state.assets.common_particle_bank,
                &state.assets.particle_bank,
                &mut state.particles,
                &mut state.rng,
            ))?;
    }
    Ok(())
}

/// The stale-move table is the player's (Player_GetStaleMoveTableIndexPtr):
/// a player's second fighter (Nana, or a sleeping transformation) and its
/// first keep copies, and whichever changed last hands its copy over. The
/// one that takes it restales its current move's hitboxes to come
/// (ftColl_8007ABD0 reads the table as each is made).
pub(super) fn share_stale_tables(state: &mut InitialState) {
    for second in 0..state.fighters.len() {
        if !state.fighters[second].0.player.secondary {
            continue;
        }
        let Some(first) = partner_of(state, second) else {
            continue;
        };
        let (a, b) = pair(&mut state.fighters, first, second);
        let (a, b) = (&mut a.0, &mut b.0);
        let (taker, index) = if a.combat.stale.share_table(&b.combat.stale) {
            (a, first)
        } else if b.combat.stale.share_table(&a.combat.stale) {
            (b, second)
        } else {
            continue;
        };
        let stale = &taker.combat.stale;
        taker.commands.stale_multiplier = stale
            .current_move()
            .map(|_| stale.multiplier(&state.assets.fighters[index].stale_weights));
    }
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
