//! Generated from `src/melee/ft/forward.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// Whether a fighter or item is standing on ground or airborne (`GroundOrAir`).
    ///
    /// Source: `src/melee/ft/forward.h`.
    pub enum GroundOrAir: i32 {
        /// `GA_Ground`
        Ground = 0,
        /// `GA_Air`
        Air = 1,
    }
}
