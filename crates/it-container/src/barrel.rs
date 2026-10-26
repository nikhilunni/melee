//! Barrel (It_Kind_Taru), ittaru.c (802874F0..80288C88): falls in and
//! rests; on a steep floor, thrown or set rolling it rolls (it_802874F0)
//! until it slows, its lifetime ends or it breaks: thrown or rolling fast
//! into a wall, landing hard, at its damage threshold or hitting anything
//! (efAsync 0x427's boards, then one item or a blast).
//!
//! The root's roll angle is kept as the model's rotation; the staves' child
//! joint (ItemModelDesc spin depth 1) is kept apart, since its rotation X
//! turns the push box (it_80271A58 via it_802746F8). A struck barrel set rolling
//! takes its striker's fighter owner (it_8027236C, it_8027B1F4).
use crate::{no_animation, no_collision, no_physics};
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE, HIT_PRESERVE},
    BarrelState, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::{GroundOrAir, ItemKind};

pub struct Barrel;

/// it_803F58E0's anim_id column.
pub const ARTICLE_STATES: [i32; 8] = [-1, -1, -1, 0, -1, 1, 2, 3];
/// itTaruAttributes: x0..x34.
pub const SPECIAL_ATTRIBUTES: u32 = 14;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    /// Thrown or dropped, rolling in the air (itTaru_UnkMotion3).
    pub const THROWN: u16 = 3;
    /// Rolling on the ground (itTaru_UnkMotion5).
    pub const ROLLING: u16 = 5;
    /// Broken empty: the explosion's article animation (it_802885C8).
    pub const BLAST: u16 = 6;
    /// Broken open: gone after its despawn timer (it_802886C4).
    pub const OPENED: u16 = 7;
}

/// ItemCommandFlags' ITEM_UNK_0x1 (it_80287F20's drop).
const UNK_0X1: u32 = 1;
/// The step a break resumes at once its boards are made.
const BREAK: u8 = 0;
/// efAsync 0x427, the boards (melee_ef::BOX_FRAGMENTS_REQUEST).
const BOARDS_EFFECT: u16 = 0x427;
/// efAsync 0x421 (it_802787B4): the end of its lifetime.
const LIFETIME_END_EFFECT: u16 = 0x421;
/// `xDD8 = 40` once broken.
const DESPAWN_FRAMES: i32 = 40;
/// Camera_RequestQuake(QuakeKind_Small, ...).
const SMALL_QUAKE: u16 = 2;
/// it_3F14_Logic2_Spawned: the staves' tilt, 75 degrees plus up to 20.
const TILT_BASE: f32 = 1.308_996_9;
const TILT_RANGE: f32 = 0.349_065_84;
/// it_802874F0: the roll per unit of speed on the ground, and in the air.
const GROUND_ROLL: f32 = 0.122_173_05;
const AIR_ROLL: f32 = 0.104_719_76;
/// it_802874F0: a floor whose normal leans more than this pulls.
const PULLING_LEAN: f32 = 0.1;
/// it_80288194's push down a steep floor at rest.
const STEEP_PUSH: f32 = 0.1;

/// itTaruAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0 / x4: the weights of an item and of the blast.
    fn item_weight(&self) -> i32 {
        self.0[0].to_bits() as i32
    }
    fn blast_weight(&self) -> i32 {
        self.0[1].to_bits() as i32
    }
    /// x8: one weak hit in this many sets it rolling (fctiwz'd).
    fn roll_odds(&self) -> f32 {
        self.0[2]
    }
    /// xC: the damage that breaks it.
    fn damage_threshold(&self) -> f32 {
        self.0[3]
    }
    /// x10: the roll speed per point of damage.
    fn hit_speed(&self) -> f32 {
        self.0[4]
    }
    /// x14: the steepest floor it rests on.
    fn resting_lean(&self) -> f32 {
        self.0[5]
    }
    /// x20: the fastest roll.
    /// x18: the staves' straightening, in calls (it_80287690's divisor).
    fn straightening_frames(&self) -> f32 {
        self.0[6]
    }
    fn top_speed(&self) -> f32 {
        self.0[8]
    }
    /// x24: a pulling floor's acceleration per unit of its normal's X.
    fn pull(&self) -> f32 {
        self.0[9]
    }
    /// x28: a level floor's braking.
    fn braking(&self) -> f32 {
        self.0[10]
    }
    /// x2C: a pulling floor's speed limit, and a wall breaks a roll faster.
    fn wall_break_speed(&self) -> f32 {
        self.0[11]
    }
    /// x30: what a wall leaves of the roll as it turns it back.
    fn wall_rebound(&self) -> f32 {
        self.0[12]
    }
    /// x34: a landing faster than this breaks it (thrown) or bounces it
    /// (falling).
    fn landing_break_speed(&self) -> f32 {
        self.0[13]
    }
}

