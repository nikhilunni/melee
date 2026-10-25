//! Motion-Sensor Bomb (It_Kind_MSBomb), itmsbomb.c (8028FE90..802908D8):
//! it falls in and rests; thrown or dropped, it sticks to the first surface
//! it touches and arms (it_80290314); armed, anything its inert hitbox
//! touches (Item.touched) or any hit sets it off (it_80290614).
//!
//! it_8027B288 / it_8027B564 at the blast only keep the player's attack
//! statistics (xD90, xDA8), which nothing simulated reads.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE, HIT_PRESERVE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::{mp::ItEcb, GroundOrAir, ItemKind};

pub struct MotionSensorBomb;

/// ItemStateTable_MsBomb's anim_id column.
pub const ARTICLE_STATES: [i32; 8] = [-1, -1, -1, 1, 0, 0, 2, -1];
/// itMsBomb_Attrs: x0..x14.
pub const SPECIAL_ATTRIBUTES: u32 = 6;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    /// Armed, stuck to a surface.
    pub const ARMED: u16 = 4;
    /// Armed, its surface gone, falling until it sticks again.
    pub const ARMED_FALL: u16 = 5;
    pub const BLAST: u16 = 6;
}

/// Item_80268E5C flag bit 0 (it/forward.h).
const UNK_0X1: u32 = 1;
/// Item_8026AE84(ip, 243, ...): the bomb sticking.
const STICK_SOUND: u32 = 243;
/// it_80272C08's efSync_Spawn: the blast.
const EXPLOSION_EFFECT: u16 = 0x410;

/// itMsBomb_Attrs.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the fall's tilt (the model's rotation X).
    fn fall_tilt(&self) -> f32 {
        self.0[0]
    }
    /// x4: how far a wall it sticks to pushes it out.
    fn wall_offset(&self) -> f32 {
        self.0[1]
    }
    /// x8: the box a throw gives it.
    fn thrown_box(&self) -> ItEcb {
        ItEcb {
            top: self.0[2],
            bottom: self.0[3],
            right: self.0[4],
            left: self.0[5],
        }
    }
}

static STATES: [ItemStateRow; 8] = [
    row(0, no_animation, no_physics, rest_collision),
    row(1, no_animation, gravity, fall_collision),
    row(2, no_animation, no_physics, no_collision),
    row(3, no_animation, falling_physics, flight_collision),
    row(4, no_animation, no_physics, armed_collision),
    row(5, no_animation, falling_physics, flight_collision),
    row(6, blast_animation, no_physics, no_collision),
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

/// Whether it has armed before (xDD4 b0): a second arming keeps its
/// hitboxes.
fn armed_before(item: &ItemCore) -> bool {
    match item.scratch {
        ItemScratch::MotionSensor { armed_before } => armed_before,
        _ => unreachable!("a Motion-Sensor Bomb's scratch"),
    }
}

impl ItemLogic for MotionSensorBomb {
    const KIND: ItemKind = ItemKind::MSBomb;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itMSBomb_Logic19_Spawned (8028FEE0): never armed, taking no hits
    /// (it_802756D0), then the fall.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.scratch = ItemScratch::MotionSensor {
            armed_before: false,
        };
        item.hurt_intangible = true;
        start_falling(item, assets);
    }
    /// itMSBomb_Logic19_PickedUp: state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itMSBomb_Logic19_Dropped: as a throw.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        leave_hand(item, context.assets);
    }
    /// itMSBomb_Logic19_Thrown (802901CC): state 3 with the throw's damage,
    /// the kind's thrown box in use (xBFC, it_80275D5C) and the ECB no
    /// longer following the model's rotation (xDCE b7).
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        leave_hand(item, context.assets);
    }
    /// itMSBomb_Logic19_DmgDealt: thrown, it pops back off what it hit,
    /// not grabbable, its hitboxes gone.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::THROWN {
            knocked_back(item, context);
        }
        false
    }
    /// itMSBomb_Logic19_DmgReceived: armed, a hit sets it off.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if matches!(item.motion, motion::ARMED | motion::ARMED_FALL) {
            blast(item, context.assets);
        }
        false
    }
    /// itMSBomb_Logic19_Reflected -> it_80273030: it turns back at the
    /// reflection's speed and its lifetime restarts from the half life.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// itMSBomb_Logic19_Clanked.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        knocked_back(item, context);
        false
    }
    /// itMSBomb_Logic19_HitShield: thrown, as DmgDealt.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::THROWN {
            knocked_back(item, context);
        }
        false
    }
    /// itMSBomb_Logic19_ShieldBounced: thrown, it glances off the shield.
    fn shield_bounced(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::THROWN {
            unimplemented!("itMSBomb_Logic19_ShieldBounced (itmsbomb.c:308): a thrown Motion-Sensor Bomb glancing off a shield is not witnessed");
        }
        false
    }
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itMSBomb_Logic19_EnteredAir (itmsbomb.c:271): a Motion-Sensor Bomb carried down a slope is not witnessed")
    });
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
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

/// Item_ApplyFallingPhysics: gravity, then the falling spin.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// it_8028FF8C (8028FF8C): the fall, the model tilted by x0
/// (HSD_JObjSetRotationX).
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALL, ANIM_UPDATE, assets);
    item.rotation.x = Attributes(&assets.special_attributes).fall_tilt();
}

/// it_8028FF1C (8028FF1C): grabbable, still (it_80273454), at rest.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = Vec3::ZERO;
    change(item, motion::REST, ANIM_UPDATE, assets);
}

