//! The order the HUD's s_link 17 procs run in (p_link 15): each player's
//! damage HUD (ifStatus_802F5B48 / 802F4EDC) and stock HUD (fn_802F9410).
//! ifStatus_802F665C makes them per player in port order; a revival's
//! ifStatus_802F6508 makes the player's stock HUD again (ifStock_802F98E8:
//! HSD_GObjPLink_80390228, then GObj_Create(14, 15, 0)), which puts it at
//! the end of the list.

/// Six players' damage and stock HUDs.
const MOST: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HudOrder {
    /// (stock HUD, player) in proc order.
    entries: [(bool, u8); MOST],
    len: usize,
}

impl HudOrder {
    /// ifStatus_802F665C: player by player, the damage HUD then the stock HUD.
    pub(crate) fn new(players: usize) -> Self {
        assert!(2 * players <= MOST, "more than six HUD players");
        let mut entries = [(false, 0); MOST];
        for player in 0..players {
            entries[2 * player] = (false, player as u8);
            entries[2 * player + 1] = (true, player as u8);
        }
        Self {
            entries,
            len: 2 * players,
        }
    }

    /// What the `proc`-th interface proc runs: two entries at its place.
    pub(crate) fn share(&self, proc: usize) -> &[(bool, u8)] {
        let start = (2 * proc).min(self.len);
        &self.entries[start..(start + 2).min(self.len)]
    }

    /// ifStatus_802F6508 for a revived player: its stock HUD goes last.
    pub(crate) fn revived(&mut self, player: usize) {
        let Some(at) = self.entries[..self.len]
            .iter()
            .position(|&e| e == (true, player as u8))
        else {
            return;
        };
        let entry = self.entries[at];
        self.entries.copy_within(at + 1..self.len, at);
        self.entries[self.len - 1] = entry;
    }
}

#[cfg(test)]
mod tests {
    use super::HudOrder;

    #[test]
    fn a_revived_players_stock_hud_runs_last() {
        let mut order = HudOrder::new(2);
        assert_eq!(order.share(0), &[(false, 0), (true, 0)]);
        assert_eq!(order.share(1), &[(false, 1), (true, 1)]);
        order.revived(0);
        assert_eq!(order.share(0), &[(false, 0), (false, 1)]);
        assert_eq!(order.share(1), &[(true, 1), (true, 0)]);
        order.revived(1);
        assert_eq!(order.share(1), &[(true, 0), (true, 1)]);
    }
}
