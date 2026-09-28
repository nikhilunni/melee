//! Mario's cape (It_Kind_Mario_Cape), itmariocape.c (802B2560..802B2890).
//! Mario's side special creates it in his right hand (it_802B2560); it
//! lives while he stays in the cape's motions. Its script raises two
//! command variables that sparkle along the cape's edges (efAlt 0x47E on
//! bone 6, 0x47D on bone 16).
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct MarioCape;

/// it_803F70F8's anim_id column.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
/// ftData.x48_items index (ftMr_Init_OnLoad registers it third).
pub const ARTICLE_INDEX: u32 = 2;
/// The cape's model bones the sparkles follow (xBBC_dynamicBoneTable).
pub const SPARKLE_BONES: [usize; 2] = [16, 6];

/// efAsync_Spawn(item, xBC0, 0, id, bone): the sparkles, hsd_8039EFAC(0, 1,
/// 0x3F0 / 0x3F1, bone) on efAlt's dispatch.
const TOP_SPARKLE: u16 = 0x47D;
const BOTTOM_SPARKLE: u16 = 0x47E;

/// ftMr_SpecialS_CheckItemCapeRemove: ftMr_MS_SpecialS..SpecialAirS.
const OWNER_MOTIONS: std::ops::RangeInclusive<u16> = 345..=346;

static STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation,
        physics: no_physics,
        collision: no_collision,
    },
];

/// it_3F2F.c's Logic41 row.
impl ItemLogic for MarioCape {
    const KIND: ItemKind = ItemKind::MarioCape;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// it_2725_Logic41_PickedUp (802B2700): the command variables cleared,
    /// the state (802B2734 passes the item to ftLib_800865CC, which reads
    /// Item +0xE0, a zeroed dynamic-bone word: state 0), one animation step
    /// (Item_802694CC); it_80274574 only rescales the model.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.command_variables[0] = 0;
        item.command_variables[1] = 0;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
        item.advance_animation(ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            // it_802B2674: destroyed with the owner's reset already done.
            ItemControl::Remove => item.destroyed = true,
            // it_802B26C0 / it_802B26E0 -> it_8026B724 / it_8026B73C: the
            // owner's hitlag freezes the cape (xDC8 x3; x7 is never set).
            ItemControl::OwnerHitlag(frozen) => item.frozen = frozen,
            _ => unreachable!("{control:?} sent to Mario's cape"),
        }
    }
}

/// itMariocape_UnkMotion1_Anim (802B2788): a raised command variable
/// sparkles its bone (variable 0 on bone 16, then variable 1 on bone 6);
/// the cape goes once its owner leaves the cape's motions.
fn animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[0] != 0 {
        item.command_variables[0] = 0;
        item.spawn_async(ItemEvent::BoneGenerator {
            id: TOP_SPARKLE,
            bone: SPARKLE_BONES[0],
        });
    }
    if item.command_variables[1] != 0 {
        item.command_variables[1] = 0;
        item.spawn_async(ItemEvent::BoneGenerator {
            id: BOTTOM_SPARKLE,
            bone: SPARKLE_BONES[1],
        });
    }
    ctx.owner
        .is_none_or(|owner| !OWNER_MOTIONS.contains(&owner.motion))
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
