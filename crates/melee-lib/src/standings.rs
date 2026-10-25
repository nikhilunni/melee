//! A match's standings (gm_80166378, gm_1601.c:2981-3079): each player's
//! score (fn_8016588C) and the ranks Player_80033BB8 (gm_8016C5C0) and the
//! results read from them (fn_80165AC0, fn_80165E7C, fn_80165FA4).
use crate::{initial_state::InitialState, ko_counts::KoCounts, match_clock::Scoring};
use melee_types::MatchKind;

/// Player slots the standings hold.
const SLOTS: usize = 6;
/// fn_8016588C_clamp: a score stays within +-(2^24 - 1).
const SCORE_LIMIT: i32 = (1 << 24) - 1;
/// fn_8016588C's base for an eliminated player in a stock match: its
/// elimination time in seconds counts up from here (0xFF000001).
const ELIMINATED_BASE: u32 = 0xFF00_0001;

/// What the scene holds of one player (StaticPlayer).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlayerRecord {
    pub slot: u8,
    pub team: Option<u8>,
    /// Player_GetFalls: the player's fighters' falls.
    pub falls: u32,
    /// Player_GetSuicideCount.
    pub self_destructs: u32,
    /// Player_GetStocks.
    pub stocks: u8,
    /// Player_GetMatchFrameCount: the match frame its last stock went
    /// (Player_UpdateMatchFrameCount); `None` for retail's -1.
    pub eliminated_at: Option<u32>,
}

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
    /// stocks.
    pub stocks: u8,
    /// x28: the elimination frame, or the match's frame count (gm_80166378
    /// fills -1 with gm_GetFrameCount).
    pub frames: u32,
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

    /// fn_8016588C for a stock match (x5 == 1): the stocks left, or for an
    /// eliminated player x28 / 60 + 0xFF000001 (the later out, the higher).
    pub(crate) fn stock_score(&self) -> i32 {
        let v = if self.stocks != 0 {
            i32::from(self.stocks)
        } else {
            (self.frames / 60).wrapping_add(ELIMINATED_BASE) as i32
        };
        v.clamp(-SCORE_LIMIT, SCORE_LIMIT)
    }

    fn score(&self, scoring: Scoring) -> i32 {
        match scoring.match_kind {
            MatchKind::Time => self.time_score(scoring.self_destruct_penalty),
            MatchKind::Stock => self.stock_score(),
            kind => unimplemented!("fn_8016588C: a {kind:?} match's score (coins, bonus points)"),
        }
    }
}

