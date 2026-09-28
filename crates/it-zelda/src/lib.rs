//! Zelda's articles (ftData.x48_items of PlZd.dat, registered by
//! ftZd_Init_OnLoad): Din's Fire [0] and its explosion [1].
pub mod din_fire;
pub mod explode;

pub use din_fire::DinFire;
pub use explode::DinFireExplode;
