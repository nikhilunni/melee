//! Thunder Jolt: the ball (It_Kind_Pikachu_TJolt_Ground,
//! itpikachutjoltground.c, 802B3368..802B3E14) flies down from Pikachu;
//! on meeting a surface it spawns the crawler (It_Kind_Pikachu_TJolt_Air,
//! itpikachutjoltair.c, 802B3EFC..802B4580) and rides the crawler's
//! animated joint 6 along the surface, re-settling the crawler at each new
//! contact. The two point at each other (see [`melee_it::LinkRequest`]).
use gekko_math::msl::{cosf, sinf};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, JoltState, LinkMessage, LinkRequest, LinkTarget,
    SpawnItem,
};
use melee_lb::trigf::atan2f;
use melee_types::ItemKind;

pub struct ThunderJoltBall;
pub struct ThunderJoltCrawler;

/// it_803F7190's anim_id column: flying, then riding the crawler.
pub const BALL_ARTICLE_STATES: [i32; 2] = [0, 1];
const FLYING: u16 = 0;
const RIDING: u16 = 1;
/// it_803F71D8's anim_id column: the crawler's one state.
pub const CRAWLER_ARTICLE_STATES: [i32; 1] = [0];

/// efSync_Spawn 0x4BD on the ball's JObj: its trail.
const TRAIL: u16 = 0x4BD;
/// The crawler's joint the ball rides (xBBC_dynamicBoneTable->bones[6]).
pub const RIDE_JOINT: usize = 6;
/// itPikachutjoltair_UnkMotion0_Anim: frames a crawler lasts on a surface.
const CRAWL_FRAMES: i32 = 24;
/// M_PI, M_TAU and M_PI_2 as the C's doubles.
const PI: f64 = std::f64::consts::PI;
const TAU: f64 = std::f64::consts::TAU;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;

/// itPikachutJoltGroundAttributes (the ball's special attributes).
struct BallAttributes<'a>(&'a [f32]);
impl BallAttributes<'_> {
    /// x0: the ball's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the flight's angle facing right.
    fn angle(&self) -> f32 {
        self.0[1]
    }
    /// x8: the flight's speed.
    fn speed(&self) -> f32 {
        self.0[2]
    }
    /// xC: the largest turn between surfaces it follows (zero: any).
    fn turn_limit(&self) -> f32 {
        self.0[3]
    }
}

fn jolt(item: &mut ItemCore) -> &mut JoltState {
    match &mut item.scratch {
        ItemScratch::Jolt(state) => state,
        _ => unreachable!("a Thunder Jolt article without its state"),
    }
}

/// it_8026E9A4 (8026E9A4): mpCheckAllRemap from `from` to `to`; on a hit,
/// `to` becomes the contact and the line's normal is returned.
fn map_contact(map: &mut melee_mp::CollMap, from: Vec3, to: &mut Vec3) -> Option<Vec3> {
    let hit = map.check_all_remap(-1, -1, from.x, from.y, to.x, to.y)?;
    *to = hit.pos;
    Some(hit.normal)
}

/// Wrap into [0, 2pi]: each step adds or subtracts the double M_TAU and
/// rounds back to single (retail 802B38B0..D4).
fn wrap_angle(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > TAU {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

static BALL_STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: BALL_ARTICLE_STATES[0],
        animation: flying_anim,
        physics: flying_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: BALL_ARTICLE_STATES[1],
        animation: riding_anim,
        physics: stop,
        collision: riding_collision,
    },
];

impl ItemLogic for ThunderJoltBall {
    const KIND: ItemKind = ItemKind::PikachuTJoltGround;
    const STATES: &'static [ItemStateRow] = &BALL_STATES;
    const PARTNER_BONE: Option<usize> = Some(RIDE_JOINT);
    /// it_2725_Logic106_Destroyed: it_802B43B0 on the crawler.
    const UNLINKS_PARTNER_ON_DESTROY: bool = true;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::Jolt(JoltState::default());
    }
    /// itPikachuThunderJolt_Spawn (802B338C) once Item_80268B18 returns: a
    /// wall between Pikachu and the spawn point puts the ball at Pikachu;
    /// the lifetime; it_802B3554's set-up.
    fn spawned_with_map(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        map: &mut melee_mp::CollMap,
    ) {
        let mut contact = spawn.previous_position;
        if map_contact(map, spawn.position, &mut contact).is_some() {
            item.position = spawn.position;
        }
        item.command_variables = [0; 4];
        let a = BallAttributes(&assets.special_attributes);
        // it_80275158: both timers.
        item.life_timer = a.lifetime();
        item.half_life = a.lifetime() * common.half_life_scale;
        item.partner = None;
        let state = jolt(item);
        state.frames = 0;
        state.previous_position = spawn.position;
        state.normal = Vec3::ZERO;
        set_up_ball(item, assets);
    }
    /// it_2725_Logic106_DmgDealt: the crawler lets go, the trail goes.
    fn damage_dealt(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end_ball(item)
    }
    /// it_2725_Logic106_Clanked.
    fn clanked(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end_ball(item)
    }
    /// it_2725_Logic106_Absorbed.
    fn absorbed(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end_ball(item)
    }
    /// it_2725_Logic106_HitShield.
    fn hit_shield(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end_ball(item)
    }
    /// it_2725_Logic106_Reflected (802B3C38): the ball turns round; the
    /// angle gains the double M_PI and wraps.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.facing = -item.facing;
        item.rotation.y = (HALF_PI * f64::from(item.facing)) as f32;
        let state = jolt(item);
        state.angle = wrap_angle((f64::from(state.angle) + PI) as f32);
        false
    }
    /// it_2725_Logic106_ShieldBounced (802B3D84): the flight mirrors off
    /// the shield.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, ctx.shield_normal);
        let velocity = item.velocity;
        jolt(item).angle = wrap_angle(atan2f(velocity.y, velocity.x));
        item.facing = if velocity.x >= 0.0 { 1.0 } else { -1.0 };
        item.rotation.y = (HALF_PI * f64::from(item.facing)) as f32;
        false
    }
}

