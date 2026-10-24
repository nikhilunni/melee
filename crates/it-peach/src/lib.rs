//! Peach's articles (ftData.x48_items of PlPe.dat): Peach Bomber's blast
//! (itpeachexplode.c), the turnip (itpeachturnip.c), the parasol
//! (itpeachparasol.c), Toad
//! (itpeachtoad.c) and Toad's spores (itpeachtoadspore.c). The parasol and
//! Toad hang on the owner's joint 109 for as long as the owner stays in the
//! motions that use them. Kirby's copied Toad spawns Toad and its spores
//! under kinds of its own (It_Kind_Kirby_PeachToad, _PeachToadSpore), from
//! his copy archive PlKbCpPe.dat, with the same item code.
use melee_it::{desc::ItemAssets, ItemCore};
use melee_types::ItemKind;

pub mod explode;
pub mod parasol;
pub mod spore;
pub mod toad;
pub mod turnip;

pub use explode::PeachExplode;
pub use parasol::PeachParasol;
pub use spore::{KirbyPeachToadSpore, PeachToadSpore};
pub use toad::{KirbyPeachToad, PeachToad};
pub use turnip::PeachTurnip;

/// The kinds one owner's Toad takes, and the owner's Toad rows.
pub trait ToadOwner: 'static {
    const TOAD: ItemKind;
    const SPORE: ItemKind;
    /// ftPe_SpecialN_IsActive (8011E3A4: ftPe_MS_SpecialN..SpecialAirNHit)
    /// or ftKb_SpecialNPe_8010C4AC (ftKb_MS_PeSpecialLw..PeSpecialAirLwHit).
    const OWNER_MOTIONS: std::ops::RangeInclusive<u16>;
    /// The spore's logic table has Reflected, Clanked, Absorbed,
    /// ShieldBounced and HitShield (it_3F2F.c:1355-1372, Peach's); Kirby's
    /// (it_3F2F.c:1782-1798) has only DmgDealt and EvtUnk, so a clank or a
    /// shield leaves his spore flying.
    const SPORE_DEFENSE_EVENTS: bool;
}
/// Peach's own Toad.
pub struct Peach;
impl ToadOwner for Peach {
    const TOAD: ItemKind = ItemKind::PeachToad;
    const SPORE: ItemKind = ItemKind::PeachToadSpore;
    const OWNER_MOTIONS: std::ops::RangeInclusive<u16> = 365..=368;
    const SPORE_DEFENSE_EVENTS: bool = true;
}
/// Kirby's copied Toad.
pub struct Kirby;
impl ToadOwner for Kirby {
    const TOAD: ItemKind = ItemKind::KirbyPeachToad;
    const SPORE: ItemKind = ItemKind::KirbyPeachToadSpore;
    const OWNER_MOTIONS: std::ops::RangeInclusive<u16> = 449..=452;
    const SPORE_DEFENSE_EVENTS: bool = false;
}
/// ftKb_SpecialN_800F16D0 (ftkirby.c:3812-3816): PlKbCpPe.dat's
/// hat_dynamics[0] is Toad, [1] its spore.
pub const KIRBY_TOAD_SLOT: u32 = 0;
pub const KIRBY_SPORE_SLOT: u32 = 1;

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