static STATES: [ItemStateRow; 8] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: rest_animation,
        physics: no_physics,
        collision: rest_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: fall_physics,
        collision: fall_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: life_animation,
        physics: thrown_physics,
        collision: thrown_collision,
    },
    // State 4 (itTaru_UnkMotion4): nothing in ittaru.c enters it.
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: unreached_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: rolling_animation,
        physics: rolling_physics,
        collision: rolling_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[6],
        animation: crate::explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[7],
        animation: opened_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for Barrel {
    const KIND: ItemKind = ItemKind::Taru;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_3F14_Logic2_Spawned (80287458): facing left, never sliding (xD5C
    /// 0), the ECB upright (xDCE b7), the roll at rest on level ground,
    /// then the fall (it_80287D0C). The tilt's draw is in [`Self::launched`].
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.facing = -1.0;
        item.slide_mode = 0;
        item.ecb_follows_rotation = false;
        item.spin_joint_rotation = Some(authored_staves(assets));
        item.command_variables[0] = 0;
        item.command_variables[1] = 0;
        item.scratch = ItemScratch::Barrel(BarrelState {
            floor_normal: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        });
        start_falling(item, assets);
    }
    /// it_3F14_Logic2_Spawned's HSD_Randf, the last thing it does before
    /// the fall: nothing draws between it and Item_80268B18's return.
    /// Retail 802874AC / 802874B0: fmuls, then fadds.
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &melee_it::desc::ItemCommonData,
        _spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let tilt = TILT_BASE + TILT_RANGE * rng.randf();
        barrel_mut(item).tilt = tilt;
    }
    /// itTaru_Logic2_PickedUp: xDB0 (the staves' straightening, drawing
    /// only), state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.command_variables[1] = 1;
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itTaru_Logic2_Thrown: the model's authored rotation (lb_8000B804),
    /// not grabbable, state 3.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        reset_model_rotation(item, context.assets);
        item.grabbable = false;
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itTaru_Logic2_Dropped -> it_80287F20.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        drop_rolling(item, context.assets);
    }
    /// it_3F14_Logic2_DmgDealt: unbroken, it breaks.
    fn damage_dealt(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// it_3F14_Logic2_Clanked.
    fn clanked(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// it_3F14_Logic2_HitShield.
    fn hit_shield(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// it_3F14_Logic2_Reflected.
    fn reflected(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// it_3F14_Logic2_DmgReceived (80288AA4): unbroken, its damage
    /// threshold breaks it; a weaker hit at rest sets it rolling one time
    /// in x8, and a hit while rolling pushes the roll.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if barrel(item).opened {
            return false;
        }
        let a = Attributes(&context.assets.special_attributes);
        if item.damage_percent as f32 >= a.damage_threshold() {
            break_open(item);
            return false;
        }
        // xCA0 * x10: this frame's damage times the roll per point (retail
        // 80288BB4 / 80288C2C: fmuls).
        let push = item.pending_damage_taken as f32 * a.hit_speed();
        if item.motion == motion::REST {
            let odds = gekko_math::msl::fctiwz(a.roll_odds());
            let rolls = crate::with_event_rng(context, |rng| rng.randi(odds) == 0);
            if rolls {
                // Away from the hit (-xCCC), at the push (fmuls 80288BB8).
                item.facing = -item.hit_direction;
                barrel_mut(item).roll_speed = item.facing * push;
                take_striker_as_owner(item);
                start_rolling(item, context.assets);
            }
        } else if item.motion == motion::ROLLING {
            // retail 80288C30: fmadds(-xCCC, push, xDE4).
            let away = -item.hit_direction;
            let state = barrel_mut(item);
            state.roll_speed = gekko_math::fma::fmadds(away, push, state.roll_speed);
            take_striker_as_owner(item);
            item.hits_owner = true;
            item.strikes_kindred_items = true;
        }
        false
    }
    /// The break after the boards (it_3F14_Logic2_inline): HSD_Randi(x0 +
    /// x4) < x0 lets an item out (it_802886C4), otherwise the blast
    /// (it_802885C8).
    fn resumed(item: &mut ItemCore, step: u8, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) {
        assert_eq!(step, BREAK, "ittaru.c resumes only its break");
        let a = Attributes(&assets.special_attributes);
        let item_weight = a.item_weight();
        if rng.randi(item_weight + a.blast_weight()) < item_weight {
            open(item, assets);
        } else {
            blast(item, assets);
        }
    }
}

fn barrel(item: &ItemCore) -> &BarrelState {
    match &item.scratch {
        ItemScratch::Barrel(state) => state,
        _ => unreachable!("a barrel's scratch"),
    }
}
fn barrel_mut(item: &mut ItemCore) -> &mut BarrelState {
    match &mut item.scratch {
        ItemScratch::Barrel(state) => state,
        _ => unreachable!("a barrel's scratch"),
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_80287D0C (80287D0C): not grabbable, the fall.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    change(item, motion::FALL, ANIM_UPDATE, assets);
}

/// itResetVelocity, grabbable (it_8026B390), at rest.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::REST, ANIM_UPDATE, assets);
}

/// it_80287F20 (80287F20): not grabbable, state 3 keeping its hitboxes.
fn drop_rolling(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    change(item, motion::THROWN, HIT_PRESERVE | DROP_UPDATE | UNK_0X1, assets);
}

/// it_802881FC (802881FC): rolling at its roll speed, not grabbable,
/// state 5.
fn start_rolling(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity.x = barrel(item).roll_speed;
    item.velocity.y = 0.0;
    item.grabbable = false;
    change(item, motion::ROLLING, ANIM_UPDATE, assets);
}

/// The break (it_3F14_Logic2_inline): the boards, then [`Barrel::resumed`].
fn break_open(item: &mut ItemCore) {
    item.events.push(ItemEvent::Effect {
        id: BOARDS_EFFECT,
        position: item.position,
    });
    item.defer_callback(BREAK);
}

/// it_8027236C: xCEC names a fighter hit; xCF0 names the striking
/// article's fighter owner, not the article itself (80271160..80271194).
/// it_8027B1F4 resets attack ID to 1 and instance to 0 (8027B214..8027B224).
fn take_striker_as_owner(item: &mut ItemCore) {
    if item.struck_by_item {
        if item.motion != motion::REST || item.owner.is_some() {
            unimplemented!("it_8027236C: article ownership transfer to an owned or nonresting barrel (itcoll.c:1373)");
        }
        if item.hit_by.is_none() || item.hit_by_secondary {
            unimplemented!("it_8027236C: article without a primary fighter owner strikes a barrel (itcoll.c:1373)");
        }
        // An unowned resting barrel has no stale attack. Its new rolling
        // hitbox is authored in this callback, after the identity reset.
        assert!(item.stale_source.is_none());
        assert_eq!(item.stale_multiplier, 1.0);
    }
    item.owner = item.hit_by;
    item.owner_secondary = false;
    item.stale_source = None;
}

fn break_unless_opened(item: &mut ItemCore) {
    if !barrel(item).opened {
        break_open(item);
    }
}

/// it_802886C4 (802886C4): the sound, a small quake, one item (coins and
/// monsters aside), then hidden, taking no hits, still, no destroy effect,
/// broken with its despawn timer, not grabbable, state 7.
fn open(item: &mut ItemCore, assets: &ItemAssets) {
    item.events.push(ItemEvent::Quake {
        kind: SMALL_QUAKE,
        joint: 0,
        offset: Vec3::ZERO,
    });
    let ecb = item.collision.as_ref().expect("a barrel's collision").ecb;
    let rise = 0.5 * (ecb.top.y + ecb.bottom.y);
    item.events.push(ItemEvent::CrateContents {
        roll: None,
        position: Vec3::new(item.position.x, item.position.y + rise, item.position.z),
        previous_position: Vec3::new(item.position.x, item.position.y, 0.0),
    });
    item.hidden = true;
    item.hurt_intangible = true;
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    item.destroy_effect_suppressed = true;
    let state = barrel_mut(item);
    state.opened = true;
    state.despawn_timer = DESPAWN_FRAMES;
    item.grabbable = false;
    change(item, motion::OPENED, ANIM_UPDATE, assets);
}

/// it_802885C8 (802885C8): broken empty, the blast (as the Box's).
fn blast(item: &mut ItemCore, assets: &ItemAssets) {
    crate::explode(item, assets);
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    let state = barrel_mut(item);
    state.opened = true;
    state.despawn_timer = DESPAWN_FRAMES;
    // retail 0x8028867C: Item_80268E5C(gobj, 6, 2).
    change(item, motion::BLAST, ANIM_UPDATE, assets);
}

/// itTaru_UnkMotion0_Anim: with one frame of life left, effect 0x421.
fn rest_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.life_timer <= 1.0 {
        item.events.push(ItemEvent::DestroyEffect {
            id: LIFETIME_END_EFFECT,
            root: None,
        });
    }
    false
}

