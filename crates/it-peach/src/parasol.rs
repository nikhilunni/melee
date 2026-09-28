//! Peach's parasol, itpeachparasol.c (802BDA40..802BDE14).
use crate::{clamp_to_animation_end, no_collision, no_physics};
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemControl, ItemCore,
    ItemLogic, ItemStateRow,
};
use melee_types::ItemKind;

pub struct PeachParasol;

/// it_803F74F8's anim_id column.
pub const ARTICLE_STATES: [i32; 3] = [-1, 0, 1];
/// Motion 1 plays the opening animation, motion 2 holds it open.
pub const OPENING: u16 = 1;
pub const OPEN: u16 = 2;
/// it_804D5518: the frames of article states 0 and 1 (it_802BDA40).
pub const ARTICLE_FRAMES: [i32; 2] = [15, 16];
/// Item_8026AE84(item, 0xF7, 0x7F, 0x40) in it_802BDD40.
const OPENING_SOUND: u32 = 0xF7;

/// ftPeach_MotionState / ftCommon_MotionState ids ftPe_SpecialHi_NotActive
/// (8011D680) accepts.
mod owner_motion {
    pub const SPECIAL_HI_START: u16 = 361;
    pub const SPECIAL_AIR_HI_END: u16 = 364;
    pub const PEACH_PARASOL_OPEN: u16 = 369;
    pub const PEACH_PARASOL_FALL: u16 = 370;
    pub const ITEM_PARASOL_OPEN: u16 = 144;
    pub const ITEM_PARASOL_FALL_SPECIAL: u16 = 146;
    pub const FALL_SPECIAL: u16 = 35;
    pub const FALL_SPECIAL_B: u16 = 37;
    pub const LANDING_FALL_SPECIAL: u16 = 43;
}

static STATES: [ItemStateRow; 3] = [row(0), row(1), row(2)];
const fn row(motion: usize) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion],
        animation,
        physics: no_physics,
        collision: no_collision,
    }
}

impl ItemLogic for PeachParasol {
    const KIND: ItemKind = ItemKind::PeachParasol;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// Item_8026AB54 takes it into the owner's hand at once; nothing else
    /// may pick it up while it hangs there.
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.held = true;
    }
    /// itPeachParasol_Logic60_PickedUp (802BDC70): open at once, the
    /// animation sixteen frames in, then the ordinary rate.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        set_open(item, 16.0, ctx.assets);
        item.advance_animation(ctx.assets);
        clamp_to_animation_end(item, ctx.assets, ARTICLE_STATES[OPEN as usize]);
        item.animation_rate = 1.0;
    }
    /// it_3F2F.c: the parasol's dropped callback is NULL.
    fn dropped(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) {}
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::Remove => item.destroyed = true,
            ItemControl::ParasolOpening(rate) => set_opening(item, rate, assets),
            ItemControl::ParasolOpen(rate) => set_open(item, rate, assets),
            ItemControl::OwnerHitlag(_) => {
                unimplemented!("it_802BDBF8 / it_802BDC18: parasol owner hitlag")
            }
            _ => unreachable!("{control:?} sent to Peach's parasol"),
        }
    }
}

/// it_802BDD40 (802BDD40): the opening animation at `rate`.
pub fn set_opening(item: &mut ItemCore, rate: f32, assets: &ItemAssets) {
    item.sound_requests.push(OPENING_SOUND);
    item.animation_rate = rate;
    item.change_motion_with(OPENING, ARTICLE_STATES[OPENING as usize], ANIM_UPDATE, assets);
}

/// it_802BDDB4 (802BDDB4): the open pose at `rate`.
pub fn set_open(item: &mut ItemCore, rate: f32, assets: &ItemAssets) {
    item.animation_rate = rate;
    item.change_motion_with(OPEN, ARTICLE_STATES[OPEN as usize], ANIM_UPDATE, assets);
}

/// it_802BDC38 (802BDC38): the parasol is fully open.
fn fully_open(item: &ItemCore) -> bool {
    item.motion == OPEN && item.animation_frame >= 16.0
}

/// ftPe_SpecialHi_NotActive (8011D680) with notUsingParasol inlined.
fn owner_done(motion: u16, item: &ItemCore) -> bool {
    use owner_motion::*;
    !((SPECIAL_HI_START..=SPECIAL_AIR_HI_END).contains(&motion)
        || (PEACH_PARASOL_OPEN..=PEACH_PARASOL_FALL).contains(&motion)
        || (ITEM_PARASOL_OPEN..=ITEM_PARASOL_FALL_SPECIAL).contains(&motion)
        || (FALL_SPECIAL..=FALL_SPECIAL_B).contains(&motion)
        || (motion == LANDING_FALL_SPECIAL && !fully_open(item)))
}

/// itPeachparasol_UnkMotion2_Anim (802BDCC0), every row: the parasol goes
/// once its owner has left up special, the parasol states, special fall
/// and (unless fully open) special landing. The owner's references are
/// released through itPeachParasol_Logic60_Destroyed by the scene.
fn animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    clamp_to_animation_end(item, ctx.assets, ARTICLE_STATES[usize::from(item.motion)]);
    match ctx.owner {
        Some(owner) => owner_done(owner.motion, item),
        None => true,
    }
}
