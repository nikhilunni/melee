//! Link's and Young Link's hookshot (It_Kind_Link_HShot /
//! It_Kind_CLink_HShot), itlinkhookshot.c (802A2418..802A7D40).
//!
//! The grabs and the aerial hookshot create it in the thrower's hand
//! (it_802A2BA4) with a chain of links ([`chain`]). The article itself only
//! hangs from the hand and changes state; each state's physics proc
//! installs the step its thrower's accessory proc then runs every frame
//! (`linkhookshot.x10`), reported here as [`OwnerRequest::ArticleStep`].
//! The chain and its steps live with the thrower, whose procs retail runs
//! them from. The claw's spin and the chain models are drawing only.
pub mod chain;

use melee_it::{desc::ItemAssets, state_change::*, *};
use melee_types::ItemKind;

/// it_803F6998's anim_id column: none of the nine states animates.
pub const ARTICLE_STATES: [i32; 9] = [-1; 9];
/// itLinkHookshotAttributes x0..x50 (the three joints after are the
/// chain's models).
pub const SPECIAL_ATTRIBUTES: u32 = 21;
/// ftData.x48_items index (ftLk_Init_OnLoad registers it third).
pub const ARTICLE_INDEX: u32 = 2;

/// The article's states (it_803F6998), each naming the thrower's step.
pub mod motion {
    /// In the hand before the throw (fn_802A2E4C): the claw at the thumb.
    pub const HELD: u16 = 0;
    /// Flying out (it_802A2EE4).
    pub const EXTENDING: u16 = 1;
    /// Falling back from a wall (fn_802A3110).
    pub const FALLING_BACK: u16 = 2;
    /// Fully out, hanging (it_802A3254).
    pub const HANGING: u16 = 3;
    /// Reeling in (fn_802A33A0).
    pub const REELING: u16 = 4;
    /// Reeling in a caught fighter (it_802A3500).
    pub const REELING_CATCH: u16 = 5;
    /// Stuck in a wall, and the thrower's wall hang (it_802A3630,
    /// it_802A3828, it_802A39FC).
    pub const WALL: u16 = 6;
    pub const WALL_CLIMB: u16 = 7;
    pub const WALL_HANG: u16 = 8;
}

fn step_physics(item: &mut ItemCore) {
    // itLinkhookshot_UnkMotionN_Phys: linkhookshot.x10 = the state's step.
    if let Some(owner) = item.owner {
        item.owner_request = Some((owner, OwnerRequest::ArticleStep(item.motion as u8)));
    }
}
fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
/// itLinkhookshot_UnkMotion8_Anim (802A2D88): the claw model's spin and
/// scale by how much chain is out; drawing only.
fn claw_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    step_physics(item);
}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

const fn row(animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool) -> ItemStateRow {
    ItemStateRow {
        animation_id: -1,
        animation,
        physics,
        collision: no_collision,
    }
}

static STATES: [ItemStateRow; 9] = [
    row(no_animation),
    row(claw_animation),
    row(claw_animation),
    row(claw_animation),
    row(claw_animation),
    row(claw_animation),
    row(claw_animation),
    row(claw_animation),
    row(claw_animation),
];

/// The hookshot of Link (`YOUNG = false`) or Young Link.
pub struct Hookshot<const YOUNG: bool>;

/// it_3F2F.c's Link and Young Link hookshot logic rows (Link's adds
/// it_802A2418's spawned callback, which only clears x10).
impl<const YOUNG: bool> ItemLogic for Hookshot<YOUNG> {
    const KIND: ItemKind = if YOUNG {
        ItemKind::CLinkHShot
    } else {
        ItemKind::LinkHShot
    };
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itLinkHookshot_Logic20_PickedUp (802A7688): state 0 on the way into
    /// the hand (Item_8026AB54), at the holder's scale.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(motion::HELD, -1, ANIM_UPDATE, context.assets);
    }
    /// Item_80268E5C from the thrower's steps (it_802A76EC and kin).
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::Motion(state) => {
                item.change_motion_with(state, -1, ANIM_UPDATE, assets);
            }
            ItemControl::Remove => item.destroyed = true,
            _ => unimplemented!("hookshot item control {control:?}"),
        }
    }
    /// it_802A7D40 (802A7D40): it_8026B894, then the thrower is gone.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
}
