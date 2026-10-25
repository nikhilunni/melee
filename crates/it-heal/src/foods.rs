//! Food (It_Kind_Foods), itfoods.c (8028F9D8..8028FE90): a dish drawn at
//! random falls in, lands with a clink and rests until a fighter eats it.
use crate::{gravity, no_animation, no_collision, no_physics, word};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemLogic, ItemPhysicsContext,
    ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct Food;

/// it_803F5DB0's anim_id column: no state plays a joint animation.
pub const ARTICLE_STATES: [i32; 4] = [-1; 4];
/// itFoodsAttributes[28]: one 16-byte row per dish.
pub const SPECIAL_ATTRIBUTES: u32 = DISH_ROW * DISHES;

/// Words per itFoodsAttributes row: x0, the dish's joint x4, its heal x8, xC.
const DISH_ROW: u32 = 4;
/// The rows ItCo.usd carries; row 0's x0 (the draw's bound) names as many.
const DISHES: u32 = 28;
/// itFoodsAttributes x8: what eating the dish heals.
const DISH_HEAL: usize = 2;

/// Item_8026AE84(ip, 0x107, 127, 64): the landing's clink.
const LANDING_SOUND: u32 = 0x107;

mod motion {
    pub const FALL: u16 = 0;
    pub const REST: u16 = 1;
    pub const HELD: u16 = 2;
}

static STATES: [ItemStateRow; 4] = [
    row(0, gravity, fall_collision),
    row(1, no_physics, rest_collision),
    row(2, no_physics, no_collision),
    row(3, gravity, fall_collision),
];

const fn row(
    state: usize,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation: no_animation,
        physics,
        collision,
    }
}

impl ItemLogic for Food {
    const KIND: ItemKind = ItemKind::Foods;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itFoods_Logic18_Spawned (8028FBA8)'s fall (it_8028FC5C); its dish
    /// draw follows in [`Self::launched`].
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.scratch = ItemScratch::Heal { amount: 0, dish: 0 };
        change(item, motion::FALL, assets);
    }
    /// itFoods_Logic18_Spawned (8028FBA8): HSD_Randi(row 0's x0) picks the
    /// dish, whose x8 is the heal. Its model (x4, it_80273318) is the
    /// presentation's. The draw is the creation's last: nothing between it
    /// and Item_80268B18's return draws or reads the scratch.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        _common: &ItemCommonData,
        _spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let dishes = word(assets, 0);
        let dish = rng.randi(dishes) as u32;
        assert!(dish < DISHES, "itFoodsAttributes has {DISHES} rows");
        let amount = word(assets, (dish * DISH_ROW) as usize + DISH_HEAL);
        item.scratch = ItemScratch::Heal { amount, dish };
    }
    /// itFoods_Logic18_PickedUp: the model shown (display only), state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, context.assets);
    }
    /// itFoods_Logic18_Dropped (itfoods.c:160): nothing drops Food before
    /// its pickup eats it.
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!("itFoods_Logic18_Dropped (itfoods.c:160): dropped Food is not witnessed")
    }
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// itFoods_UnkMotion0_Coll / 3_Coll -> it_8026E414: a floor stops it
/// (it_8028FCE8: itResetVelocity, the clink, state 1).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        item.velocity = Vec3::ZERO;
        item.sound_requests.push(LANDING_SOUND);
        change(item, motion::REST, ctx.assets);
    }
    false
}

/// itFoods_UnkMotion1_Coll -> it_8026D62C: off the floor it falls again
/// (it_8028FC5C).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}
