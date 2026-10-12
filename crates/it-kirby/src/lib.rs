//! Kirby's articles (ftData.x48_items of PlKb.dat, registered by
//! ftKb_Init_OnLoad): Final Cutter's beam [0], the hammer [1] and the star of
//! a lost ability [2]. The last (an inhaled item, [3]) is not ported.
pub mod ability_star;
pub mod cutter_beam;
pub mod hammer;

pub use ability_star::AbilityStar;
pub use cutter_beam::CutterBeam;
pub use hammer::Hammer;
