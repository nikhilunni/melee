//! Green Shell (It_Kind_G_Shell), itgshell.c (8028B8D8..8028CFE0): it
//! falls in and rests; thrown, dropped or struck it slides along the floor
//! at a fixed speed (it_8028C3A8) until its lifetime runs out, its hits
//! knocking it back up in a bounce (it_8028BC2C); a fighter landing on it
//! kicks it away or stops it (it_8028CF68).
//!
//! The shell's spin (Item_UpdateRollingShellRotation's child joint) and its
//! un-hiding (it_80272940) are drawing only.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, ShellState, SlidePass,
};
use melee_types::{GroundOrAir, ItemKind};

pub struct GreenShell;

/// it_803F5BA8's anim_id column.
pub const ARTICLE_STATES: [i32; 10] = [-1, -1, -1, 0, 0, 0, -1, 0, -1, -1];
/// itGShell_Attrs: x0..x3C.
pub const SPECIAL_ATTRIBUTES: u32 = 16;

pub(crate) mod motion {
    /// At rest on the floor.
    pub const REST: u16 = 0;
    /// Falling, from the spawn, a ledge or a bounce.
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    pub const DROPPED: u16 = 4;
    /// Sliding on the floor (fast), and all but still (slow).
    pub const GROUND_FAST: u16 = 5;
    pub const GROUND_SLOW: u16 = 6;
    /// The same in the air.
    pub const AIR_FAST: u16 = 7;
    pub const AIR_SLOW: u16 = 8;
    /// Carried down a slope at rest (itGShell_Logic14_EnteredAir).
    pub const SLOPE: u16 = 9;
}

/// Item_8026AE84's sound ids: the throw and the stomp.
const THROW_SOUND: u32 = 242;
const STOMP_SOUND: u32 = 241;
/// efAsync 0x405 (it_8028B8D8): the slide's spark, from the model root.
const SLIDE_SPARK: u16 = 0x405;

/// itGShell_Attrs.
pub(crate) struct Attributes<'a>(pub(crate) &'a [f32]);
impl Attributes<'_> {
    /// x0: the spawned shell's slide lifetime.
    pub(crate) fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the fastest slide.
    pub(crate) fn top_speed(&self) -> f32 {
        self.0[1]
    }
    /// x8: a slide slower than this is all but still.
    pub(crate) fn still_speed(&self) -> f32 {
        self.0[2]
    }
    /// xC: a bounce's sideways speed, as a share of the slide's.
    pub(crate) fn bounce_scale(&self) -> f32 {
        self.0[3]
    }
    /// x10: a bounce's upward speed.
    pub(crate) fn bounce_rise(&self) -> f32 {
        self.0[4]
    }
    /// x14: the slide speed per point of damage taken.
    pub(crate) fn hit_speed(&self) -> f32 {
        self.0[5]
    }
    /// x1C: the slide speed a stomp kicks it to.
    pub(crate) fn kick_speed(&self) -> f32 {
        self.0[7]
    }
    /// x24: frames before a slide's hits reach its owner.
    pub(crate) fn owner_grace(&self) -> f32 {
        self.0[9]
    }
    /// x28: frames a struck shell's hitbox cannot clank.
    pub(crate) fn clank_frames(&self) -> f32 {
        self.0[10]
    }
    /// x2C: frames a bounced shell cannot be stomped.
    pub(crate) fn stomp_frames(&self) -> f32 {
        self.0[11]
    }
    /// x30: frames between the slide's sparks.
    pub(crate) fn spark_interval(&self) -> f32 {
        self.0[12]
    }
    /// x34: the spark's offset from the model root.
    pub(crate) fn spark_offset(&self) -> Vec3 {
        Vec3::new(self.0[13], self.0[14], self.0[15])
    }
}

