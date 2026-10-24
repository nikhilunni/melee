//! Bowser's code that Kirby's copy ability runs too: the Fire Breath
//! (ftkoopaspecialn.c and ftkirbyspecialkoopa.c, the same states over two
//! users' rows), with the motion helpers Bowser's specials share.
//! `ft-koopa` and `ft-kirby` both depend on this crate, never on each other.
pub mod common;
pub mod fire_breath;
