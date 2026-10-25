//! Maxim Tomato (It_Kind_Tomato), ittomato.c (802841B4..80284620): it hops
//! up out of nowhere, falls and rests until a fighter eats it.
use crate::{gravity, landed, no_animation, no_collision, no_physics, word};
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemCore, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::ItemKind;

pub struct MaximTomato;

/// it_803F5740's anim_id column: no state plays a joint animation.
pub const ARTICLE_STATES: [i32; 6] = [-1; 6];
/// MaximTomatoSpecialAttr: x0..x14.
pub const SPECIAL_ATTRIBUTES: u32 = 6;

mod motion {
    /// The spawn's hop and fall.
    pub const HOP: u16 = 0;
    pub const REST: u16 = 1;
    /// Off a floor it rested on.
    pub const FALL: u16 = 2;
    pub const HELD: u16 = 3;
}

/// MaximTomatoSpecialAttr x0 (heal_amount_0): what eating it heals.
const HEAL: usize = 0;
/// MaximTomatoSpecialAttr x14: the hop's upward speed at spawn.
const HOP_SPEED: usize = 5;

static STATES: [ItemStateRow; 6] = [
    row(0, gravity, fall_collision),
    row(1, no_physics, rest_collision),
    row(2, gravity, fall_collision),
    row(3, no_physics, no_collision),
    row(4, gravity, fall_collision),
    row(5, no_physics, slide_collision),
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

impl ItemLogic for MaximTomato {
    const KIND: ItemKind = ItemKind::Tomato;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itTomato_Logic9_Spawned (802842A8): grabbable (it_8026B390), a hop
    /// up at x14, the heal x0, not the stage's own (x4_b0, x8), then the
    /// hop (it_80284358).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.grabbable = true;
        item.velocity = Vec3::new(0.0, assets.special_attributes[HOP_SPEED], 0.0);
        item.scratch = ItemScratch::Heal {
            amount: word(assets, HEAL),
            dish: 0,
        };
        change(item, motion::HOP, assets);
    }
    /// itTomato_Logic9_PickedUp: the model shown (display only), state 3.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, context.assets);
    }
    /// itTomato_Logic9_Dropped (ittomato.c:165): nothing drops a Maxim
    /// Tomato before its pickup eats it.
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!(
            "itTomato_Logic9_Dropped (ittomato.c:165): a dropped Maxim Tomato is not witnessed"
        )
    }
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itTomato_Logic9_EnteredAir (ittomato.c:175): a Maxim Tomato sliding down a slope is not witnessed")
    });
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// itTomato_UnkMotion0_Coll / 4_Coll -> it_8026E15C: a settled landing
/// stops it (itResetVelocity) and rests it (it_802843E4).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if landed(item, ctx) {
        item.velocity = Vec3::ZERO;
        change(item, motion::REST, ctx.assets);
    }
    false
}

/// itTomato_UnkMotion1_Coll -> it_8026D62C: off the floor it falls
/// (it_80284458).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}

/// itTomato_UnkMotion5_Coll -> it_8026E8C4 (reached only through the
/// guarded EnteredAir).
fn slide_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itTomato_UnkMotion5_Coll (ittomato.c:187): a Maxim Tomato sliding down a slope is not witnessed")
}