static STATES: [ItemStateRow; 10] = [
    row(0, no_animation, no_physics, rest_collision),
    row(1, fall_animation, gravity, fall_collision),
    row(2, no_animation, no_physics, no_collision),
    row(3, clank_animation, gravity, thrown_collision),
    row(4, clank_animation, gravity, thrown_collision),
    row(5, ground_animation, ground_physics, ground_collision),
    row(6, ground_animation, ground_physics, ground_collision),
    row(7, air_animation, gravity, thrown_collision),
    row(8, air_animation, gravity, thrown_collision),
    row(9, no_animation, no_physics, slope_collision),
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

impl ItemLogic for GreenShell {
    const KIND: ItemKind = ItemKind::GShell;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itGShell_Logic14_Spawned (8028BD5C): the slide lifetime (and only
    /// the half-life from it, it_80275174), one bounce, the timers, then
    /// the fall (it_8028C018).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        let a = Attributes(&assets.special_attributes);
        let lifetime = a.lifetime();
        item.half_life = lifetime * assets.half_life_scale;
        item.scratch = ItemScratch::Shell(ShellState {
            lifetime,
            bounces: 1,
            bounced_before: false,
            clank_pending: false,
            clank_timer: a.clank_frames(),
            stomp_pending: false,
            stomp_timer: a.stomp_frames(),
            restore_hurtbox: true,
            ..Default::default()
        });
        start_falling(item, assets);
    }
    /// itGShell_Logic14_PickedUp: its hits spare its owner (it_80275474),
    /// the stomp timer stops, state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        spare_owner(item);
        shell_mut(item).stomp_pending = false;
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itGShell_Logic14_Thrown: one bounce, state 3, the throw's sound.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        shell_mut(item).bounces = 1;
        change(
            item,
            motion::THROWN,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
        item.sound_requests.push(THROW_SOUND);
    }
    /// itGShell_Logic14_Dropped: state 4.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(
            item,
            motion::DROPPED,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
    }
    const ENTERED_AIR: Option<EnteredAir> = Some(enter_slope);
    /// itGShell_Logic14_DmgDealt: it_80272940 only shows the model.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itGShell_Logic14_DmgReceived -> it_8028BAD8.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        struck(item, context);
        false
    }
    /// itGShell_Logic14_Reflected: it_80273030, then the slide lifetime
    /// restarts from the half-life.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        shell_mut(item).lifetime = item.half_life;
        false
    }
    /// itGShell_Logic14_Clanked: the bounce.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        knock_up(item, context);
        false
    }
    /// itGShell_Logic14_HitShield: thrown it pops back off the shield
    /// (itColl_BounceOffVictim); sliding it bounces.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        match item.motion {
            motion::THROWN | motion::DROPPED => item.bounce_off_victim(context.victim_bounce),
            motion::GROUND_FAST..=motion::AIR_SLOW => knock_up(item, context),
            _ => {}
        }
        false
    }
    /// itGShell_Logic14_ShieldBounced: thrown it glances off the shield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if matches!(item.motion, motion::THROWN | motion::DROPPED) {
            let _ = context;
            unimplemented!("itGShell_Logic14_ShieldBounced (itgshell.c:645): a thrown shell glancing off a shield is not witnessed");
        }
        false
    }
}

pub(crate) fn shell(item: &ItemCore) -> &ShellState {
    match &item.scratch {
        ItemScratch::Shell(state) => state,
        _ => unreachable!("a shell's scratch"),
    }
}
pub(crate) fn shell_mut(item: &mut ItemCore) -> &mut ShellState {
    match &mut item.scratch {
        ItemScratch::Shell(state) => state,
        _ => unreachable!("a shell's scratch"),
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn attributes(assets: &ItemAssets) -> Attributes<'_> {
    Attributes(&assets.special_attributes)
}

/// it_80275474: its hits spare its owner and its owner's items.
fn spare_owner(item: &mut ItemCore) {
    item.hits_owner = false;
    item.strikes_kindred_items = false;
}

/// it_80275444: its hits reach its owner and its owner's items.
fn reach_owner(item: &mut ItemCore) {
    item.hits_owner = true;
    item.strikes_kindred_items = true;
}

/// it_80274CAC with Item.jumped_on = it_8028CF68: fighters may stomp it.
fn allow_stomp(item: &mut ItemCore) {
    item.stompable = true;
    item.jumped_on = Some(stomped);
}

/// it_8028C018 (8028C018): the fall; no stomping.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALL, ANIM_UPDATE, assets);
    item.stompable = false;
}

/// it_8028BE54 (8028BE54): at rest, grabbable, its hits reaching its
/// owner (it_80275444) and its owner's reaching it (it_802754D4), taking
/// hits again (it_802756E0), stompable.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    reach_owner(item);
    item.hurt_by_owner = true;
    item.hurt_intangible = false;
    item.velocity = Vec3::ZERO;
    change(item, motion::REST, ANIM_UPDATE, assets);
    shell_mut(item).stomp_pending = false;
    allow_stomp(item);
}

