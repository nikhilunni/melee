//! The Screw Attack (It_Kind_ScBall, itscball.c 80294AA8..80294D44): it
//! falls in and rests; held, it turns its holder's jumps into Screw
//! Attacks (ItemScrew, ft_800D2D0C); thrown, it flies until a floor.
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

/// It_Kind_ScBall.
pub struct ScrewAttack;

/// it_803F6220's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [-1, -1, -1, 0, -1];
/// The Screw Attack's special attributes (none read).
pub const SPECIAL_ATTRIBUTES: u32 = 0;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
}

static STATES: [ItemStateRow; 5] = [
    row(motion::REST, no_physics, rest_collision),
    row(motion::FALL, gravity_physics, fall_collision),
    row(motion::HELD, no_physics, no_collision),
    row(motion::THROWN, falling_physics, fall_collision),
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

impl ItemLogic for ScrewAttack {
    const KIND: ItemKind = ItemKind::ScBall;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itScball_Logic30_Spawned: the fall (it_80294B58).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itScball_Logic30_PickedUp: state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itScball_Logic30_Dropped: the fall again (state 1, flags 6).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FALL, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itScball_Logic30_Thrown: state 3.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itScball_Logic30_EnteredAir: state 4.
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itScball_Logic30_EnteredAir (itscball.c:122): a Screw Attack sliding down a slope is not witnessed")
    });
    /// itScball_Logic30_DmgDealt / Clanked: it is gone.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itScball_Logic30_Reflected: it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// itScball_Logic30_HitShield: itColl_BounceOffVictim.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itRabbitC_Logic30_ShieldBounced: itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_80294AD0 (80294AD0): stopped, grabbable, at rest.
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

/// itScball_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itScball_UnkMotion3_Phys -> Item_ApplyFallingPhysics.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itScball_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itScball_UnkMotion1_Coll / UnkMotion3_Coll: it_8026E15C(it_80294AD0).
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

/// itScball_UnkMotion4_Coll, after the guarded EnteredAir.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itScball_UnkMotion4_Coll (itscball.c:137): a Screw Attack sliding down a slope is not witnessed")
}
