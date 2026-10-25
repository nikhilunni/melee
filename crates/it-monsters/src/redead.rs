//! The ReDead (It_Kind_Leadead), itleadead.c (802E8BCC..802EAF28): out of
//! a crate it falls, settles and walks, turning toward the nearest fighter
//! and at edges and walls; now and then it lunges, and a fighter its lunge
//! catches is held (the fighter's CaptureLeadead) until the fighter's mash
//! timer lets it go or the ReDead is hit. A light hit makes it reel, a
//! heavy one knocks it down (or staggers it with no room to fall), and a
//! killing one sends it tumbling or flying at the screen. A fighter's grab
//! may catch it and carry or throw it (states 13, 14).
//!
//! Unported rows (0, the generator's spawn; 17) refuse with their state.
use crate::zako::{self, monster, monster_mut};
use gekko_math::{
    fma::{fmadds, fnmsubs},
    msl::fctiwz,
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, MonsterCommonData, ReDeadAttributes},
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    CaptiveView, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext,
    ItemPhysicsContext, ItemStateRow, LockOnFighter, LockOnTargets, MonsterBehavior,
    ReDeadCaptive, ReDeadState,
};
use melee_types::GroundOrAir;

/// it_803F8EB0's anim_id column.
pub const ARTICLE_STATES: [i32; 18] = [0, -1, 1, 2, 3, 4, 5, 6, -1, 7, 1, -1, -1, 8, 9, -1, -1, -1];

/// Item_80268E5C's ITEM_UNK_0x1 (it/forward.h), which a hit's reel passes.
const UNK_0X1: u32 = 1;

pub(crate) mod motion {
    /// it_802E8F24: standing still, about to walk.
    pub const SETTLE: u16 = 1;
    pub const WALK: u16 = 2;
    /// it_802E9BA0: turning round.
    pub const TURN: u16 = 3;
    /// it_802E9D50: the lunge whose hitbox catches a fighter.
    pub const LUNGE: u16 = 4;
    /// it_802EAAEC: clinging to the fighter it caught.
    pub const CLING: u16 = 5;
    /// it_802EAE80: let go when the fighter's timer ran out.
    pub const LET_GO: u16 = 6;
    /// it_802E9738: knocked over by a heavy hit.
    pub const KNOCKDOWN: u16 = 7;
    /// it_802E98E0: lying down.
    pub const DOWN: u16 = 8;
    /// it_802E9A00: getting up.
    pub const RISE: u16 = 9;
    /// it_802E9308.
    pub const FALL: u16 = 10;
    /// it_802E93C8: knocked back by a light hit.
    pub const REEL: u16 = 11;
    /// it_802E9494: knocked back while lying down.
    pub const DOWN_REEL: u16 = 12;
    /// itLeadead_Logic1_PickedUp: carried by a fighter.
    pub const HELD: u16 = 13;
    /// itLeadead_Logic1_Thrown: thrown by a fighter.
    pub const THROWN: u16 = 14;
    /// it_802EA2A0: killed, tumbling away.
    pub const TUMBLE: u16 = 15;
    /// it_802EA334: killed, flying at the screen.
    pub const SCREEN: u16 = 16;
}

/// it_802E8CD8's it_802EA804(gobj, 15.0): the floor a heavy hit needs
/// ahead to knock it down.
const KNOCKDOWN_ROOM: f32 = 15.0;
/// itLeadead_UnkMotion2_Phys's walk timer while turning (it_802E9BA0).
const TURN_FRAMES: i32 = 0x8C;
/// itLeadead_UnkMotion4_Phys: the lunge moves from this frame...
const LUNGE_START: i32 = 0x24;
/// ...and stops after this one.
const LUNGE_END: i32 = 0x71;
/// it_802EA6F4: how far above or below a fighter may stand for a lunge.
const LUNGE_HEIGHT: f32 = 5.0;
/// it_802EAAEC: joint 1 holds the body this far in front of the fighter.
const CLING_DEPTH: f32 = -12.0;
/// The model joint that carries the body (it_80272CC0(gobj, 1)).
const BODY_JOINT: usize = 1;
/// it_802E93C8: the least upward speed of a reel that leaves the floor.
const REEL_LIFT: f32 = 0.2;
/// Camera_RequestQuake(QuakeKind_Small) (retail 802E8DCC: li r3, 2).
const SMALL_QUAKE: u16 = 2;
/// itLeadead_UnkMotion7_Phys / 8_Phys: frames to lean with the floor.
const DOWN_LEAN_FRAMES: f32 = 5.0;
/// itLeadead_UnkMotion9_Anim: the settle's timer after getting up.
const RISE_SETTLE_FRAMES: i32 = 0xA;
/// itLeadead_UnkMotion1_Phys: frames to lean into the floor while settling.
const SETTLE_LEAN_FRAMES: f32 = 2.0;
/// Item_8026AE84(ip, 0x135, 0x7F, 0x40): the catch's sound.
const CATCH_SOUND: u32 = 0x135;