/// itGShell_Logic14_EnteredAir (8028CD8C): the slope carries it (xDC8
/// x1F), state 9.
fn enter_slope(item: &mut ItemCore, assets: &ItemAssets) {
    let _ = (item, assets);
    unimplemented!("itGShell_Logic14_EnteredAir (itgshell.c:547): a shell carried down a slope is not witnessed");
}

/// it_8028C3A8 (8028C3A8) and it_8028C898 (8028C898), on the floor and in
/// the air: not grabbable, its hits sparing its owner but reaching its
/// owner's items (it_80275474, it_80275414), the common lifetime
/// (it_80275158), no slope slide (xD5C), the owner grace and spark timers,
/// the slide held to x4 and faced along (it_80272980). Under x8 it is all
/// but still (grabbable; on the floor its owner's hits reach it,
/// it_802754D4). Stompable either way.
fn start_slide(item: &mut ItemCore, assets: &ItemAssets, grounded: bool) {
    let a = attributes(assets);
    item.grabbable = false;
    spare_owner(item);
    item.strikes_kindred_items = true;
    let lifetime = assets.common_lifetime;
    item.life_timer = lifetime;
    item.half_life = lifetime * assets.half_life_scale;
    item.slide_mode = 0;
    let state = shell_mut(item);
    state.owner_grace = a.owner_grace();
    state.spark_timer = a.spark_interval();
    state.stomp_pending = false;
    if gekko_math::msl::fabsf(item.velocity.x) > a.top_speed() {
        // (float) dir, then fmuls by x4.
        let direction: f32 = if item.velocity.x < 0.0 { -1.0 } else { 1.0 };
        item.velocity.x = direction * a.top_speed();
    }
    item.face_velocity();
    let still = gekko_math::msl::fabsf(item.velocity.x) < a.still_speed();
    match (grounded, still) {
        (true, true) => {
            change(item, motion::GROUND_SLOW, ANIM_UPDATE, assets);
            item.grabbable = true;
            item.hurt_by_owner = true;
        }
        (true, false) => change(item, motion::GROUND_FAST, ANIM_UPDATE, assets),
        (false, true) => {
            change(item, motion::AIR_SLOW, ANIM_UPDATE, assets);
            item.grabbable = true;
        }
        (false, false) => change(item, motion::AIR_FAST, ANIM_UPDATE, assets),
    }
    allow_stomp(item);
}

/// it_8028C3A8 or it_8028C898 by where it is.
fn slide_where_it_is(item: &mut ItemCore, assets: &ItemAssets) {
    let grounded = item.ground_or_air != GroundOrAir::Air;
    start_slide(item, assets, grounded);
}

/// it_8028BAD8 (8028BAD8), a hit taken: away from the hit at the damage
/// times x14 (added to a slide's speed, retail 8028BBB4: fmadds), the
/// striker its owner (it_8027236C), no hits taken, sliding where it is,
/// its first hitbox unable to clank for x28 frames.
fn struck(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    let a = attributes(context.assets);
    // retail 8028BB60 / 8028BBB0: (float) xCA0 * x14, fmuls.
    let push = item.pending_damage_taken as f32 * a.hit_speed();
    let away = -item.hit_direction;
    match item.motion {
        motion::REST | motion::FALL | motion::THROWN | motion::DROPPED | motion::SLOPE => {
            // retail 8028BB64: fmuls.
            item.velocity.x = away * push;
            shell_mut(item).restore_hurtbox = true;
        }
        motion::GROUND_FAST..=motion::AIR_SLOW => {
            item.velocity.x = gekko_math::fma::fmadds(away, push, item.velocity.x);
            shell_mut(item).restore_hurtbox = true;
        }
        _ => {}
    }
    crate::take_striker_as_owner(item);
    item.hurt_intangible = true;
    slide_where_it_is(item, context.assets);
    shell_mut(item).clank_pending = true;
    item.set_hitbox_clank(0, false);
}

