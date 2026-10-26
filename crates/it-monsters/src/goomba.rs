//! The Goomba out of a crate (It_Kind_Kuriboh), itoldkuri.c
//! (802D73F0..802D84D8): it stands a moment, then paces left and right,
//! turning at the end of each walk cycle or when an item pushes it, and
//! falls off edges. A fighter may stomp it (flattened on the floor, knocked
//! out of the air), carry it and throw it; a light hit makes it reel, a
//! killing one sends it tumbling or flying at the screen.
use crate::zako::{self, monster, monster_mut};
use gekko_math::msl::fctiwz;
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, KuribohAttributes, MonsterCommonData},
    push::PUSHED_BY_ITEM,
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemPhysicsContext,
    ItemStateRow, MonsterBehavior,
};
use melee_types::GroundOrAir;

/// it_803F8320's anim_id column.
pub const ARTICLE_STATES: [i32; 12] = [-1, 0, 1, 0, 1, 3, 4, 2, 2, 3, 3, 3];

pub(crate) mod motion {
    /// it_802D747C: standing where it landed.
    pub const STAND: u16 = 0;
    /// it_802D758C: walking left.
    pub const WALK_LEFT: u16 = 1;
    /// it_802D775C: walking right.
    pub const WALK_RIGHT: u16 = 2;
    /// it_802D7AF0: falling, facing left or right.
    pub const FALL_LEFT: u16 = 3;
    pub const FALL_RIGHT: u16 = 4;
    /// itOldKuri_Logic0_PickedUp: carried.
    pub const HELD: u16 = 5;
    /// it_2725_Logic0_Thrown.
    pub const THROWN: u16 = 6;
    /// it_802D813C: stomped flat on the floor.
    pub const FLATTENED: u16 = 7;
    /// it_802D81FC: stomped in the air.
    pub const STOMPED_AIR: u16 = 8;
    /// it_2725_Logic0_DmgReceived: knocked back by a light hit.
    pub const REEL: u16 = 9;
    /// it_802D82C4: killed, tumbling away.
    pub const TUMBLE: u16 = 10;
    /// it_802D839C: killed, flying at the screen.
    pub const SCREEN: u16 = 11;
}

/// it_8027C0A8(gobj, 0.0, 5.0): the model leans with the floor over five
/// frames, facing the camera (facing 0).
const LEAN_FRAMES: f32 = 5.0;
/// it_802D813C's it_8027C0A8(gobj, 0.0, 0.0): flattened, at once.
const FLAT_LEAN_FRAMES: f32 = 0.0;
/// The facing argument every lean passes: the model faces the camera.
const CAMERA_FACING: f32 = 0.0;
/// it_802D7AF0's and the walks' restart flags (0x12): the animation and the
/// hitboxes kept.
const RESTART: u32 = ANIM_UPDATE | HIT_PRESERVE;
/// The reel's least rise off the floor, compared as a double
/// (retail 0x802D8034: lfd @309 = 0.2) and stored as a single.
const REEL_LIFT: f64 = 0.2;
/// Camera_RequestQuake(QuakeKind_Small) (retail 0x802D7FE0: li r3, 2).
const SMALL_QUAKE: u16 = 2;
/// Item_8026AE84(ip, 0x12E, 0x7F, 0x40): the stomp's sound.
const STOMP_SOUND: u32 = 0x12E;

pub(crate) static STATES: [ItemStateRow; 12] = [
    row(motion::STAND, no_animation, stand_physics, walk_collision),
    row(
        motion::WALK_LEFT,
        walk_left_animation,
        walk_left_physics,
        walk_left_collision,
    ),
    row(
        motion::WALK_RIGHT,
        walk_right_animation,
        walk_right_physics,
        walk_right_collision,
    ),
    row(
        motion::FALL_LEFT,
        fall_left_animation,
        fall_physics,
        fall_collision,
    ),
    row(
        motion::FALL_RIGHT,
        fall_right_animation,
        fall_physics,
        fall_collision,
    ),
    row(motion::HELD, held_animation, no_physics, no_collision),
    row(
        motion::THROWN,
        thrown_animation,
        fall_physics,
        thrown_collision,
    ),
    row(
        motion::FLATTENED,
        flattened_animation,
        no_physics,
        no_collision,
    ),
    row(
        motion::STOMPED_AIR,
        stomped_air_animation,
        fall_physics,
        stomped_air_collision,
    ),
    row(motion::REEL, reel_animation, reel_physics, reel_collision),
    row(
        motion::TUMBLE,
        tumble_animation,
        no_physics,
        tumble_collision,
    ),
    row(
        motion::SCREEN,
        screen_animation,
        screen_physics,
        no_collision,
    ),
];

