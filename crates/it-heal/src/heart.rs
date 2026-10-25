//! Heart Container (It_Kind_Heart), itheart.c (80283AE4..80284180): it
//! hops up out of nowhere, falls and rests, spinning its model about Y,
//! until a fighter eats it.
use crate::{gravity, landed, no_animation, no_collision, no_physics, word};
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemCore, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::ItemKind;

pub struct HeartContainer;

/// it_803F56C8's anim_id column: no state plays a joint animation.
pub const ARTICLE_STATES: [i32; 5] = [-1; 5];
/// HeartContainerAttr: x0..x18.
pub const SPECIAL_ATTRIBUTES: u32 = 7;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
}

/// HeartContainerAttr x0: what eating it heals.
const HEAL: usize = 0;
/// HeartContainerAttr x14: the hop's upward speed at spawn.
const HOP_SPEED: usize = 5;

static STATES: [ItemStateRow; 5] = [
    row(0, spin, no_physics, rest_collision),
    row(1, spin, gravity, fall_collision),
    row(2, no_animation, no_physics, no_collision),
    row(3, spin, gravity, fall_collision),
    row(4, no_animation, no_physics, slide_collision),
];

const fn row(
    state: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for HeartContainer {
    const KIND: ItemKind = ItemKind::Heart;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itHeart_Logic8_Spawned (80283BF0): a hop up at x14, the heal x0, not
    /// one of the stage's own hearts (xDD8 b0, xDDC), then the fall
    /// (it_80283DD4).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.velocity = Vec3::new(0.0, assets.special_attributes[HOP_SPEED], 0.0);
        item.scratch = ItemScratch::Heal {
            amount: word(assets, HEAL),
            dish: 0,
        };
        change(item, motion::FALL, assets);
    }
    /// itHeart_Logic8_PickedUp: the model shown and unspun (display only),
    /// state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, context.assets);
    }
    /// itHeart_Logic8_Dropped (itheart.c:150): nothing drops a Heart
    /// Container before its pickup eats it.
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!(
            "itHeart_Logic8_Dropped (itheart.c:150): a dropped Heart Container is not witnessed"
        )
    }
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itHeart_Logic8_EnteredAir (itheart.c:160): a Heart Container sliding down a slope is not witnessed")
    });
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// itHeart_UnkMotion0_Anim / 3_Anim: the model's child joint turns by x18
/// about Y each frame (HSD_JObjAddRotationY). Display only: nothing reads
/// the child joint.
fn spin(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// it_80283C7C (80283C7C): grabbable (it_8026B390), still, at rest.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = Vec3::ZERO;
    change(item, motion::REST, assets);
}

/// itHeart_UnkMotion0_Coll -> it_8026D62C: off the floor it falls
/// (it_80283DD4).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}

/// itHeart_UnkMotion3_Coll -> it_8026E15C: a settled landing rests
/// (it_80283C7C).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if landed(item, ctx) {
        rest(item, ctx.assets);
    }
    false
}

/// itHeart_UnkMotion4_Coll -> it_8026E8C4 (reached only through the guarded
/// EnteredAir).
fn slide_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itHeart_UnkMotion4_Coll (itheart.c:178): a Heart Container sliding down a slope is not witnessed")
}