/// gm_80166378's tallies of every player, at match frame `frame_count`.
pub(crate) fn tallies(players: &[PlayerRecord], kos: &KoCounts, frame_count: u32) -> [Tally; SLOTS] {
    let mut out = [Tally::default(); SLOTS];
    let mut teams = [None; 4];
    for p in players {
        if let Some(t) = teams.get_mut(usize::from(p.slot)) {
            *t = p.team;
        }
    }
    for p in players {
        let own_side = kos.own_side(p.slot, &teams);
        out[usize::from(p.slot)] = Tally {
            present: true,
            team: p.team,
            kos: kos.total(p.slot, &teams),
            falls: (p.falls as i32).wrapping_add(own_side as i32),
            self_destructs: (p.self_destructs as u16).wrapping_add(own_side as u16),
            stocks: p.stocks,
            frames: p.eliminated_at.unwrap_or(frame_count),
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

/// fn_80165E7C: a team's score from its members' in slot order. A stock
/// match keeps the best eliminated member's (negative) score until a
/// member with stocks replaces it, then sums the members with stocks; other
/// matches sum.
fn team_score(team: Option<i32>, player: i32, kind: MatchKind) -> i32 {
    let team = team.unwrap_or(0);
    if kind != MatchKind::Stock {
        return team.wrapping_add(player);
    }
    if player < 0 {
        if team == 0 || (team < 0 && team < player) {
            player
        } else {
            team
        }
    } else if team < 0 {
        player
    } else {
        team.wrapping_add(player)
    }
}

/// gm_8016C5C0: each slot's is_big_loser, or in a team battle its team's.
pub(crate) fn ranks(tallies: &[Tally; SLOTS], scoring: Scoring) -> [u8; SLOTS] {
    let scores: [Option<i32>; SLOTS] = tallies.map(|t| t.present.then(|| t.score(scoring)));
    if !tallies.iter().any(|t| t.present && t.team.is_some()) {
        return losers(&scores);
    }
    let mut team_scores = [None; 5];
    for (t, score) in tallies.iter().zip(&scores) {
        if let (Some(team), Some(score)) = (t.team, score) {
            let slot: &mut Option<i32> = &mut team_scores[usize::from(team)];
            *slot = Some(team_score(*slot, *score, scoring.match_kind));
        }
    }
    let team_ranks = losers(&team_scores);
    let mut ranks = [0; SLOTS];
    for (rank, t) in ranks.iter_mut().zip(tallies) {
        if let Some(team) = t.team {
            *rank = team_ranks[usize::from(team)];
        }
    }
    ranks
}

/// Each player's record as the scene holds it now.
pub(crate) fn records(state: &InitialState) -> ([PlayerRecord; SLOTS], usize) {
    use crate::scene_fighter::with_fighter;
    let mut players = [PlayerRecord::default(); SLOTS];
    let mut count = 0;
    for fighter in &state.fighters {
        let (slot, team, falls, sd, secondary, stocks) = with_fighter!(fighter, |f| (
            f.player.id,
            f.player.team,
            f.player.falls,
            f.player.self_destructs,
            f.player.secondary,
            f.player.stocks
        ));
        // Player_GetFalls sums a player's two fighters' falls.
        match players[..count].iter_mut().find(|p| p.slot == slot) {
            Some(p) => {
                p.falls += falls;
                if !secondary {
                    p.self_destructs = sd;
                    p.stocks = stocks;
                }
            }
            None => {
                players[count] = PlayerRecord {
                    slot,
                    team,
                    falls,
                    self_destructs: if secondary { 0 } else { sd },
                    stocks,
                    eliminated_at: state.eliminated_at[usize::from(slot)],
                };
                count += 1;
            }
        }
    }
    (players, count)
}

/// gm_8016C5C0 for every slot, as the scene holds the match now.
pub(crate) fn current_ranks(state: &InitialState) -> [u8; SLOTS] {
    let (players, count) = records(state);
    ranks(
        &tallies(&players[..count], &state.ko_counts, state.clock.frame_count),
        state.clock.scoring,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(slot: u8, falls: u32, self_destructs: u32) -> PlayerRecord {
        PlayerRecord {
            slot,
            falls,
            self_destructs,
            ..Default::default()
        }
    }

    #[test]
    fn a_time_score_counts_kos_falls_and_self_destructs() {
        let mut kos = KoCounts::default();
        kos.record(0, 1); // P1 KOs P2 twice
        kos.record(0, 1);
        kos.record(1, 1); // P2 KOs itself
        // P2 fell four times: two to P1, its own KO, one self-destruct.
        let t = tallies(&[player(0, 0, 0), player(1, 4, 1)], &kos, 0);
        assert_eq!(t[0].time_score(-1), 2);
        // falls 4 + 1 own KO; self-destructs 1 + 1: 0 - (5 - 2) - 2.
        assert_eq!(t[1].time_score(-1), -5);
        let time = Scoring {
            match_kind: MatchKind::Time,
            self_destruct_penalty: -1,
        };
        assert_eq!(ranks(&t, time)[..2], [0, 1]);
    }

    #[test]
    fn tied_scores_share_the_top_rank() {
        let t = tallies(&[player(0, 0, 0), player(2, 0, 0)], &KoCounts::default(), 0);
        let time = Scoring {
            match_kind: MatchKind::Time,
            self_destruct_penalty: -1,
        };
        assert_eq!(ranks(&t, time)[0], 0);
        assert_eq!(ranks(&t, time)[2], 0);
    }

    #[test]
    fn a_stock_team_keeps_its_living_members_stocks() {
        // Team 0: P1 out at frame 600, P3 with 2 stocks; team 1: 1 stock each.
        let records = [
            PlayerRecord { slot: 0, team: Some(0), eliminated_at: Some(600), ..Default::default() },
            PlayerRecord { slot: 1, team: Some(1), stocks: 1, ..Default::default() },
            PlayerRecord { slot: 2, team: Some(0), stocks: 2, ..Default::default() },
            PlayerRecord { slot: 3, team: Some(1), stocks: 1, ..Default::default() },
        ];
        let t = tallies(&records, &KoCounts::default(), 3600);
        assert!(t[0].stock_score() < 0);
        let stock = Scoring {
            match_kind: MatchKind::Stock,
            self_destruct_penalty: -1,
        };
        // Both teams score 2: a tie for the top.
        assert_eq!(ranks(&t, stock)[..4], [0, 0, 0, 0]);
    }
}