/// The bounce a clank, a shield or a stomp gives a sliding shell (the
/// shellHit inline, it_8028BC2C's first case): no hits taken, its hits
/// reaching its owner (it_80275444), back at xC of its speed times
/// HSD_Randf and up at x10; after its first bounce HSD_Randi(ItCo +48 &
/// 0xF) bounces are left; airborne (it_802762BC) in the fall, not
/// stompable for x2C frames.
fn bounce(item: &mut ItemCore, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) {
    let a = attributes(assets);
    item.hurt_intangible = true;
    reach_owner(item);
    // -vel.x * xC, then * HSD_Randf (no fused operation).
    let back = -item.velocity.x * a.bounce_scale();
    item.velocity.x = back * rng.randf();
    item.velocity.y = a.bounce_rise();
    if shell(item).bounced_before {
        let limit = i32::from(assets.throw_break & 0xF);
        shell_mut(item).bounces = rng.randi(limit);
    } else {
        shell_mut(item).bounced_before = true;
    }
    item.enter_air();
    start_falling(item, assets);
    let state = shell_mut(item);
    state.stomp_pending = true;
    state.stomp_timer = a.stomp_frames();
}

fn with_rng<R>(context: &ItemEventContext<'_>, f: impl FnOnce(&mut gekko_math::HsdRng) -> R) -> R {
    let cell = context.rng.expect("a shell's event callback draws");
    let mut rng = cell.get();
    let result = f(&mut rng);
    cell.set(rng);
    result
}

/// The shellHit inline (Clanked, HitShield).
fn knock_up(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    with_rng(context, |rng| bounce(item, context.assets, rng));
}

/// it_8028CF68 (8028CF68), Item.jumped_on: it_8028BC2C, then the stomp's
/// sound.
fn stomped(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    kicked(item, context);
    item.sound_requests.push(STOMP_SOUND);
    false
}

/// it_8028BC2C (8028BC2C): a sliding shell bounces (its hits reaching its
/// owner, its bounces from then on drawn, airborne in the fall); a resting
/// or thrown one is kicked away from the stomper (xCD0) at x1C, the
/// stomper its owner (it_802723FC). Either way it takes no hits and its
/// first hitbox cannot clank for x28 frames.
fn kicked(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    let a = attributes(context.assets);
    match item.motion {
        motion::GROUND_FAST..=motion::AIR_SLOW => {
            item.hurt_intangible = true;
            with_rng(context, |rng| bounce(item, context.assets, rng));
            shell_mut(item).restore_hurtbox = true;
        }
        motion::REST | motion::FALL | motion::THROWN | motion::DROPPED | motion::SLOPE => {
            // retail: x1C * -xCD0, fmuls.
            item.velocity.x = a.kick_speed() * -item.stomp_direction;
            item.owner = item.stomped_by;
            item.owner_secondary = false;
            shell_mut(item).restore_hurtbox = true;
            slide_where_it_is(item, context.assets);
        }
        _ => {}
    }
    item.hurt_intangible = true;
    shell_mut(item).clank_pending = true;
    item.set_hitbox_clank(0, false);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_80272860 with ItemAttr x10 / x14.
fn gravity(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// it_8028B988 (8028B988): x28 frames after a hit its first hitbox may
/// clank again, and (xDEC b2) it takes hits again (it_802756E0).
fn count_clank_frames(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    let state = shell_mut(item);
    if !state.clank_pending {
        return;
    }
    state.clank_timer -= 1.0;
    if state.clank_timer > 0.0 {
        return;
    }
    state.clank_pending = false;
    state.clank_timer = a.clank_frames();
    let restore = state.restore_hurtbox;
    item.set_hitbox_clank(0, true);
    if restore {
        item.hurt_intangible = false;
    }
}

/// itGshell_UnkMotion1_Anim (8028C048): once the stomp timer of a bounce
/// runs out it may be stomped again.
fn fall_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if shell(item).stomp_pending && !item.stompable {
        if shell(item).stomp_timer <= 0.0 {
            allow_stomp(item);
        } else {
            shell_mut(item).stomp_timer -= 1.0;
        }
    }
    false
}

/// itGshell_UnkMotion3_Anim / _4_Anim: the clank timer.
fn clank_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    count_clank_frames(item, ctx.assets);
    false
}

/// The owner grace (xDD8: then it_80275444 unless its hits already reach
/// the owner), the clank timer and on the floor the sparks (it_8028B8D8).
fn slide_timers(item: &mut ItemCore, assets: &ItemAssets) {
    if shell(item).owner_grace <= 0.0 {
        if !item.hits_owner {
            reach_owner(item);
        }
    } else {
        shell_mut(item).owner_grace -= 1.0;
    }
    count_clank_frames(item, assets);
    if matches!(item.motion, motion::GROUND_FAST | motion::GROUND_SLOW) {
        spark(item, assets);
    }
}

