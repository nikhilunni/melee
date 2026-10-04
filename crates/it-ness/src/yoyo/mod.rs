//! Ness's yo-yo, itnessyoyo.c (802BE598..802C016C): the up and down
//! smashes' article.
//!
//! The item is the yo-yo's handle in Ness's right hand (Item_8026AB54 on
//! FtPart_R2ndNa). Its string of ItemLinks ([`string`]) and the point the
//! smashes' hitbox follows (u.ns.yoyo_hitbox_pos) are read and written from
//! both sides, so they live with Ness: the item keeps its motion state and
//! the scene runs Ness's ARTICLE_PHYSICS hook as its phys_cb
//! (`OWNER_PHYSICS`). The item has no hitbox of its own: the smashes' hit is
//! Ness's hitbox 0, placed on the yo-yo by his accessory.
pub mod string;

use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemControl, ItemCore, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct NessYoyo;

/// ftData.x48_items index (ftNs_Init_OnLoad registers it eleventh).
pub const ARTICLE_INDEX: u32 = 10;
/// it_803F7558's anim_id column: no state plays an article animation.
pub const ARTICLE_STATES: [i32; 4] = [-1; 4];

/// The yo-yo's motion states (Item_80268E5C's state argument).
pub mod motion {
    /// In the hand (itNessYoyo_Logic59_PickedUp): it_802BF900 poses the
    /// first string link on the hand.
    pub const HELD: u16 = 0;
    /// Out on its string at the smash's hitbox point (it_802C0010):
    /// it_802BF28C.
    pub const TETHERED: u16 = 1;
    /// Thrown for the charge (it_802BFE5C): it_802BF4A0 swings it.
    pub const SWINGING: u16 = 2;
    /// Reeled in (it_802BFEC4): it_802BF800.
    pub const REELING: u16 = 3;
}

/// ftNs_MS_AttackHi4 (342) .. ftNs_MS_AttackLw4Release (347): the motions
/// the yo-yo outlives (itNessyoyo_UnkMotion3_Anim, 0x156..0x15B).
const OWNER_YOYO_MOTIONS: core::ops::RangeInclusive<u16> = 342..=347;

const STATE: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation: owner_in_smash,
    physics: owners_physics,
    collision: no_collision,
};
static STATES: [ItemStateRow; 4] = [STATE; 4];

impl ItemLogic for NessYoyo {
    const KIND: ItemKind = ItemKind::NessYoyo;
    const STATES: &'static [ItemStateRow] = &STATES;
    const OWNER_PHYSICS: bool = true;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itNessYoyo_Logic59_PickedUp (802BFE34): Item_8026AB54 puts it in
    /// state 0.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(motion::HELD, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // Item_80268E5C(gobj, state, ITEM_ANIM_UPDATE) from Ness's side
            // (it_802C0010, it_802BFE5C, it_802BFEC4).
            ItemControl::Motion(state) => {
                item.change_motion_with(
                    state,
                    ARTICLE_STATES[usize::from(state)],
                    ANIM_UPDATE,
                    assets,
                );
            }
            // it_802BE958: Item_8026A8EC, Ness's side already undone.
            ItemControl::Remove => item.destroyed = true,
            // it_802BE598 / it_802BE5B8 -> it_8026B724 / it_8026B73C.
            ItemControl::OwnerHitlag(frozen) => item.frozen = frozen,
            _ => unreachable!("{control:?} sent to Ness's yo-yo"),
        }
    }
}

/// itNessyoyo_UnkMotion3_Anim (802BEE88), every state's: the spin (the
/// model's x rotation by u.ns.x223C, drawn only) and the end once the owner
/// has left the yo-yo smashes or is gone (it_802BE958_inline; the owner's
/// side is its ARTICLE_DESTROYED hook).
fn owner_in_smash(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let ended = ctx
        .owner
        .is_none_or(|owner| !OWNER_YOYO_MOTIONS.contains(&owner.motion));
    // it_802BE958_inline: ip->owner = NULL, so Item_8026A8EC plays the
    // destroy effect (destroy_type 0) of an item still in the hand.
    item.owner_released |= ended;
    ended
}

/// itNessyoyo_UnkMotion0..3_Phys run as Ness's ARTICLE_PHYSICS hook.
fn owners_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
