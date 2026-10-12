//! Kirby's hammer (It_Kind_Kirby_Hammer), itkirbyhammer.c
//! (802ADC08..802ADDD0).
//!
//! The side special creates it in Kirby's right hand (it_802ADC54) and
//! destroys it (it_802ADC34) when the script puts it away, when the swing
//! lands or leaves the floor, or when Kirby is hit or dies. It has one
//! motion state with no callbacks: it does nothing but hang there with its
//! effect (efAlt 0x498 swung on the ground, 0x499 in the air). The swing's
//! hitboxes are Kirby's own.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, KirbyHammerState, SpawnItem,
};
use melee_types::ItemKind;

pub struct Hammer;

/// ftData.x48_items index (ftKb_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;
/// it_803F6C98's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];

/// efAsync_Spawn(item, &xBC0, 0, id, it_80272C90(item)): the article's
/// effect by the swing (setupHammerParticles: 1176 on the ground, 1177 in
/// the air).
const GROUND_EFFECT: u16 = 0x498;
const AIR_EFFECT: u16 = 0x499;
/// it_80272C90: the model desc's attach joint (x8 = 1). The model has no
/// bone table (x4 = 0), so it is the root's first child, which is joint 1
/// in depth-first order too.
const ATTACH_JOINT: usize = 1;

/// it_803F6C98: `{ 0, NULL, NULL, NULL }`.
static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation: no_animation,
    physics: no_physics,
    collision: no_collision,
}];

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

impl ItemLogic for Hammer {
    const KIND: ItemKind = ItemKind::KirbyHammer;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// Item_8026AB54 takes it into Kirby's hand at once.
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// setupHammerParticles: `kirbyhammer.x0 = vars`, before the pickup.
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.scratch = ItemScratch::KirbyHammer(KirbyHammerState {
            airborne: spawn.spawn_argument == 1,
        });
    }
    /// itKirbyHammer_Logic8_PickedUp (802ADD8C): Item_80268E5C(gobj, 0,
    /// ITEM_ANIM_UPDATE). setupHammerParticles then asks for the effect on
    /// the attach joint.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
        let ItemScratch::KirbyHammer(state) = item.scratch else {
            panic!("hammer scratch missing")
        };
        item.spawn_async(ItemEvent::BoneModel {
            id: if state.airborne {
                AIR_EFFECT
            } else {
                GROUND_EFFECT
            },
            bone: ATTACH_JOINT,
        });
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            // it_802ADC34: Item_8026A8EC. The Destroyed callback
            // (itKirbyHammer_Logic8_Destroyed) lets go of the owner.
            ItemControl::Remove => item.destroyed = true,
            _ => unreachable!("{control:?} sent to Kirby's hammer"),
        }
    }
}
