c_enum! {
    /// A Versus match's rule set (StartMeleeRules match_kind; mn/forward.h
    /// `Mode`, which gm_80167BC8 copies from the VS rules' mode).
    pub enum MatchKind: i32 {
        /// KOs score; the timer ends the match.
        Time = 0,
        /// Each player starts with stocks; the last with any wins.
        Stock = 1,
        /// Coins score; the timer ends the match.
        Coin = 2,
        /// Bonus points score; the timer ends the match.
        Bonus = 3,
    }
}
