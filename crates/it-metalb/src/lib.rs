//! The Metal Box (It_Kind_MetalB, itmetalb.c 802953C4..80295748): it falls
//! in and rests, leaning with the floor. Picked up (ftpickupitem.c:188) or
//! broken by a hit that interacts with items (its DmgReceived), it turns
//! the fighter metal (ftLib_800871A8) for its x0 frames or x4 damage.
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow, OwnerRequest,
};
use melee_types::ItemKind;

/// It_Kind_MetalB.
pub struct MetalBox;

/// it_803F62C0's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [-1, -1, -1, -1, -1];
/// The Metal Box's attributes: x0 (the metal's frames), x4 (the damage it
/// takes).
pub const SPECIAL_ATTRIBUTES: u32 = 2;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
}

/// it_8026B560 / it_8026B574, as ftCo_800C8348's int arguments (fctiwz):
/// the metal's frames and the damage that ends it.
pub fn metal(assets: &ItemAssets) -> (i32, i32) {
    let a = &assets.special_attributes;
    (gekko_math::msl::fctiwz(a[0]), gekko_math::msl::fctiwz(a[1]))
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

impl ItemLogic for MetalBox {
    const KIND: ItemKind = ItemKind::MetalB;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itMetalB_Logic32_Spawned: the ECB stays upright (xDCE b7), then the fall
    /// (it_80295498).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ecb_follows_rotation = false;
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itMetalB_Logic32_PickedUp: state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itMetalB_Logic32_Dropped: the fall (state 1, flags 6).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FALL, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itMetalB_Logic32_EnteredAir: state 4.
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itMetalB_Logic32_EnteredAir (itmetalb.c:131): a Metal Box sliding down a slope is not witnessed")
    });
    /// itMetalB_Logic32_DmgReceived (80295628): the box breaks. When the
    /// hit's fighter (it_8027236C: xCEC, else the hitting item's owner)
    /// struck with x43_b0 (xDCF b6), that fighter turns metal
    /// (ftLib_800871A8, from inside the callback: the owner request).
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if let Some(player) = item.hit_by.filter(|_| item.hit_interacts) {
            let (frames, health) = metal(context.assets);
            item.owner_request = Some((
                player,
                OwnerRequest::Metal {
                    secondary: item.hit_by_secondary,
                    frames,
                    health,
                },
            ));
        }
        true
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_802953FC (802953FC): stopped, grabbable, at rest.
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

/// itMetalb_UnkMotion1_Phys / UnkMotion3_Phys: it_80272860 with ItemAttr
/// x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itMetalb_UnkMotion0_Coll: it_8026D62C (off the floor it falls), then
/// the lean with the floor (it_80276CB8).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    item.lean_with_floor();
    false
}

/// itMetalb_UnkMotion1_Coll: it_8026E15C(it_802953FC).
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

/// itMetalb_UnkMotion3_Coll: it_8026DA08. Nothing enters state 3 (no
/// Thrown callback, it_803F62C0's row 3 only).
fn bounce_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unreachable!("itMetalb_UnkMotion3_Coll (itmetalb.c:107): no logic enters state 3")
}

/// itMetalb_UnkMotion4_Coll, after the guarded EnteredAir.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itMetalb_UnkMotion4_Coll (itmetalb.c:146): a Metal Box sliding down a slope is not witnessed")
}
