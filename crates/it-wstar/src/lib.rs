//! Warp Star (It_Kind_WStar), itwstar.c (80294364..802948D8): it falls in
//! and rests, spinning, until a fighter picks it up; the pickup is a ride
//! (ftCo_WarpStar.c), whose rows drive the star's ridden and blast states.
//!
//! Ported so far: the fall and rest. The pickup and ride fail closed on the
//! fighter's side (ftpickupitem_8009447C).
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, AirLanding, ItemAnimationContext,
    ItemCollisionContext, ItemCore, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::ItemKind;

pub struct WarpStar;

/// it_803F61B0's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [0, -1, -1, 2, -1, 1];
/// itWstarAttributes: x0..x24 and seven 8-byte flight rows.
pub const SPECIAL_ATTRIBUTES: u32 = 10 + 2 * 7;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
}

/// itWstarAttributes x0: the resting spin, degrees a frame (it_80274658).
const REST_SPIN: usize = 0;

static STATES: [ItemStateRow; 6] = [
    row(0, rest_physics, rest_collision),
    row(1, gravity, fall_collision),
    row(2, no_physics, no_collision),
    row(3, no_physics, no_collision),
    row(4, no_physics, slide_collision),
    row(5, no_physics, no_collision),
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

impl ItemLogic for WarpStar {
    const KIND: ItemKind = ItemKind::WStar;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itWStar_Logic29_Spawned (802944F4): its ECB stays upright (xDCE b7
    /// off), the ride's model and hitbox scales 1 (xDD4, xDD8), the fall
    /// (it_80294624).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ecb_follows_rotation = false;
        item.scratch = ItemScratch::Float(1.0);
        change(item, motion::FALL, assets);
    }
    /// it_3F14_Logic29_PickedUp: the fighter's pickup rides it
    /// (ftCo_800C4724), which is not ported.
    fn picked_up(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!("it_3F14_Logic29_PickedUp (itwstar.c:153): a Warp Star picked up")
    }
    /// itWStar_Logic29_Dropped (itwstar.c:167): only a rider hit or KO'd
    /// drops it.
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!("itWStar_Logic29_Dropped (itwstar.c:167): a dropped Warp Star")
    }
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itWStar_Logic29_EnteredAir (itwstar.c:230): a Warp Star sliding down a slope is not witnessed")
    });
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itWstar_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itWstar_UnkMotion0_Phys: the spin at x0 degrees a frame
/// (it_80274658), turned (it_80274A64).
fn rest_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.update_spin(ctx.assets.special_attributes[REST_SPIN]);
    item.spin();
}

/// itWstar_UnkMotion0_Coll -> it_8026D62C: off the floor it falls
/// (it_80294624).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}

/// itWstar_UnkMotion1_Coll -> it_8026E15C: a settled landing rests
/// (it_8029455C: still, grabbable, state 0).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => {}
        AirLanding::Landed => {
            item.velocity = Vec3::ZERO;
            item.grabbable = true;
            change(item, motion::REST, ctx.assets);
        }
        AirLanding::Broken => unreachable!("it_8026DDFC: a Warp Star is never thrown"),
    }
    false
}

/// itWstar_UnkMotion4_Coll -> it_8026E8C4 (reached only through the
/// guarded EnteredAir).
fn slide_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itWstar_UnkMotion4_Coll (itwstar.c:242): a Warp Star sliding down a slope is not witnessed")
}
