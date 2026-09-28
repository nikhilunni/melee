//! Yoshi Bomb's landing stars (It_Kind_Yoshi_Star), ityoshistar.c
//! (802B2FC8..802B3348): one state that slides outward along the floor,
//! speeding up, until its animation ends or its hit lands.
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct YoshiStar;

/// efSync_Spawn id of it_80272BA4's vanishing puff.
const VANISH_EFFECT: u16 = 0x411;

/// it_803F7158: the single motion state, article state 0.
static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: 0,
    animation,
    physics,
    collision,
}];

/// The star's special attributes (StarAttrs).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the launch speed along the facing.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x4: the per-frame acceleration along the facing.
    fn acceleration(&self) -> f32 {
        self.0[1]
    }
}

impl ItemLogic for YoshiStar {
    const KIND: ItemKind = ItemKind::YoshiStar;
    const STATES: &'static [ItemStateRow] = &STATES;
    // it_3F2F.c: the star's picked_up callback is NULL.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802B322C (802B322C): launch along the facing, not grabbable,
    /// motion 0 with ITEM_ANIM_UPDATE.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        // Retail 802B3258 fmuls.
        item.velocity.x = Attributes(&assets.special_attributes).speed() * item.facing;
        item.velocity.y = assets.launch_vertical_velocity;
        item.grabbable = false;
        item.change_motion_with(0, 0, ANIM_UPDATE, assets);
    }
    /// it_802B309C: a landed hit ends the star.
    fn damage_dealt(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        vanish(item)
    }
    /// it_802B30C0.
    fn clanked(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        vanish(item)
    }
    /// it_802B3108.
    fn absorbed(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        vanish(item)
    }
    /// it_802B30E4.
    fn hit_shield(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        vanish(item)
    }
    /// it_802B314C -> itReflectItemAndUpdateRotation: it_80273030, then the
    /// model turns to the new facing ((f32) M_PI / 2 times the facing).
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        item.rotation.y = std::f32::consts::FRAC_PI_2 * item.facing;
        false
    }
    /// it_802B312C -> itColl_BounceOffShield.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("it_802B312C: itColl_BounceOffShield (it_2725.c:430)")
    }
}

/// it_80272BA4 (80272BA4): the puff at the star, its sound, and
/// it_80274C60 (no more hitlag); the callback's true removes the star.
fn vanish(item: &mut ItemCore) -> bool {
    item.events.push(ItemEvent::Effect {
        id: VANISH_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    true
}

/// itYoshistar_UnkMotion0_Anim (802B3294): removed once the animation ends
/// (it_80272C6C).
fn animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    ctx.assets.animation_ends[0].is_some_and(|end| item.animation_frame >= end)
}

/// itYoshistar_UnkMotion0_Phys (802B32C8): it_80272860's gravity, then the
/// facing-signed acceleration (retail 802B3304 fmadds).
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    let acceleration = Attributes(&ctx.assets.special_attributes).acceleration();
    item.velocity.x = gekko_math::fma::fmadds(acceleration, item.facing, item.velocity.x);
}

/// itYoshistar_UnkMotion0_Coll (802B3324): it_8026E0F4, never landing.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.airborne_pass(ctx.map);
    false
}
