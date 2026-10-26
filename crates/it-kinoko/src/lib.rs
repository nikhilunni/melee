//! The Super and Poison Mushrooms (It_Kind_Kinoko, itkinoko.c
//! 80293660..802939A0; It_Kind_DKinoko, itdkinoko.c 802939A0..80293D94).
//! The two kinds share their code: they fall in, then walk along the floor
//! at their x0 speed, turning at walls; off an edge they fall again,
//! slowing by x4 a frame. Their hitbox is a power-up's (use kind 4): a
//! fighter it touches takes it (ftColl_80077C60: x200C or x2010) and the
//! Mushroom is gone (DmgDealt).
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::ItemKind;

/// It_Kind_Kinoko.
pub struct SuperMushroom;
/// It_Kind_DKinoko.
pub struct PoisonMushroom;

/// it_803F6110 / it_803F6130's anim_id column.
pub const ARTICLE_STATES: [i32; 2] = [0, 0];
/// KinokoAttrs: x0 (the walking speed), x4 (the airborne slowdown).
pub const SPECIAL_ATTRIBUTES: u32 = 2;
/// KinokoAnim (it_80293660): the Super Mushroom's attributes x8 and xC
/// are the joint animations a size change plays (growing or shrinking to
/// the new size, then back).
pub const SIZE_ANIMATIONS: [u32; 2] = [0x8, 0xC];

mod motion {
    pub const WALK: u16 = 0;
    pub const FALL: u16 = 1;
}

static STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: walk_physics,
        collision: walk_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: fall_physics,
        collision: fall_collision,
    },
];

impl ItemLogic for SuperMushroom {
    const KIND: ItemKind = ItemKind::Kinoko;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itKinoko_Logic26_Spawned.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        spawn(item, assets);
    }
    /// No PickedUp callback: Item_IsGrabbable refuses it.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itKinoko_Logic26_DmgDealt: gone once it touched a fighter.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
}

impl ItemLogic for PoisonMushroom {
    const KIND: ItemKind = ItemKind::DKinoko;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itDKinoko_Logic27_Spawned.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        spawn(item, assets);
    }
    /// No PickedUp callback: Item_IsGrabbable refuses it.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itDKinoko_Logic27_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
}

/// The Spawned callbacks: facing the stage's centre, no slope slide
/// (xD5C 0), standing still on an upright normal, then the fall
/// (it_8029385C).
fn spawn(item: &mut ItemCore, assets: &ItemAssets) {
    item.facing = if item.position.x > 0.0 { -1.0 } else { 1.0 };
    item.slide_mode = 0;
    item.scratch = ItemScratch::Kinoko {
        speed: 0.0,
        normal: Vec3::new(0.0, 1.0, 0.0),
    };
    change(item, motion::FALL, ANIM_UPDATE, assets);
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn walking(item: &mut ItemCore) -> (&mut f32, &mut Vec3) {
    match &mut item.scratch {
        ItemScratch::Kinoko { speed, normal } => (speed, normal),
        _ => unreachable!("a Mushroom's scratch"),
    }
}

fn attributes(assets: &ItemAssets) -> (f32, f32) {
    (assets.special_attributes[0], assets.special_attributes[1])
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// itKinoko_UnkMotion0_Phys: on the ground, x0 along the facing, its
/// horizontal speed times the floor normal's y.
fn walk_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.ground_or_air == melee_types::GroundOrAir::Air {
        return;
    }
    let (walk, _) = attributes(ctx.assets);
    let facing = item.facing;
    let (speed, normal) = walking(item);
    *speed = walk * facing;
    let velocity = *speed * normal.y;
    item.velocity.x = velocity;
}

/// itKinoko_UnkMotion0_Coll: a grounded pass (it_8026D78C) whose loss of
/// the floor falls (it_80293884, keeping the hitbox) into the air; on the
/// ground the floor's normal (it_80276408) or no lean, and a wall turns
/// the Mushroom round.
fn walk_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.ground_pass(ctx.map) {
        change(item, motion::FALL, 1 | HIT_PRESERVE, ctx.assets);
        item.enter_air();
    }
    if item.ground_or_air == melee_types::GroundOrAir::Air {
        return false;
    }
    let floor = item.floor_contact_normal();
    let wall = item.touches_wall();
    {
        let (_, normal) = walking(item);
        match floor {
            Some(floor) => *normal = floor,
            None => normal.x = 0.0,
        }
    }
    if wall {
        item.facing = -item.facing;
        let (speed, normal) = walking(item);
        *speed = -*speed;
        let velocity = *speed * normal.y;
        item.velocity.x = velocity;
    }
    false
}

/// itKinoko_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14, then
/// x4 off the horizontal speed (to rest when below it).
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    let (_, slowdown) = attributes(ctx.assets);
    let speed = if item.velocity.x < 0.0 {
        -item.velocity.x
    } else {
        item.velocity.x
    };
    if speed < slowdown {
        item.velocity.x = 0.0;
        return;
    } else if item.velocity.x > 0.0 {
        item.velocity.x -= slowdown;
    }
    if item.velocity.x < 0.0 {
        item.velocity.x += slowdown;
    }
}

/// itKinoko_UnkMotion1_Coll: it_8026E7E0's landing walks on (it_802936E4);
/// on the ground the floor's normal.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing =
        item.air_collision_landing_quietly(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => {}
        AirLanding::Landed => land(item, ctx.assets),
        AirLanding::Broken => return true,
    }
    if item.ground_or_air != melee_types::GroundOrAir::Air {
        if let Some(floor) = item.floor_contact_normal() {
            let (_, normal) = walking(item);
            *normal = floor;
        }
    }
    false
}

/// it_802936E4 (802936E4): walking on at the kept speed, grabbable, its
/// landing sound (0x107, not simulated), state 0 keeping the hitbox.
fn land(item: &mut ItemCore, assets: &ItemAssets) {
    let (speed, _) = walking(item);
    let speed = *speed;
    item.velocity = Vec3::new(speed, 0.0, 0.0);
    item.grabbable = true;
    change(item, motion::WALK, 1 | HIT_PRESERVE, assets);
}
