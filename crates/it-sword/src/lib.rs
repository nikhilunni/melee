//! The plain battering items (it_8026B30C kind 2), swung through the
//! fighter's swing states (ftswing.c): the Beam Sword (It_Kind_Sword,
//! itsword.c 80284E10..80285FC4) and the Home-Run Bat (It_Kind_Bat,
//! itbat.c 80284854..80284D90). Both have the same five states: the spawn
//! into a fall, lying on the ground, the hold, throwing and dropping (a
//! thrown one bounces off what it hits) and landing.
//!
//! The blade's glow (it_80284E30 and the x4..x54 timers behind it) only
//! scales joints 3 and 6 of the model and feeds the swing's afterimage
//! (it_802852B8); no game state reads it, so it is not modelled. The bat's
//! pickup, drop and throw unhide its model's child joint
//! (HSD_JObjClearFlagsAll), which no game state reads either.
//!
//! The Fan ([`harisen`]) and the Star Rod with its star ([`starrod`]) have
//! their own files.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemStateRow, SlidePass,
};
use melee_types::ItemKind;

/// It_Kind_Sword.
pub struct Sword;
/// It_Kind_Bat.
pub struct Bat;

/// it_803F5800's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [0, 0, 0, 1, 0];
/// itSword_UnkArticle1: x0..x18 (the afterimage bytes at x1C are not read).
pub const SPECIAL_ATTRIBUTES: u32 = 7;
/// it_803F57B0's anim_id column.
pub const BAT_ARTICLE_STATES: [i32; 5] = [-1, -1, -1, 0, -1];
/// itBatAttributes: x0..xC_vel.
pub const BAT_SPECIAL_ATTRIBUTES: u32 = 4;

/// What distinguishes one plain battering item from another: its kind and
/// its state table. The callbacks are the same code in both files.
pub trait Battering: 'static {
    const KIND: ItemKind;
    /// The state table's anim_id column.
    const ARTICLE_STATES: [i32; 5];
    const TABLE: &'static [ItemStateRow];
}
impl Battering for Sword {
    const KIND: ItemKind = ItemKind::Sword;
    const ARTICLE_STATES: [i32; 5] = ARTICLE_STATES;
    const TABLE: &'static [ItemStateRow] = &SWORD_STATES;
}
impl Battering for Bat {
    const KIND: ItemKind = ItemKind::Bat;
    const ARTICLE_STATES: [i32; 5] = BAT_ARTICLE_STATES;
    const TABLE: &'static [ItemStateRow] = &BAT_STATES;
}

mod motion {
    /// At rest on the floor (UnkMotion0).
    pub const GROUNDED: u16 = 0;
    /// Falling after the spawn or off a ledge (UnkMotion1).
    pub const FALL: u16 = 1;
    /// In a fighter's hand (UnkMotion2).
    pub const HELD: u16 = 2;
    /// Thrown or dropped (UnkMotion3's callbacks, article state 1/0).
    pub const THROWN: u16 = 3;
    /// Sliding down a floor (UnkMotion4).
    pub const SLIDING: u16 = 4;
}

/// itSword_UnkArticle1 xC / itBatAttributes xC_vel: the spawn's upward speed.
fn spawn_speed(assets: &ItemAssets) -> f32 {
    assets.special_attributes[3]
}

/// it_803F5800 / it_803F57B0: rows 1 and 3 share UnkMotion3's callbacks.
const fn states<B: Battering>() -> [ItemStateRow; 5] {
    [
        ItemStateRow {
            animation_id: B::ARTICLE_STATES[0],
            animation: no_animation,
            physics: no_physics,
            collision: grounded_collision::<B>,
        },
        ItemStateRow {
            animation_id: B::ARTICLE_STATES[1],
            animation: no_animation,
            physics: falling_physics,
            collision: falling_collision::<B>,
        },
        ItemStateRow {
            animation_id: B::ARTICLE_STATES[2],
            animation: no_animation,
            physics: no_physics,
            collision: no_collision,
        },
        ItemStateRow {
            animation_id: B::ARTICLE_STATES[3],
            animation: no_animation,
            physics: falling_physics,
            collision: falling_collision::<B>,
        },
        // State 4: carried off the floor at rest (EnteredAir).
        ItemStateRow {
            animation_id: B::ARTICLE_STATES[4],
            animation: no_animation,
            physics: no_physics,
            collision: sliding_collision::<B>,
        },
    ]
}
static SWORD_STATES: [ItemStateRow; 5] = states::<Sword>();
static BAT_STATES: [ItemStateRow; 5] = states::<Bat>();

