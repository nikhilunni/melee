//! Peach's articles (ftData.x48_items of PlPe.dat): Peach Bomber's blast
//! (itpeachexplode.c), the turnip (itpeachturnip.c), the parasol
//! (itpeachparasol.c), Toad
//! (itpeachtoad.c) and Toad's spores (itpeachtoadspore.c). The parasol and
//! Toad hang on the owner's joint 109 for as long as the owner stays in the
//! motions that use them.
use melee_it::{desc::ItemAssets, ItemCore};

pub mod explode;
pub mod parasol;
pub mod spore;
pub mod toad;
pub mod turnip;

pub use explode::PeachExplode;
pub use parasol::PeachParasol;
pub use spore::PeachToadSpore;
pub use toad::PeachToad;
pub use turnip::PeachTurnip;

/// lbGetJObjCurrFrame: a non-looping joint animation stops at its last
/// frame, so the item's x5CC does too.
fn clamp_to_animation_end(item: &mut ItemCore, assets: &ItemAssets, article_state: i32) {
    if let Some(end) = usize::try_from(article_state)
        .ok()
        .and_then(|state| assets.animation_ends.get(state).copied().flatten())
    {
        if item.animation_frame > end {
            item.animation_frame = end;
        }
    }
}

fn no_physics(_item: &mut ItemCore, _ctx: &melee_it::ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut melee_it::ItemCollisionContext<'_>) -> bool {
    false
}
