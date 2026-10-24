//! Zelda's code that Kirby's copy ability runs too: Nayru's Love
//! (ftzeldaspecialn.c and ftkirbyspecialzelda.c, the same states over two
//! users' rows), with the motion helpers Zelda's specials share. `ft-zelda`
//! and `ft-kirby` both depend on this crate, never on each other.
pub mod common;
pub mod nayrus_love;
