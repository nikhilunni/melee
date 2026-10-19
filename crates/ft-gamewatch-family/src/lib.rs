//! Mr. Game & Watch's code that Kirby's copy ability runs too: Chef
//! (ftgamewatchspecialn.c and ftkirbyspecialgamewatch.c, the same states
//! over two users' rows) and the motion helpers it needs. `ft-gamewatch`
//! and `ft-kirby` both depend on this crate, never on each other.
pub mod chef;
pub mod motion;
