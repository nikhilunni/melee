//! Red Shell (It_Kind_R_Shell), itrshell.c (8028CFE0..8028E6C0): as the
//! Green Shell it falls in and rests, but once thrown, struck or kicked it
//! drives along the floor toward the nearest fighter (it_8026B5E4),
//! turning at edges and walls, until its lifetime runs out; it bounces
//! back off what it hits, and breaks after x54 hits, blows or stomps.
//!
//! The shell's spin (the child joint's rotation Y) and its un-hiding
//! (it_80272940) are drawing only.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE, HIT_PRESERVE},
    EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, RedShellState, SlidePass,
};
use melee_types::{GroundOrAir, ItemKind};

pub struct RedShell;

/// it_803F5C48's anim_id column.
pub const ARTICLE_STATES: [i32; 8] = [-1, -1, -1, 0, 0, 0, 0, -1];
/// itRShell_Attrs: x0..x54.
pub const SPECIAL_ATTRIBUTES: u32 = 22;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    pub const DROPPED: u16 = 4;
    /// Driving along the floor, and in the air.
    pub const GROUND: u16 = 5;
    pub const AIR: u16 = 6;
    /// Carried down a slope at rest (it_3F14_Logic15_EnteredAir).
    pub const SLOPE: u16 = 7;
}

/// Item_8026AE84's sound ids: the throw and the stomp.
const THROW_SOUND: u32 = 0xF2;
const STOMP_SOUND: u32 = 0xF1;
/// efAsync 0x405 (it_8028CFE0): the drive's spark.
const DRIVE_SPARK: u16 = 0x405;

/// itRShell_Attrs.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the driving lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the speed a landing drives at (through it_8028D56C).
    fn landing_speed(&self) -> f32 {
        self.0[1]
    }
    /// x8: the homing's acceleration.
    fn homing(&self) -> f32 {
        self.0[2]
    }
    /// xC: the fastest drive.
    fn top_speed(&self) -> f32 {
        self.0[3]
    }
    /// x10: a blow slower than this does not set it driving.
    fn start_speed(&self) -> f32 {
        self.0[4]
    }
    /// x14: it_8028D56C's rebound weights (X, Y) and scale (Z).
    fn rebound(&self) -> Vec3 {
        Vec3::new(self.0[5], self.0[6], self.0[7])
    }
    /// x20 / x24: a hit's speed per point of damage, set and added.
    fn hit_speed(&self) -> f32 {
        self.0[8]
    }
    fn hit_push(&self) -> f32 {
        self.0[9]
    }
    /// x28: a stomp's kick (x2C, the kick a driving shell takes, is
    /// guarded).
    fn kick_speed(&self) -> f32 {
        self.0[10]
    }
    /// x3C: frames before its hits reach its owner.
    fn owner_grace(&self) -> f32 {
        self.0[15]
    }
    /// x40: frames between homing targets.
    fn retarget_frames(&self) -> f32 {
        self.0[16]
    }
    /// x44: frames between sparks.
    fn spark_interval(&self) -> f32 {
        self.0[17]
    }
    /// x48: the spark's offset from the model root.
    fn spark_offset(&self) -> Vec3 {
        Vec3::new(self.0[18], self.0[19], self.0[20])
    }
    /// x54 (an int): the hits it takes before it breaks.
    fn hits(&self) -> i32 {
        self.0[21].to_bits() as i32
    }
}

static STATES: [ItemStateRow; 8] = [
    row(0, no_animation, no_physics, rest_collision),
    row(1, no_animation, gravity, fall_collision),
    row(2, no_animation, no_physics, no_collision),
    row(3, grace_animation, gravity, thrown_collision),
    row(4, grace_animation, gravity, thrown_collision),
    row(5, ground_animation, ground_physics, ground_collision),
    row(6, air_animation, gravity, thrown_collision),
    row(7, no_animation, no_physics, slope_collision),
];

