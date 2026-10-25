//! Each player's KOs (StaticPlayer.kos_by_player, +0x70) and the total the
//! match standings make of them.

/// Player slots (Player_CheckSlot: 0..6).
const SLOTS: usize = 6;

/// `counts[source][fallen]`: the stocks `fallen` lost while `source` was the
/// last to hit it. A player's own row entry counts its falls to its own hits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KoCounts([[u32; SLOTS]; SLOTS]);

impl KoCounts {
    /// Player_UpdateKOsBySlot (0x80034FA8) for a player's first fighter
    /// (its second argument clear, 0x80035018): one more KO, saturating
    /// (0x8003502C). The HUD calls that follow feed no compared key.
    pub(crate) fn record(&mut self, source: u8, fallen: u8) {
        let count = &mut self.0[usize::from(source)][usize::from(fallen)];
        *count = count.saturating_add(1);
    }

    /// gm_8016C75C (0x8016C75C): the standings' x20 for `player`, which
    /// gm_80166378 sums over every other player (gm_1601.c:3050-3056), or
    /// in a team battle over the other teams' players (gm_1601.c:3028-3040,
    /// `teams` by slot). Its own entry, and a teammate's, count as
    /// self-destructs instead.
    pub(crate) fn total(&self, player: u8, teams: &[Option<u8>; 4]) -> i32 {
        let row = &self.0[usize::from(player)];
        let own = teams[usize::from(player)];
        let total: u32 = (0..SLOTS)
            .filter(|&other| other != usize::from(player))
            .filter(|&other| own.is_none() || teams.get(other).copied().flatten() != own)
            .map(|other| row[other])
            .fold(0, u32::wrapping_add);
        total as i32
    }

    /// gm_80166378's other half (gm_1601.c:3034-3037, 3050-3054): the KOs
    /// `player` scored on itself, and in a team battle on its teammates,
    /// which count as its self-destructs and falls.
    pub(crate) fn own_side(&self, player: u8, teams: &[Option<u8>; 4]) -> u32 {
        let row = &self.0[usize::from(player)];
        let own = teams.get(usize::from(player)).copied().flatten();
        (0..SLOTS)
            .filter(|&other| {
                other == usize::from(player)
                    || (own.is_some() && teams.get(other).copied().flatten() == own)
            })
            .map(|other| row[other])
            .fold(0, u32::wrapping_add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_players_total_leaves_out_its_own_falls() {
        let mut counts = KoCounts::default();
        counts.record(1, 0);
        counts.record(1, 3);
        counts.record(1, 1);
        counts.record(0, 1);
        let none = [None; 4];
        assert_eq!(counts.total(1, &none), 2);
        assert_eq!(counts.total(0, &none), 1);
        assert_eq!(counts.total(3, &none), 0);
        // Teams 0/1 against 2/3: player 1's KO of its teammate 0 is a
        // self-destruct.
        let teams = [Some(0), Some(0), Some(1), Some(1)];
        assert_eq!(counts.total(1, &teams), 1);
    }
}
