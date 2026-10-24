//! Samus's code that Kirby's copy ability runs too: the Charge Shot
//! (ftsamusspecialn.c and ftkirbyspecialsamus.c, the same states over two
//! users' rows) and the motion helpers it needs. `ft-samus` and `ft-kirby`
//! both depend on this crate, never on each other.
pub mod charge_shot;
pub mod motion;