pub(crate) static STATES: [ItemStateRow; 18] = {
    const UNPORTED: ItemStateRow = ItemStateRow {
        animation_id: -1,
        animation: unported_animation,
        physics: unported_physics,
        collision: unported_collision,
    };
    let mut rows = [UNPORTED; 18];
    let mut i = 0;
    while i < rows.len() {
        rows[i].animation_id = ARTICLE_STATES[i];
        i += 1;
    }
    rows[motion::SETTLE as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: settle_physics,
        collision: grounded_collision,
    };
    rows[motion::WALK as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: walk_animation,
        physics: walk_physics,
        collision: walk_collision,
    };
    rows[motion::TURN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: turn_animation,
        physics: no_physics,
        collision: grounded_collision,
    };
    rows[motion::LUNGE as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: no_animation,
        physics: lunge_physics,
        collision: lunge_collision,
    };
    rows[motion::CLING as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: cling_animation,
        physics: no_physics,
        collision: no_collision,
    };
    rows[motion::LET_GO as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[6],
        animation: let_go_animation,
        physics: no_physics,
        collision: grounded_collision,
    };
    rows[motion::FALL as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[10],
        animation: no_animation,
        physics: fall_physics,
        collision: fall_collision,
    };
    rows[motion::KNOCKDOWN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[7],
        animation: knockdown_animation,
        physics: down_physics,
        collision: down_collision,
    };
    rows[motion::DOWN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[8],
        animation: no_animation,
        physics: lying_physics,
        collision: down_collision,
    };
    rows[motion::RISE as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[9],
        animation: rise_animation,
        physics: no_physics,
        collision: down_collision,
    };
    rows[motion::DOWN_REEL as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[12],
        animation: down_reel_animation,
        physics: reel_physics,
        collision: reel_collision,
    };
    rows[motion::TUMBLE as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[15],
        animation: no_animation,
        physics: no_physics,
        collision: tumble_collision,
    };
    // it_803F8EB0[13]: no collision callback.
    rows[motion::HELD as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[13],
        animation: held_animation,
        physics: no_physics,
        collision: no_collision,
    };
    rows[motion::THROWN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[14],
        animation: thrown_animation,
        physics: fall_physics,
        collision: thrown_collision,
    };
    // itLeadead_UnkMotion16_Coll -> it_8027C794: no map pass.
    rows[motion::SCREEN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[16],
        animation: no_animation,
        physics: screen_physics,
        collision: no_collision,
    };
    // it_803F8EB0[11]: UnkMotion11_Anim with UnkMotion12's Phys and Coll.
    rows[motion::REEL as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[11],
        animation: reel_animation,
        physics: reel_physics,
        collision: reel_collision,
    };
    rows
};

fn attributes(assets: &ItemAssets) -> ReDeadAttributes {
    assets.redead.expect("typed ReDead attributes")
}
pub(crate) fn redead(item: &ItemCore) -> &ReDeadState {
    match &monster(item).behavior {
        MonsterBehavior::ReDead(state) => state,
        _ => panic!("ReDead scratch"),
    }
}
pub(crate) fn redead_mut(item: &mut ItemCore) -> &mut ReDeadState {
    match &mut monster_mut(item).behavior {
        MonsterBehavior::ReDead(state) => state,
        _ => panic!("ReDead scratch"),
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[usize::from(motion)], flags, assets);
}

fn collision_facing(facing: f32) -> i32 {
    if facing == -1.0 {
        -1
    } else {
        1
    }
}

/// it_802EA478 (802EA478): the ECB box for a pose: the birth box (xE50) or
/// the box in use, reshaped for the knockdown's frames.
pub(crate) fn set_box(item: &mut ItemCore, pose: u8, variant: u8) {
    let mut b = if variant == 0 {
        redead(item).collision_box
    } else {
        item.collision_box
    };
    match (pose, variant) {
        (1, 1) => b.top -= 0.3,
        (1, 2) => {
            b.top -= 0.1;
            b.right += 0.1;
        }
        (1, 3) => {
            b.top -= 0.4;
            b.right += 0.5;
        }
        (2, _) => {
            b.right = 0.9 * b.top;
            b.top = 3.0;
            b.left *= 2.0;
        }
        (3, 1) => {
            b.top += 0.1;
            b.right -= 0.05;
            b.left = 0.0;
        }
        (3, 2) => b.top += 0.1,
        (3, 3) => {
            b.top += 0.2;
            b.right -= 0.1;
        }
        (3, 4) => {
            b.left = redead(item).collision_box.left;
            b.right -= 0.1;
        }
        _ => {}
    }
    item.set_collision_box(b);
    item.set_collision_facing(collision_facing(item.facing));
}

// --- State entries -------------------------------------------------------

/// it_802E8F24 (802E8F24): stopped where it stands (it_8027CAD8), pushable
/// (xDC8 x1A), about to walk.
pub(crate) fn settle(item: &mut ItemCore, assets: &ItemAssets) {
    zako::halt(item);
    item.pushable = true;
    change(item, motion::SETTLE, ANIM_UPDATE, assets);
}

/// it_802E9038 (802E9038) and its inlined copies: walking along its facing
/// at x0->x4, upright, tangible again (it_802756E0), with the walk timer
/// started if it had run out.
fn start_walk(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    // retail: fmuls.
    item.velocity = Vec3::new(item.facing * a.walk_speed, 0.0, 0.0);
    zako::face(item, item.facing);
    item.hurt_intangible = false;
    let state = redead_mut(item);
    state.frames = 0;
    if state.timer == 0 {
        state.timer = a.walk_frames;
    }
    set_box(item, 0, 0);
    change(item, motion::WALK, ANIM_UPDATE, assets);
}

/// it_802E9308 (802E9308): falling, keeping xC of its horizontal speed.
pub(crate) fn start_fall(item: &mut ItemCore, assets: &ItemAssets) {
    redead_mut(item).timer = 0;
    item.velocity.x *= attributes(assets).fall_speed_scale;
    change(item, motion::FALL, ANIM_UPDATE, assets);
}