/// it_802B3554 (802B3554): not grabbable, shown, state 0, the flight's
/// angle (the attribute facing right, pi plus its size facing left, in
/// double), and the trail.
fn set_up_ball(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    item.held = false;
    item.hidden = false;
    item.change_motion_with(FLYING, BALL_ARTICLE_STATES[0], ANIM_UPDATE, assets);
    let attribute = BallAttributes(&assets.special_attributes).angle();
    let angle = if item.facing == 1.0 {
        attribute
    } else {
        (PI + f64::from(attribute.abs())) as f32
    };
    let state = jolt(item);
    state.angle = angle;
    state.trail = true;
    item.events.push(ItemEvent::OwnEffect { id: TRAIL });
}

/// The ball's events that end it: the crawler lets go (it_802B43B0) and
/// efLib_DestroyAll takes the trail.
fn end_ball(item: &mut ItemCore) -> bool {
    if let Some(crawler) = item.partner.take() {
        item.link_requests.push(LinkRequest {
            target: LinkTarget::Item(crawler),
            message: LinkMessage::Unlinked,
        });
    }
    item.events.push(ItemEvent::DestroyEffects);
    jolt(item).trail = false;
    true
}

/// itPikachutjoltground_UnkMotion0_Anim (802B3654): the ray's start, then
/// the lifetime (it_80273130).
fn flying_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let position = item.position;
    jolt(item).previous_position = position;
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itPikachutjoltground_UnkMotion0_Phys (802B3784): the flight's speed
/// along its angle (separate fmuls).
fn flying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let speed = BallAttributes(&ctx.assets.special_attributes).speed();
    let angle = jolt(item).angle;
    item.velocity.x = speed * cosf(angle);
    item.velocity.y = speed * sinf(angle);
}

/// itResetVelocity.
fn stop(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity = Vec3::ZERO;
}

/// itPikachutjoltground_UnkMotion0_Coll (802B3808): at a surface the
/// trail goes, the ball starts riding, and the crawler spawns facing along
/// the flight's turn from the surface (the angle's difference against the
/// double M_PI), settles there, and the ball's hitboxes go (it_802725D4).
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let state = *jolt(item);
    jolt(item).previous_normal = state.normal;
    let mut position = item.position;
    let Some(normal) = map_contact(ctx.map, state.previous_position, &mut position) else {
        return false;
    };
    item.position = position;
    jolt(item).normal = normal;
    item.events.push(ItemEvent::DestroyEffects);
    jolt(item).trail = false;
    item.change_motion_with(RIDING, BALL_ARTICLE_STATES[1], ANIM_UPDATE, ctx.assets);
    let surface = atan2f(normal.y, normal.x);
    let difference = wrap_angle(atan2f(item.velocity.y, item.velocity.x) - surface);
    let facing = if f64::from(difference) < PI {
        -1.0
    } else {
        1.0
    };
    jolt(item).previous_normal = normal;
    // it_802B4224: Item_InitSpawnOnPlane at the contact for the owner.
    let owner = item.owner.expect("Thunder Jolt owner");
    // The crawler is the ball's kind plus one (ip2->kind + 1).
    let crawler = ItemKind::try_from(i32::from(item.kind) + 1).expect("crawler kind");
    let spawn = SpawnItem::attached(crawler, owner, position, facing);
    item.link_requests.push(LinkRequest {
        target: LinkTarget::Spawn(spawn),
        message: LinkMessage::Settle { position, normal },
    });
    jolt(item).frames = 0;
    clear_hitboxes(item, ctx.assets);
    false
}

/// it_802725D4 (802725D4): every hitbox goes, and the capsules update.
fn clear_hitboxes(item: &mut ItemCore, assets: &ItemAssets) {
    item.hitboxes.fill(None);
    for history in &mut item.reflection_history {
        history.clear();
    }
    item.update_hitboxes(assets);
}

/// itPikachutjoltground_UnkMotion1_Anim (802B3694): the ball goes once the
/// crawler lets go of it or its lifetime runs out; otherwise it takes the
/// crawler's joint 6 (it_802B3F20, z zero).
fn riding_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let Some(partner) = ctx.partner else {
        return true;
    };
    if partner.partner != Some(item.id) {
        return true;
    }
    if flying_anim(item, ctx) {
        return true;
    }
    let mut position = partner.bone_position.expect("the crawler's joint");
    position.z = 0.0;
    item.position = position;
    false
}