const fn row(
    motion: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion as usize],
        animation,
        physics,
        collision,
    }
}

fn attributes(assets: &ItemAssets) -> KuribohAttributes {
    assets.kuriboh.expect("typed Goomba attributes")
}
fn common(assets: &ItemAssets) -> MonsterCommonData {
    assets
        .monster_common
        .expect("it_804D6D40 for the monster bank")
}

/// itOldkuri's scratch: xDF4 the walk speed, xDF8 the stand's countdown,
/// xDFC whether the walk has run a full cycle (and may turn).
fn walk_speed(item: &ItemCore) -> f32 {
    match monster(item).behavior {
        MonsterBehavior::Kuriboh { walking_speed, .. } => walking_speed,
        _ => panic!("Goomba scratch"),
    }
}
fn set_walk_speed(item: &mut ItemCore, speed: f32) {
    if let MonsterBehavior::Kuriboh { walking_speed, .. } = &mut monster_mut(item).behavior {
        *walking_speed = speed;
    }
}
fn stand_timer(item: &ItemCore) -> i32 {
    match monster(item).behavior {
        MonsterBehavior::Kuriboh { spawn_timer, .. } => spawn_timer,
        _ => panic!("Goomba scratch"),
    }
}
fn set_stand_timer(item: &mut ItemCore, frames: i32) {
    if let MonsterBehavior::Kuriboh { spawn_timer, .. } = &mut monster_mut(item).behavior {
        *spawn_timer = frames;
    }
}
fn cycled(item: &ItemCore) -> bool {
    match monster(item).behavior {
        MonsterBehavior::Kuriboh { cycled, .. } => cycled,
        _ => panic!("Goomba scratch"),
    }
}
fn set_cycled(item: &mut ItemCore, value: bool) {
    if let MonsterBehavior::Kuriboh { cycled, .. } = &mut monster_mut(item).behavior {
        *cycled = value;
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[usize::from(motion)], flags, assets);
}

/// it_802D848C (802D848C): the state change, then stompable again
/// (it_80274CAC) with it_802D8098 as Item.jumped_on.
pub(crate) fn change_stompable(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    change(item, motion, flags, assets);
    item.stompable = true;
    item.jumped_on = Some(stomped);
}

// --- State entries ---------------------------------------------------------

/// it_802D747C (802D747C): stopped (it_8027CAD8), leaning with the floor,
/// tangible again (it_802756E0), standing.
pub(crate) fn stand(item: &mut ItemCore, assets: &ItemAssets) {
    zako::halt(item);
    zako::lean_with_floor(item, CAMERA_FACING, LEAN_FRAMES);
    item.hurt_intangible = false;
    change_stompable(item, motion::STAND, ANIM_UPDATE, assets);
}

/// it_802D758C (802D758C) and its inlined copies: the walk speed (xDF4,
/// the facing times x0->x4, fmuls) along X, facing left, a new cycle.
fn walk_left(item: &mut ItemCore, assets: &ItemAssets) {
    start_walk(item, -1.0, motion::WALK_LEFT, assets);
}

/// it_802D775C (802D775C): the same, facing right.
fn walk_right(item: &mut ItemCore, assets: &ItemAssets) {
    start_walk(item, 1.0, motion::WALK_RIGHT, assets);
}

fn start_walk(item: &mut ItemCore, facing: f32, motion: u16, assets: &ItemAssets) {
    // The speed takes the facing before the turn.
    let speed = item.facing * attributes(assets).walk_speed;
    set_walk_speed(item, speed);
    item.velocity = Vec3::new(speed, 0.0, 0.0);
    item.facing = facing;
    set_cycled(item, false);
    change_stompable(item, motion, ANIM_UPDATE, assets);
}

/// it_802D7AF0 (802D7AF0): off the floor it falls the way it faces.
fn fall_off(item: &mut ItemCore, assets: &ItemAssets) {
    let motion = if item.facing > 0.0 {
        motion::FALL_RIGHT
    } else {
        motion::FALL_LEFT
    };
    change_stompable(item, motion, RESTART, assets);
}

// --- Callbacks -------------------------------------------------------------