/// it_802E9BA0 (802E9BA0): stopped, turning round.
fn start_turn(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    redead_mut(item).timer = TURN_FRAMES;
    change(item, motion::TURN, ANIM_UPDATE, assets);
}

/// it_802E9D50 (802E9D50): stopped, the lunge's count from zero.
fn start_lunge(item: &mut ItemCore, assets: &ItemAssets) {
    redead_mut(item).timer = 0;
    item.velocity = Vec3::ZERO;
    change(item, motion::LUNGE, ANIM_UPDATE, assets);
}

/// it_802E93C8 (802E93C8): knocked back by a light hit (it_8027B798). A
/// launch off the floor rises at least 0.2; otherwise it counts as
/// grounded, whatever it was. It faces as before the hit.
fn start_reel(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    let a = attributes(context.assets);
    let (velocity, leaves_floor) = item.knockback_launch(&context.launch);
    item.velocity = velocity;
    if leaves_floor {
        if item.velocity.y <= REEL_LIFT {
            item.velocity.y = REEL_LIFT;
        }
        item.enter_air();
    } else {
        item.land_on_floor();
    }
    item.facing = redead(item).facing_before_hit;
    item.set_collision_facing(collision_facing(item.facing));
    let state = redead_mut(item);
    state.timer = a.reel_frames;
    state.cooldown = a.lunge_cooldown;
    change(item, motion::REEL, UNK_0X1, context.assets);
}

/// it_802E9738 (802E9738): stopped, knocked over; the staggers start over.
fn start_knockdown(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    let state = redead_mut(item);
    state.frames = 0;
    state.staggers = 0;
    change(item, motion::KNOCKDOWN, ANIM_UPDATE, assets);
}

/// it_802E98E0 (802E98E0): lying still, no longer pushable (xDC8 x1A),
/// for its damage (xC9C) times x1A frames (mullw).
fn start_lying(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.pushable = false;
    let frames = item
        .damage_percent
        .wrapping_mul(attributes(assets).stun_per_damage);
    let state = redead_mut(item);
    state.timer = frames;
    state.frames = 0;
    change(item, motion::DOWN, ANIM_UPDATE, assets);
}

/// it_802E9A00 (802E9A00): getting up.
fn start_rise(item: &mut ItemCore, assets: &ItemAssets) {
    let state = redead_mut(item);
    state.timer = 0;
    state.frames = 0;
    change(item, motion::RISE, ANIM_UPDATE, assets);
}

/// it_802E9494 (802E9494): hit while lying, knocked back at three tenths
/// of the launch's rise (no less than just above the fall acceleration,
/// fadds) and none of its sideways speed, still lying (state 12).
fn start_down_reel(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    let a = attributes(context.assets);
    let (velocity, leaves_floor) = item.knockback_launch(&context.launch);
    item.velocity = velocity;
    if leaves_floor {
        if item.velocity.y <= REEL_LIFT {
            item.velocity.y = REEL_LIFT;
        }
        item.enter_air();
    } else {
        item.land_on_floor();
    }
    item.velocity.x = 0.0;
    // retail: fmuls.
    item.velocity.y *= 0.3;
    let fall = context.assets.fall_acceleration;
    if item.velocity.y <= fall {
        item.velocity.y = fall + 0.1;
    }
    item.facing = redead(item).facing_before_hit;
    item.set_collision_facing(collision_facing(item.facing));
    let state = redead_mut(item);
    state.frames = 1;
    state.timer = a.reel_frames;
    state.cooldown = a.lunge_cooldown;
    change(item, motion::DOWN_REEL, UNK_0X1, context.assets);
}

// --- Callbacks -------------------------------------------------------------

fn no_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {}
fn no_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_8026D62C(gobj, it_802E9308): off the floor it falls. The callers'
/// it_8027C794 returns false.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_fall(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion1_Phys (802E8FCC): leaning into the floor while the
/// timer runs, then walking.
fn settle_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if redead(item).timer == 0 {
        zako::face(item, item.facing);
        start_walk(item, ctx.assets);
        return;
    }
    zako::lean_toward(item, None, item.facing, SETTLE_LEAN_FRAMES);
    redead_mut(item).timer -= 1;
}

/// itLeadead_UnkMotion2_Anim (802E91A0): the walk cycle restarts.
fn walk_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        start_walk(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion2_Phys (802E9208): with the timer run out it turns
/// toward a fighter behind it, else restarts the timer; while it runs, a
/// fighter in reach ahead may draw a lunge. A push against its facing turns
/// it; the speed follows the floor.
fn walk_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let behind = fighter_behind(item, ctx.targets);
    let timer = redead(item).timer;
    if timer == 0 {
        if behind {
            start_turn(item, ctx.assets);
        } else {
            redead_mut(item).timer = attributes(ctx.assets).walk_frames;
        }
    } else {
        redead_mut(item).timer = timer - 1;
        if !behind && lunge_target(item, ctx) {
            start_lunge(item, ctx.assets);
        }
    }
    redead_mut(item).frames += 1;
    let facing = item.facing;
    if (facing > 0.0 && item.nudge.x < 0.0) || (facing < 0.0 && item.nudge.x > 0.0) {
        start_turn(item, ctx.assets);
    }
    let normal = floor_normal(item);
    item.velocity = zako::along_floor(item.velocity, normal, item.facing);
}