/// itTaru_UnkMotion3/4/5_Anim's lifetime, which these states run
/// themselves (not grabbable, Item_80269528 skips them): at its end effect
/// 0x421 and the barrel goes. The x34 warning blink is drawing only.
fn count_down_life(item: &mut ItemCore) -> bool {
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.events.push(ItemEvent::DestroyEffect {
            id: LIFETIME_END_EFFECT,
            root: None,
        });
        return true;
    }
    false
}

fn life_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    count_down_life(item)
}

/// itTaru_UnkMotion5_Anim: command variable 0 at 1 (the script's) turns
/// its hits on its owner and kin (it_80275444), then the lifetime.
fn rolling_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[0] == 1 {
        item.command_variables[0] = 2;
        item.hits_owner = true;
        item.strikes_kindred_items = true;
    }
    count_down_life(item)
}

fn unreached_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itTaru_UnkMotion4: no barrel code enters state 4")
}

/// itTaru_UnkMotion7_Anim: gone when the despawn timer runs out.
fn opened_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let state = barrel_mut(item);
    state.despawn_timer -= 1;
    state.despawn_timer <= 0
}

/// itTaru_UnkMotion0_Coll (80287B6C): off the ground it falls
/// (it_802762BC, it_80287D0C); on a floor steeper than x14 it rolls down
/// it (it_80288194), otherwise it leans with it (it_8027649C).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let attributes = Attributes(&ctx.assets.special_attributes);
    if !item.ground_pass(ctx.map) {
        item.enter_air();
        start_falling(item, ctx.assets);
        return false;
    }
    if item.ground_or_air == GroundOrAir::Air {
        return false;
    }
    let Some(normal) = item.floor_contact_normal() else {
        return false;
    };
    barrel_mut(item).floor_normal = normal;
    let up = Vec3::new(0.0, 1.0, 0.0);
    if melee_lb::vector::angle_xy(normal, up) > attributes.resting_lean() {
        item.facing = if normal.x > 0.0 { 1.0 } else { -1.0 };
        barrel_mut(item).roll_speed = STEEP_PUSH * item.facing;
        start_rolling(item, ctx.assets);
        return false;
    }
    item.rotation.z = item.floor_lean();
    false
}