/// it_8028B8D8 (8028B8D8): every x30 frames a spark at x34 (its X turned
/// against the facing) from the model root, efAsync kind 2.
fn spark(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    if shell(item).spark_timer <= 0.0 {
        let mut offset = a.spark_offset();
        offset.x *= -item.facing;
        item.spawn_async(ItemEvent::JointEffect {
            id: SLIDE_SPARK,
            joint: 0,
            offset,
        });
        shell_mut(item).spark_timer = a.spark_interval();
    } else {
        shell_mut(item).spark_timer -= 1.0;
    }
}

/// itGshell_UnkMotion6_Anim (states 5 and 6): the timers; it goes when
/// the slide lifetime is out.
fn ground_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    slide_timers(item, ctx.assets);
    let state = shell_mut(item);
    if state.lifetime <= 0.0 {
        return true;
    }
    state.lifetime -= 1.0;
    false
}

/// itGshell_UnkMotion8_Anim (states 7 and 8): as on the floor, but the
/// lifetime only counts down.
fn air_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    slide_timers(item, ctx.assets);
    let state = shell_mut(item);
    if state.lifetime > 0.0 {
        state.lifetime -= 1.0;
    }
    false
}

/// itGshell_UnkMotion6_Phys: all but still and not yet grabbable, it
/// becomes grabbable, loses its hitboxes (it_802725D4) and its owner's
/// hits reach it (it_802754D4).
fn ground_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = attributes(ctx.assets);
    if gekko_math::msl::fabsf(item.velocity.x) < a.still_speed() && !item.grabbable {
        unimplemented!(
            "itGshell_UnkMotion6_Phys (itgshell.c:441): a slide slowed under x8 is not witnessed"
        );
    }
}

/// Item_UpdateRollingShellRotation's lean (it_80276CB8); the child
/// joint's spin is drawing only.
fn lean(item: &mut ItemCore) {
    if item.ground_or_air == GroundOrAir::Ground {
        item.lean_with_floor();
    }
}

/// itGshell_UnkMotion0_Coll (8028BF84): off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling(item, ctx.assets);
    }
    lean(item);
    false
}

/// itGshell_UnkMotion1_Coll (8028C0B8): with a bounce left a landing
/// settles (it_8026E32C) into rest; with none the landing ends it
/// (it_8026DF34).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if shell(item).bounces != 0 {
        if item.bounce_to_rest(ctx.map, ctx.assets, ctx.entered_air) {
            rest(item, ctx.assets);
        }
        false
    } else {
        item.air_pass(ctx.map)
    }
}

/// itGshell_UnkMotion3/4/8_Coll -> it_8026E414 with it_8028BA2C: walls
/// and ceilings bounce it; a landing slides it.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let contact = item.airborne_collision(ctx.map, ctx.assets);
    if contact.floor {
        land(item, ctx);
    }
    false
}

/// it_8028BA2C (8028BA2C): on the floor (the release box restored,
/// it_80275D5C) all but still it stops, resting unless the floor slides
/// it (it_80277040: itGShell_Logic14_EnteredAir); faster it slides.
fn land(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) {
    let a = attributes(ctx.assets);
    item.restore_collision_box();
    if gekko_math::msl::fabsf(item.velocity.x) < a.still_speed() {
        item.velocity = Vec3::ZERO;
        if !item.slides(ctx.assets) {
            rest(item, ctx.assets);
        } else {
            enter_slope(item, ctx.assets);
        }
    } else {
        start_slide(item, ctx.assets, true);
    }
}

/// itGshell_UnkMotion6_Coll (8028C7C0): off the floor it slides on in the
/// air (it_8028C898); on it, it leans, and a wall it moves into turns it
/// back (it_8027770C, it_80272980).
fn ground_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_slide(item, ctx.assets, false);
    }
    lean(item);
    if item.bounce_off_wall(ctx.assets) {
        item.face_velocity();
    }
    false
}

/// itGshell_UnkMotion9_Coll (8028CDD8) -> it_8026E8C4: the slope's slide
/// ends at rest or off the floor in the fall; on the floor it leans.
fn slope_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => rest(item, ctx.assets),
        SlidePass::Airborne => start_falling(item, ctx.assets),
        SlidePass::Sliding => {}
    }
    lean(item);
    false
}
