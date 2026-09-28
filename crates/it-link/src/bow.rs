//! Link's and Young Link's bow (It_Kind_Link_Bow / It_Kind_CLink_Bow),
//! itlinkbow.c (802AF1A4..802AF918).
//!
//! It hangs from the archer's right thumb and plays the row of the bow
//! special he is in (ftLk_SpecialN_GetIndex): a switch between the grounded
//! and aerial row keeps its frame. It goes once the release has played 24
//! frames, once the archer has left the special or let go of it.
use melee_it::{desc::ItemAssets, state_change::*, *};
use melee_types::ItemKind;

/// it_803F6E98's anim_id column: the six draw rows, then none.
pub const ARTICLE_STATES: [i32; 7] = [0, 1, 2, 3, 4, 5, -1];
/// No special attributes.
pub const SPECIAL_ATTRIBUTES: u32 = 0;
/// ftData.x48_items index.
pub const ARTICLE_INDEX: u32 = 4;

/// The stage with no row (ftLk_SpecialNIndex_None).
const NO_STAGE: u16 = 6;
/// The release rows' last frame.
const RELEASE_FRAMES: f32 = 24.0;
/// The grounded and aerial releases (states 2 and 5).
const RELEASES: [u16; 2] = [2, 5];
/// The x2071_b6 bit of Fighter.x2070 (the archer's motion flags).
const FLAG_2071_B6: u32 = 0x0002_0000;

fn state(item: &mut ItemCore) -> &mut BowState {
    match &mut item.scratch {
        ItemScratch::Bow(state) => state,
        _ => unreachable!("a bow without its state"),
    }
}

fn anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    bow_animation(item, ctx)
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
/// itLinkbow_UnkMotion6_Anim / _Coll: the stage without a row ends it.
fn gone(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    true
}
fn gone_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    true
}

const fn row(animation_id: i32) -> ItemStateRow {
    ItemStateRow {
        animation_id,
        animation: anim,
        physics: no_physics,
        collision: no_collision,
    }
}

static STATES: [ItemStateRow; 7] = [
    row(0),
    row(1),
    row(2),
    row(3),
    row(4),
    row(5),
    ItemStateRow {
        animation_id: -1,
        animation: gone,
        physics: no_physics,
        collision: gone_collision,
    },
];

/// The bow of Link (`YOUNG = false`) or Young Link.
pub struct Bow<const YOUNG: bool>;

impl<const YOUNG: bool> ItemLogic for Bow<YOUNG> {
    const KIND: ItemKind = if YOUNG {
        ItemKind::CLinkBow
    } else {
        ItemKind::LinkBow
    };
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802AF1A4 (802AF1A4) once Item_80268B18 returns: command
    /// variables clear, the archer and his scale.
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        item.scratch = ItemScratch::Bow(BowState {
            // ftLib_800869D4: the archer's model scale (1 in a versus match).
            scale: 1.0,
            archer: spawn.owner,
        });
    }
    /// itLinkBow_Logic100_PickedUp (802AF...): into the hand the archer's
    /// pointer is not yet set, so the stage reads as none: the aerial
    /// draw's row, one frame played.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        let archer = state(item).archer;
        if archer.is_none() || item.owner != archer {
            return;
        }
        let stage = context.owner.and_then(|o| o.article_stage);
        let row = match stage {
            Some(0) => 0,
            Some(3) | None => 3,
            Some(_) => return,
        };
        item.change_motion_with(row, ARTICLE_STATES[row as usize], ANIM_UPDATE, context.assets);
        item.advance_animation(context.assets);
    }
    /// itLinkBow_Logic100_Destroyed: the archer lets go (UnsetFv14).
    fn destroyed(_item: &mut ItemCore) {}
    /// it_802AF304 from the archer (ftLk_SpecialN_ProcessFv14).
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => item.destroyed = true,
            _ => unimplemented!("bow item control {control:?}"),
        }
    }
    /// itLinkBow_Logic100_EvtUnk: it_8026B894.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
}

/// itLinkbow_UnkMotion5_Anim (802AF64C).
fn bow_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if RELEASES.contains(&item.motion) {
        let frame = item.animation_frame;
        if frame == 0.0 || frame >= RELEASE_FRAMES {
            return true;
        }
    } else {
        follow_archer(item, ctx);
    }
    let archer = state(item).archer;
    if archer.is_none() || item.owner != archer {
        return true;
    }
    // ftLk_SpecialN_IsActiveAnd2071b6: the archer left the special (or a
    // row of it carries x2071_b6).
    let Some(owner) = ctx.owner else {
        return false;
    };
    let first = 344u16;
    let in_special = (first..first + 6).contains(&owner.motion);
    let flags = owner.motion_flags.unwrap_or(0);
    !in_special || flags & FLAG_2071_B6 != 0
}

/// it_802AF32C (802AF32C): the row of the archer's stage; switching
/// between a grounded and an aerial row keeps the frame (Item_80268DD4).
fn follow_archer(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    const SWAPPED: [u16; 7] = [3, 4, 5, 0, 1, 2, 6];
    let archer = state(item).archer;
    if archer.is_none() || item.owner != archer {
        return;
    }
    let stage = ctx
        .owner
        .and_then(|o| o.article_stage)
        .map_or(NO_STAGE, u16::from);
    if item.motion == stage {
        return;
    }
    let keep = (item.motion == SWAPPED[stage as usize]).then_some(item.animation_frame);
    item.change_motion_with(stage, ARTICLE_STATES[stage as usize], ANIM_UPDATE, ctx.assets);
    item.advance_animation(ctx.assets);
    if let Some(frame) = keep {
        // Item_80268DD4: HSD_JObjReqAnimAll(frame), then one AnimAll that
        // plays it without advancing.
        item.animation_frame = frame;
    }
}
