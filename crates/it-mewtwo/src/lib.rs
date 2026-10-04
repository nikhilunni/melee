//! Mewtwo's articles (ftData.x48_items of PlMt.dat, registered by
//! ftMt_Init_OnLoad): Disable's projectile [0] and the Shadow Ball [1].
pub mod disable;
pub mod shadow_ball;

pub use disable::MewtwoDisable;
pub use shadow_ball::MewtwoShadowBall;