/// itLeadead_UnkMotion2_Coll (802E9294): it_8026D8A4 turns at an edge; on
/// the floor a wall turns it, off it it falls.
fn walk_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let (grounded, edge) = item.walk_ground_pass(ctx.map);
    if edge {
        start_turn(item, ctx.assets);
    }
    if grounded {
        if item.wall_bits() != 0 {
            start_turn(item, ctx.assets);
        }
    } else {
        start_fall(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion3_Anim (802E9AD8): the about-face when the timer
/// reaches zero, then walking again at the animation's end with three walk
/// timers.
fn turn_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if redead(item).timer == 0 {
        item.facing = -item.facing;
        item.set_collision_facing(collision_facing(item.facing));
    }
    redead_mut(item).timer -= 1;
    if item.article_animation_ended(ctx.assets) {
        redead_mut(item).timer = attributes(ctx.assets).walk_frames * 3;
        start_walk(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion4_Phys (802E9DA0): the lunge's count; it moves at
/// x14 between frames 0x25 and 0x71.
fn lunge_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = attributes(ctx.assets);
    let timer = redead(item).timer;
    if timer > LUNGE_START {
        // retail: fmuls.
        item.velocity.x = a.lunge_speed * item.facing;
    }
    if timer > LUNGE_END {
        item.velocity.x = 0.0;
    }
    redead_mut(item).timer = timer + 1;
}

/// itLeadead_UnkMotion4_Coll (802E9DF0): off the floor it falls; at the
/// animation's end a wall turns it, else it walks again (the timer 1). The
/// speed follows the floor.
fn lunge_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_fall(item, ctx.assets);
    }
    if item.article_animation_ended(ctx.assets) {
        if item.wall_bits() != 0 {
            start_turn(item, ctx.assets);
        } else {
            redead_mut(item).timer = 1;
            start_walk(item, ctx.assets);
        }
    }
    let normal = floor_normal(item);
    item.velocity = zako::along_floor(item.velocity, normal, item.facing);
    false
}

/// itLeadead_UnkMotion5_Anim (802E9F7C): the hold's cycle restarts with its
/// hitbox kept, the body again in front of the fighter.
fn cling_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::CLING, ANIM_UPDATE | HIT_PRESERVE, ctx.assets);
        item.joint_translation_z = Some((BODY_JOINT, redead(item).cling_depth));
    }
    false
}

/// itLeadead_UnkMotion6_Anim (802EA024): when letting go has played, the
/// lunge waits x1C frames and it settles.
fn let_go_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        let a = attributes(ctx.assets);
        let state = redead_mut(item);
        state.cooldown = a.lunge_cooldown;
        state.timer = 0;
        settle(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion10_Phys (802E935C): gravity (it_80272860).
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itLeadead_UnkMotion10_Coll (802E938C): it_8026E414 settles it on a
/// landing (it_802E8F24).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        settle(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion11_Anim (802E95E8): once the reel's timer has run
/// out, on the floor it settles.
fn reel_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let timer = redead(item).timer;
    if timer == 0 {
        if item.ground_or_air == GroundOrAir::Ground {
            settle(item, ctx.assets);
        }
    } else if timer > 0 {
        redead_mut(item).timer = timer - 1;
    }
    false
}

/// itLeadead_UnkMotion12_Phys (802E96B4): gravity in the air, unlimited.
fn reel_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.ground_or_air == GroundOrAir::Air {
        item.velocity.y -= ctx.assets.fall_acceleration;
    }
}

/// itLeadead_UnkMotion12_Coll (802E96D8): in the air it_8026DB40; a wall
/// bounces it (it_8027781C), a floor stops it there. On the floor
/// it_8026D82C, which lets it fall off.
fn reel_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ground_or_air == GroundOrAir::Air {
        let contact = u32::from(item.air_pass(ctx.map)) | item.wall_bits();
        if contact & 0xC != 0 {
            item.bounce_velocity(ctx.map, ctx.assets);
        }
        if contact & 1 != 0 {
            item.land_on_floor();
            item.velocity = Vec3::ZERO;
        }
    } else if !item.ground_pass(ctx.map) {
        item.enter_air();
    }
    false
}

/// itLeadead_UnkMotion7_Anim (802E9768): the box sinks as it topples;
/// at frame 0x51 it lies down (it_802E98E0, whose zero the count then
/// steps past).
fn knockdown_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let frames = redead(item).frames;
    match frames {
        ..0x1B => set_box(item, 1, 0),
        0x1B..0x2A => set_box(item, 1, 1),
        0x2A..0x42 => set_box(item, 1, 2),
        0x42..0x51 => set_box(item, 1, 3),
        _ => {
            set_box(item, 2, 0);
            start_lying(item, ctx.assets);
        }
    }
    redead_mut(item).frames += 1;
    false
}

/// itLeadead_UnkMotion7_Phys (802E9898): the speed follows the floor and
/// the model leans with it over five frames (it_8027C0A8).
fn down_physics(item: &mut ItemCore, _: &ItemPhysicsContext<'_>) {
    let normal = floor_normal(item);
    item.velocity = zako::along_floor(item.velocity, normal, item.facing);
    zako::lean_with_floor(item, item.facing, DOWN_LEAN_FRAMES);
}

/// itLeadead_UnkMotion8_Phys (802E9968): the stun's count, then up.
fn lying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let timer = redead(item).timer;
    if timer == 0 {
        start_rise(item, ctx.assets);
    } else {
        redead_mut(item).timer = timer - 1;
    }
    down_physics(item, ctx);
}