/// itTaru_UnkMotion1_Phys -> it_80272860: gravity.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itTaru_UnkMotion1_Coll (80287D80): on a floor (it_8026DA08) it lands
/// (it_802762B0) with its sound; slower than x34 it rests, faster it
/// bounces up at ItemAttr x58 of its speed.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_pass(ctx.map) {
        item.land_on_floor();
        let speed = gekko_math::msl::fabsf(item.velocity.y);
        if speed < Attributes(&ctx.assets.special_attributes).landing_break_speed() {
            rest(item, ctx.assets);
        } else {
            item.enter_air();
            item.velocity.y *= -ctx.assets.bounce_scale;
        }
    }
    false
}

/// itTaru_UnkMotion3_Phys: gravity, the roll (it_802874F0) and its angle
/// (it_80287690).
fn thrown_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    roll(item, ctx.assets);
    turn(item, ctx.assets);
}

/// itTaru_UnkMotion3_Coll (80287FB0): on a floor it lands with its sound;
/// faster than x34 it breaks, slower than ItemAttr x5C it rolls on at its
/// sideways speed (it_802881B4), between it bounces; a ceiling stops it
/// rising; a wall breaks it.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let attributes = Attributes(&ctx.assets.special_attributes);
    if item.air_pass(ctx.map) {
        item.land_on_floor();
        let speed = gekko_math::msl::fabsf(item.velocity.y);
        if speed > attributes.landing_break_speed() {
            break_unless_opened(item);
            return false;
        }
        if speed < ctx.assets.rest_speed {
            barrel_mut(item).roll_speed = item.facing * gekko_math::msl::fabsf(item.velocity.x);
            // it_802881B4: lb_8000B804, then it_802881FC.
            reset_model_rotation(item, ctx.assets);
            start_rolling(item, ctx.assets);
        } else {
            item.enter_air();
            item.velocity.y *= -ctx.assets.bounce_scale;
        }
    } else if item.ceiling_bits() != 0 {
        if item.velocity.y > 0.0 {
            item.velocity.y = 0.0;
        }
    } else if item.wall_bits() != 0 {
        break_unless_opened(item);
        return false;
    }
    turn(item, ctx.assets);
    false
}

