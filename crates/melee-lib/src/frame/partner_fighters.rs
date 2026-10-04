//! A player's two fighters that reach into each other (Popo and Nana): the
//! scene owns both, so it hands a fighter its partner around each proc
//! (`CharacterCallbacks::OBSERVE_PARTNER` / `ACT_ON_PARTNER`, and the
//! revival states' common view, `PartnerView`). They share what retail
//! keeps on the player (StaticPlayer): the stock count, the stale table and
//! the spawn point; the leader's death ends both (ftCo_800BFD9C).
use crate::initial_state::InitialState;
use crate::scene_fighter::{with_fighter, SceneFighter};
use anyhow::Result;
use melee_ft::fighter::{
    life::LifeState, partner::PartnerView, MotionData, RetailTrig, SpawnContext,
};

/// The other fighter of `index`'s player, if it has one.
fn partner_of(state: &InitialState, index: usize) -> Option<usize> {
    let player = state.fighters[index].0.player.id;
    (0..state.fighters.len())
        .find(|&other| other != index && state.fighters[other].0.player.id == player)
}

/// Before one of `index`'s procs: the fighter reads its partner, what the
/// common revival states read (ft_0D4D.c's Player_GetEntityAtIndex) and
/// what its character's callbacks do.
pub(super) fn observe(
    state: &mut InitialState,
    index: usize,
    proc: melee_ft::fighter::state::FighterProc,
) {
    let Some(partner) = partner_of(state, index) else {
        return;
    };
    let view = PartnerView::of(&state.fighters[partner].0);
    state.fighters[index].0.core.partner = Some(view);
    let Some(observe) = state.fighters[index].0.character.table().observe_partner else {
        return;
    };
    let (fighter, partner) = pair(&mut state.fighters, index, partner);
    with_fighter!(fighter, |f| with_fighter!(partner, |p| observe(f, p, proc)));
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

/// After a fighter's proc: a stock loss (ftCo_800D34E0) counted a KO for
/// the player that last hit the fighter (Player_UpdateKOsBySlot) and
/// changed the player's stock count, which its other fighter's copy follows
/// (the emptied stale table follows by `share_stale_tables`).
pub(super) fn share_fall(state: &mut InitialState, index: usize) {
    if !std::mem::take(&mut state.fighters[index].fell) {
        return;
    }
    if let Some(source) = state.fighters[index].fall_credit.take() {
        let fallen = state.fighters[index].player.id;
        state.ko_counts.record(source, fallen);
    }
    let Some(partner) = partner_of(state, index) else {
        return;
    };
    let stocks = state.fighters[index].player.stocks;
    state.fighters[partner].player.stocks = stocks;
}

fn awaiting_respawn(fighter: &SceneFighter) -> bool {
    matches!(
        fighter.state_data,
        MotionData::Life(LifeState::AwaitingRespawn)
    )
}

/// ftCo_800BFD9C (800BFD9C) once a death's countdown ends: Sleep; the
/// leader's death takes its partner out too (ftCo_800D4F24); then
/// gm_80167320's respawn (fn_8016719C, Player_80032070). The dying fighter's
/// own Sleep lasts no tick: its revival follows at once.
pub(super) fn complete_death(state: &mut InitialState, index: usize) -> Result<()> {
    if !awaiting_respawn(&state.fighters[index]) {
        return Ok(());
    }
    let partner = partner_of(state, index);
    if state.fighters[index].player.secondary {
        let leader = partner.expect("a second fighter's leader");
        return complete_partner_death(state, index, leader);
    }
    let leads = state.fighters[index].capabilities.leads_partner;
    if let Some(partner) = partner.filter(|_| leads) {
        if !state.fighters[partner].status.disabled {
            vanish(state, partner)?;
        }
    }
    let InitialState {
        fighters,
        assets,
        revival_offsets,
        map,
        rng,
        spawn_counter,
        ..
    } = state;
    let fighter = &mut fighters[index];
    // ftCo_800BFD04 (800BFD04): Sleep for an instant. Its motion change turns
    // the root back to the facing (fighter.c:1173-1175), which a screen KO
    // had turned to the screen. The respawn probe (ft_80082A68) reads its ECB
    // from that pose, and the first Rebirth collision splits its move by the
    // ECB change (mpColl_80043754): a wide enough one adds a step and moves
    // the summed position by an ulp.
    with_fighter!(fighter, |f| f.enter_sleep(&assets.fighters[index]))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let platform = fighter.place_revival(&assets.arena, revival_offsets);
    with_fighter!(fighter, |f| f.revive_at(
        &assets.fighters[index],
        platform,
        SpawnContext {
            map,
            stage_camera: &assets.stage_camera,
            rng,
            counter: spawn_counter,
        },
    ))
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    // Player_80032070: an asleep partner revives beside the leader.
    if let Some(partner) = partner.filter(|_| leads) {
        if fighters[partner].status.disabled {
            let leader = fighters[index].player.clone();
            with_fighter!(&mut fighters[partner], |f| f.revive_beside_leader(
                &assets.fighters[partner],
                platform,
                &leader,
                SpawnContext {
                    map,
                    stage_camera: &assets.stage_camera,
                    rng,
                    counter: spawn_counter,
                },
            ))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
    }
    Ok(())
}

/// ftCo_800BFD9C for the second fighter whose own death ends first
/// (ftcolanim.c:49-60): Sleep (ftCo_800BFD04), then gm_80167320(slot, 1)
/// (gm_1601.c:3324-3351): fn_8016719C places the player's spawn point anew,
/// taking a revival slot, and Player_80032070(slot, 1) (player.c:379-381)
/// revives the fighter there only while its leader is in Rebirth or
/// RebirthWait. Otherwise it sleeps until the leader's next revival brings
/// it back (`complete_death`).
fn complete_partner_death(state: &mut InitialState, index: usize, leader: usize) -> Result<()> {
    if state.fighters[leader].reviving() {
        unimplemented!(
            "Player_80032070(slot, 1): the second fighter revives at its own spawn \
             slot while its leader is on its platform"
        );
    }
    let InitialState {
        fighters,
        assets,
        revival_offsets,
        ..
    } = state;
    let fighter = &mut fighters[index];
    with_fighter!(fighter, |f| f.enter_sleep(&assets.fighters[index]))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    fighter.place_revival(&assets.arena, revival_offsets);
    Ok(())
}

/// ftCo_800D4F24(partner, 1): the partner vanishes with its leader. The
/// puff is efSync (immediate), so the partner's queue flushes at once,
/// ahead of the leader's revival.
fn vanish(state: &mut InitialState, partner: usize) -> Result<()> {
    with_fighter!(&mut state.fighters[partner], |f| f
        .vanish_with_leader(&state.assets.fighters[partner]))
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    with_fighter!(&mut state.fighters[partner], |f| state
        .effects
        .flush::<RetailTrig>(
            melee_ef::EffectTiming::Immediate,
            partner,
            &mut f.core,
            &state.assets.common_particle_bank,
            &state.assets.particle_bank,
            &mut state.particles,
            &mut state.rng,
        ))?;
    Ok(())
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