/// itLeadead_UnkMotion7_Coll and its siblings: it_8026D6F4, whose lost
/// floor starts the fall (it_802E9308) without leaving the ground.
fn down_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.ground_pass(ctx.map) {
        start_fall(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion9_Anim (802E9A28): the box grows back as it rises;
/// at the animation's end it settles with the timer at ten.
fn rise_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let frames = redead(item).frames;
    match frames {
        ..0x2E => set_box(item, 2, 0),
        0x2E..0x57 => set_box(item, 3, 1),
        0x57..0x82 => set_box(item, 3, 2),
        0x82..0xAA => set_box(item, 3, 3),
        0xAA..0x105 => set_box(item, 3, 4),
        _ => set_box(item, 0, 0),
    }
    redead_mut(item).frames += 1;
    if item.article_animation_ended(ctx.assets) {
        redead_mut(item).timer = RISE_SETTLE_FRAMES;
        settle(item, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion12_Anim (802E9690): once the reel's timer has run
/// out, on the floor it lies down again.
fn down_reel_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let timer = redead(item).timer;
    if timer == 0 {
        if item.ground_or_air == GroundOrAir::Ground {
            start_lying(item, ctx.assets);
        }
    } else if timer > 0 {
        redead_mut(item).timer = timer - 1;
    }
    false
}

fn floor_normal(item: &ItemCore) -> Vec3 {
    item.collision
        .as_ref()
        .expect("item map collision")
        .floor
        .normal
}

/// itLeadead_UnkMotion13_Anim (802EA0FC): the carried cycle restarts.
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::HELD, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion14_Anim (802EA1C4): the thrown cycle restarts with
/// its hitbox kept (flags 0x12).
fn thrown_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::THROWN, ANIM_UPDATE | HIT_PRESERVE, ctx.assets);
    }
    false
}

/// itLeadead_UnkMotion14_Coll -> it_8027C824(gobj, NULL) (8027C824): the
/// first surface it meets (it_8026DAA8) ends it, after it_8027CC88's 1P-mode
/// drop (none here) and the zako generator's notice (it_8027CE44).
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) & 0xF != 0
}

/// itLeadead_Logic1_PickedUp (802EA0E0): carried (state 13).
pub(crate) fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    change(item, motion::HELD, ANIM_UPDATE, ctx.assets);
}

/// itLeadead_Logic1_Dropped (802EA140): it_8027CBA4 (the holder's
/// facing, airborne, the holder's hits land on it (it_802754D4), not
/// grabbable), then the fall (it_802E9308).
pub(crate) fn dropped(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    item.facing = ctx.owner.expect("the dropping holder").facing;
    item.enter_air();
    item.hurt_by_owner = true;
    item.grabbable = false;
    start_fall(item, ctx.assets);
}

/// itLeadead_Logic1_Thrown (802EA17C): it_8027CB3C (the thrower's player
/// as its source, the thrower's facing, striking kindred items but not its
/// owner (it_80275414, it_80275474), airborne, not grabbable), then
/// thrown (state 14) with its timer cleared.
pub(crate) fn thrown(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    item.facing = ctx.owner.expect("the throwing holder").facing;
    item.strikes_kindred_items = true;
    item.hits_owner = false;
    item.strikes_kindred_items = false;
    item.enter_air();
    item.grabbable = false;
    redead_mut(item).timer = 0;
    change(item, motion::THROWN, ANIM_UPDATE, ctx.assets);
}

// --- Fighter queries ------------------------------------------------------

/// ftLib_8008627C(&pos, NULL) (8008627C): the fighter whose camera point
/// (ftLib_800866DC) is nearest, in the list's order; out-of-play fighters
/// (x221F_b3) are skipped (retail 0x8008632C: fmadds, Y product first).
fn nearest_fighter(position: Vec3, targets: &LockOnTargets) -> Option<&LockOnFighter> {
    let mut nearest = None;
    let mut best = f32::MAX;
    for fighter in targets.fighters.iter() {
        if fighter.disabled {
            continue;
        }
        let dx = position.x - fighter.position.x;
        let dy = position.y - fighter.position.y;
        let distance = fmadds(dx, dx, dy * dy);
        if distance < best {
            best = distance;
            nearest = Some(fighter);
        }
    }
    nearest
}

/// it_802EA674 (802EA674): whether the nearest fighter stands behind it.
fn fighter_behind(item: &ItemCore, targets: &LockOnTargets) -> bool {
    nearest_fighter(item.position, targets).is_some_and(|fighter| {
        let toward = if item.position.x < fighter.cur_pos.x {
            1.0
        } else {
            -1.0
        };
        toward != item.facing
    })
}

/// it_802EA6F4 (802EA6F4): with the lunge's cooldown spent, one frame in
/// x1E (HSD_Randi) it looks at the nearest fighter: one not already held
/// by a ReDead or Like Like (it_8027CA7C), within x10 ahead or behind and
/// 5 above or below, draws a lunge. The cooldown counts down otherwise.
fn lunge_target(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) -> bool {
    let a = attributes(ctx.assets);
    if redead(item).cooldown != 0 {
        redead_mut(item).cooldown -= 1;
        return false;
    }
    let mut rng = ctx.rng.get();
    let draw = rng.randi(a.lunge_odds);
    ctx.rng.set(rng);
    if draw != 0 {
        return false;
    }
    let Some(fighter) = nearest_fighter(item.position, ctx.targets) else {
        return false;
    };
    if held_by_monster(fighter.motion) {
        return false;
    }
    let mut dx = fighter.cur_pos.x - item.position.x;
    if dx < 0.0 {
        dx = -dx;
    }
    if dx > a.lunge_reach {
        return false;
    }
    let mut dy = fighter.cur_pos.y - item.position.y;
    if dy < 0.0 {
        dy = -dy;
    }
    dy <= LUNGE_HEIGHT
}

