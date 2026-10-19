//! The pan of Kirby's copied Chef (It_Kind_Kirby_GameWatchChefPan),
//! itkirbygamewatchchefpan.c (802C74D8..802C775C).
//!
//! Mr. Game & Watch's pan is part of his model; Kirby's is an article that
//! the copied Chef's first accessory (fn_8010CFB0) hangs on his left thumb
//! (it_802C74D8: Item_AttachGameWatchArticle). It has one motion state with
//! no animation and goes once Kirby leaves the two Chef rows, when he is hit
//! or dies, or when he loses the ability. Its owner freezes it in his
//! hitlag (fn_8010D100 / ftKb_SpecialNGw_8010D130).
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemControl, ItemCore, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct ChefPan;

/// it_803F7958's anim_id column: one motion state, no article state.
pub const ARTICLE_STATES: [i32; 1] = [-1];

/// ftKb_MS_GwSpecialN and ftKb_MS_GwSpecialAirN: the owner rows the pan
/// lives in (ftKb_SpecialNGw_8010D160).
pub const OWNER_ROWS: std::ops::RangeInclusive<u16> = 510..=511;

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics: no_physics,
    collision: no_collision,
}];

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itKirbygamewatchchefpan_UnkMotion0_Anim (802C76E4): the pan goes once
/// its owner is gone or outside the Chef rows (ftKb_SpecialNGw_8010D160);
/// its Destroyed callback then lets go of the owner.
fn animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    ctx.owner.is_none_or(|o| !OWNER_ROWS.contains(&o.motion))
}

impl ItemLogic for ChefPan {
    const KIND: ItemKind = ItemKind::KirbyGameWatchChefPan;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// Item_8026AB54 takes it onto Kirby's thumb at once.
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// itKirbyGameWatchChefPan_Logic113_PickedUp (802C7698): with an owner,
    /// motion 0 (Item_80268E5C, ITEM_ANIM_UPDATE) and one animation step
    /// (Item_802694CC).
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        if item.owner.is_some() {
            item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
            item.advance_animation(ctx.assets);
        }
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            // it_802C760C: the Destroyed callback, then Item_8026A8EC.
            ItemControl::Remove => item.destroyed = true,
            // it_802C7658 / it_802C7678: it_8026B724 / it_8026B73C.
            ItemControl::OwnerHitlag(frozen) => item.frozen = frozen,
            _ => unreachable!("{control:?} sent to Kirby's Chef pan"),
        }
    }
}