const fn row(
    state: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for RedShell {
    const KIND: ItemKind = ItemKind::RShell;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// It homes on the nearest fighter (it_8026B5E4).
    const LOCKS_ON: bool = true;
    /// it_3F14_Logic15_Spawned (8028D5A0): the lifetime (and only the
    /// half-life from it, it_80275174), the hits it may take, the owner
    /// grace, one bounce, no homing yet, then the fall (it_8028D7F0).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        let a = Attributes(&assets.special_attributes);
        let lifetime = a.lifetime();
        item.half_life = lifetime * assets.half_life_scale;
        item.scratch = ItemScratch::RedShell(RedShellState {
            lifetime,
            hits_left: a.hits(),
            broken: false,
            owner_grace: a.owner_grace(),
            bounces: 1,
            bounced_before: false,
            acceleration: 0.0,
            ..Default::default()
        });
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itRShell_Logic15_PickedUp: its hits spare its owner, state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        spare_owner(item);
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// it_3F14_Logic15_Thrown: one bounce, the owner grace, state 3, the
    /// throw's sound.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        let grace = Attributes(&context.assets.special_attributes).owner_grace();
        let state = red_mut(item);
        state.bounces = 1;
        state.owner_grace = grace;
        change(
            item,
            motion::THROWN,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
        item.sound_requests.push(THROW_SOUND);
    }
    /// it_3F14_Logic15_Dropped: the owner grace, state 4.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        red_mut(item).owner_grace = Attributes(&context.assets.special_attributes).owner_grace();
        change(
            item,
            motion::DROPPED,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
    }
    const ENTERED_AIR: Option<EnteredAir> = Some(enter_slope);
    /// it_3F14_Logic15_DmgDealt: it rebounds (it_8028D3B8); out of hits
    /// it breaks.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        rebound(item, context);
        red(item).broken
    }
    /// it_3F14_Logic15_DmgReceived -> it_8028D100.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        struck(item, context.assets);
        red(item).broken
    }
    /// itRShell_Logic15_Reflected: it_80273030, then the lifetime restarts
    /// from the half-life.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        red_mut(item).lifetime = item.half_life;
        false
    }
    /// itRShell_Logic15_Clanked -> itColl_BounceOffVictim.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!(
            "itRShell_Logic15_Clanked (itrshell.c:738): a clanking Red Shell is not witnessed"
        )
    }
    /// it_3F14_Logic15_HitShield: thrown it pops back off the shield;
    /// driving it rebounds.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        match item.motion {
            motion::THROWN | motion::DROPPED => {
                unimplemented!("it_3F14_Logic15_HitShield (itrshell.c:744): a thrown Red Shell on a shield is not witnessed")
            }
            motion::GROUND | motion::AIR => {
                rebound(item, context);
                red(item).broken
            }
            _ => false,
        }
    }
    /// itRShell_Logic15_ShieldBounced: thrown it glances off the shield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if matches!(item.motion, motion::THROWN | motion::DROPPED) {
            let _ = context;
            unimplemented!("itRShell_Logic15_ShieldBounced (itrshell.c:766): a Red Shell glancing off a shield is not witnessed");
        }
        false
    }
}

fn red(item: &ItemCore) -> &RedShellState {
    match &item.scratch {
        ItemScratch::RedShell(state) => state,
        _ => unreachable!("a Red Shell's scratch"),
    }
}
fn red_mut(item: &mut ItemCore) -> &mut RedShellState {
    match &mut item.scratch {
        ItemScratch::RedShell(state) => state,
        _ => unreachable!("a Red Shell's scratch"),
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn attributes(assets: &ItemAssets) -> Attributes<'_> {
    Attributes(&assets.special_attributes)
}

/// it_80275474.
fn spare_owner(item: &mut ItemCore) {
    item.hits_owner = false;
    item.strikes_kindred_items = false;
}

/// it_80275444.
fn reach_owner(item: &mut ItemCore) {
    item.hits_owner = true;
    item.strikes_kindred_items = true;
}

/// it_8028D56C (8028D56C): a rebound speed from `speed` and `direction`
/// (retail 8028D58C..94: fmuls, fmadds, fmuls).
fn rebound_speed(assets: &ItemAssets, speed: f32, direction: f32) -> f32 {
    let w = attributes(assets).rebound();
    let f = gekko_math::fma::fmadds(speed, -w.x, -w.y * direction);
    f * w.z
}

/// it_8028D7F0 (8028D7F0): the fall.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALL, ANIM_UPDATE, assets);
}

/// it_8028D62C (8028D62C): at rest, grabbable, its hits reaching its
/// owner's items (it_80275414), taking hits (it_802756E0), the owner
/// grace, stompable.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.strikes_kindred_items = true;
    item.hurt_intangible = false;
    item.velocity = Vec3::ZERO;
    red_mut(item).owner_grace = attributes(assets).owner_grace();
    change(item, motion::REST, ANIM_UPDATE, assets);
    allow_stomp(item);
}

fn allow_stomp(item: &mut ItemCore) {
    item.stompable = true;
    item.jumped_on = Some(stomped);
}

/// it_3F14_Logic15_EnteredAir: the slope carries it (xDC8 x1F), state 7.
fn enter_slope(_item: &mut ItemCore, _assets: &ItemAssets) {
    unimplemented!("it_3F14_Logic15_EnteredAir (itrshell.c:684): a Red Shell carried down a slope is not witnessed")
}

