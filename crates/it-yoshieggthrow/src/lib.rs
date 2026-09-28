//! Yoshi's Egg Throw egg (It_Kind_Yoshi_EggThrow), ityoshieggthrow.c
//! (802B2890..802B2FA8). Yoshi's up-special creates the egg in his hand
//! (it_802B2A10), then sends it out on a command (it_802B28C8). The thrown
//! egg falls under ItemAttr gravity and bursts when its lifetime runs out,
//! when it touches the stage, or when its hitbox connects (it_802B2C38);
//! the burst's own hitbox lives for the second lifetime.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::*, *};
use melee_types::ItemKind;

pub struct YoshiEggThrow;

/// it_803F7118's anim_id column: the article state each motion state plays.
pub const ARTICLE_STATES: [i32; 3] = [-1, 0, 1];
/// itYoshiEggThrowAttributes: two floats (x0, x4).
pub const SPECIAL_ATTRIBUTES: u32 = 2;
/// ftData.x48_items index (ftYs_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;

mod motion {
    /// In Yoshi's hand: no callbacks (it_803F7118[0]).
    pub const HELD: u16 = 0;
    /// Thrown and falling (itYoshieggthrow_UnkMotion1).
    pub const THROWN: u16 = 1;
    /// The burst (itYoshieggthrow_UnkMotion2).
    pub const BURST: u16 = 2;
}

/// efAsync ids of the burst (efsync.c): generator 0x2328 at the egg, and
/// the shell fragments scaled by the parameter.
const BURST_SPARKLE: u16 = 0x4CE;
const BURST_SHELL: u16 = 0x4CF;
/// Item_8026AE84's burst sound.
const BURST_SOUND: u32 = 0x44618;

/// itYoshiEggThrowAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: frames the thrown egg flies before it bursts.
    fn flight_frames(&self) -> f32 {
        self.0[0]
    }
    /// x4: frames the burst lasts.
    fn burst_frames(&self) -> f32 {
        self.0[1]
    }
}

static STATES: [ItemStateRow; 3] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: thrown_animation,
        physics: thrown_physics,
        collision: thrown_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: burst_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

/// it_3F2F.c's Yoshi egg throw logic row.
impl ItemLogic for YoshiEggThrow {
    const KIND: ItemKind = ItemKind::YoshiEggThrow;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itYoshiEggThrow_Logic43_PickedUp (802B2AE0).
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// it_802B2C04 (802B2C04).
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        burst_unless_bursting(item, context.assets);
        false
    }
    /// it_802B2E5C (802B2E5C) -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// it_2725_Logic43_Clanked (802B2D50), it_802B2C38 inlined.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        burst_unless_bursting(item, context.assets);
        false
    }
    /// it_802B2F88 (802B2F88) -> itColl_BounceOffShield.
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("it_802B2F88: itColl_BounceOffShield (Yoshi egg off a shield)")
    }
    /// it_802B2E7C (802B2E7C).
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        burst_unless_bursting(item, context.assets);
        false
    }
    /// it_802B2890 (802B2890), from Yoshi's take-damage and death callbacks
    /// (ftYs_SpecialS_8012E270): the egg still in his hand is let go and
    /// destroyed at once. A thrown egg is no longer his x2238.
    fn control(item: &mut ItemCore, control: ItemControl) {
        match control {
            ItemControl::Remove if item.held => {
                item.owner = None;
                item.held = false;
                item.destroyed = true;
            }
            ItemControl::Remove => {}
            _ => unimplemented!("Yoshi egg item control {control:?}"),
        }
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_802B28C8 (802B28C8): the thrown state with its flight lifetime
/// (it_802B2B08), out of the hand at `launch.velocity` (it_8027429C), not
/// counted as a speed-damage throw (xDC8 x14) and never pickable
/// (it_8026B3A8), nudged by `launch.offset` and spinning at
/// `launch.spin_degrees`.
pub fn launch(
    item: &mut ItemCore,
    launch: &Launch,
    half_life_scale: f32,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
) {
    change(item, motion::THROWN, ANIM_UPDATE, assets);
    // it_80275158: the lifetime and its half (it_804D6D28->x4C).
    let frames = Attributes(&assets.special_attributes).flight_frames();
    item.life_timer = frames;
    item.half_life = frames * half_life_scale;
    // it_8027429C: it_80273B50 at the hand, then it_80273F34. A hold-kind
    // 8 article leaves from the hand joint moved by its negated attachment
    // translation (lb_8000B1CC -> PSMTXMultVec).
    let t = assets.attachment_translation;
    let mut hand = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(&launch.hand, &Vec3::new(-t.x, -t.y, -t.z), &mut hand);
    item.leave_hand(launch.velocity, hand, assets);
    item.end_hold(launch.center, launch.attack, map, assets);
    item.stale_multiplier = launch.attack_stale;
    item.speed_damage = false;
    item.grabbable = false;
    // retail 802B2928..48: three separate fadds.
    item.position = Vec3::new(
        item.position.x + launch.offset.x,
        item.position.y + launch.offset.y,
        item.position.z + launch.offset.z,
    );
    item.update_spin(launch.spin_degrees);
}

/// it_802B2C04 / it_2725_Logic43_Clanked / it_802B2E7C: a burst egg does
/// not burst again.
fn burst_unless_bursting(item: &mut ItemCore, assets: &ItemAssets) {
    if item.motion != motion::BURST {
        burst(item, assets);
    }
}

/// it_802B2C38 (802B2C38): the burst state keeps the flight hitbox
/// (ANIM_UPDATE | HIT_PRESERVE), the model hides (it_8026BB44), the egg
/// stops and the burst lifetime starts; the sparkle and the shell pieces
/// go on the egg's efAsync queue. it_80273598's rumble is presentation;
/// it_8026BD24 (xDD0 b3) only stops a common item's pickup lifetime.
fn burst(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::BURST, ANIM_UPDATE | HIT_PRESERVE, assets);
    item.hidden = true;
    // it_8027518C's common explosion lifetime, overwritten below.
    item.life_timer = assets.explosion_lifetime;
    // it_80273454 -> itResetVelocity.
    item.velocity = Vec3::ZERO;
    item.life_timer = Attributes(&assets.special_attributes).burst_frames();
    item.spawn_async(ItemEvent::RootEffect {
        id: BURST_SPARKLE,
        parameter: None,
    });
    item.spawn_async(ItemEvent::RootEffect {
        id: BURST_SHELL,
        parameter: Some(1.0),
    });
    item.sound_requests.push(BURST_SOUND);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itYoshieggthrow_UnkMotion1_Anim (802B2B5C): burst once the flight
/// lifetime is spent, else count it down (retail 802B2B8C: fsubs 1.0).
fn thrown_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.life_timer <= 0.0 {
        burst(item, ctx.assets);
    } else {
        item.life_timer -= 1.0;
    }
    false
}

/// itYoshieggthrow_UnkMotion1_Phys (802B2BA8): it_80272860 with ItemAttr
/// x10 / x14, and no spin update.
fn thrown_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itYoshieggthrow_UnkMotion1_Coll (802B2BD8): it_8026E5A0 bursts the egg
/// on any floor, wall or ceiling contact, after the pass's own bounce.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let contact = item.airborne_collision(ctx.map, ctx.assets);
    if contact.floor || contact.ceiling || contact.left_wall || contact.right_wall {
        burst(item, ctx.assets);
    }
    false
}

/// itYoshieggthrow_UnkMotion2_Anim (802B2D30) -> it_802751D8.
fn burst_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}
