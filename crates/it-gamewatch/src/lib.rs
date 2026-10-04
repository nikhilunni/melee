//! Mr. Game & Watch's articles (ftData.x48_items of PlGw.dat, registered by
//! ftGw_Init_OnLoad in this order): the jab's Greenhouse sprayer, the down
//! tilt's Manhole, the forward smash's Fire torch, the neutral aerial's
//! Parachute, the back aerial's Turtle, the up aerial's Spitball Sparky
//! breath, Judgment's hammer and number, Oil Panic's oil, Chef's food and
//! Fire's rescue trampoline.
//!
//! All but the food and the trampoline hang on one of the owner's parts
//! (Item_AttachGameWatchArticle: Item_8026AB54, then it_8027CE64's costume
//! colour, which is drawing only) and live while the owner stays in the
//! motions that use them.
use melee_it::{ItemCollisionContext, ItemCore, ItemPhysicsContext};

pub mod attack;
pub mod attack_air;

pub use attack::{Fire, Greenhouse, Manhole};
pub use attack_air::{Breath, Parachute, Turtle};

/// ftGameWatch_MotionState ids the articles' owner checks read.
pub mod owner_motion {
    pub const ATTACK_11: u16 = 341;
    pub const ATTACK_100_START: u16 = 342;
    pub const ATTACK_100_END: u16 = 344;
    pub const ATTACK_LW3: u16 = 345;
    pub const ATTACK_S4: u16 = 346;
    pub const ATTACK_AIR_N: u16 = 347;
    pub const ATTACK_AIR_B: u16 = 348;
    pub const ATTACK_AIR_HI: u16 = 349;
    pub const LANDING_AIR_N: u16 = 350;
    pub const LANDING_AIR_B: u16 = 351;
    pub const LANDING_AIR_HI: u16 = 352;
}

/// ftData.x48_items indices (ftGw_Init_OnLoad's it_8026B3F8 calls).
pub mod article_index {
    pub const GREENHOUSE: u32 = 0;
    pub const MANHOLE: u32 = 1;
    pub const FIRE: u32 = 2;
    pub const PARACHUTE: u32 = 3;
    pub const TURTLE: u32 = 4;
    pub const BREATH: u32 = 5;
}

pub(crate) fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
pub(crate) fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// The control messages every hanging article takes: its owner's damage or
/// death destroys it (Item_8026A8EC, the owner's reset already done) and
/// its owner's hitlag freezes it (it_8026B724 / it_8026B73C: xDC8 x3).
pub(crate) fn common_control(item: &mut ItemCore, control: melee_it::ItemControl) -> bool {
    match control {
        melee_it::ItemControl::Remove => item.destroyed = true,
        melee_it::ItemControl::OwnerHitlag(frozen) => item.frozen = frozen,
        _ => return false,
    }
    true
}
