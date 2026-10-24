//! The Ice Climbers' code that Kirby's copy ability runs too: the Ice Shot
//! (ftpopospecialn.c and ftkirbyspecialiceclimber.c, the same states over
//! two users' rows), with the motion helpers the climbers' specials share.
//! `ft-iceclimbers` and `ft-kirby` both depend on this crate, never on each
//! other.
pub mod common;
pub mod ice_shot;
