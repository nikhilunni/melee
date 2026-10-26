//! The Parasol (It_Kind_Parasol, itparasol.c 8028B08C..8028B8B8): a
//! battering item that its holder also opens to float down
//! (ftCo_ItemParasolOpen.c and its siblings). In the hand it plays one of
//! seven animations (states 4..10) the holder picks through
//! ftCommon_8007E83C, at a rate the holder sets (x5D0); thrown or dropped
//! it drifts down, its canopy turning with its fall speed.
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SlidePass,
};
use melee_types::ItemKind;

/// It_Kind_Parasol.
pub struct Parasol;

/// it_803F5AB0's anim_id column.
pub const ARTICLE_STATES: [i32; 11] = [-1, 4, 0, -1, 1, 2, 3, 4, 5, 6, 7];
/// itParasolAttributes: x0 (the fall's braking), x4 (a throw's braking),
/// x8 and xC (the canopy's turn per frame and per unit of fall speed).
pub const SPECIAL_ATTRIBUTES: u32 = 4;

/// it_803F5B60: the frames of each article state's animation
/// (it_8028B08C).
const ARTICLE_FRAMES: [i32; 8] = [20, 16, 45, 2, 20, 30, 40, 16];

mod motion {
    pub const GROUNDED: u16 = 0;
    pub const FALL: u16 = 1;
    pub const THROWN: u16 = 2;
    pub const SLIDING: u16 = 3;
    /// The first of the held states (it_8028B618).
    pub const HELD: u16 = 4;
}

/// itParasol_Logic13_Thrown's Item_8026AE84 (and the held states past 4).
const SWOOSH_SOUND: u32 = 0xF7;
/// MTXDegToRad(1).
const DEG_TO_RAD: f32 = 0.017_453_292;

/// it_8028B08C (8028B08C): the frames of held state `state`'s animation.
pub fn state_frames(state: u16) -> i32 {
    ARTICLE_FRAMES[ARTICLE_STATES[usize::from(state)] as usize]
}

static STATES: [ItemStateRow; 11] = [
    row(0, no_animation, no_physics, grounded_collision),
    row(1, canopy_animation, fall_physics, landing_collision),
    row(2, canopy_animation, thrown_physics, landing_collision),
    row(3, no_animation, no_physics, sliding_collision),
    row(4, no_animation, no_physics, no_collision),
    row(5, no_animation, no_physics, no_collision),
    row(6, no_animation, no_physics, no_collision),
    row(7, no_animation, no_physics, no_collision),
    row(8, no_animation, no_physics, no_collision),
    row(9, no_animation, no_physics, no_collision),
    row(10, no_animation, no_physics, no_collision),
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

impl ItemLogic for Parasol {
    const KIND: ItemKind = ItemKind::Parasol;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itParasol_Logic13_Spawned (8028B0B8): its ECB stays upright (xDCE b7
    /// off), the fall (it_8028B17C); the canopy's turn starts at rest.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ecb_follows_rotation = false;
        item.scratch = ItemScratch::Float(0.0);
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itParasol_Logic13_PickedUp -> it_8028B618(1.0): state 4.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        hold(item, motion::HELD, 1.0, context.assets);
    }
    /// itParasol_Logic13_Thrown: the swoosh, state 2.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.sound_requests.push(SWOOSH_SOUND);
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itParasol_Logic13_Dropped: as thrown.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        Self::thrown(item, context);
    }
    const ENTERED_AIR: Option<EnteredAir> =
        Some(|item, assets| change(item, motion::SLIDING, ANIM_UPDATE, assets));
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itParasol_Logic13_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
    /// it_8028B618 .. it_8028B850 from the holder (ftCommon_8007E83C).
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::ParasolState { state, rate } => hold(item, state, rate, assets),
            other => unimplemented!("itparasol.c: control {other:?}"),
        }
    }
}

/// animSpeed (itparasol.c:196): past state 4 the swoosh plays; x5D0 takes
/// `rate`, then held state `state`.
fn hold(item: &mut ItemCore, state: u16, rate: f32, assets: &ItemAssets) {
    if state > motion::HELD {
        item.sound_requests.push(SWOOSH_SOUND);
    }
    item.animation_rate = rate;
    change(item, state, ANIM_UPDATE, assets);
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_8028B0EC (8028B0EC): grabbable, stopped, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = hsd_types::Vec3::ZERO;
    change(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itParasol_UnkMotion2_Anim (8028B1A4): the canopy (the root's child)
/// turns about Y by (x8 + |vel.y * xC|) degrees, signed by the facing
/// (retail: fmuls, fabs, fadds, fmuls by the facing, then fmadds with the
/// degree constant).
fn canopy_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let a = &ctx.assets.special_attributes;
    let ItemScratch::Float(turn) = item.scratch else {
        unreachable!("a Parasol's canopy turn")
    };
    let degrees = (a[2] + gekko_math::msl::fabsf(item.velocity.y * a[3])) * item.facing;
    item.scratch = ItemScratch::Float(gekko_math::fma::fmadds(DEG_TO_RAD, degrees, turn));
    false
}

/// decelerateItemX: the X speed brakes toward zero by `braking`.
fn brake(item: &mut ItemCore, braking: f32) {
    if gekko_math::msl::fabsf(item.velocity.x) > braking {
        if item.velocity.x > braking {
            item.velocity.x -= braking;
        }
        if item.velocity.x < -braking {
            item.velocity.x += braking;
        }
    } else {
        item.velocity.x = 0.0;
    }
}

/// itParasol_UnkMotion1_Phys: it_80272860, then braking by x0.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    brake(item, ctx.assets.special_attributes[0]);
}

/// itParasol_UnkMotion2_Phys: it_80272860, then braking by x4.
fn thrown_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    brake(item, ctx.assets.special_attributes[1]);
}

/// itParasol_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itParasol_UnkMotion2_Coll: it_8026E15C(it_8028B0EC).
fn landing_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            come_to_rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// it_8026E8C4(it_8028B0EC, it_8028B17C).
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => come_to_rest(item, ctx.assets),
        SlidePass::Airborne => change(item, motion::FALL, ANIM_UPDATE, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}