/// it_8027CA7C (8027CA7C): CaptureLeadead (0x14D) or CaptureLikelike (0x14E).
fn held_by_monster(motion: u16) -> bool {
    matches!(motion, 0x14D | 0x14E)
}

// --- The hold -----------------------------------------------------------

/// it_802EAF28 (802EAF28): the fighter it already holds, which a hit of its
/// element on another fighter does not replace (ftCo_800C74F4).
pub fn holds_fighter(item: &ItemCore) -> bool {
    matches!(&item.scratch, melee_it::ItemScratch::Monster(m)
        if matches!(&m.behavior, MonsterBehavior::ReDead(s) if s.captive.is_some()))
}

/// xDD4 +4: the zako generator's slot, -1 for a crate's enemy.
pub fn joint_index(item: &ItemCore) -> i32 {
    monster(item).joint_index
}

/// it_802EAAEC (802EAAEC), from the caught fighter's ftCo_800C7590: stopped
/// and clinging, its body 12 in front of the fighter's TopN; the blast
/// zones no longer remove it (xDCC b3) and no grab may catch it (xDD0 b4,
/// xD09). It turns to face the fighter, settles once onto the floor
/// (it_8026D564) and its root follows the fighter's part (lb_8000C1C0).
pub fn cling(
    item: &mut ItemCore,
    captive: ReDeadCaptive,
    fighter_position: Vec3,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
) {
    redead_mut(item).cling_depth = CLING_DEPTH;
    item.blast_zone_checked = false;
    item.velocity = Vec3::ZERO;
    item.catchable = false;
    change(item, motion::CLING, ANIM_UPDATE, assets);
    item.joint_translation_z = Some((BODY_JOINT, CLING_DEPTH));
    item.catch_ignored = true;
    item.sound_requests.push(CATCH_SOUND);
    item.facing = if item.position.x < fighter_position.x {
        1.0
    } else {
        -1.0
    };
    item.set_collision_facing(collision_facing(item.facing));
    zako::face(item, item.facing);
    item.ground_pass(map);
    let state = redead_mut(item);
    state.captive = Some(captive);
}

/// The held fighter as the scene sees it now (see [`CaptiveView`]).
pub fn set_captive_view(item: &mut ItemCore, view: CaptiveView) {
    redead_mut(item).captive_view = Some(view);
}

/// it_802EAC8C (802EAC8C): the hold ends where the body was, 12 in front of
/// the fighter's part (lb_8000B1CC), set onto the fighter's floor or swept
/// out from the fighter's middle (it_802759DC); the lunge waits again.
fn unclasp(
    item: &mut ItemCore,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
    release_floor_depth: f32,
) {
    item.constrained_root = None;
    let view = redead(item)
        .captive_view
        .expect("it_802EAC8C: the held fighter's view");
    let offset = Vec3::new(0.0, 0.0, -redead(item).cling_depth);
    let mut position = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(&view.part, &offset, &mut position);
    item.position = position;
    place_after_release(item, &view, map, release_floor_depth);
    let a = attributes(assets);
    item.joint_translation_z = Some((BODY_JOINT, 0.0));
    let state = redead_mut(item);
    state.captive = None;
    state.captive_view = None;
    state.cooldown = a.lunge_cooldown;
    state.timer = 0;
    item.blast_zone_checked = true;
}

/// it_802759DC (802759DC): on the fighter's floor chain, a floor below the
/// item no more than ItCo +F0 under it sets it down there; otherwise an
/// airborne pass from the fighter's middle to the item decides whether it
/// stands.
fn place_after_release(
    item: &mut ItemCore,
    view: &CaptiveView,
    map: &mut melee_mp::CollMap,
    release_floor_depth: f32,
) {
    let target = item.position;
    if map.line_is_active(view.floor_line) {
        let floor = map.floor_below(&target, -1, -1);
        if floor != -1 && map.lines_connected(floor, view.floor_line) {
            item.collision.as_mut().expect("item map collision").floor.index = floor;
            let Some(probe) = map.floor_probe(floor, &target) else {
                unimplemented!(
                    "it_802759DC (itmaplib.c:60): mpLib_8004DD90_Floor leaves y_out unset"
                );
            };
            if probe.delta >= release_floor_depth {
                item.position = Vec3::new(target.x, target.y + probe.delta, target.z);
                let position = item.position;
                melee_mp::set_position(
                    item.collision.as_mut().expect("item map collision"),
                    &position,
                );
                item.land_on_floor();
                return;
            }
        }
    }
    // itmaplib.c:69-85.
    let start = Vec3::new(
        view.position.x + 0.0,
        view.position.y + view.ecb_middle,
        view.position.z + 0.0,
    );
    let collision = item.collision.as_mut().expect("item map collision");
    collision.last_pos = start;
    melee_mp::mark_ecb_clear(collision);
    collision.cur_pos = target;
    let landed = map.air_collide_pass(collision, None);
    if landed {
        item.land_on_floor();
        item.position = item.collision.as_ref().expect("item map collision").cur_pos;
    } else {
        item.enter_air();
        item.position = target;
    }
}

