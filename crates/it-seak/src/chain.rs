//! Sheik's chain, itseakchain.c (802BAEEC..802BD01C).
//!
//! The item is the chain's handle in Sheik's hand: it holds only its
//! motion state. Its twenty links (ItemLink GObjs, it_802BAF2C) and their
//! physics belong to Sheik (ft-seak's `special_s::chain`), whose code reads
//! and writes them from both sides; the scene runs the owner's
//! ARTICLE_ACCESSORY hook as this kind's on_accessory (`OWNER_ACCESSORY`).
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemControl, ItemCore, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct SeakChain;

/// ftData.x48_items index (ftSk_Init_OnLoad registers it fourth).
pub const ARTICLE_INDEX: u32 = 3;
/// it_803F7438's anim_id column: no state plays an article animation.
pub const ARTICLE_STATES: [i32; 5] = [-1; 5];

/// The chain's motion states (Item_80268E5C's state argument).
pub mod motion {
    /// In Sheik's hand, before the throw (it_2725_Logic54_PickedUp:
    /// on_accessory fn_802BB428).
    pub const HELD: u16 = 0;
    /// Paying out (it_802BCFC4: fn_802BB44C).
    pub const EXTENDING: u16 = 1;
    /// The head struck a wall paying out (it_802BCF2C: fn_802BB574).
    pub const FALLING: u16 = 2;
    /// Fully out, swung by the stick (it_802BCED4: fn_802BB694).
    pub const SWINGING: u16 = 3;
    /// Reeled in (it_802BCF84: fn_802BB784).
    pub const RETRACTING: u16 = 4;
}

/// ftSk_MS_SpecialSStart (349) .. ftSk_MS_SpecialAirSEnd (354).
const OWNER_CHAIN_MOTIONS: core::ops::RangeInclusive<u16> = 349..=354;

const STATE: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation: owner_in_chain,
    physics: no_physics,
    collision: no_collision,
};
static STATES: [ItemStateRow; 5] = [STATE; 5];

impl ItemLogic for SeakChain {
    const KIND: ItemKind = ItemKind::SeakChain;
    const STATES: &'static [ItemStateRow] = &STATES;
    const OWNER_ACCESSORY: bool = true;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itSeakChain_Spawn: it_80272A3C hides the handle's own model (the
    /// links draw the chain).
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &melee_it::desc::ItemCommonData,
        _spawn: &melee_it::SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.hidden = true;
    }
    /// it_2725_Logic54_PickedUp (802BCE7C): Item_8026AB54 puts it in state 0.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(motion::HELD, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // Item_80268E5C(gobj, state, ITEM_ANIM_UPDATE) from Sheik's side.
            ItemControl::Motion(state) => {
                item.change_motion_with(
                    state,
                    ARTICLE_STATES[usize::from(state)],
                    ANIM_UPDATE,
                    assets,
                );
            }
            // it_802BB20C: Item_8026A8EC, Sheik's side already undone.
            ItemControl::Remove => item.destroyed = true,
            // it_802BAEEC / it_802BAF0C -> it_8026B724 / it_8026B73C.
            ItemControl::OwnerHitlag(frozen) => item.frozen = frozen,
            _ => unreachable!("{control:?} sent to Sheik's chain"),
        }
    }
}

/// itSeakchain_UnkMotion4_Anim (802BB8B8): the chain goes once its owner
/// leaves the chain's motions (notInSpecialS), or has none.
fn owner_in_chain(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    ctx.owner
        .is_none_or(|owner| !OWNER_CHAIN_MOTIONS.contains(&owner.motion))
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
