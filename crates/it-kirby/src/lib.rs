//! Kirby's articles (ftData.x48_items of PlKb.dat, registered by
//! ftKb_Init_OnLoad): Final Cutter's beam [0] and the hammer [1]. The two
//! after them (the star of a lost ability, an inhaled item) are not ported.
pub mod cutter_beam;
pub mod hammer;

pub use cutter_beam::CutterBeam;
pub use hammer::Hammer;
