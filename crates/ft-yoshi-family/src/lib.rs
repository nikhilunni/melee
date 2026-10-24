//! Yoshi's code that Kirby's copy ability runs too: Egg Lay
//! (ftyoshispecialn.c and ftkirbyspecialyoshi.c, the same states over two
//! users' rows). `ft-yoshi` and `ft-kirby` both depend on this crate, never
//! on each other. The captured fighter's states are common code
//! (`melee_ft::fighter::capture_yoshi`).
pub mod egg_lay;