fn no_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {}
fn no_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itOldkuri_UnkMotion0_Phys (802D74D8): with the countdown run out it
/// walks the way it faces (right on exactly 1.0), else counts down; then
/// the lean.
fn stand_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let timer = stand_timer(item);
    if timer == 0 {
        if item.facing == 1.0 {
            walk_right(item, ctx.assets);
        } else {
            walk_left(item, ctx.assets);
        }
    } else {
        set_stand_timer(item, timer - 1);
    }
    zako::lean_with_floor(item, CAMERA_FACING, LEAN_FRAMES);
}

/// itOldkuri_UnkMotion0_Coll (802D7550): it_8026D62C, off the floor the
/// fall (it_802D7AF0); it_8027C794 returns false.
fn walk_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        fall_off(item, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion1_Anim (802D7604): at the cycle's end, still facing
/// left, it walks on with the cycle done (xDFC, flags 0x12); otherwise it
/// turns right.
fn walk_left_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        if item.facing == -1.0 {
            set_cycled(item, true);
            change_stompable(item, motion::WALK_LEFT, RESTART, ctx.assets);
        } else {
            walk_right(item, ctx.assets);
        }
    }
    false
}

/// itOldkuri_UnkMotion1_Phys (802D7680): a cycled walk an item pushes
/// (it_802750E8(gobj, 2)) turns right; the speed follows the floor
/// (it_8027C8D0), then the lean.
fn walk_left_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if cycled(item) && item.push_contacts & PUSHED_BY_ITEM != 0 {
        walk_right(item, ctx.assets);
    }
    item.velocity = zako::along_floor(item.velocity, floor_normal(item), item.facing);
    zako::lean_with_floor(item, CAMERA_FACING, LEAN_FRAMES);
}

/// itOldkuri_UnkMotion1_Coll (802D76F0): it_8026D62C, then a right wall
/// (it_80276308 == 4) turns a cycled walk right.
fn walk_left_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        fall_off(item, ctx.assets);
    }
    if item.wall_bits() == 4 && cycled(item) {
        walk_right(item, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion2_Anim (802D77E8): the mirror of the left walk's.
fn walk_right_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        if item.facing == 1.0 {
            set_cycled(item, true);
            change_stompable(item, motion::WALK_RIGHT, RESTART, ctx.assets);
        } else {
            walk_left(item, ctx.assets);
        }
    }
    false
}

/// itOldkuri_UnkMotion2_Phys (802D78B0): as the left walk's, but the walk
/// speed (xDF4) is written back to X before it follows the floor.
fn walk_right_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if cycled(item) && item.push_contacts & PUSHED_BY_ITEM != 0 {
        walk_left(item, ctx.assets);
    }
    item.velocity.x = walk_speed(item);
    item.velocity = zako::along_floor(item.velocity, floor_normal(item), item.facing);
    zako::lean_with_floor(item, CAMERA_FACING, LEAN_FRAMES);
}

