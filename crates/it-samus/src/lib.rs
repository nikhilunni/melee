//! Samus's articles (ftData.x48_items of PlSs.dat, registered by
//! ftSs_Init_OnLoad): the bomb, the charge shot and the missile.
pub mod bomb;
pub mod charge;
pub mod missile;

pub use bomb::SamusBomb;
pub use charge::SamusCharge;
pub use missile::SamusMissile;
