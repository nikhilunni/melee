//! The healing items a fighter eats: Heart Container (itheart.c), Maxim
//! Tomato (ittomato.c) and Food (itfoods.c). Each falls in and rests until a
//! fighter picks it up; the fighter's pickup eats it
//! (ftpickupitem_8009447C: Fighter_8006CF5C with it_8026B47C's amount, then
//! Item_8026A8EC), so the held states only last until the pickup's
//! animation ends.
//!
//! Their dropped and sliding (EnteredAir) states are not witnessed and fail
//! closed: nothing drops a consumable before it is eaten.
use melee_it::{
    desc::ItemAssets, AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore,
    ItemPhysicsContext, ItemScratch,
};

pub mod foods;
pub mod heart;
pub mod tomato;

pub use foods::Food;
pub use heart::HeartContainer;
pub use tomato::MaximTomato;

/// it_8026B47C (8026B47C): what eating `item` heals; 0 for any other kind.
pub fn heal_amount(item: &ItemCore) -> i32 {
    match item.scratch {
        ItemScratch::Heal { amount, .. } => amount,
        _ => 0,
    }
}

/// An s32 special attribute word.
fn word(assets: &ItemAssets, index: usize) -> i32 {
    assets.special_attributes[index].to_bits() as i32
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_80272860 with ItemAttr x10 / x14.
fn gravity(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// it_8026E15C: an airborne pass with landings; true once it settled on a
/// floor and the caller's callback runs.
fn landed(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => true,
        AirLanding::Broken => unreachable!("it_8026DDFC: a healing item is never thrown"),
    }
}
