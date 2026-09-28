//! Toad's spores, itpeachtoadspore.c (802BE214..802BE9D8): a model-less
//! article with one hitbox whose script ends it, drifting out under a
//! per-frame speed decay.
use crate::no_collision;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCore, ItemEvent, ItemEventContext, ItemLogic, ItemPhysicsContext,
    ItemStateRow,
};
use melee_types::ItemKind;

pub struct PeachToadSpore;

/// it_803F7548's anim_id column.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itPeachToadSporeAttributes: x0..xC.
pub const SPECIAL_ATTRIBUTES: u32 = 4;
/// it_802BE2E8: it_80275158(item, 60.0f).
const LIFETIME: f32 = 60.0;
/// efSync_Spawn 0x4D3: the spore's two trailing generators.
const SPORE_EFFECT: u16 = 0x4D3;

/// itPeachToadSporeAttributes (special attributes).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the slowest launch.
    fn minimum_speed(&self) -> f32 {
        self.0[0]
    }
    /// x4: the random extra launch speed.
    fn speed_range(&self) -> f32 {
        self.0[1]
    }
    /// x8: the per-frame velocity multiplier.
    fn decay(&self) -> f32 {
        self.0[2]
    }
    /// xC: the launch cone, in radians.
    fn cone(&self) -> f32 {
        self.0[3]
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision: no_collision,
}];

impl ItemLogic for PeachToadSpore {
    const KIND: ItemKind = ItemKind::PeachToadSpore;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_8026B3A8 clears xDC8 x15; there is no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802BE2E8 (802BE2E8): the lifetime, a random speed and launch
    /// angle in the cone above Peach, the hidden bare JObj, the two trailing
    /// generators, then the state's script.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &melee_it::SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        // it_80275158: both timers.
        item.life_timer = LIFETIME;
        item.half_life = LIFETIME * common.half_life_scale;
        // xDAC_itcmd_var0.
        item.command_variables[0] = 0;
        // retail 802BE340: fmadds x4 * rand + x0.
        let speed = gekko_math::fma::fmadds(a.speed_range(), rng.randf(), a.minimum_speed());
        let random = rng.randf();
        // retail 802BE350..5C: fmuls xC * rand, fsubs (f32)pi - xC, then
        // fmadds 0.5 * that + the product.
        let spread = a.cone() * random;
        let remainder = std::f32::consts::PI - a.cone();
        let angle = gekko_math::fma::fmadds(0.5, remainder, spread);
        // retail 802BE368..80: separate fmuls.
        item.velocity.x = item.facing * (speed * gekko_math::msl::sinf(angle));
        item.velocity.y = speed * gekko_math::msl::cosf(angle);
        // it_80272A3C hides the model; it_8026B3A8 clears xDC8 x15.
        item.hidden = true;
        item.grabbable = false;
        item.events.push(ItemEvent::OwnEffect { id: SPORE_EFFECT });
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    }
    /// itPeachToadSpore_Logic92_DmgDealt: efLib_DestroyAll, then gone.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itPeachToadSpore_Logic68_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itPeachToadSpore_Logic68_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itPeachToadSpore_Logic68_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    fn reflected(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itPeachToadSpore_Logic68_Reflected: it_80273030 keeping the lifetime")
    }
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itPeachToadSpore_Logic68_ShieldBounced: itColl_BounceOffShield")
    }
}

/// itPeachtoadspore_UnkMotion0_Anim (802BE408): the script ends it (xDAC),
/// or the lifetime runs out.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[0] != 0 {
        return true;
    }
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        return true;
    }
    false
}

/// itPeachtoadspore_UnkMotion0_Phys (802BE458): two separate fmuls.
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let decay = Attributes(&ctx.assets.special_attributes).decay();
    item.velocity.x *= decay;
    item.velocity.y *= decay;
}
