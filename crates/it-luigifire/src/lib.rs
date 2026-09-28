//! Luigi's fireball (It_Kind_Luigi_Fire), itluigifireball.c
//! (802C01AC..802C0500). Luigi's neutral special spawns it at his hand
//! (it_802C01AC); it flies level at attribute x0 under ItemAttr gravity,
//! bounces along the floor, and ends when its lifetime runs out, a bounce
//! leaves it slower than attribute xC, or its hit connects.
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct LuigiFire;

/// it_803F75C0's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itUnkAttributes x0..xC.
pub const SPECIAL_ATTRIBUTES: u32 = 4;
/// ftData.x48_items index (ftLg_Init_OnLoad registers it).
pub const ARTICLE_INDEX: u32 = 0;

/// efAsync_Spawn(item, xBC0, 1, 1288, root): efSync's generator 0x4652 at
/// a bounce.
const BOUNCE_EFFECT: u16 = 0x508;

/// The fireball's special attributes (itUnkAttributes).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the launch speed.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x4: the lifetime, in frames.
    fn lifetime(&self) -> f32 {
        self.0[1]
    }
    /// xC: a bounce slower than this ends the fireball.
    fn minimum_bounce_speed(&self) -> f32 {
        self.0[3]
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

/// it_3F2F.c's Logic89 row.
impl ItemLogic for LuigiFire {
    const KIND: ItemKind = ItemKind::LuigiFire;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802C01AC ends with it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    /// it_802C027C (802C027C): level flight at x0 along the facing
    /// (802C02A4: fmuls), the lifetime, and motion 0.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        item.velocity.x = a.speed() * item.facing;
        item.velocity.y = 0.0;
        item.velocity.z = 0.0;
        // it_80275158: both timers.
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    }
    /// itLuigiFireball_Logic89_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itLuigiFireball_Logic89_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itLuigiFireball_Logic89_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itLuigiFireball_Logic89_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itLuigiFireball_Logic89_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itLuigiFireball_Logic89_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(ctx.shield_normal);
        false
    }
}

/// itLuigifireball_UnkMotion0_Anim (802C02E4): the lifetime counts down;
/// the fireball ends when it is spent.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itLuigifireball_UnkMotion0_Phys -> Item_ApplyFallingPhysics: gravity,
/// then the common falling spin.
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itLuigifireball_UnkMotion0_Coll (802C0368): it_8026D9A0's pass that
/// never lands, then it_8027781C's bounce. A bounce that leaves the
/// fireball slower than attribute xC ends it (802C03A0..B4: two fmuls and
/// an fadds, then the inlined sqrtf_accurate); a faster one spawns the
/// bounce effect (no sound, unlike Mario's).
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
    // Kirby's copy (It_Kind_Kirby_LuigiFire) shares this callback with
    // effect 0x4B2; only Luigi's fireball is registered.
    item.spawn_async(ItemEvent::RootEffect {
        id: BOUNCE_EFFECT,
        parameter: None,
    });
    false
}
