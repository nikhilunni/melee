//! Fighter-list traversal for grabs; character implementations stay in melee-ft.
use super::*;
use crate::scene_fighter::{with_fighter, SceneFighter};
use melee_ft::fighter::grab::{self, GrabLink};
use melee_ft::fighter::grab_throw;

fn pair(
    fighters: &mut [SceneFighter],
    first: usize,
    second: usize,
) -> (&mut SceneFighter, &mut SceneFighter) {
    assert_ne!(first, second);
    if first < second {
        let (left, right) = fighters.split_at_mut(second);
        (&mut left[first], &mut right[0])
    } else {
        let (left, right) = fighters.split_at_mut(first);
        (&mut right[0], &mut left[second])
    }
}

pub(super) fn select(state: &mut InitialState, player: usize) -> Result<()> {
    let mut nearest = None;
    let mut distance = f32::MAX;
    for other in 0..state.fighters.len() {
        if other == player {
            continue;
        }
        let (attacker, victim) = pair(&mut state.fighters, player, other);
        let candidate = with_fighter!(attacker, |a| with_fighter!(victim, |v| grab::candidate(
            v, a
        )));
        if let Some(candidate) = candidate {
            if candidate < distance {
                distance = candidate;
                nearest = Some(other);
            }
        }
    }
    if let Some(other) = nearest {
        // gm_8016C5C0 / fn_8016588C: stock standings, ties share a rank.
        let stocks = with_fighter!(&state.fighters[other], |f| f.player.stocks);
        let rank = state
            .fighters
            .iter()
            .filter(|f| with_fighter!(f, |f| f.player.stocks > stocks))
            .count() as u8;
        let (attacker, victim) = pair(&mut state.fighters, player, other);
        with_fighter!(attacker, |a| with_fighter!(victim, |v| grab::capture_pair(
            v,
            a,
            &state.assets.fighters[other],
            &state.assets.fighters[player],
            &mut state.map,
            rank
        )))
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }
    Ok(())
}

pub(super) fn align(state: &mut InitialState, player: usize) -> Result<()> {
    // fn_800DAD18 is a physics callback; Fighter_procUpdate skips it during
    // hitlag. It still runs for a capture pinned in the captor's mouth.
    if with_fighter!(&state.fighters[player], |f| f.status.disabled
        || f.in_hitlag()
        || matches!(
            f.motion_state.id,
            melee_types::CommonMotionState::ThrownF
                | melee_types::CommonMotionState::ThrownB
                | melee_types::CommonMotionState::ThrownHi
                | melee_types::CommonMotionState::ThrownLw
        ))
    {
        return Ok(());
    }
    let link = with_fighter!(&state.fighters[player], |f| f.combat.grab);
    let Some(GrabLink::Captured { captor }) = link else {
        return Ok(());
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == captor))
        .expect("live captor");
    let (victim, attacker) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(
        victim,
        |v| grab::align_capture(v, a, &state.assets.fighters[player], &mut state.map)
    ))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub(super) fn sync_wait(state: &mut InitialState, player: usize) -> Result<()> {
    let victim = with_fighter!(&state.fighters[player], |f| {
        if f.motion_state.id == melee_types::CommonMotionState::CatchWait {
            if let Some(GrabLink::Holding { victim, .. }) = f.combat.grab {
                Some(victim)
            } else {
                None
            }
        } else {
            None
        }
    });
    let Some(victim) = victim else {
        return Ok(());
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == victim))
        .expect("live captured fighter");
    let pulled = with_fighter!(&state.fighters[other], |f| matches!(
        f.motion_state.id,
        melee_types::CommonMotionState::CapturePulledLw
            | melee_types::CommonMotionState::CapturePulledHi
    ));
    if !pulled {
        return Ok(());
    }
    let (attacker, victim) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        let entered = grab::capture_wait(v, &state.assets.fighters[other]);
        if entered.is_ok() {
            grab::constrain_in_mouth(
                &mut v.core,
                &mut a.core,
                &state.assets.fighters[other],
                &state.assets.fighters[player],
            );
        }
        entered
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub(super) fn throw_input(state: &mut InitialState, player: usize) -> Result<()> {
    let Some(throw) = with_fighter!(&state.fighters[player], |f| {
        grab_throw::requested(f, &state.assets.fighters[player])
    }) else {
        return Ok(());
    };
    let link = with_fighter!(&state.fighters[player], |f| f.combat.grab);
    let Some(GrabLink::Holding { victim, .. }) = link else {
        panic!("throw without victim");
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == victim))
        .expect("live victim");
    let (attacker, victim) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        grab_throw::enter_throw(
            throw,
            v,
            a,
            &state.assets.fighters[other],
            &state.assets.fighters[player],
        )
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub(super) fn constrain(state: &mut InitialState, player: usize) {
    if !with_fighter!(&state.fighters[player], |f| f.combat.thrown_pose.is_some()) {
        return;
    }
    let link = with_fighter!(&state.fighters[player], |f| f.combat.grab);
    let Some(GrabLink::Captured { captor }) = link else {
        panic!("thrown without captor");
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == captor))
        .expect("live captor");
    let (victim, attacker) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        grab_throw::update_constraint(
            v,
            a,
            &state.assets.fighters[player],
            &state.assets.fighters[other],
        )
    }));
}

