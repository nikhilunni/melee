//! Samus's articles (ftData.x48_items of PlSs.dat, registered by
//! ftSs_Init_OnLoad): the missile.
pub mod charge;
pub mod missile;

pub use charge::SamusCharge;
pub use missile::SamusMissile;
