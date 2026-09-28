//! Toad, itpeachtoad.c (802BDE18..802BE210).
use crate::{clamp_to_animation_end, no_collision, no_physics};
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemControl, ItemCore,
    ItemLogic, ItemStateRow,
};
use melee_types::ItemKind;

pub struct PeachToad;

/// it_803F7528's anim_id column.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
/// Motion 0 waits in front of Peach; motion 1 releases the spores.
pub const WAITING: u16 = 0;
pub const COUNTER: u16 = 1;
/// it_802BE100: lb_8000BA0C(jobj, 10) then HSD_JObjAnimAll.
const COUNTER_START_FRAME: f32 = 10.0;

/// ftPe_SpecialN_IsActive (8011E3A4): ftPe_MS_SpecialN..SpecialAirNHit.
const OWNER_MOTIONS: std::ops::RangeInclusive<u16> = 365..=368;

static STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: waiting,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: counter,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for PeachToad {
    const KIND: ItemKind = ItemKind::PeachToad;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.held = true;
    }
    /// itPeachToad_Logic91_PickedUp (802BE02C). it_8026BB44 hides the model
    /// until itPeachtoad_UnkMotion0_Anim shows it at frame 7; it_80274574
    /// only rescales it.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(WAITING, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
        item.advance_animation(ctx.assets);
        item.hidden = true;
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::Remove => item.destroyed = true,
            ItemControl::Counter => {
                // it_802BE100 (802BE100): x5CC follows at the next advance.
                item.change_motion_with(COUNTER, ARTICLE_STATES[1], ANIM_UPDATE, assets);
                item.animation_frame = COUNTER_START_FRAME;
            }
            ItemControl::OwnerHitlag(_) => {
                unimplemented!("it_802BDFA0 / it_802BDFC0: Toad owner hitlag")
            }
            _ => unreachable!("{control:?} sent to Toad"),
        }
    }
}

/// The owner check both rows share (itpeachtoad_inline_2): Toad goes once
/// its owner leaves the neutral special.
fn owner_done(ctx: &ItemAnimationContext<'_>) -> bool {
    ctx.owner
        .is_none_or(|owner| !OWNER_MOTIONS.contains(&owner.motion))
}

/// itPeachtoad_UnkMotion0_Anim (802BE07C).
fn waiting(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    clamp_to_animation_end(item, ctx.assets, ARTICLE_STATES[0]);
    if item.animation_frame == 7.0 {
        item.hidden = false;
    }
    if item.animation_frame >= 53.0 {
        item.hidden = true;
    }
    owner_done(ctx)
}

/// itPeachtoad_UnkMotion1_Anim (802BE184).
fn counter(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    clamp_to_animation_end(item, ctx.assets, ARTICLE_STATES[1]);
    if item.animation_frame >= 60.0 {
        item.hidden = true;
    }
    owner_done(ctx)
}
