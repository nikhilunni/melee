//! A Time match's standings (gm_80166378, gm_1601.c:2981-3079): each
//! player's score from KOs, falls and self-destructs, and the ranks
//! Player_80033BB8 (gm_8016C5C0) and the results read from them.
use crate::{initial_state::InitialState, ko_counts::KoCounts};

/// Player slots the standings hold.
const SLOTS: usize = 6;
/// fn_8016588C_clamp: a score stays within +-(2^24 - 1).
const SCORE_LIMIT: i32 = (1 << 24) - 1;

/// One player's tally (MatchPlayerData).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    pub present: bool,
    pub team: Option<u8>,
    /// x20: KOs on other players (other teams' in a team battle).
    pub kos: i32,
    /// x24: falls, with the player's KOs of itself and its teammates.
    pub falls: i32,
    /// xA (u16): self-destructs, with the same KOs.
    pub self_destructs: u16,
}

impl Tally {
    /// fn_8016588C for a Time match (x5 == 0): x20 - (x24 - xA) + xA * xC,
    /// clamped.
    pub(crate) fn time_score(&self, penalty: i8) -> i32 {
        let a = i32::from(self.self_destructs);
        let v = self
            .kos
            .wrapping_sub(self.falls.wrapping_sub(a))
            .wrapping_add(a.wrapping_mul(i32::from(penalty)));
        v.clamp(-SCORE_LIMIT, SCORE_LIMIT)
    }
}

/// gm_80166378's tallies of every player: `falls` and `self_destructs` per
/// slot (Player_GetFalls, Player_GetSuicideCount), the KO table.
pub(crate) fn tallies(
    players: &[(u8, Option<u8>, u32, u32)],
    kos: &KoCounts,
) -> [Tally; SLOTS] {
    let mut out = [Tally::default(); SLOTS];
    let mut teams = [None; 4];
    for &(slot, team, ..) in players {
        if let Some(t) = teams.get_mut(usize::from(slot)) {
            *t = team;
        }
    }
    for &(slot, team, falls, self_destructs) in players {
        let i = usize::from(slot);
        let own_side = kos.own_side(slot, &teams);
        out[i] = Tally {
            present: true,
            team,
            kos: kos.total(slot, &teams),
            falls: (falls as i32).wrapping_add(own_side as i32),
            self_destructs: (self_destructs as u16).wrapping_add(own_side as u16),
        };
    }
    out
}

/// fn_80165AC0 / fn_80165FA4: how many players (or teams) scored more.
fn losers<const N: usize>(scores: &[Option<i32>; N]) -> [u8; N] {
    scores.map(|own| match own {
        None => 0,
        Some(own) => scores.iter().flatten().filter(|&&other| own < other).count() as u8,
    })
}

/// gm_8016C5C0 in a Time match: each slot's is_big_loser, or in a team
/// battle its team's (fn_80165E7C sums the members' scores).
pub(crate) fn time_ranks(tallies: &[Tally; SLOTS], penalty: i8) -> [u8; SLOTS] {
    let scores: [Option<i32>; SLOTS] = tallies.map(|t| t.present.then(|| t.time_score(penalty)));
    let mut ranks = [0; SLOTS];
    if tallies.iter().any(|t| t.present && t.team.is_some()) {
        // team_standings[team]: active teams, score the members' sum.
        let mut team_scores = [None; 5];
        for (t, score) in tallies.iter().zip(&scores) {
            if let (Some(team), Some(score)) = (t.team, score) {
                let slot: &mut Option<i32> = &mut team_scores[usize::from(team)];
                *slot = Some(slot.unwrap_or(0).wrapping_add(*score));
            }
        }
        let team_ranks = losers(&team_scores);
        for (rank, t) in ranks.iter_mut().zip(tallies) {
            if let Some(team) = t.team {
                *rank = team_ranks[usize::from(team)];
            }
        }
    } else {
        ranks = losers(&scores);
    }
    ranks
}

/// Each player's tally as the scene holds it now.
pub(crate) fn current(state: &InitialState) -> [Tally; SLOTS] {
    use crate::scene_fighter::with_fighter;
    let mut players = [(0u8, None, 0u32, 0u32); SLOTS];
    let mut count = 0;
    for fighter in &state.fighters {
        let (slot, team, falls, sd, secondary) = with_fighter!(fighter, |f| (
            f.player.id,
            f.player.team,
            f.player.falls,
            f.player.self_destructs,
            f.player.secondary
        ));
        // Player_GetFalls sums a player's two fighters' falls.
        match players[..count].iter_mut().find(|p| p.0 == slot) {
            Some(p) => {
                p.2 += falls;
                if !secondary {
                    p.3 = sd;
                }
            }
            None => {
                players[count] = (slot, team, falls, if secondary { 0 } else { sd });
                count += 1;
            }
        }
    }
    tallies(&players[..count], &state.ko_counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_score_counts_kos_falls_and_self_destructs() {
        let mut kos = KoCounts::default();
        kos.record(0, 1); // P1 KOs P2
        kos.record(0, 1);
        kos.record(1, 1); // P2 KOs itself
        // P2 fell three times, one a self-destruct beside its own KO.
        let t = tallies(&[(0, None, 0, 0), (1, None, 4, 1)], &kos);
        assert_eq!(t[0].time_score(-1), 2);
        // falls 4 + 1 own KO; self-destructs 1 + 1: 0 - (5 - 2) - 2.
        assert_eq!(t[1].time_score(-1), -5);
        assert_eq!(time_ranks(&t, -1)[..2], [0, 1]);
    }

    #[test]
    fn tied_scores_share_the_top_rank() {
        let kos = KoCounts::default();
        let t = tallies(&[(0, None, 0, 0), (2, None, 0, 0)], &kos);
        assert_eq!(time_ranks(&t, -1)[0], 0);
        assert_eq!(time_ranks(&t, -1)[2], 0);
    }
}
