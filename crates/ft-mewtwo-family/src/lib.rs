//! Mewtwo's code that Kirby's copy ability runs too: the Shadow Ball
//! (ftmewtwospecialn.c and ftkirbyspecialmewtwo.c, the same states over two
//! users' rows) and the motion helpers it needs. `ft-mewtwo` and `ft-kirby`
//! both depend on this crate, never on each other.
pub mod motion;
pub mod shadow_ball;