/// itRshell_StopInit: not grabbable, not stompable (it_80274C88), the
/// common lifetime (it_80275158), no slope slide, the owner grace, homing
/// at once, the spark timer, its hits reaching its owner's items
/// (it_80275414), held to xC and faced along (it_80272980).
fn stop_init(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    item.grabbable = false;
    item.stompable = false;
    let lifetime = assets.common_lifetime;
    item.life_timer = lifetime;
    item.half_life = lifetime * assets.half_life_scale;
    item.slide_mode = 0;
    let state = red_mut(item);
    state.owner_grace = a.owner_grace();
    state.retarget_timer = 0.0;
    state.spark_timer = a.spark_interval();
    item.strikes_kindred_items = true;
    clamp_speed(item, assets);
    item.face_velocity();
}

/// itRshell_ClampVel (it_8028D4E4): held to xC, (float) sign times xC.
fn clamp_speed(item: &mut ItemCore, assets: &ItemAssets) {
    let top = attributes(assets).top_speed();
    if gekko_math::msl::fabsf(item.velocity.x) > top {
        let sign: f32 = if item.velocity.x < 0.0 { -1.0 } else { 1.0 };
        item.velocity.x = sign * top;
    }
}

/// The state change's flags: from rest, the fall or a slope the hitboxes
/// start afresh; otherwise they carry on (ITEM_HIT_PRESERVE).
fn drive_flags(item: &ItemCore) -> u32 {
    if matches!(item.motion, motion::REST | motion::FALL | motion::SLOPE) {
        ANIM_UPDATE
    } else {
        ANIM_UPDATE | HIT_PRESERVE
    }
}

/// it_8028DAE4 (8028DAE4): driving on the floor.
fn drive_on_floor(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity.y = 0.0;
    stop_init(item, assets);
    let flags = drive_flags(item);
    change(item, motion::GROUND, flags, assets);
}

/// it_8028E170 (8028E170): driving in the air.
fn drive_in_air(item: &mut ItemCore, assets: &ItemAssets) {
    stop_init(item, assets);
    let flags = drive_flags(item);
    change(item, motion::AIR, flags, assets);
}

fn drive_where_it_is(item: &mut ItemCore, assets: &ItemAssets) {
    if item.ground_or_air == GroundOrAir::Air {
        drive_in_air(item, assets);
    } else {
        drive_on_floor(item, assets);
    }
}

/// it_8028D390 (8028D390): one hit fewer; at none it breaks (xDF0).
fn count_hit(item: &mut ItemCore) {
    let state = red_mut(item);
    state.hits_left -= 1;
    if state.hits_left == 0 {
        state.broken = true;
    }
}

/// How a blow's speed is applied (it_8028D100 / it_8028D26C's cases).
enum Blow {
    /// Resting, falling, thrown or on a slope: the speed is set, and the
    /// shell takes no hits once it drives.
    Set,
    /// Driving: a hit counts and the speed is added.
    Add,
    /// Held: nothing.
    None,
}

fn blow(item: &ItemCore) -> Blow {
    match item.motion {
        motion::REST | motion::FALL | motion::THROWN | motion::DROPPED | motion::SLOPE => Blow::Set,
        motion::GROUND | motion::AIR => Blow::Add,
        _ => Blow::None,
    }
}

/// The blow's end (it_8028D100 / it_8028D26C): faster than x10 it drives
/// where it is, with a new owner (and no hits taken after a set speed);
/// otherwise its hitboxes go (it_802725D4).
fn after_blow(
    item: &mut ItemCore,
    assets: &ItemAssets,
    set: bool,
    owner: impl FnOnce(&mut ItemCore),
) {
    if gekko_math::msl::fabsf(item.velocity.x) > attributes(assets).start_speed() {
        owner(item);
        if set {
            item.hurt_intangible = true;
        }
        drive_where_it_is(item, assets);
    } else {
        item.clear_hitboxes();
    }
}

/// it_8028D100 (8028D100), a hit taken: away from the hit at the damage
/// times x20, or a driving shell's hit counted and x24's push added
/// (retail 8028D1E0: fmadds); the striker owns it (it_8027236C).
fn struck(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    let damage = item.pending_damage_taken as f32;
    let away = -item.hit_direction;
    let set = match blow(item) {
        Blow::Set => {
            // retail: (float) xCA0 * x20, then fmuls by -xCCC.
            item.velocity.x = away * (damage * a.hit_speed());
            true
        }
        Blow::Add => {
            count_hit(item);
            item.velocity.x = gekko_math::fma::fmadds(away, damage * a.hit_push(), item.velocity.x);
            false
        }
        Blow::None => false,
    };
    after_blow(item, assets, set, crate::take_striker_as_owner);
}

