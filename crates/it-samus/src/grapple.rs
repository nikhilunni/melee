//! Samus's grapple beam (It_Kind_Samus_GBeam), the article of
//! itsamusgrapple.c. Its owner drives the rope from its own accessory procs
//! (ft-samus's grapple module); the article only exists, hangs where it was
//! made and takes the state its owner gives it (Item_80268E5C). Its nine
//! states have no model animation and no animation or collision procs;
//! their physics only chooses the owner's step (xDD4 unk_10).
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct SamusGrapple;

/// it_803F73A8's anim_id column: no article state.
pub const ARTICLE_STATES: [i32; 9] = [-1; 9];
/// itSamusGrappleAttributes: 0xB0 bytes, read by the owner.
pub const SPECIAL_ATTRIBUTES: u32 = 0;
/// ftData.x48_items index (ftSs_Init_OnLoad registers it fourth).
pub const ARTICLE_INDEX: u32 = 3;

const STATE: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation: no_animation,
    physics: no_physics,
    collision: no_collision,
};
static STATES: [ItemStateRow; 9] = [STATE; 9];

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_3F2F.c's Logic53 row.
impl ItemLogic for SamusGrapple {
    const KIND: ItemKind = ItemKind::SamusGBeam;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itSamusGrapple_Logic53_PickedUp (802BA980): the hanging state
    /// (it_802A2428's scale is the model's).
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(0, -1, ANIM_UPDATE, ctx.assets);
    }
    /// The owner's Item_80268E5C (ANIM_UPDATE) and it_802B7B84's
    /// Item_8026A8EC.
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::Motion(state) => item.change_motion_with(state, -1, ANIM_UPDATE, assets),
            ItemControl::Remove => item.destroyed = true,
            _ => unimplemented!("grapple beam item control {control:?}"),
        }
    }
    /// itSamusGrapple_Logic53_EvtUnk: it_8026B894, and the beam's owner
    /// (xDD4 x8) if it was the one removed.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
}