/// itPikachutjoltground_UnkMotion1_Coll (802B3A44): a new contact along
/// the way (after at least one frame's travel) settles the crawler there,
/// unless the surface turned more than the attribute allows
/// (lbVector_Angle between the normals).
fn riding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if jolt(item).frames >= 0 {
        let state = *jolt(item);
        jolt(item).previous_normal = state.normal;
        let mut position = item.position;
        if let Some(normal) = map_contact(ctx.map, state.previous_position, &mut position) {
            item.position = position;
            jolt(item).normal = normal;
            let limit = BallAttributes(&ctx.assets.special_attributes).turn_limit();
            if limit != 0.0 && melee_lb::vector::angle(normal, state.normal) > limit.abs() {
                return true;
            }
            let Some(crawler) = item.partner else {
                return true;
            };
            item.link_requests.push(LinkRequest {
                target: LinkTarget::Item(crawler),
                message: LinkMessage::Settle { position, normal },
            });
            jolt(item).frames = 0;
        }
    }
    jolt(item).frames += 1;
    false
}

static CRAWLER_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: CRAWLER_ARTICLE_STATES[0],
    animation: crawling_anim,
    physics: stop,
    collision: never_lands,
}];

impl ItemLogic for ThunderJoltCrawler {
    const KIND: ItemKind = ItemKind::PikachuTJoltAir;
    const STATES: &'static [ItemStateRow] = &CRAWLER_STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::Jolt(JoltState::default());
    }
    /// it_802B4224 (802B4224) once Item_80268B18 returns: the model turned
    /// to the facing (double M_PI_2), then it_802B43D0: not grabbable,
    /// shown, state 0 and a first animation step (Item_802694CC).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        _common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        jolt(item).frames = 0;
        item.rotation.y = (HALF_PI * f64::from(item.facing)) as f32;
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.change_motion_with(0, CRAWLER_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.advance_animation(assets);
    }
    /// it_802B3F88 (802B3F88): settle at a contact. The copied collision
    /// data has no reader in the crawler. The model turns to the normal
    /// (negated facing left), its animation restarts keeping the hitboxes,
    /// and takes a step.
    fn link_received(item: &mut ItemCore, message: LinkMessage, assets: &ItemAssets) -> bool {
        let LinkMessage::Settle { position, normal } = message else {
            unimplemented!("Thunder Jolt crawler message {message:?}");
        };
        item.position = position;
        let state = jolt(item);
        state.normal = normal;
        state.frames = 0;
        item.root_translation = position;
        face_normal(item);
        item.change_motion_with(
            0,
            CRAWLER_ARTICLE_STATES[0],
            ANIM_UPDATE | HIT_PRESERVE,
            assets,
        );
        item.advance_animation(assets);
        false
    }
    /// it_2725_Logic107_DmgDealt: the ball lets go.
    fn damage_dealt(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        end_crawler(item, ctx.assets)
    }
    /// it_2725_Logic107_Clanked.
    fn clanked(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        end_crawler(item, ctx.assets)
    }
    /// it_2725_Logic107_Absorbed.
    fn absorbed(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        end_crawler(item, ctx.assets)
    }
    /// it_2725_Logic107_HitShield.
    fn hit_shield(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        end_crawler(item, ctx.assets)
    }
    /// it_2725_Logic107_ShieldBounced.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        end_crawler(item, ctx.assets)
    }
    fn reflected(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("it_2725_Logic107_Reflected: the crawler turned at joint 6's pose")
    }
}

/// HSD_JObjSetRotationX from the normal: atan2f(x, y), negated facing
/// left.
fn face_normal(item: &mut ItemCore) {
    let normal = jolt(item).normal;
    let angle = atan2f(normal.x, normal.y);
    item.rotation.x = if item.facing == 1.0 { angle } else { -angle };
}

/// The crawler's endings (itPikachuTJoltAir_Anim_Destroy and the events):
/// its hitboxes go (it_802725D4) and the ball lets go of it (it_802B3544).
fn end_crawler(item: &mut ItemCore, assets: &ItemAssets) -> bool {
    clear_hitboxes(item, assets);
    if let Some(ball) = item.partner.take() {
        item.link_requests.push(LinkRequest {
            target: LinkTarget::Item(ball),
            message: LinkMessage::Unlinked,
        });
    }
    true
}

/// itPikachutjoltair_UnkMotion0_Anim (802B4448): the model follows the
/// normal; after 24 frames on one surface, or once the ball no longer
/// points back at it, the crawler goes.
fn crawling_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    face_normal(item);
    let state = jolt(item);
    state.frames += 1;
    let linked = ctx
        .partner
        .is_some_and(|partner| partner.partner == Some(item.id));
    if jolt(item).frames <= CRAWL_FRAMES && linked {
        return false;
    }
    end_crawler(item, ctx.assets)
}

/// itPikachutjoltair_UnkMotion0_Coll: nothing.
fn never_lands(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