macro_rules! battering_logic {
    ($ty:ty) => {
        impl ItemLogic for $ty {
            const KIND: ItemKind = <$ty as Battering>::KIND;
            const STATES: &'static [ItemStateRow] = <$ty as Battering>::TABLE;
            /// itSword_Logic12_Spawned (80285338) / itBat_Logic11_Spawned:
            /// grabbable (it_8026B390), rising at xC, then the fall
            /// (it_802855F8 / it_802849F0).
            fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
                item.grabbable = true;
                item.velocity = Vec3::new(0.0, spawn_speed(assets), 0.0);
                start_falling::<$ty>(item, assets);
            }
            /// itSword_Logic12_PickedUp (80285840) / itBat_Logic11_PickedUp:
            /// the hold state.
            fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
                change::<$ty>(item, motion::HELD, ANIM_UPDATE, context.assets);
                item.land_count = 0;
            }
            /// itSword_Logic12_Thrown (80285CE4) / itBat_Logic11_Thrown.
            fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
                leave_hand::<$ty>(item, context.assets);
            }
            /// itSword_Logic12_Dropped (80285C1C) / itBat_Logic11_Dropped:
            /// as a throw.
            fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
                leave_hand::<$ty>(item, context.assets);
            }
            /// itSword_Logic12_EnteredAir (80285DF0) / itBat_Logic11_EnteredAir:
            /// a floor's slope carries the item off (state 4).
            const ENTERED_AIR: Option<EnteredAir> = Some(|item, assets| {
                change::<$ty>(item, motion::SLIDING, ANIM_UPDATE, assets)
            });
            /// DmgDealt (8028602C for the sword): a thrown one pops back.
            fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
                if item.motion == motion::THROWN {
                    item.bounce_off_victim(context.victim_bounce);
                }
                false
            }
            /// Reflected -> it_80273030.
            fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
                item.reverse_on_reflect(context.reflected_speed);
                false
            }
            /// Clanked -> itColl_BounceOffVictim.
            fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
                item.bounce_off_victim(context.victim_bounce);
                false
            }
            /// HitShield -> itColl_BounceOffVictim.
            fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
                item.bounce_off_victim(context.victim_bounce);
                false
            }
            /// ShieldBounced -> itColl_BounceOffShield.
            fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
                item.bounce_off_shield(context.shield_normal);
                false
            }
        }
    };
}
battering_logic!(Sword);
battering_logic!(Bat);

fn change<B: Battering>(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, B::ARTICLE_STATES[motion as usize], flags, assets);
}

/// Thrown / Dropped: grabbable again (it_8026B390), then the thrown state.
fn leave_hand<B: Battering>(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    change::<B>(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, assets);
}

/// it_802855F8 / it_802849F0: the fall, landings counted afresh.
fn start_falling<B: Battering>(item: &mut ItemCore, assets: &ItemAssets) {
    change::<B>(item, motion::FALL, ANIM_UPDATE, assets);
    item.land_count = 0;
}

/// it_80285424 / it_80284974: at rest on the floor.
fn come_to_rest<B: Battering>(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.land_count = 0;
    change::<B>(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// UnkMotion3_Phys -> Item_ApplyFallingPhysics (it_80272860, it_80274658):
/// gravity, then the falling spin.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// UnkMotion0_Coll (802855CC for the sword): it_8026D62C, off the floor it
/// falls.
fn grounded_collision<B: Battering>(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling::<B>(item, ctx.assets);
    }
    false
}

/// UnkMotion3_Coll (802857D8 for the sword) -> it_8026E15C: bounces off
/// walls and ceilings; a settled landing rests; a thrown one may break on
/// its first landing.
fn falling_collision<B: Battering>(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne => false,
        AirLanding::Landed => {
            come_to_rest::<B>(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
        // it_8026DD5C ran EnteredAir instead of the rest.
        AirLanding::Sliding => false,
    }
}

/// UnkMotion4_Coll (80285F78 for the sword) -> it_8026E8C4: on the floor
/// it rests, off it it falls.
fn sliding_collision<B: Battering>(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => come_to_rest::<B>(item, ctx.assets),
        SlidePass::Airborne => start_falling::<B>(item, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}

pub mod harisen;
pub use harisen::Harisen;
pub mod starrod;
pub use starrod::{StarRod, StarRodStar};

/// it_80292394 / it_80295748: where a held kind's shot leaves it, in the
/// hand joint's frame; zero for the kinds that never shoot from a swing.
pub fn shot_offset(kind: ItemKind, assets: &melee_it::desc::ItemAssets) -> hsd_types::Vec3 {
    match kind {
        ItemKind::StarRod => starrod::star_offset(assets),
        _ => hsd_types::Vec3::ZERO,
    }
}