/// itMSBomb_Logic19_Thrown (802901CC), which Dropped calls too.
fn leave_hand(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, assets);
    item.use_collision_box(Attributes(&assets.special_attributes).thrown_box());
    item.ecb_follows_rotation = false;
}

/// DmgDealt / Clanked / HitShield: itColl_BounceOffVictim, not grabbable
/// (it_8026B3A8), its hitboxes gone (it_802725D4).
fn knocked_back(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    item.bounce_off_victim(context.victim_bounce);
    item.grabbable = false;
    item.clear_hitboxes();
}

/// itMsbomb_UnkMotion0_Coll -> it_8026D62C: off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling(item, ctx.assets);
    }
    false
}

/// itMsbomb_UnkMotion1_Coll -> it_8026E15C: a settled landing rests
/// (it_8028FF1C).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        melee_it::AirLanding::Airborne | melee_it::AirLanding::Sliding => false,
        melee_it::AirLanding::Broken => true,
        melee_it::AirLanding::Landed => {
            rest(item, ctx.assets);
            false
        }
    }
}

/// itMsbomb_UnkMotion3_Coll / _5_Coll -> it_8026E71C: any surface arms it
/// (it_80290314).
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_any(ctx.map) {
        arm(item, ctx);
    }
    false
}

/// it_80290314 (80290314): armed on the surface, keeping its hitboxes if
/// it armed before; then it_80290314_inline.
fn arm(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) {
    let flags = if armed_before(item) {
        HIT_PRESERVE | UNK_0X1
    } else {
        ANIM_UPDATE
    };
    change(item, motion::ARMED, flags, ctx.assets);
    item.land_on_floor();
    stick(item, ctx.assets);
}

/// it_80290314_inline: still (it_80273454); on its surface (always, as
/// it_802762B0 ran first) the spin stops (it_80274740), the model leans
/// against the surface (it_802765BC) and it sticks there (it_80290238);
/// then not grabbable, the touch callback, not pushed (xDC8 x1A), taking
/// hits (it_802756E0), its hits reaching its owner (it_80275444) and its
/// owner's reaching it (it_802754D4), armed.
fn stick(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    if item.ground_or_air == GroundOrAir::Ground {
        item.reset_spin();
        item.lean_with_contact();
        stick_to_surface(item, assets);
    }
    item.grabbable = false;
    item.touched = Some(touched);
    item.pushable = false;
    item.hurt_intangible = false;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.hurt_by_owner = true;
    item.scratch = ItemScratch::MotionSensor { armed_before: true };
}

/// it_80290238 (80290238): stuck to the line (xDCE b3); on a ceiling it
/// hangs below it by the thrown box's height (retail 802902A0: fmadds),
/// on a wall it sits out by x4; the sticking sound.
fn stick_to_surface(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    item.stuck_to_line = true;
    let collision = item.collision.as_ref().expect("item map collision");
    let env = collision.env_flags as u32;
    use melee_types::mp::collide;
    if env & collide::FLOOR_MASK != 0 {
        item.floor_line = collision.floor.index;
    } else if item.ceiling_bits() == 2 {
        let b = a.thrown_box();
        item.position.y = gekko_math::fma::fmadds(item.scale, b.top - b.bottom, item.position.y);
    } else {
        let wall = item.wall_bits();
        if wall == 8 {
            item.position.x += a.wall_offset();
        }
        if wall == 4 {
            item.position.x -= a.wall_offset();
        }
    }
    item.sound_requests.push(STICK_SOUND);
}

/// itMsbomb_UnkMotion4_Coll (802903E8): while its line lives
/// (it_8026D604) the line hears of it (mpColl_80043558) and the model
/// leans with it (it_80276934); a line gone (it_8026D938) drops it armed
/// (it_8029047C).
fn armed_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let line = item.floor_line;
    if ctx.map.line_is_active(line) {
        if let Some(collision) = item.collision.as_mut() {
            ctx.map.notify_line_joint(collision, line);
        }
        let normal = ctx.map.line_get_normal(line);
        item.lean_to_normal(normal);
    } else {
        unimplemented!("it_8029047C (itmsbomb.c:173): an armed Motion-Sensor Bomb losing its line is not witnessed");
    }
    false
}

/// itMsbomb_UnkMotion7_Coll -> it_8026E8C4 (reached only through the
/// guarded EnteredAir).
fn slope_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itMsbomb_UnkMotion7_Coll (itmsbomb.c:276): a Motion-Sensor Bomb sliding down a slope is not witnessed")
}

/// it_802908D8 (802908D8), Item.touched: the blast.
fn touched(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    blast(item, context.assets);
    false
}

/// it_802905D8 / the touch: state 6, then it_80290614 (80290614): hidden,
/// its lifetime stopped (it_8026BD24) and set to the blast's
/// (it_8027518C, no destroy effect), a radial gust, the blast effect
/// (it_80272C08, no more hitlag), its hits reaching its owner, taking no
/// hits.
fn blast(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::BLAST, ANIM_UPDATE, assets);
    item.hidden = true;
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.mark_exploding();
    // lb_800119DC(&pos, 120, 1.0, 0.02, pi/3).
    item.events.push(ItemEvent::Gust {
        center: item.position,
        frames: 120,
        strength: 1.0,
        decay: 0.02,
        phase_step: std::f32::consts::FRAC_PI_3,
    });
    item.events.push(ItemEvent::Effect {
        id: EXPLOSION_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.hurt_intangible = true;
}

/// itMsbomb_UnkMotion6_Anim -> it_802751D8: the blast's lifetime.
fn blast_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}