/// itOldkuri_UnkMotion2_Coll (802D7988): a left wall (8) turns a cycled
/// walk left.
fn walk_right_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        fall_off(item, ctx.assets);
    }
    if item.wall_bits() == 8 && cycled(item) {
        walk_left(item, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion3_Anim (802D7A44): the fall's cycle restarts, the
/// hitboxes kept.
fn fall_left_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change_stompable(item, motion::FALL_LEFT, RESTART, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion4_Anim (802D7B50).
fn fall_right_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change_stompable(item, motion::FALL_RIGHT, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion3_Phys / 4_Phys / 6_Phys / 8_Phys: it_80272860.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itOldkuri_UnkMotion3_Coll / 4_Coll (802D7A8C): the stand's countdown
/// is xC (fctiwz), then it_8026E414 stands it on a landing (it_802D747C).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    set_stand_timer(item, fctiwz(attributes(ctx.assets).stand_frames));
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        stand(item, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion5_Anim (802D7C58): the carried cycle restarts
/// (it_80275474, it_80274740 first; flags 2).
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        carried(item, ctx.assets);
    }
    false
}

/// itOldKuri_Logic0_PickedUp (802D7C14): no hits on its owner or kindred
/// items (it_80275474), the spin stopped (it_80274740), carried.
pub(crate) fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    carried(item, ctx.assets);
}

fn carried(item: &mut ItemCore, assets: &ItemAssets) {
    item.hits_owner = false;
    item.strikes_kindred_items = false;
    item.reset_spin();
    change(item, motion::HELD, ANIM_UPDATE, assets);
}

/// it_2725_Logic0_Dropped (802D7CB0): it_8027CBA4 (the holder's facing,
/// airborne, the holder's hits land on it (it_802754D4), not grabbable),
/// then the fall the way it faces, the hitboxes kept.
pub(crate) fn dropped(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    item.facing = ctx.owner.expect("the dropping holder").facing;
    item.enter_air();
    item.hurt_by_owner = true;
    item.grabbable = false;
    let motion = if item.facing > 0.0 {
        motion::FALL_RIGHT
    } else {
        motion::FALL_LEFT
    };
    change_stompable(item, motion, RESTART, ctx.assets);
}

/// it_2725_Logic0_Thrown (802D7D1C): it_8027CB3C (the thrower's facing,
/// striking kindred items but not its owner, airborne, not grabbable),
/// not stompable (it_80274C88), the countdown cleared, thrown, the model
/// upright facing the camera (it_8027C56C(gobj, 0)).
pub(crate) fn thrown(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    item.facing = ctx.owner.expect("the throwing holder").facing;
    item.strikes_kindred_items = true;
    item.hits_owner = false;
    item.strikes_kindred_items = false;
    item.enter_air();
    item.grabbable = false;
    item.stompable = false;
    set_stand_timer(item, 0);
    change(item, motion::THROWN, ANIM_UPDATE, ctx.assets);
    zako::face(item, CAMERA_FACING);
}

/// itOldkuri_UnkMotion6_Anim (802D7D7C).
fn thrown_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::THROWN, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion6_Coll -> it_8027C824(gobj, NULL): the first surface
/// it meets (it_8026DAA8) ends it, after it_8027CC88's 1P-mode drop (none
/// here) and the zako generator's notice (it_8027CE44).
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) & 0xF != 0
}

/// itOldkuri_UnkMotion7_Anim (802D8180): flattened until the animation
/// ends (it_8027CC88 drops nothing outside 1P mode).
fn flattened_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.article_animation_ended(ctx.assets)
}

/// itOldkuri_UnkMotion8_Anim (802D8244).
fn stomped_air_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.article_animation_ended(ctx.assets)
}

/// itOldkuri_UnkMotion8_Coll -> it_8026DFB0 (8026DFB0): an airborne pass,
/// then the wall and ceiling bits; any contact ends it.
fn stomped_air_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let floor = u32::from(item.air_pass(ctx.map));
    let walls = item.wall_bits();
    let ceiling = item.ceiling_bits();
    (floor | walls | ceiling) & 0xF != 0
}

/// itOldkuri_UnkMotion9_Anim (802D7E04): at the reel's end, airborne it
/// reels on; on the floor it stands (it_802D747C's body).
fn reel_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        if item.ground_or_air == GroundOrAir::Air {
            change(item, motion::REEL, ANIM_UPDATE, ctx.assets);
        } else {
            stand(item, ctx.assets);
        }
    }
    false
}

/// itOldkuri_UnkMotion9_Phys (802D7E88): gravity in the air, unlimited.
fn reel_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.ground_or_air == GroundOrAir::Air {
        item.velocity.y -= ctx.assets.fall_acceleration;
    }
}

/// itOldkuri_UnkMotion9_Coll (802D7EB4): in the air it_8026E414 stands it
/// on a landing; on the floor it_8026D62C lets it fall off.
fn reel_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ground_or_air == GroundOrAir::Air {
        if item.airborne_collision(ctx.map, ctx.assets).floor {
            stand(item, ctx.assets);
        }
    } else if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        fall_off(item, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion10_Anim (802D8314).
fn tumble_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::TUMBLE, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion10_Coll -> it_8027C79C: each surface cries out and
/// bounces it on; past it_804D6D40 +0 bounces it is gone.
fn tumble_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    zako::tumble_bounce(item, ctx)
}

/// itOldkuri_UnkMotion11_Anim (802D840C).
fn screen_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::SCREEN, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itOldkuri_UnkMotion11_Phys (802D8458): gravity, unlimited.
fn screen_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.y -= ctx.assets.fall_acceleration;
}

fn floor_normal(item: &ItemCore) -> Vec3 {
    item.collision
        .as_ref()
        .expect("item map collision")
        .floor
        .normal
}

// --- Events ----------------------------------------------------------------