/// itTaru_UnkMotion5_Phys: the roll and its angle.
fn rolling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    roll(item, ctx.assets);
    turn(item, ctx.assets);
}

/// itTaru_UnkMotion5_Coll (80288324): it_8026D62C (off the ground the
/// drop, it_80287F20); on the ground the floor's normal is kept (level
/// with none), and a wall breaks a roll faster than x2C or turns it back
/// at x30 of its speed, its hits again reaching its owner and kin.
fn rolling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let attributes = Attributes(&ctx.assets.special_attributes);
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        drop_rolling(item, ctx.assets);
    }
    if item.ground_or_air != GroundOrAir::Air {
        match item.floor_contact_normal() {
            Some(normal) => barrel_mut(item).floor_normal = normal,
            None => barrel_mut(item).floor_normal.x = 0.0,
        }
        if item.touches_wall() {
            let speed = gekko_math::msl::fabsf(barrel(item).roll_speed);
            if speed > attributes.wall_break_speed() {
                break_unless_opened(item);
                return false;
            }
            item.facing = -item.facing;
            let state = barrel_mut(item);
            state.roll_speed = -(state.roll_speed * attributes.wall_rebound());
            item.velocity.x = barrel(item).roll_speed * barrel(item).floor_normal.y;
            item.hits_owner = true;
            item.strikes_kindred_items = true;
        }
    }
    turn(item, ctx.assets);
    false
}

