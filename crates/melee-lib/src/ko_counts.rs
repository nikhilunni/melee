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
    /// gm_80166378 sums over every other player (gm_1601.c:3050-3056; the
    /// team variant is not ported). Its own entry counts as a self-destruct
    /// instead.
    pub(crate) fn total(&self, player: u8) -> i32 {
        let row = &self.0[usize::from(player)];
        let total: u32 = (0..SLOTS)
            .filter(|&other| other != usize::from(player))
            .map(|other| row[other])
            .fold(0, u32::wrapping_add);
        total as i32
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
        assert_eq!(counts.total(1), 2);
        assert_eq!(counts.total(0), 1);
        assert_eq!(counts.total(3), 0);
    }
}