/// it_2725_Logic0_DmgReceived (802D7F38): the element's extra damage
/// (it_8027CBFC) adds to xC9C (int to float, fadds, fctiwz); beyond
/// x0->x0, or thrown, it dies; otherwise it reels (the inlined
/// it_8027B798 launch: a launch off the floor rises at least 0.2).
pub(crate) fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    let a = attributes(context.assets);
    let common = common(context.assets);
    let extra = zako::element_damage(item, &common);
    item.damage_percent = fctiwz(item.damage_percent as f32 + extra);
    if item.damage_percent > a.health || item.motion == motion::THROWN {
        zako::hurt_voice(item, context.rng.expect("it_8027C9D8 draws"));
        item.stompable = false;
        item.hurt_intangible = true;
        item.hits_owner = false;
        item.strikes_kindred_items = false;
        generator_notice(item);
        quake(item);
        if draw_randf(context) < common.screen_chance {
            start_screen_flight(item, context);
        } else {
            start_tumble(item, context, &common);
        }
        return false;
    }
    let (velocity, leaves_floor) = item.knockback_launch(&context.launch);
    item.velocity = velocity;
    if leaves_floor {
        if f64::from(item.velocity.y) <= REEL_LIFT {
            item.velocity.y = REEL_LIFT as f32;
        }
        item.enter_air();
    } else {
        item.land_on_floor();
    }
    set_stand_timer(item, 0);
    change(item, motion::REEL, ANIM_UPDATE, context.assets);
    false
}

/// it_802D8098 (802D8098), Item.jumped_on: a fighter stomped it. No more
/// hurt capsules (it_802756D0), not stompable (it_80274C88), no hits
/// (it_80275474), the zako generator's notice and a small quake; in the
/// air it is knocked down (it_802D81FC: X speed times x8), on the floor
/// flattened (it_802D813C: stopped, leaning flat at once, flags 2). Then
/// the stomp's sound; the stomper's rumble (ftLib_80086D40) is not game
/// state.
fn stomped(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    item.hurt_intangible = true;
    item.stompable = false;
    item.hits_owner = false;
    item.strikes_kindred_items = false;
    generator_notice(item);
    quake(item);
    if item.ground_or_air == GroundOrAir::Air {
        // retail: fmuls.
        item.velocity.x *= attributes(context.assets).stomp_air_speed_scale;
        change(item, motion::STOMPED_AIR, ANIM_UPDATE, context.assets);
    } else {
        item.velocity = Vec3::ZERO;
        zako::lean_with_floor(item, CAMERA_FACING, FLAT_LEAN_FRAMES);
        change(item, motion::FLATTENED, ANIM_UPDATE, context.assets);
    }
    item.sound_requests.push(STOMP_SOUND);
    false
}

/// it_802D82C4 (802D82C4): airborne, launched along the hit
/// (it_8027B964(gobj, false)), its bounces counted from zero (xDEC), the
/// model upright facing the camera.
fn start_tumble(item: &mut ItemCore, context: &ItemEventContext<'_>, common: &MonsterCommonData) {
    item.enter_air();
    zako::launch(item, common, &context.launch);
    monster_mut(item).bounces = 0;
    zako::face(item, CAMERA_FACING);
    change(item, motion::TUMBLE, ANIM_UPDATE, context.assets);
}

/// it_802D839C (802D839C): airborne, flying at the camera (it_8027BA54),
/// the model upright facing the camera.
fn start_screen_flight(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    item.enter_air();
    zako::fly_at_screen(item, context);
    item.enter_air();
    zako::face(item, CAMERA_FACING);
    change(item, motion::SCREEN, ANIM_UPDATE, context.assets);
}

/// it_8027CE44 -> grZakoGenerator_801CACB8: a crate's enemy has no slot.
fn generator_notice(item: &ItemCore) {
    assert_eq!(
        monster(item).joint_index,
        -1,
        "it_8027CE44: a generator's enemy"
    );
}

/// Camera_RequestQuake(QuakeKind_Small, &ip->pos).
fn quake(item: &mut ItemCore) {
    item.events.push(melee_it::ItemEvent::Quake {
        kind: SMALL_QUAKE,
        joint: 0,
        offset: Vec3::ZERO,
    });
}

fn draw_randf(context: &ItemEventContext<'_>) -> f32 {
    let cell = context.rng.expect("it_2725_Logic0_DmgReceived draws");
    let mut rng = cell.get();
    let value = rng.randf();
    cell.set(rng);
    value
}