/// it_802874F0 (802874F0): on the ground a floor leaning more than 0.1
/// pulls the roll (fmadds 80287558) up to x2C, a level one brakes it by
/// x28; the facing follows the roll, which is held to x20 and drives the
/// X speed along the floor; the angle turns with the speed (fmadds
/// 80287668). In the air the angle turns at a fixed rate (fmadds
/// 80287684).
fn roll(item: &mut ItemCore, assets: &ItemAssets) {
    use gekko_math::{fma::fmadds, msl::fabsf};
    let a = Attributes(&assets.special_attributes);
    let facing = item.facing;
    if item.ground_or_air == GroundOrAir::Air {
        let state = barrel_mut(item);
        state.roll_angle = fmadds(AIR_ROLL, -facing, state.roll_angle);
        return;
    }
    let state = barrel_mut(item);
    let lean = state.floor_normal.x;
    if fabsf(lean) > PULLING_LEAN {
        if fabsf(state.roll_speed) < a.wall_break_speed() {
            state.roll_speed = fmadds(lean, a.pull(), state.roll_speed);
        }
    } else if fabsf(state.roll_speed) < a.braking() {
        state.roll_speed = 0.0;
    } else if state.roll_speed > 0.0 {
        state.roll_speed -= a.braking();
    } else {
        state.roll_speed += a.braking();
    }
    let speed = state.roll_speed;
    if (speed > 0.0 && item.facing < 0.0) || (speed < 0.0 && item.facing > 0.0) {
        item.facing *= -1.0;
    }
    let facing = item.facing;
    let state = barrel_mut(item);
    if fabsf(state.roll_speed) > a.top_speed() {
        state.roll_speed = a.top_speed() * facing;
    }
    let velocity_x = state.roll_speed * state.floor_normal.y;
    let turn = GROUND_ROLL * fabsf(state.roll_speed);
    state.roll_angle = fmadds(turn, -facing, state.roll_angle);
    item.velocity.x = velocity_x;
}

/// The staves' standing rotation Z (it_80287690: 1.5707964) and the band
/// around it the straightening stops in.
const STAVES_UPRIGHT: f32 = 1.570_796_4;
const STAVES_UPRIGHT_ABOVE: f32 = 1.588_249_6;
const STAVES_UPRIGHT_BELOW: f32 = 1.553_343;
/// it_80287690: the tilt's band, one degree.
const ONE_DEGREE: f32 = 0.017_453_292;

/// it_80287690 (80287690): after a pickup (xDB0) the staves straighten
/// by 1.5707964 / x18 a call, rotation Z toward upright and X toward the
/// tilt (xDDC), ending xDB0 once Z is within its band; otherwise they
/// stand upright at the tilt. The root's rotation Z is the roll angle.
fn turn(item: &mut ItemCore, assets: &ItemAssets) {
    let step = STAVES_UPRIGHT / Attributes(&assets.special_attributes).straightening_frames();
    let tilt = barrel(item).tilt;
    let mut staves = item.spin_joint_rotation.unwrap_or_default();
    if item.command_variables[1] != 0 {
        let z = staves.z;
        staves.z = if z > STAVES_UPRIGHT_ABOVE + step {
            z - step
        } else if z < STAVES_UPRIGHT_BELOW - step {
            z + step
        } else {
            item.command_variables[1] = 0;
            STAVES_UPRIGHT
        };
        let x = staves.x;
        // Separate fadds: (tilt + one degree) + step.
        staves.x = if x > tilt + ONE_DEGREE + step {
            x - step
        } else if x < tilt - ONE_DEGREE - step {
            x + step
        } else {
            tilt
        };
    } else {
        staves.z = STAVES_UPRIGHT;
        staves.x = tilt;
    }
    item.spin_joint_rotation = Some(staves);
    item.rotation.z = barrel(item).roll_angle;
}

/// The staves' authored rotation (the model's first child).
fn authored_staves(assets: &ItemAssets) -> Vec3 {
    let child = assets.visual.model.child.as_ref().map(|c| c.rotation);
    child.map_or(Vec3::ZERO, |r| Vec3::new(r.x, r.y, r.z))
}

/// lb_8000B804 on the whole model: the root and the staves as authored.
fn reset_model_rotation(item: &mut ItemCore, assets: &ItemAssets) {
    item.reset_model_rotation(assets);
    item.spin_joint_rotation = Some(authored_staves(assets));
}