/// it_8028E6C0 (8028E6C0), Item.jumped_on: it_8028D26C, the stomp's sound;
/// out of hits it breaks.
fn stomped(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    kicked(item, context.assets);
    item.sound_requests.push(STOMP_SOUND);
    red(item).broken
}

/// it_8028D26C (8028D26C): as a hit, at x28 (or x2C added, retail
/// 8028D304: fmadds) away from the stomper (xCD0), who owns it
/// (it_802723FC).
fn kicked(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    let away = -item.stomp_direction;
    let set = match blow(item) {
        Blow::Set => {
            item.velocity.x = a.kick_speed() * away;
            true
        }
        Blow::Add => {
            unimplemented!(
                "it_8028D26C (itrshell.c:149): a driving Red Shell stomped is not witnessed"
            )
        }
        Blow::None => false,
    };
    after_blow(item, assets, set, |item| {
        item.owner = item.stomped_by;
        item.owner_secondary = item.stomped_by_secondary;
    });
}

/// it_8028D3B8 (8028D3B8): one hit fewer; unless broken it takes hits
/// again, reaches its owner (it_80275444), rebounds (it_8028D56C on its
/// speed and xCB8) facing along; after its first rebound its bounces are
/// drawn (HSD_Randi(ItCo +48 & 0xF)); unless driving it drives where it
/// is.
fn rebound(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    count_hit(item);
    if red(item).broken {
        return;
    }
    item.hurt_intangible = false;
    reach_owner(item);
    item.velocity.x = rebound_speed(context.assets, item.velocity.x, item.out_damage_direction);
    item.face_velocity();
    if red(item).bounced_before {
        let limit = i32::from(context.assets.throw_break & 0xF);
        let cell = context.rng.expect("a Red Shell's rebound draws");
        let mut rng = cell.get();
        red_mut(item).bounces = rng.randi(limit);
        cell.set(rng);
    } else {
        red_mut(item).bounced_before = true;
    }
    if !matches!(item.motion, motion::GROUND | motion::AIR) {
        drive_where_it_is(item, context.assets);
    }
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

fn gravity(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// The owner grace (xDD8): then it_80275444 unless its hits already
/// reach the owner.
fn count_owner_grace(item: &mut ItemCore) {
    if red(item).owner_grace <= 0.0 {
        if !item.hits_owner {
            reach_owner(item);
        }
    } else {
        red_mut(item).owner_grace -= 1.0;
    }
}

/// itRshell_UnkMotion3/4_Anim.
fn grace_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    count_owner_grace(item);
    false
}

/// it_8028CFE0 (8028CFE0): every x44 frames a spark at x48 (its X turned
/// against the facing), efAsync kind 2.
fn spark(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    if red(item).spark_timer <= 0.0 {
        let mut offset = a.spark_offset();
        offset.x *= -item.facing;
        item.spawn_async(ItemEvent::JointEffect {
            id: DRIVE_SPARK,
            joint: 0,
            offset,
        });
        red_mut(item).spark_timer = a.spark_interval();
    } else {
        red_mut(item).spark_timer -= 1.0;
    }
}

/// itRshell_UnkMotion5_Anim (8028DB9C): out of lifetime it goes; else the
/// lifetime and owner grace count down and on the floor it sparks.
fn ground_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if red(item).lifetime <= 0.0 {
        return true;
    }
    red_mut(item).lifetime -= 1.0;
    count_owner_grace(item);
    if item.motion == motion::GROUND {
        spark(item, ctx.assets);
    }
    false
}

/// itRshell_UnkMotion6_Anim (8028E1F0): as on the floor while lifetime
/// is left; out of it, nothing.
fn air_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if red(item).lifetime > 0.0 {
        red_mut(item).lifetime -= 1.0;
        count_owner_grace(item);
        if item.motion == motion::GROUND {
            spark(item, ctx.assets);
        }
    }
    false
}

/// The homing's push this frame (xDE0) and the speed it gives; the speed
/// is held to xC only while it agrees with the push.
fn push(item: &mut ItemCore, assets: &ItemAssets) {
    let acceleration = red(item).acceleration;
    item.velocity.x += acceleration;
    let push_sign = if acceleration < 0.0 { -1 } else { 1 };
    let speed_sign = if item.velocity.x < 0.0 { -1 } else { 1 };
    if speed_sign == push_sign {
        clamp_speed(item, assets);
    }
}

