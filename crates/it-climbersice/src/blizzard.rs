//! The Blizzard's puffs (It_Kind_IceClimber_Blizzard), itclimbersblizzard.c
//! (802C2144..802C2470). While the Blizzard's script keeps it going, the
//! climber's accessory spawns one every few frames (itClimbersBlizzard_Spawn)
//! at a random angle within attributes xC..x10 of straight ahead; it flies
//! at attribute x4 under attribute x8's pull until its lifetime ends, and a
//! wall or ceiling ends it.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct ClimbersBlizzard;

/// ftData.x48_items index (ftPp_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;
/// it_803F76A8's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itClimbersBlizzardAttributes x0..x10.
pub const SPECIAL_ATTRIBUTES: u32 = 5;

/// M_PI_2 and M_TAU (itclimbersblizzard.c), doubles in retail
/// (802C22A4..802C22DC: lfd @175 / @176).
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const TAU: f64 = std::f64::consts::TAU;

/// itClimbersBlizzardAttributes (it/itCharItems.h).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the lifetime (it_80275158).
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the speed.
    fn speed(&self) -> f32 {
        self.0[1]
    }
    /// x8: added to the vertical speed each frame.
    fn pull(&self) -> f32 {
        self.0[2]
    }
    /// xC / x10: the angle's range, above the horizontal.
    fn angle_low(&self) -> f32 {
        self.0[3]
    }
    fn angle_high(&self) -> f32 {
        self.0[4]
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

/// it_3F2F.c's Ice Climbers blizzard row.
impl ItemLogic for ClimbersBlizzard {
    const KIND: ItemKind = ItemKind::IceClimberBlizzard;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itClimbersBlizzard_Spawn clears xDC8 x15 (it_8026B3A8).
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itClimbersBlizzard_Spawn ends with it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    /// itClimbersBlizzard_Spawn (802C2144) once Item_80268B18 returns:
    /// itemVar x0 takes ItemAttr x60 (never read) and
    /// itClimbersBlizzard_802C2248 launches it; then it_8026B3A8.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        launch(item, assets, common, rng);
        item.grabbable = false;
    }
    /// itClimbersBlizzard_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itClimbersBlizzard_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itClimbersBlizzard_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itClimbersBlizzard_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itClimbersBlizzard_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itClimbersBlizzard_ShieldBounced.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
}

/// itClimbersBlizzard_802C2248 (802C2248): one HSD_Randf picks the angle in
/// xC..x10 (802C2290: fmadds), mirrored for a left-facing puff; a quarter
/// turn back in double precision (fsub, frsp), wrapped into 0..TAU; the
/// velocity at x4 along it (fmuls each), the lifetime, then motion 0
/// (itClimbersBlizzard_802C2358).
fn launch(
    item: &mut ItemCore,
    assets: &ItemAssets,
    common: &ItemCommonData,
    rng: &mut gekko_math::HsdRng,
) {
    let a = Attributes(&assets.special_attributes);
    let random = rng.randf();
    let low = a.angle_low();
    let angle = gekko_math::fma::fmadds(a.angle_high() - low, random, low);
    let angle = if item.facing == 1.0 { angle } else { -angle };
    let mut direction = (f64::from(angle) - HALF_PI) as f32;
    while direction < 0.0 {
        direction = (f64::from(direction) + TAU) as f32;
    }
    while f64::from(direction) > TAU {
        direction = (f64::from(direction) - TAU) as f32;
    }
    item.velocity.x = a.speed() * gekko_math::msl::cosf(direction);
    item.velocity.y = a.speed() * gekko_math::msl::sinf(direction);
    item.velocity.z = 0.0;
    // it_80275158: both timers.
    let lifetime = a.lifetime();
    item.life_timer = lifetime;
    item.half_life = lifetime * common.half_life_scale;
    item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
}

/// itClimbersBlizzard_UnkMotion0_Anim (802C2380): Item_TickLifetime.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.life_timer <= 0.0 {
        return true;
    }
    item.life_timer -= 1.0;
    false
}

/// itClimbersBlizzard_UnkMotion0_Phys (802C23B4): attribute x8 joins the
/// vertical speed (fadds).
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.y += Attributes(&ctx.assets.special_attributes).pull();
}

/// itClimbersBlizzard_UnkMotion0_Coll (802C23D4): it_8026D9A0's pass that
/// never lands; a wall (it_80276308) or ceiling (it_802763E0) ends it.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.airborne_pass(ctx.map);
    let walls = item.wall_bits();
    let ceiling = item.ceiling_bits();
    walls | ceiling != 0
}
