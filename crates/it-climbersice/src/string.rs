//! The Belay's rope (It_Kind_IceClimber_GumStrings), itclimbersstring.c
//! (802C2750..802C3AA4).
//!
//! The item is the rope's handle in Popo's left hand (it_802C27D4 at
//! L4thNb, Item_8026AB54): it holds only its motion state. Its links
//! (ItemLinks, it_802C248C) and their steps belong to Popo
//! (ft-iceclimbers' `special_hi::rope`); the scene runs his
//! ARTICLE_ACCESSORY hook as this kind's on_accessory (`OWNER_ACCESSORY`).
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemControl, ItemCore, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct ClimbersString;

/// ftData.x48_items index (ftPp_Init_OnLoad registers it third).
pub const ARTICLE_INDEX: u32 = 2;
/// it_803F76B8's anim_id column: no state plays an article animation.
pub const ARTICLE_STATES: [i32; 4] = [-1; 4];
/// itClimbersStringAttributes x0..x20 (the two joints after are the
/// links' models).
pub const SPECIAL_ATTRIBUTES: u32 = 9;

/// ftPp_MS_SpecialHiStart_0 (347) .. ftPp_MS_SpecialAirHiThrow_1 (356):
/// Popo's Belay.
const OWNER_BELAY_MOTIONS: core::ops::RangeInclusive<u16> = 0x15B..=0x164;

const STATE: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation: owner_in_belay,
    physics: no_physics,
    collision: no_collision,
};
static STATES: [ItemStateRow; 4] = [STATE; 4];

impl ItemLogic for ClimbersString {
    const KIND: ItemKind = ItemKind::IceClimberGumStrings;
    const STATES: &'static [ItemStateRow] = &STATES;
    const OWNER_ACCESSORY: bool = true;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_2725_Logic70_PickedUp (802C37BC): Item_8026AB54 puts it in state
    /// 0 (it_8026BB44 shows its model).
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // Item_80268E5C(gobj, state, ...) from Popo's Belay
            // (it_802C3950, it_802C3810, it_802C3864) or his reel-in.
            ItemControl::Motion(state) => {
                item.change_motion_with(
                    state,
                    ARTICLE_STATES[usize::from(state)],
                    ANIM_UPDATE,
                    assets,
                );
            }
            // it_802C2750: Item_8026A8EC, Popo's side already undone.
            // Outside the item's procs no destroy_type is set, so
            // ItemSwitch plays nothing (recorded in iceclimbers_belay_fd_fox4).
            ItemControl::Remove => item.destroyed = true,
            _ => unreachable!("{control:?} sent to the Belay's rope"),
        }
    }
}

/// itClimbersstring_UnkMotion3_Anim (802C2BF0): the rope goes once its
/// owner leaves the Belay, or has none. Its cleanup
/// (itClimbersstring_Cleanup) lets go of the owner (ip->owner = NULL)
/// before Item_80269528 destroys it, so Item_8026A8EC's ItemSwitch plays
/// the destroy effect (item.c:1993: `!x13 || owner == NULL`). The port
/// keeps `owner` for the owner's ARTICLE_DESTROYED notice
/// (ftPp_SpecialS_8012114C) and drops the hold flag instead.
fn owner_in_belay(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let gone = ctx
        .owner
        .is_none_or(|owner| !OWNER_BELAY_MOTIONS.contains(&owner.motion));
    if gone {
        item.held = false;
    }
    gone
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