/// itRshell_UnkMotion5_Phys (8028DC5C): every x40 frames the nearest
/// fighter (it_8026B5E4) turns the push toward it (x8), which then
/// applies; slow with its first hitbox off, the hitbox comes back
/// (it_80272674); facing along the speed.
fn ground_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = attributes(ctx.assets);
    if red(item).retarget_timer <= 0.0 {
        if let Some(target) = nearest_fighter(item.position, ctx.targets) {
            let toward: f32 = if target.x - item.position.x < 0.0 {
                -1.0
            } else {
                1.0
            };
            red_mut(item).acceleration = a.homing() * toward;
            push(item, ctx.assets);
            if item.hitboxes[0].is_none()
                && gekko_math::msl::fabsf(item.velocity.x) <= a.start_speed()
            {
                item.restore_hitbox(0, ctx.assets);
            }
            item.face_velocity();
        }
        red_mut(item).retarget_timer = a.retarget_frames();
    } else {
        push(item, ctx.assets);
        item.face_velocity();
        red_mut(item).retarget_timer -= 1.0;
    }
}

/// ftLib_8008627C (8008627C) for no fighter: the nearest fighter in play
/// (strictly nearer, in list order), its camera bone (ftLib_800866DC).
fn nearest_fighter(position: Vec3, targets: &melee_it::LockOnTargets) -> Option<Vec3> {
    let mut best = f32::MAX;
    let mut result = None;
    for fighter in targets.fighters.iter().filter(|f| !f.disabled) {
        let dx = position.x - fighter.position.x;
        let dy = position.y - fighter.position.y;
        // retail 80086328 / 8008632C: fmuls, then fmadds.
        let distance = gekko_math::fma::fmadds(dy, dy, dx * dx);
        if distance < best {
            best = distance;
            result = Some(fighter.position);
        }
    }
    result
}

/// it_80276CB8's lean (the child joint's spin is drawing only).
fn lean(item: &mut ItemCore) {
    if item.ground_or_air == GroundOrAir::Ground {
        item.lean_with_floor();
    }
}

/// itRshell_UnkMotion0_Coll: off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling(item, ctx.assets);
    }
    lean(item);
    false
}

/// itRshell_UnkMotion1_Coll: with a bounce left a landing settles
/// (it_8026E32C) into rest; with none the landing ends it (it_8026DF34).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if red(item).bounces != 0 {
        if item.bounce_to_rest(ctx.map, ctx.assets, ctx.entered_air) {
            rest(item, ctx.assets);
        }
        false
    } else {
        item.air_pass(ctx.map)
    }
}

/// it_8026E414 with it_8028D090 (states 3, 4 and 6): a landing drives on
/// the floor at it_8028D56C(x4, facing).
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let contact = item.airborne_collision(ctx.map, ctx.assets);
    if contact.floor {
        item.restore_collision_box();
        drive_on_floor(item, ctx.assets);
        let speed = attributes(ctx.assets).landing_speed();
        item.velocity.x = rebound_speed(ctx.assets, speed, item.facing);
        item.face_velocity();
    }
    false
}

/// itRshell_UnkMotion5_Coll (8028DFE4): a grounded pass that stops at
/// edges (it_8026D8A4: an edge turns it back, fn_8028D4A8); off the floor
/// it drives in the air; a wall turns it back too, its hits reaching its
/// owner; then the lean.
fn ground_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let (grounded, edge) = item.walk_ground_pass(ctx.map);
    if edge {
        turn_back(item);
    }
    if !grounded {
        unimplemented!("itRshell_UnkMotion5_Coll (itrshell.c:615): a driving Red Shell leaving the floor is not witnessed");
    }
    if item.wall_bits() != 0 {
        unimplemented!("itRshell_UnkMotion5_Coll (itrshell.c:615): a Red Shell turned back by a wall is not witnessed");
    }
    lean(item);
    false
}

/// fn_8028D4A8 / itRshell_UM5C_Reverse: speed and push reversed, faced
/// along.
fn turn_back(item: &mut ItemCore) {
    item.velocity.x = -item.velocity.x;
    let state = red_mut(item);
    state.acceleration = -state.acceleration;
    item.face_velocity();
}

/// itRshell_UnkMotion7_Coll -> it_8026E8C4: the slope's slide ends at rest
/// or off the floor in the fall; then the lean.
fn slope_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => rest(item, ctx.assets),
        SlidePass::Airborne => start_falling(item, ctx.assets),
        SlidePass::Sliding => {}
    }
    lean(item);
    false
}
