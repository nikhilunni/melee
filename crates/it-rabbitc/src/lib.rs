//! The Bunny Hood (It_Kind_RabbitC, itrabbitc.c 80294DC0..802953C4): it
//! falls in and rests; picked up it is worn on the head (ftCommon_8007FA58:
//! dropped from the hand and hung from the head part again) until its
//! wearer loses it. Its model's joint 3 shows only off the head
//! (it_80272A18 / it_80272A3C), which is presentation.
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

/// It_Kind_RabbitC.
pub struct BunnyHood;

/// it_803F6270's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [-1, -1, -1, -1, -1];
/// The Bunny Hood's attributes: x0 (the frames it is worn).
pub const SPECIAL_ATTRIBUTES: u32 = 1;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
}

/// it_8026B54C as ftCommon_8007FA58 stores it in x2014 (fctiwz).
pub fn frames(assets: &ItemAssets) -> i32 {
    gekko_math::msl::fctiwz(assets.special_attributes[0])
}

static STATES: [ItemStateRow; 5] = [
    row(motion::REST, no_physics, rest_collision),
    row(motion::FALL, gravity_physics, fall_collision),
    row(motion::HELD, no_physics, no_collision),
    row(3, gravity_physics, bounce_collision),
    row(4, no_physics, sliding_collision),
];

const fn row(
    state: u16,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state as usize],
        animation: no_animation,
        physics,
        collision,
    }
}

impl ItemLogic for BunnyHood {
    const KIND: ItemKind = ItemKind::RabbitC;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itRabbitC_Logic31_Spawned: the fall (it_802951C0).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itRabbitC_Logic31_PickedUp: state 2 (joint 3 hidden).
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itRabbitC_Logic31_Dropped: the fall (state 1, flags 6; joint 3
    /// shown).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FALL, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itRabbitC_Logic31_EnteredAir: state 4.
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itRabbitC_Logic31_EnteredAir (itrabbitc.c:184): a Bunny Hood sliding down a slope is not witnessed")
    });
    /// No DmgDealt, Reflected, Clanked or HitShield callback; the logic
    /// row's ShieldBounced is itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_80295138 (80295138): stopped, grabbable, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = hsd_types::Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::REST, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itRabbitc_UnkMotion1_Phys / UnkMotion3_Phys: it_80272860 with ItemAttr
/// x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itRabbitc_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itRabbitc_UnkMotion1_Coll: it_8026E15C(it_80295138).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            come_to_rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itRabbitc_UnkMotion3_Coll: it_8026DA08. No logic enters state 3 (no
/// Thrown callback).
fn bounce_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unreachable!("itRabbitc_UnkMotion3_Coll (itrabbitc.c:170): no logic enters state 3")
}

/// itRabbitc_UnkMotion4_Coll, after the guarded EnteredAir.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itRabbitc_UnkMotion4_Coll (itrabbitc.c:196): a Bunny Hood sliding down a slope is not witnessed")
}
