//! Generated from `src/melee/pl/forward.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// Who controls a player slot (`Gm_PKind`).
    ///
    /// Source: `src/melee/pl/forward.h`.
    pub enum PlayerKind: i32 {
        /// `Gm_PKind_Human`
        Human = 0,
        /// `Gm_PKind_Cpu`
        Cpu = 1,
        /// `Gm_PKind_Demo`
        Demo = 2,
        /// `Gm_PKind_NA` — slot not in use
        Na = 3,
        /// `Gm_PKind_Boss`
        Boss = 4,
    }
}
