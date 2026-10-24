//! Donkey Kong's code that Kirby's copy ability runs too: the Giant Punch
//! (ftdonkeyspecialn.c and ftkirbyspecialdonkey.c, the same states over two
//! users' rows) and the motion helpers it needs. `ft-donkey` and `ft-kirby`
//! both depend on this crate, never on each other.
pub mod giant_punch;
pub mod motion;
