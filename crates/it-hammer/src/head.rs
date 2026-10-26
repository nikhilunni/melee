//! The Hammer's lost head (It_Kind_Hammer_Head, ithammerhead.c): launched
//! from the swinging Hammer (it_80299C48), it falls spinning, comes to rest
//! and lasts its x4 frames; it can be picked up and thrown like any light
//! item.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

/// It_Kind_Hammer_Head.
pub struct HammerHead;

/// it_803F6640's anim_id column.
pub const ARTICLE_STATES: [i32; 4] = [-1, -1, 0, -1];
/// itHammerheadAttributes: initial_velocity, x4 (the lifetime).
pub const SPECIAL_ATTRIBUTES: u32 = 2;

mod motion {
    pub const FALL: u16 = 0;
    pub const HELD: u16 = 1;
    pub const THROWN: u16 = 2;
    pub const REST: u16 = 3;
}

/// it_80299C48's launch: `direction` is the unit vector from the handle
/// joint to the head (its x made positive), `facing` its sign. The spawn
/// carries both (`velocity` holds the direction until `launched`).
pub fn spawn(owner: u8, head: hsd_types::Vec3, center: hsd_types::Vec3, direction: hsd_types::Vec3, facing: f32) -> SpawnItem {
    let mut spawn = SpawnItem::ray(ItemKind::HammerHead, owner, center, facing);
    spawn.previous_position = hsd_types::Vec3::new(head.x, head.y, 0.0);
    spawn.position = center;
    spawn.velocity = direction;
    spawn
}

static STATES: [ItemStateRow; 4] = [
    row(motion::FALL, timed_animation, spinning_physics, fall_collision),
    row(motion::HELD, no_animation, no_physics, no_collision),
    row(motion::THROWN, timed_animation, spinning_physics, fall_collision),
    row(motion::REST, timed_animation, no_physics, rest_collision),
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

fn initial_velocity(assets: &ItemAssets) -> f32 {
    assets.special_attributes[0]
}

fn lifetime(assets: &ItemAssets) -> f32 {
    assets.special_attributes[1]
}

fn set_lifetime(item: &mut ItemCore, lifetime: f32, half_life_scale: f32) {
    item.life_timer = lifetime;
    item.half_life = lifetime * half_life_scale;
}

impl ItemLogic for HammerHead {
    const KIND: ItemKind = ItemKind::HammerHead;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itHammerHead_Logic40_Spawned (its lifetime, it_80275158), then
    /// it_80299C48's set-up: it_80299D7C (airborne, grabbable, state 0 with
    /// flags 1 | DROP_UPDATE), the facing and the velocity: the direction
    /// times initial_velocity (times the facing for x), plus 0.5 up.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        set_lifetime(item, lifetime(assets), common.half_life_scale);
        if item.ground_or_air != melee_types::GroundOrAir::Air {
            item.enter_air();
        }
        item.grabbable = true;
        change(item, motion::FALL, DROP_UPDATE, assets);
        item.facing = spawn.facing;
        let speed = initial_velocity(assets);
        let direction = spawn.velocity;
        item.velocity = hsd_types::Vec3::new(
            direction.x * speed * item.facing,
            // retail 80299D28: fmadds.
            gekko_math::fma::fmadds(direction.y, speed, 0.5),
            item.velocity.z,
        );
    }
    /// itHammerHead_Logic40_PickedUp: a fresh lifetime, state 1.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        set_lifetime(item, lifetime(context.assets), context.assets.half_life_scale);
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itHammerHead_Logic40_Dropped: as thrown.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        throw(item, context.assets);
    }
    /// itHammerHead_Logic40_Thrown: airborne, grabbable, state 2.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        throw(item, context.assets);
    }
    /// itHammerHead_Logic40_DmgDealt / Clanked / HitShield /
    /// DmgReceived: it goes on.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn damage_received(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itHammerHead_Logic40_Reflected: it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// itHammerHead_Logic40_Thrown (80299E28).
fn throw(item: &mut ItemCore, assets: &ItemAssets) {
    if item.ground_or_air != melee_types::GroundOrAir::Air {
        item.enter_air();
    }
    item.grabbable = true;
    change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itHammerhead_UnkMotion2_Anim / UnkMotion3_Anim: the lifetime counts
/// down; at 0 it is gone. Its blinking near the end (it_802728C8) is
/// presentation.
fn timed_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itHammerhead_UnkMotion2_Phys: it_80272860 with ItemAttr x10 / x14,
/// then it_80274658's spin by ItCo +68.
fn spinning_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itHammerhead_UnkMotion2_Coll: it_8026E15C(it_80299FB4): stopped,
/// grabbable, at rest (state 3, flags 1).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            item.velocity.y = 0.0;
            item.velocity.x = 0.0;
            item.grabbable = true;
            change(item, motion::REST, 0, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itHammerhead_UnkMotion3_Coll: it_8026D62C, off the floor thrown again.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        throw(item, ctx.assets);
    }
    false
}