/// it_802EAE80 (802EAE80), from the fighter's ftCo_CaptureLeadead_Anim when
/// its timer runs out: let go, and on the floor play the release (state 6,
/// untouchable by a grab, it_80275258), else fall.
pub fn let_go(
    item: &mut ItemCore,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
    release_floor_depth: f32,
) {
    unclasp(item, map, assets, release_floor_depth);
    if item.ground_or_air == GroundOrAir::Ground {
        item.velocity = Vec3::ZERO;
        item.catchable = false;
        change(item, motion::LET_GO, ANIM_UPDATE, assets);
    } else {
        start_fall(item, assets);
    }
}

/// it_802E8CD8's first step (itleadead.c:173-176), which the scene runs
/// just before the item's damage callback: a ReDead holding a fighter lets
/// go (it_802EAC8C) and the fighter steps back (ftCo_800C7800, through
/// the owner request). True when it held one.
pub fn release_on_hit(
    item: &mut ItemCore,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
    release_floor_depth: f32,
) -> bool {
    let Some(captive) = redead(item).captive else {
        return false;
    };
    unclasp(item, map, assets, release_floor_depth);
    item.owner_request = Some((
        captive.player,
        melee_it::OwnerRequest::ReDeadLetGo {
            secondary: captive.secondary,
        },
    ));
    true
}

// --- Damage ---------------------------------------------------------------

/// it_802E8CD8 (802E8CD8): a hit frees a held fighter (ftCo_800C7800, the
/// fighter's CaptureCut) and restarts the lunge's cooldown. The element's
/// extra damage (it_8027CBFC) adds to xC9C; beyond x0->x0 it dies, beyond
/// x8 of that it may be knocked down; a lighter hit makes it reel.
pub(crate) fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    let a = attributes(context.assets);
    // itleadead.c:173-176: the scene let a held fighter go first
    // (`release_on_hit`), which needs the stage's map.
    assert!(
        redead(item).captive.is_none(),
        "it_802E8CD8: the hold ends before the hit's own work"
    );
    let facing = item.facing;
    let state = redead_mut(item);
    state.timer = 0;
    state.cooldown = a.lunge_cooldown;
    state.facing_before_hit = facing;
    if matches!(item.motion, motion::KNOCKDOWN | motion::DOWN_REEL) {
        return false;
    }
    let common = context
        .assets
        .monster_common
        .expect("it_804D6D40 for the monster bank");
    let extra = zako::element_damage(item, &common);
    // retail 802E8D60..90: int to float, fadds, fctiwz.
    item.damage_percent = fctiwz(item.damage_percent as f32 + extra);
    if item.damage_percent > a.health || item.motion == motion::THROWN {
        die(item, context, &common);
    } else if item.motion == motion::DOWN {
        start_down_reel(item, context);
    } else if item.damage_percent as f32 > a.health as f32 * a.stagger_share {
        // retail 802E8E30..60: both ints to float, fmuls, fcmpo.
        let map = context.map.expect("it_802EA804 reads the stage");
        if floor_ahead(item, map, KNOCKDOWN_ROOM) {
            start_knockdown(item, context.assets);
        } else if redead(item).staggers > 1 && draw_randi(context, 2) == 0 {
            // itleadead.c:205-209: a third stagger without room to fall
            // over ends it, at the screen or tumbling.
            if draw_randf(context) < common.screen_chance {
                start_screen_flight(item, context);
            } else {
                start_tumble(item, context, &common);
            }
        } else {
            item.velocity.x *= 1.5;
            redead_mut(item).staggers += 1;
            start_reel(item, context);
        }
    } else {
        start_reel(item, context);
    }
    false
}

/// it_802E8CD8's killing hit (itleadead.c:185-198): a hurt voice
/// (it_8027C9D8), no more hurt capsules (it_802756D0), the zako generator's
/// notice (it_8027CE44: a crate's enemy has no slot) and a small quake
/// at its position; then one time in it_804D6D40 +8 it flies at the screen
/// (it_802EA334), otherwise it tumbles (it_802EA2A0).
fn die(item: &mut ItemCore, context: &ItemEventContext<'_>, common: &MonsterCommonData) {
    zako::hurt_voice(item, context.rng.expect("it_8027C9D8 draws"));
    item.hurt_intangible = true;
    assert_eq!(monster(item).joint_index, -1, "it_8027CE44: a generator's enemy");
    item.events.push(melee_it::ItemEvent::Quake {
        kind: SMALL_QUAKE,
        joint: 0,
        offset: Vec3::ZERO,
    });
    if draw_randf(context) < common.screen_chance {
        start_screen_flight(item, context);
    } else {
        start_tumble(item, context, common);
    }
}

/// it_802EA2A0 (802EA2A0): airborne (it_802762BC), launched (it_8027B964),
/// facing as before the hit, its bounces counted from zero (xDEC).
fn start_tumble(item: &mut ItemCore, context: &ItemEventContext<'_>, common: &MonsterCommonData) {
    item.enter_air();
    zako::launch(item, common, &context.launch);
    item.facing = redead(item).facing_before_hit;
    monster_mut(item).bounces = 0;
    change(item, motion::TUMBLE, UNK_0X1 | ANIM_UPDATE, context.assets);
}

