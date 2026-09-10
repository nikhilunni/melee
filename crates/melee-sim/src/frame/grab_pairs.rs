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

pub(super) fn align(state: &mut InitialState, player: usize) {
    if with_fighter!(&state.fighters[player], |f| f.combat.thrown_pose.is_some()) {
        return;
    }
    let link = with_fighter!(&state.fighters[player], |f| f.combat.grab);
    let Some(GrabLink::Captured { captor }) = link else {
        return;
    };
    let other = state
        .fighters
        .iter()
        .position(|f| with_fighter!(f, |f| f.spawn_number == captor))
        .expect("live captor");
    let (victim, attacker) = pair(&mut state.fighters, player, other);
    with_fighter!(attacker, |a| with_fighter!(
        victim,
        |v| grab::align_capture(v, a, &state.assets.fighters[player])
    ));
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
    with_fighter!(&mut state.fighters[other], |f| {
        if f.motion_state.id == melee_types::CommonMotionState::CapturePulledLw {
            grab::capture_wait(f, &state.assets.fighters[other])
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Ok(())
    })
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
    let captor = with_fighter!(&state.fighters[player], |f| {
        if f.motion_state.id == melee_types::CommonMotionState::CaptureWaitLw {
            let melee_ft::fighter::MotionData::Capture(capture) = &f.state_data else {
                panic!("capture scratch missing")
            };
            if capture.timer <= 0.0 {
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
        )
    }))
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
