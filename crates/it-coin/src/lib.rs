//! A Coin match's coin (It_Kind_Unk4), it_2E5A.c (802E5AC4..802E6AE8): the
//! spill's flight (state 0, or 1 for coins no fighter can take, which fall
//! through the stage), and the rest on the floor (state 2) until its
//! lifetime ends. A fighter takes a coin through its pickup sphere
//! (ft_8007C77C, in the scene); the spill's plan is `melee_it::coin`.
use melee_it::{
    coin::CoinAttributes,
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemLogic, ItemOwner,
    ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct Coin;

/// it_803F8BD8's anim_id column.
pub const ARTICLE_STATES: [i32; 3] = [0, -1, 0];
pub use melee_it::coin::SPECIAL_ATTRIBUTES;

/// Item_80268E5C flag bit 0 (it/forward.h).
const UNK_0X1: u32 = 1;

mod motion {
    /// Flying from the spill (it_2E5A_UnkMotion0).
    pub const FLIGHT: u16 = 0;
    /// Flying, with no map collision: a screen or star KO's coins.
    pub const UNCOLLECTABLE_FLIGHT: u16 = 1;
    /// At rest on a floor (it_2E5A_UnkMotion2).
    pub const REST: u16 = 2;
}

static STATES: [ItemStateRow; 3] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: flight_animation,
        physics: flight_physics,
        collision: flight_collision,
    },
    // it_803F8BD8[1]: no collision callback.
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: flight_animation,
        physics: flight_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: rest_animation,
        physics: no_physics,
        collision: rest_collision,
    },
];

impl ItemLogic for Coin {
    const KIND: ItemKind = ItemKind::Unk4;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802E609C's it_802750F8 after each coin's set-up.
    const PROCS_AT_SPAWN: bool = true;
    /// it_802E609C after Item_80268B18: it_802E6380's set-up of the coin
    /// the spill planned, then it_802E66A0's first state.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        spawn: &melee_it::SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let (coin, collectable) = melee_it::coin::planned_coin(spawn);
        item.start_coin(&coin, collectable, &attributes(assets), common.half_life_scale);
        enter_flight(item, assets);
    }
    /// Coins are never held.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802E6A74, a collectable coin's on_accessory.
    fn accessory(
        item: &mut ItemCore,
        _owner: Option<&ItemOwner>,
        _assets: &ItemAssets,
    ) -> Option<melee_it::OwnerBlast> {
        if item.coin().collectable {
            item.follow_coin_sphere();
        }
        None
    }
}

/// it_802E66A0 (802E66A0): a collectable coin flies in state 0 (its first
/// entry plays the tier's animation, which only draws), any other in
/// state 1.
pub fn enter_flight(item: &mut ItemCore, assets: &ItemAssets) {
    if item.coin().collectable {
        if item.coin().animated {
            change(item, motion::FLIGHT, HIT_PRESERVE | UNK_0X1, assets);
        } else {
            change(item, motion::FLIGHT, ANIM_UPDATE, assets);
            item.coin_mut().animated = true;
        }
    } else {
        change(item, motion::UNCOLLECTABLE_FLIGHT, ANIM_UPDATE, assets);
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn attributes(assets: &ItemAssets) -> CoinAttributes {
    CoinAttributes::read(&assets.special_attributes)
}

/// it_2E5A_UnkMotion1_Anim (802E67A8): the animation turns faster the
/// faster it falls (802E67D4: fmadd in double, then frsp), then
/// it_802E657C's timers; true at the lifetime's end.
fn flight_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    // 802E67B8..C8: fcmpo with 0, fneg.
    let speed = if item.velocity.y < 0.0 {
        -item.velocity.y
    } else {
        item.velocity.y
    };
    item.animation_rate = gekko_math::fma::fmadd(0.1, f64::from(speed), 1.0) as f32;
    item.tick_coin_timers()
}

/// it_2E5A_UnkMotion1_Phys -> it_80272860 with ItemAttr x10 / x14.
fn flight_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_2E5A_UnkMotion0_Coll (802E6824): a landing it rests from
/// (it_802E6888), its landing count and spark flag cleared.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let a = attributes(ctx.assets);
    if item.coin_floor_pass(ctx.map, ctx.assets, &a, true, ctx.rng) {
        rest(item, ctx.assets);
        item.land_count = 0;
        item.bounce_spark_off = false;
    }
    false
}

/// it_802E6888 (802E6888): still, at the normal rate, in state 2.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Default::default();
    item.animation_rate = 1.0;
    change(item, motion::REST, HIT_PRESERVE | UNK_0X1, assets);
}

/// it_2E5A_UnkMotion2_Anim (802E68EC): the timers alone.
fn rest_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.tick_coin_timers()
}

/// it_2E5A_UnkMotion2_Coll (802E6910): a collectable coin whose floor is
/// gone flies again.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.coin().collectable {
        let a = attributes(ctx.assets);
        if !item.coin_floor_pass(ctx.map, ctx.assets, &a, false, ctx.rng) {
            enter_flight(item, ctx.assets);
        }
    }
    false
}
