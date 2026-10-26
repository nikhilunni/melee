//! The Hammer (It_Kind_Hammer, ithammer.c 80293D94..80294334) and the head
//! it can lose (It_Kind_Hammer_Head, ithammerhead.c 80299C48..8029A0F4).
//!
//! The Hammer falls in and rests. Picked up (ftpickupitem.c:182) it takes
//! its holder into the Hammer states (`melee-ft` `fighter/hammer.rs`), which
//! enable its swing hitboxes (it_8026C16C) and renew their attack instance
//! from the holder's at each loop of the swing (it_8027B070). xD4C (`ammo`)
//! is 1 while the head is on: the head's countdown ending launches it
//! (it_80293E34, the scene's `ItemRequest::HammerHeadLoss`) and the held
//! Hammer plays its headless swing (state 3). Dropped, a Hammer with its
//! head falls back to rest; a headless one is gone at its first floor.
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

/// It_Kind_Hammer.
pub struct Hammer;

/// it_803F6150's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [-1, -1, 0, 1, -1, -1];
/// itHammerData: x0 and x4 (the head's countdown range, ints), x8 (the
/// percent chance it is lost at all).
pub const SPECIAL_ATTRIBUTES: u32 = 3;
/// The article states the held Hammer animates (its swing, headless or
/// not), each posed in the hand.
pub const ANIMATED_STATES: usize = 2;
/// The model joints it_80293E34 reads (it_80272CC0): the head and the
/// handle's.
pub const HEAD_JOINT: usize = 3;
pub const HANDLE_JOINT: usize = 1;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const HELD_HEADLESS: u16 = 3;
}

/// itHammerData for it_80293DCC: the head's countdown range (x0, x4,
/// ints) and the percent chance (x8) it comes off.
pub fn head_odds(assets: &ItemAssets) -> (i32, i32, f32) {
    let a = &assets.special_attributes;
    (a[0].to_bits() as i32, a[1].to_bits() as i32, a[2])
}

static STATES: [ItemStateRow; 6] = [
    row(motion::REST, no_animation, no_physics, rest_collision),
    row(motion::FALL, no_animation, gravity_physics, fall_collision),
    row(motion::HELD, held_animation, no_physics, no_collision),
    row(motion::HELD_HEADLESS, held_animation, no_physics, no_collision),
    row(4, no_animation, gravity_physics, bounce_collision),
    row(5, no_animation, no_physics, sliding_collision),
];

const fn row(
    state: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state as usize],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for Hammer {
    const KIND: ItemKind = ItemKind::Hammer;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itHammer_Logic28_Spawned: the head on (xD4C 1), then the fall
    /// (it_8029402C).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ammo = 1;
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itHammer_Logic28_PickedUp: state 2, the hitlag callback
    /// (it_80293D94, the holder's freeze: the scene's), every hitbox off
    /// (it_802725D4) until the holder's state enables them.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
        item.clear_hitboxes();
    }
    /// itHammer_Logic28_Dropped: the fall (state 1, flags 6).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FALL, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itHammer_Logic28_EnteredAir: state 5.
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itHammer_Logic28_EnteredAir (ithammer.c:222): a Hammer sliding down a slope is not witnessed")
    });
    /// it_80293D94 (80293D94), installed by the pickup: unless it took
    /// damage (xDC8 xB), the holder freezes with it (it_8026BC14:
    /// ftLib_80086A4C with xCBC - 1).
    const ENTERED_HITLAG: Option<fn(&mut ItemCore, bool)> = Some(|item, took_damage| {
        if took_damage || !item.held {
            return;
        }
        if let Some(owner) = item.owner {
            item.owner_request = Some((
                owner,
                melee_it::OwnerRequest::Freeze {
                    secondary: item.owner_secondary,
                    // xCBC is whole (it_8026B424 fctiwz).
                    frames: item.hitlag_frames as i32 - 1,
                },
            ));
        }
    });
    /// No Thrown, DmgDealt, Reflected, Clanked, HitShield or ShieldBounced
    /// callback.
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_80293F84 (80293F84): stopped, grabbable, at rest.
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

/// itHammer_UnkMotion3_Anim (80294170): the held swing turns headless
/// (it_802941A4, state 3) once xD4C is 0.
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.ammo == 0 {
        change(item, motion::HELD_HEADLESS, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itHammer_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itHammer_UnkMotion0_Coll: it_8026D62C, off the floor it falls
/// (it_8029402C).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itHammer_UnkMotion1_Coll (802940B8): with its head, it_8026E15C
/// (it_80293F84); headless, it_8026DF34: gone at the first floor.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo == 0 {
        return item.air_pass(ctx.map);
    }
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

/// itHammer_UnkMotion4_Coll: no logic enters state 4 (no Thrown callback).
fn bounce_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unreachable!("itHammer_UnkMotion4_Coll (ithammer.c:213): no logic enters state 4")
}

/// itHammer_UnkMotion5_Coll, after the guarded EnteredAir.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itHammer_UnkMotion5_Coll (ithammer.c:233): a Hammer sliding down a slope is not witnessed")
}

pub mod head;
