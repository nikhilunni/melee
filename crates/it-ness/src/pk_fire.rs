//! PK Fire: the bolt (It_Kind_Ness_PKFire, itnesspkfire.c
//! 802AA054..802AA474) and the pillar it leaves where it hits
//! (It_Kind_Ness_PKFire_Flame, itnesspkfirepillar.c 802AA494..802AA7C4).
use crate::no_physics;
use gekko_math::fma::fnmsubs;
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow, LinkMessage, LinkRequest, LinkTarget, SpawnItem,
};
use melee_types::{GroundOrAir, ItemKind};

pub struct NessPkFire;
pub struct NessPkFirePillar;

/// ftData.x48_items indices (ftNs_Init_OnLoad).
pub const BOLT_ARTICLE_INDEX: u32 = 0;
pub const PILLAR_ARTICLE_INDEX: u32 = 1;
/// it_803F6B28's and it_803F6B60's anim_id columns.
pub const BOLT_ARTICLE_STATES: [i32; 1] = [0];
pub const PILLAR_ARTICLE_STATES: [i32; 1] = [0];
/// The bolt's x0 and x4; the pillar's x0, x4 and x8.
pub const BOLT_SPECIAL_ATTRIBUTES: u32 = 2;
pub const PILLAR_SPECIAL_ATTRIBUTES: u32 = 3;

/// The bolt's special attributes.
struct Bolt<'a>(&'a [f32]);
impl Bolt<'_> {
    /// x0: the lifetime, in frames.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the pillar rises this far above the bolt.
    fn pillar_rise(&self) -> f32 {
        self.0[1]
    }
}

/// The pillar's special attributes (itNessPKFirepillarAttributes).
struct Pillar<'a>(&'a [f32]);
impl Pillar<'_> {
    /// x0: the lifetime, in frames.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: lifetime lost per percent of damage taken.
    fn damage_cost(&self) -> f32 {
        self.0[1]
    }
    /// x8 (scale): the model's scale as the lifetime runs out.
    fn final_scale(&self) -> f32 {
        self.0[2]
    }
}

/// it_802AA054's `angle` argument (the launch angle times the facing)
/// through SpawnItem.spawn_argument, as its bits.
pub fn angle_argument(angle: f32) -> i32 {
    angle.to_bits() as i32
}

static BOLT_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: BOLT_ARTICLE_STATES[0],
    animation: bolt_anim,
    physics: no_physics,
    collision: bolt_collision,
}];

/// it_3F2F.c's Logic23 row.
impl ItemLogic for NessPkFire {
    const KIND: ItemKind = ItemKind::NessPKFire;
    const STATES: &'static [ItemStateRow] = &BOLT_STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802AA054 once Item_80268B18 returns: it_802AA1D8 (state 0 and
    /// both lifetimes), then the model turned to the launch angle.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.change_motion_with(0, BOLT_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        let lifetime = Bolt(&assets.special_attributes).lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.rotation.z = f32::from_bits(spawn.spawn_argument as u32);
    }
    /// it_2725_Logic23_DmgDealt (802AA2D0): the pillar, and the bolt ends.
    fn damage_dealt(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        leave_pillar(item, ctx.assets);
        true
    }
    /// it_2725_Logic23_Clanked (802AA33C): the same.
    fn clanked(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        leave_pillar(item, ctx.assets);
        true
    }
    /// itNessPKFire_Logic23_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itNessPKFire_Logic23_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itNessPKFire_Logic23_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// it_2725_Logic23_ShieldBounced (802AA3F0): the model's angle
    /// mirrors, then itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.rotation.z = -item.rotation.z;
        item.bounce_off_shield(ctx.shield_normal);
        false
    }
}

/// itNesspkfirepillar_802AA494 from the bolt's callbacks: the pillar at the
/// bolt raised by x4, on the stage plane, for the bolt's owner.
fn leave_pillar(item: &mut ItemCore, assets: &ItemAssets) {
    let Some(owner) = item.owner else {
        unimplemented!("itNesspkfirepillar_802AA494: a pillar for an ownerless bolt");
    };
    let mut position = item.position;
    position.y += Bolt(&assets.special_attributes).pillar_rise();
    let spawn = SpawnItem::ray(ItemKind::NessPKFireFlame, owner, position, item.facing);
    item.link_requests.push(LinkRequest {
        target: LinkTarget::Spawn(spawn),
        message: LinkMessage::Spawned,
    });
}

/// itNesspkfire_UnkMotion0_Anim (802AA22C): the lifetime counts down; the
/// bolt ends when it is spent.
fn bolt_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itNesspkfire_UnkMotion0_Coll (802AA2B0) -> it_8026E058: an airborne
/// pass; a floor or a wall ends the bolt.
fn bolt_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let floor = item.air_pass(ctx.map);
    (u32::from(floor) | item.wall_bits()) & 0xD != 0
}

static PILLAR_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: PILLAR_ARTICLE_STATES[0],
    animation: pillar_anim,
    physics: pillar_physics,
    collision: pillar_collision,
}];

/// it_3F2F.c's Logic24 row.
impl ItemLogic for NessPkFirePillar {
    const KIND: ItemKind = ItemKind::NessPKFireFlame;
    const STATES: &'static [ItemStateRow] = &PILLAR_STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itNesspkfirepillar_802AA55C: state 0 and both lifetimes.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.change_motion_with(0, PILLAR_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        let lifetime = Pillar(&assets.special_attributes).lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
    }
    /// itNesspkfirepillar_Logic24_DmgReceived (802AA75C): the damage taken
    /// so far (xC9C) shortens the lifetime (retail 802AA794: fnmsubs),
    /// never below one frame.
    fn damage_received(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        let cost = Pillar(&ctx.assets.special_attributes).damage_cost();
        item.life_timer = fnmsubs(item.damage_percent as f32, cost, item.life_timer);
        if item.life_timer <= 0.0 {
            item.life_timer = 1.0;
        }
        false
    }
}

/// itNesspkfirepillar_UnkMotion0_Anim (802AA5B0): the model shrinks toward
/// x8 with the lifetime (fsubs, fmuls, fdivs, fadds, then the article's
/// scale: no fused site); the pillar ends once the lifetime is spent.
fn pillar_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let a = Pillar(&ctx.assets.special_attributes);
    let scale = a.final_scale() + (item.life_timer * (1.0 - a.final_scale())) / a.lifetime();
    let scale = scale * ctx.assets.scale;
    item.model_scale = Vec3::new(scale, scale, scale);
    if item.life_timer <= 0.0 {
        return true;
    }
    item.life_timer -= 1.0;
    false
}

/// itNesspkfirepillar_UnkMotion0_Phys (802AA6B0): retail assigns GA_Air in
/// the test, so the pillar always falls (it_80272860 at ItemAttr x10 / x14).
fn pillar_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.ground_or_air = GroundOrAir::Air;
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itNesspkfirepillar_UnkMotion0_Coll (802AA6F4): the same assignment, so
/// it_8026E414 with it_80273454: a landing stops the pillar.
fn pillar_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.ground_or_air = GroundOrAir::Air;
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        // it_80273454 -> itResetVelocity.
        item.velocity = Vec3::ZERO;
    }
    false
}
