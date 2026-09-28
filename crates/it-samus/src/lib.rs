//! Samus's articles (ftData.x48_items of PlSs.dat, registered by
//! ftSs_Init_OnLoad): the bomb, the charge shot, the missile and the
//! grapple beam.
pub mod bomb;
pub mod charge;
pub mod grapple;
pub mod missile;

pub use bomb::SamusBomb;
pub use charge::SamusCharge;
pub use grapple::SamusGrapple;
pub use missile::SamusMissile;
