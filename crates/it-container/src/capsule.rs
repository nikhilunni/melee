//! Capsule (It_Kind_Capsule), itcapsule.c (8027CF30..8027D5F0): falls in,
//! rests, and opens when thrown into anything or struck: one item from the
//! contents table, or one time in `x4` an explosion.
use crate::{
    explode, no_animation, no_collision, no_physics, open_or_explode, Opening,
    EXPLOSION_MOTION_FLAGS,
};
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemCore,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct Capsule;

/// it_803F5468's anim_id column.
pub const ARTICLE_STATES: [i32; 7] = [-1, -1, -1, 0, 0, 1, -1];
/// ItCapsuleAttr: x0 (items let out), x4 (one explosion in x4).
pub const SPECIAL_ATTRIBUTES: u32 = 2;

mod motion {
    /// At rest (itCapsule_UnkMotion0).
    pub const REST: u16 = 0;
    /// Falling (row 1, itCapsule_UnkMotion4's callbacks).
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    pub const DROPPED: u16 = 4;
    pub const EXPLODE: u16 = 5;
    /// Sliding down a floor (itCapsule_UnkMotion6).
    pub const SLIDE: u16 = 6;
}

/// ItCapsuleAttr.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0 (a bool in the archive): how many items it lets out.
    fn contents(&self) -> u8 {
        self.0[0].to_bits() as u8
    }
    /// x4: HSD_Randi(x4) == 0 explodes.
    fn explosion_odds(&self) -> i32 {
        self.0[1].to_bits() as i32
    }
}

static STATES: [ItemStateRow; 7] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: no_physics,
        collision: rest_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: fall_physics,
        collision: fall_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: no_animation,
        physics: thrown_physics,
        collision: thrown_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: no_animation,
        physics: fall_physics,
        collision: fall_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: crate::explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[6],
        animation: no_animation,
        physics: no_physics,
        collision: slide_collision,
    },
];

impl ItemLogic for Capsule {
    const KIND: ItemKind = ItemKind::Capsule;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itCapsule_Logic0_EnteredAir: the slide (state 6).
    const ENTERED_AIR: Option<EnteredAir> =
        Some(|item, assets| change(item, motion::SLIDE, ANIM_UPDATE, assets));
    /// itCapsule_Logic0_Spawned (8027CEE4): the flags clear, then the fall
    /// (it_8027D148).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.scratch = melee_it::ItemScratch::Container(Default::default());
        start_falling(item, assets);
    }
    /// itCapsule_Logic0_PickedUp.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itCapsule_Logic0_Thrown.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itCapsule_Logic0_Dropped.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itCapsule_Logic0_DmgDealt: unless it is exploding, it opens; opened,
    /// it goes.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        open_unless_exploding(item, context)
    }
    /// itCapsule_Logic0_DmgReceived.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        open_unless_exploding(item, context)
    }
    /// itCapsule_Logic0_Clanked.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        open_unless_exploding(item, context)
    }
    /// itCapsule_Logic0_Reflected.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        open_unless_exploding(item, context)
    }
    /// itCapsule_Logic0_HitShield: a thrown capsule opens on a shield.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::THROWN {
            let assets = context.assets;
            crate::with_event_rng(context, |rng| open(item, assets, rng));
            return crate::container(item).opened;
        }
        false
    }
    /// itCapsule_Logic0_ShieldBounced -> itColl_BounceOffShield, thrown or
    /// dropped.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if matches!(item.motion, motion::THROWN | motion::DROPPED) {
            item.bounce_off_shield(context.shield_normal);
        }
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_8027D148 (8027D148): the fall.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALL, ANIM_UPDATE, assets);
}

/// it_8027D0B8 (8027D0B8): grabbable (it_8026B390), still, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = hsd_types::Vec3::ZERO;
    change(item, motion::REST, ANIM_UPDATE, assets);
}

/// it_8027CF30 (8027CF30): opens, or explodes once in x4 (it_8027D2DC).
/// A coin match's coins (it_8026F8B4) take a draw only under that rule.
fn open(item: &mut ItemCore, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) -> Opening {
    let a = Attributes(&assets.special_attributes);
    open_or_explode(item, assets, rng, a.explosion_odds(), a.contents(), |item, assets| {
        // it_8027D2DC: only once.
        if !crate::container(item).exploded {
            capsule_explode(item, assets);
        }
    })
}

/// it_8027CFE8 (8027CFE8): the blast, state 5.
fn capsule_explode(item: &mut ItemCore, assets: &ItemAssets) {
    // x5D0 = 1.0; it_80273454 -> itResetVelocity.
    item.animation_rate = 1.0;
    item.velocity = hsd_types::Vec3::ZERO;
    crate::container_mut(item).exploded = true;
    explode(item, assets);
    change(item, motion::EXPLODE, EXPLOSION_MOTION_FLAGS, assets);
}

/// itCapsule_Logic0_DmgDealt's body (an inline): not while exploding.
fn open_unless_exploding(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    if item.motion != motion::EXPLODE {
        let assets = context.assets;
        crate::with_event_rng(context, |rng| open(item, assets, rng));
    }
    crate::container(item).opened
}

/// itCapsule_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling(item, ctx.assets);
    }
    false
}

/// itCapsule_UnkMotion4_Phys -> it_80272860: gravity, no spin.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itCapsule_UnkMotion3_Phys: gravity, then the common falling spin
/// (ItCo +68).
fn thrown_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    fall_physics(item, ctx);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itCapsule_UnkMotion4_Coll -> it_8026E15C: a settled landing rests.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
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

/// itCapsule_UnkMotion3_Coll -> it_8026E414 with it_8027CF30: thrown, the
/// first floor opens it; opened, it goes.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        let cell = ctx.rng.expect("it_8027CF30 draws");
        let mut rng = cell.get();
        open(item, ctx.assets, &mut rng);
        cell.set(rng);
    }
    crate::container(item).opened
}

/// itCapsule_UnkMotion6_Coll -> it_8026E8C4: a stopped slide rests, off
/// the floor it falls.
fn slide_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        melee_it::SlidePass::Stopped => come_to_rest(item, ctx.assets),
        melee_it::SlidePass::Airborne => start_falling(item, ctx.assets),
        melee_it::SlidePass::Sliding => {}
    }
    false
}