/// it_802EA334 (802EA334): airborne, flying at 10 toward the camera's
/// eye (it_8027BA54) jittered by up to 5 sideways (HSD_Randf in double:
/// retail 8027BA84..90 fsub, fmadd, frsp) and up to 30 up (fmadds).
fn start_screen_flight(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    item.enter_air();
    let mut target = context
        .camera_position
        .expect("it_8027BA54 reads the camera");
    let cell = context.rng.expect("it_8027BA54 draws");
    let mut rng = cell.get();
    target.x = gekko_math::fma::fmadd(10.0, f64::from(rng.randf()) - 0.5, f64::from(target.x)) as f32;
    target.y = gekko_math::fma::fmadds(30.0, rng.randf(), target.y);
    cell.set(rng);
    let toward = melee_lb::vector::normalize(melee_lb::vector::difference(target, item.position));
    item.velocity = Vec3::new(toward.x * 10.0, toward.y * 10.0, toward.z * 10.0);
    item.enter_air();
    change(item, motion::SCREEN, UNK_0X1 | ANIM_UPDATE, context.assets);
}

/// itLeadead_UnkMotion16_Phys (802EA394): gravity, unlimited.
fn screen_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.y -= ctx.assets.fall_acceleration;
}

/// itLeadead_UnkMotion15_Coll -> it_8027C79C (8027C79C): every surface it
/// meets (it_8026DAA8) cries out (it_8027C9D8) and bounces it on
/// (it_8027B964(gobj, 1)); past it_804D6D40 +0 bounces it is gone
/// (it_8027CC88 drops nothing outside 1P mode).
fn tumble_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_bits(ctx.map) & 0xF == 0 {
        return false;
    }
    let common = ctx
        .assets
        .monster_common
        .expect("it_804D6D40 for the monster bank");
    zako::hurt_voice(item, ctx.rng.expect("it_8027C9D8 draws"));
    item.bounce_velocity(ctx.map, ctx.assets);
    zako::scale_launch(item, &common);
    let state = monster_mut(item);
    state.bounces += 1;
    state.bounces > common.bounce_limit
}

fn draw_randf(context: &ItemEventContext<'_>) -> f32 {
    let cell = context.rng.expect("it_802E8CD8 draws");
    let mut rng = cell.get();
    let value = rng.randf();
    cell.set(rng);
    value
}

fn draw_randi(context: &ItemEventContext<'_>, range: i32) -> i32 {
    let cell = context.rng.expect("it_802E8CD8 draws");
    let mut rng = cell.get();
    let value = rng.randi(range);
    cell.set(rng);
    value
}

/// it_802EA804 (802EA804): whether the floor goes on `range` ahead
/// (mpLib_80056C54) without turning more than 30 degrees or stepping more
/// than 3 out of its line (retail 802EA898..904: fmuls, then the negated
/// dot products as fnmsubs chains); off a walkable floor, whether it touches
/// a wall (it_802EA988).
fn floor_ahead(item: &ItemCore, map: &melee_mp::CollMap, range: f32) -> bool {
    const DEGREES: f32 = 57.295_78;
    let line = item.collision.as_ref().expect("item map collision").floor.index;
    let position = item.position;
    let here = map.walk_along_floor(line, &position, 0.0, 0.0);
    let ahead = map.walk_along_floor(line, &position, item.facing * range, 0.0);
    let Some(ahead) = ahead.filter(|walk| walk.reached) else {
        return wall_contact(item) != 0;
    };
    let (Some((_, v1)), Some((_, v2))) = (here.and_then(|w| w.surface), ahead.surface) else {
        unimplemented!("it_802EA804: a floor walk that leaves its normal unwritten");
    };
    let v3 = ahead.pos;
    let here_angle = DEGREES * melee_lb::trigf::atan2f(v1.y, v1.x);
    let ahead_angle = DEGREES * melee_lb::trigf::atan2f(v2.y, v2.x);
    let here_line = fnmsubs(
        position.z,
        v1.z,
        fnmsubs(position.y, v1.y, -(position.x * v1.x)),
    );
    let ahead_line = fnmsubs(v3.z, v2.z, fnmsubs(v3.y, v2.y, -(v3.x * v2.x)));
    let mut turn = ahead_angle - here_angle;
    if turn < 0.0 {
        turn = -turn;
    }
    if turn > 30.0 {
        return false;
    }
    let mut step = here_line - ahead_line;
    if step < 0.0 {
        step = -step;
    }
    step <= 3.0
}

/// it_802EA988 (802EA988): 8 for a left wall, 4 for a right one (which
/// wins), from the item's last map pass. Retail offsets a copy of its
/// CollData's position and never runs a pass with it.
fn wall_contact(item: &ItemCore) -> u32 {
    use melee_types::mp::collide::{LEFT_WALL_MASK, RIGHT_WALL_MASK};
    let env = item.collision.as_ref().expect("item map collision").env_flags as u32;
    let mut bits = 0;
    if env & LEFT_WALL_MASK != 0 {
        bits = 8;
    }
    if env & RIGHT_WALL_MASK != 0 {
        bits = 4;
    }
    bits
}

fn unported_animation(item: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itleadead.c: motion state {} animation", item.motion)
}
fn unported_physics(item: &mut ItemCore, _: &ItemPhysicsContext<'_>) {
    unimplemented!("itleadead.c: motion state {} physics", item.motion)
}
fn unported_collision(item: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itleadead.c: motion state {} collision", item.motion)
}
