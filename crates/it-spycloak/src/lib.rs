//! The Cloaking Device (It_Kind_Spycloak, itspycloak.c 80295C40..80295EA8):
//! it falls in and rests, leaning with the floor, until a fighter's pickup
//! consumes it (ftpickupitem_8009447C: ftCo_800C88D4, then Item_8026A8EC).
//! Its dropped (state 3) and sliding (state 4) states are not witnessed:
//! nothing drops it before the pickup consumes it.
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, AirLanding, ItemAnimationContext,
    ItemCollisionContext, ItemCore, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

/// It_Kind_Spycloak.
pub struct CloakingDevice;

/// it_803F6370's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [0, 0, 0, 0, 0];
/// The Cloaking Device's special attributes (none read).
pub const SPECIAL_ATTRIBUTES: u32 = 0;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
}

static STATES: [ItemStateRow; 5] = [
    row(0, rest_collision, no_physics),
    row(1, fall_collision, gravity_physics),
    row(2, no_collision, no_physics),
    row(3, guarded_collision, gravity_physics),
    row(4, guarded_collision, no_physics),
];

const fn row(
    state: usize,
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation: no_animation,
        physics,
        collision,
    }
}

impl ItemLogic for CloakingDevice {
    const KIND: ItemKind = ItemKind::Spycloak;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itSpyCloak_Logic33_Spawned: the fall (it_80295D04).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        change(item, motion::FALL, assets);
    }
    /// itSpyCloak_Logic33_PickedUp: state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, context.assets);
    }
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!("itSpyCloak_Logic33_Dropped (itspycloak.c:85): a dropped Cloaking Device is not witnessed")
    }
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itSpyCloak_Logic33_EnteredAir (itspycloak.c:105): a Cloaking Device sliding down a slope is not witnessed")
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

/// itSpycloak_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// it_80295C68 (80295C68): stopped, grabbable, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = hsd_types::Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::REST, assets);
}

/// itSpycloak_UnkMotion0_Coll: it_8026D62C (off the floor it falls), then
/// the lean with the floor (it_80276CB8).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ctx.assets);
    }
    item.lean_with_floor();
    false
}

/// itSpycloak_UnkMotion1_Coll: it_8026E15C(it_80295C68).
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
        AirLanding::Broken => unreachable!("it_8026DDFC: a Cloaking Device is never thrown"),
    }
}

/// States 3 and 4 follow the guarded drop and slide.
fn guarded_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itSpycloak_UnkMotion3_Coll / UnkMotion4_Coll (itspycloak.c:95-130): a dropped or sliding Cloaking Device is not witnessed")
}