pub(super) fn release(state: &mut InitialState, player: usize) -> Result<()> {
    if !with_fighter!(&state.fighters[player], |f| matches!(
        f.motion_state.id,
        melee_types::CommonMotionState::ThrowF
            | melee_types::CommonMotionState::ThrowB
            | melee_types::CommonMotionState::ThrowHi
            | melee_types::CommonMotionState::ThrowLw
    ) && f.commands.grab_release)
    {
        return Ok(());
    }
    let link = with_fighter!(&state.fighters[player], |f| f.combat.grab);
    let Some(GrabLink::Holding { victim, .. }) = link else {
        panic!("throw without victim");
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == victim))
        .expect("live victim");
    let (attacker, victim) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        grab_throw::release_throw(
            v,
            a,
            &state.assets.fighters[other],
            &state.assets.fighters[player],
            &mut state.map,
            &mut state.rng,
        )
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// CaptureWait's timer is owned by the victim; release both before overlap.
pub(super) fn escape(state: &mut InitialState, player: usize) -> Result<()> {
    let captor = with_fighter!(&mut state.fighters[player], |f| {
        if matches!(
            f.motion_state.id,
            melee_types::CommonMotionState::CaptureWaitLw
                | melee_types::CommonMotionState::CaptureWaitHi
        ) {
            let melee_ft::fighter::MotionData::Capture(capture) = &mut f.core.state_data else {
                panic!("capture scratch missing")
            };
            if std::mem::take(&mut capture.release_requested) {
                let Some(GrabLink::Captured { captor }) = f.combat.grab else {
                    panic!("capture without captor")
                };
                Some(captor)
            } else {
                None
            }
        } else {
            None
        }
    });
    let Some(captor) = captor else {
        return Ok(());
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == captor))
        .expect("live captor");
    let (victim, attacker) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        melee_ft::fighter::grab_escape::release(
            v,
            a,
            &state.assets.fighters[player],
            &state.assets.fighters[other],
            melee_ft::fighter::grab_escape::ReleaseCause::TimerExpired,
        )
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Map owns both fighters because counterpart entries recompute captor.x2170.
pub(super) fn map_capture(state: &mut InitialState, player: usize) -> Result<()> {
    let link = with_fighter!(&state.fighters[player], |f| {
        if f.status.disabled || f.combat.thrown_pose.is_some() {
            None
        } else {
            f.combat.grab
        }
    });
    let Some(GrabLink::Captured { captor }) = link else {
        return Ok(());
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == captor))
        .expect("live captor");
    let (victim, attacker) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        grab::map_capture(v, a, &state.assets.fighters[player], &mut state.map, true)
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// fn_800DA190 / DA4A0 / DA678: holding-state accessory alignment.
/// Returns the victim's index when the captor's separation released the pair.
pub(super) fn accessory(state: &mut InitialState, player: usize) -> Result<Option<usize>> {
    let victim = with_fighter!(&state.fighters[player], |f| {
        if f.status.disabled
            || f.in_hitlag()
            || !matches!(f.state_data, melee_ft::fighter::MotionData::Catch { .. })
        {
            None
        } else if let Some(GrabLink::Holding { victim, .. }) = f.combat.grab {
            Some(victim)
        } else {
            None
        }
    });
    let Some(victim) = victim else {
        return Ok(None);
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == victim))
        .expect("live capture accessory victim");
    let (attacker, victim) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(victim, |v| {
        if grab::capture_accessory(v, a, &state.assets.fighters[other]) {
            melee_ft::fighter::grab_escape::release(
                v,
                a,
                &state.assets.fighters[other],
                &state.assets.fighters[player],
                melee_ft::fighter::grab_escape::ReleaseCause::CaptorSeparation,
            )
            .map(|()| Some(other))
        } else {
            Ok(None)
        }
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Fighter_ProcessHit -> ftCo_8008EC90: a launched member of a grab pair
/// decides for both before its own hit processing.
pub(super) fn linked_hit(state: &mut InitialState, player: usize) -> Result<()> {
    let link = with_fighter!(&state.fighters[player], |f| f.combat.grab);
    let partner = match link {
        Some(GrabLink::Holding { victim, .. }) => victim,
        Some(GrabLink::Captured { captor }) => captor,
        None => return Ok(()),
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == partner))
        .expect("live grab partner");
    let (fighter, partner) = pair(&mut state.fighters, player, other);
    let pause_effects = with_fighter!(fighter, |f| with_fighter!(partner, |p| {
        melee_ft::fighter::grab_damage::resolve_linked_hit(
            f,
            p,
            &state.assets.fighters[player],
            &state.assets.fighters[other],
            &mut state.map,
            &mut state.rng,
        )
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    if pause_effects {
        // ftCo_8008EC90: the captor's pre_hitlag_cb (efLib_PauseAll) runs on
        // this captured fighter's gobj.
        state.effects.set_owner_hitlag(player, true);
    }
    Ok(())
}

/// ftCo_8008EC90's tail for a launched captured fighter whose captor was not
/// hit: after this fighter's launch, ftCommon_8007DB58 and ftCo_800DE2F0 on
/// the captor it was freed from.
pub(super) fn release_captor(state: &mut InitialState, player: usize) -> Result<()> {
    let captor =
        with_fighter!(&mut state.fighters[player], |f| f.combat.release_captor.take());
    let Some(captor) = captor else {
        return Ok(());
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == captor))
        .expect("released captor");
    with_fighter!(&mut state.fighters[other], |f| {
        melee_ft::fighter::grab_damage::launch_released_captor(
            f,
            &state.assets.fighters[other],
            &mut state.rng,
        )
    })
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// x2219_b5 for `player` (melee_ft::fighter::hitlag_link).
pub(super) fn in_hitlag(state: &InitialState, player: usize) -> bool {
    with_fighter!(&state.fighters[player], |f| f.in_hitlag())
}

/// The other member of `player`'s grab pair (x1A5C), by fighter index.
fn grab_partner(state: &InitialState, player: usize) -> Option<usize> {
    let partner = match with_fighter!(&state.fighters[player], |f| f.combat.grab)? {
        GrabLink::Holding { victim, .. } => victim,
        GrabLink::Captured { captor } => captor,
    };
    state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == partner))
}

/// Fighter_ProcessHit's Fighter_UnkRecursiveFunc_8006D044: when `player`
/// started hitlag during its ProcessHit (x2219_b5 was clear), its grab
/// partner is held in hitlag with it.
pub(super) fn hold_partner_hitlag(
    state: &mut InitialState,
    player: usize,
    was_in_hitlag: bool,
) -> Result<()> {
    if was_in_hitlag || !in_hitlag(state, player) {
        return Ok(());
    }
    let Some(other) = grab_partner(state, player) else {
        return Ok(());
    };
    let (fighter, partner) = pair(&mut state.fighters, player, other);
    let pause = with_fighter!(fighter, |f| with_fighter!(partner, |p| {
        melee_ft::fighter::hitlag_link::hold_partner(f, p)
    }));
    if pause {
        state.effects.set_owner_hitlag(other, true);
    }
    Ok(())
}

/// Fighter_8006A1BC -> Fighter_8006D10C: when `player`'s hitlag ended in its
/// status proc, a grab partner it holds is released.
pub(super) fn release_partner_hitlag(
    state: &mut InitialState,
    player: usize,
    was_in_hitlag: bool,
) -> Result<()> {
    if !was_in_hitlag || in_hitlag(state, player) {
        return Ok(());
    }
    let Some(other) = grab_partner(state, player) else {
        return Ok(());
    };
    let (fighter, partner) = pair(&mut state.fighters, player, other);
    let resume = with_fighter!(fighter, |f| with_fighter!(partner, |p| {
        melee_ft::fighter::hitlag_link::release_partner(f, p)
    }));
    if resume {
        state.effects.set_owner_hitlag(other, false);
    }
    Ok(())
}
