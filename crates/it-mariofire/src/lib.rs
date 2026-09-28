//! Mario's fireball (It_Kind_Mario_Fire), itmariofireball.c
//! (8029B6F8..8029BA9C). Mario's neutral special spawns it at his hand
//! (it_8029B6F8); it falls under ItemAttr gravity, bounces along the floor
//! with ItemAttr x58 of its speed each time, and ends when its lifetime runs
//! out, a bounce leaves it slower than attribute x10, or its hit connects.
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct MarioFire;

/// it_803F6788's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itUnkAttributes x0..x10.
pub const SPECIAL_ATTRIBUTES: u32 = 5;
/// ftData.x48_items index (ftMr_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;

/// efAsync_Spawn(item, xBC0, 1, 0x47B, root): efAlt's generator 0x3EB at a
/// bounce.
const BOUNCE_EFFECT: u16 = 0x47B;
/// Item_8026AE84(item, ~0xFFFD40C6, 0x7F, 0x40): the bounce sound
/// (retail 8029B9C4..C8: lis 3, subi 0x40C7).
const BOUNCE_SOUND: u32 = 0x2_BF39;

/// The fireball's special attributes (itUnkAttributes).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the launch speed.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x4: the launch angle, in radians.
    fn angle(&self) -> f32 {
        self.0[1]
    }
    /// x8: the lifetime, in frames.
    fn lifetime(&self) -> f32 {
        self.0[2]
    }
    /// x10: a bounce slower than this ends the fireball.
    fn minimum_bounce_speed(&self) -> f32 {
        self.0[4]
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

/// it_3F2F.c's Logic87 row.
impl ItemLogic for MarioFire {
    const KIND: ItemKind = ItemKind::MarioFire;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_8029B6F8 ends with it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    /// it_8029B7C0 (8029B7C0): the launch velocity along the attribute
    /// angle, the lifetime, and motion 0. No fused sites: 8029B800/80C and
    /// 8029B818 are separate fmuls.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        let (speed, angle) = (a.speed(), a.angle());
        item.velocity.x = item.facing * (speed * gekko_math::msl::cosf(angle));
        item.velocity.y = speed * gekko_math::msl::sinf(angle);
        item.velocity.z = 0.0;
        // it_80275158: both timers.
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    }
    /// itMarioFireball_Logic87_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMarioFireball_Logic87_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itMarioFireball_Logic87_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMarioFireball_Logic87_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMarioFireball_Logic87_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMarioFireball_Logic87_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itMarioFireball_Logic87_ShieldBounced: itColl_BounceOffShield")
    }
}

/// itMariofireball_UnkMotion0_Anim (8029B868): the lifetime counts down;
/// the fireball ends when it is spent.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itMariofireball_UnkMotion0_Phys -> Item_ApplyFallingPhysics: gravity,
/// then the common falling spin.
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itMariofireball_UnkMotion0_Coll (8029B8EC): it_8026D9A0's pass that
/// never lands, then it_8027781C's bounce. A bounce that leaves the
/// fireball slower than attribute x10 ends it (8029B928..3C: two fmuls and
/// an fadds, then the inlined sqrtf_accurate); a faster one plays the
/// bounce sound and effect.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.airborne_pass(ctx.map);
    if !item.bounce_velocity(ctx.map, ctx.assets) {
        return false;
    }
    let (x, y) = (item.velocity.x, item.velocity.y);
    let speed = gekko_math::msl::sqrtf_accurate(x * x + y * y);
    if speed < Attributes(&ctx.assets.special_attributes).minimum_bounce_speed() {
        return true;
    }
    // Dr. Mario's pill (It_Kind_DrMario_Vitamin) shares this callback with
    // effect 0x4A0 and no sound; only the fireball is registered.
    item.sound_requests.push(BOUNCE_SOUND);
    item.spawn_async(ItemEvent::RootEffect {
        id: BOUNCE_EFFECT,
        parameter: None,
    });
    false
}
