//! The Fan (It_Kind_Harisen), itharisen.c (802927E8..80292D2C). Unlike the
//! sword and bat it plays its own swing animations, driven by the holder's
//! HarisenSwing states (ftCo_HarisenSwing_Anim, ft_0CD3.c), swells on a hit
//! (it_802927E8) and vanishes when dropped onto the floor (state 8 with no
//! ammo, it_8026DF34).
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemStateRow, SlidePass, SwingAnimation,
};
use melee_types::ItemKind;

/// It_Kind_Harisen.
pub struct Harisen;

/// it_803F5FF0's anim_id column.
pub const ARTICLE_STATES: [i32; 10] = [-1, -1, -1, 0, 1, 2, 3, 4, 4, -1];
/// itHarisen_DatAttrs: x0_scale.
pub const SPECIAL_ATTRIBUTES: u32 = 1;

mod motion {
    /// At rest on the floor (UnkMotion0).
    pub const GROUNDED: u16 = 0;
    /// Falling after the spawn or off a ledge.
    pub const FALL: u16 = 1;
    /// In a fighter's hand.
    pub const HELD: u16 = 2;
    /// The swing animations (it_80292998 / 9C8 / 9F8, article states 0..2)
    /// and the return to rest (it_80292A28, article state 3).
    pub const SWING_JAB: u16 = 3;
    pub const SWING_TILT: u16 = 4;
    pub const SWING_SMASH: u16 = 5;
    pub const SWING_REST: u16 = 6;
    /// Thrown (itHarisen_Logic24_Thrown).
    pub const THROWN: u16 = 7;
    /// Dropped (itHarisen_Logic24_Dropped).
    pub const DROPPED: u16 = 8;
    /// Carried off the floor at rest (EnteredAir).
    pub const SLIDING: u16 = 9;
}

static STATES: [ItemStateRow; 10] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: no_physics,
        collision: grounded_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: gravity_physics,
        collision: falling_collision,
    },
    held_row(2),
    held_row(3),
    held_row(4),
    held_row(5),
    held_row(6),
    ItemStateRow {
        animation_id: ARTICLE_STATES[7],
        animation: no_animation,
        physics: falling_physics,
        collision: falling_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[8],
        animation: no_animation,
        physics: falling_physics,
        collision: dropped_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[9],
        animation: no_animation,
        physics: no_physics,
        collision: sliding_collision,
    },
];

/// UnkMotion6's callbacks with no collision callback (NULL).
const fn held_row(state: usize) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    }
}

impl ItemLogic for Harisen {
    const KIND: ItemKind = ItemKind::Harisen;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itHarisen_Logic24_Spawned -> it_8029290C: the fall, nothing else.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itHarisen_Logic24_PickedUp.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itHarisen_Logic24_Thrown: state 7 (flags 6); unlike the sword it
    /// stays ungrabbable until it rests.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itHarisen_Logic24_Dropped: normal speed and size again (x5D0 = 1,
    /// it_8029282C's body), then state 8 (flags 6).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.animation_rate = 1.0;
        restore_size(item, context.assets);
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itHarisen_Logic24_EnteredAir.
    const ENTERED_AIR: Option<EnteredAir> =
        Some(|item, assets| change(item, motion::SLIDING, ANIM_UPDATE, assets));
    /// itHarisen_Logic24_DmgDealt: a thrown or dropped fan pops back.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
    /// itHarisen_Logic24_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// itHarisen_Logic24_Clanked -> DmgDealt.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
    /// itHarisen_Logic24_HitShield -> DmgDealt.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
    /// itHarisen_Logic24_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
    /// The holder's calls from its HarisenSwing states.
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // it_80292998 / it_802929C8 / it_802929F8: x5D0 = the holder's
            // frame_speed_mul, then the swing's animation.
            ItemControl::SwingAnimation { swing, rate } => {
                item.animation_rate = rate;
                let state = match swing {
                    SwingAnimation::Jab => motion::SWING_JAB,
                    SwingAnimation::Tilt => motion::SWING_TILT,
                    SwingAnimation::Smash => motion::SWING_SMASH,
                };
                change(item, state, ANIM_UPDATE, assets);
            }
            // it_80292A28.
            ItemControl::SwingRest => {
                item.animation_rate = 1.0;
                change(item, motion::SWING_REST, ANIM_UPDATE, assets);
            }
            // it_802927E8: x0_scale times ItemAttr x60, on the model too.
            ItemControl::SwingSwell => {
                let scale = assets.special_attributes[0] * assets.scale;
                item.scale = scale;
                item.model_scale = hsd_types::Vec3::new(scale, scale, scale);
            }
            // it_8029282C.
            ItemControl::SwingShrink => restore_size(item, assets),
            other => unimplemented!("itharisen.c: control {other:?}"),
        }
    }
}

/// it_8029282C's body: scl = ItemAttr x60, then Item_8026849C.
fn restore_size(item: &mut ItemCore, assets: &ItemAssets) {
    item.scale = assets.scale;
    item.model_scale = hsd_types::Vec3::new(item.scale, item.scale, item.scale);
}

/// itHarisen_Logic24_DmgDealt: states 7 and 8 bounce off the victim.
fn bounce_if_flying(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    if item.motion == motion::THROWN || item.motion == motion::DROPPED {
        item.bounce_off_victim(context.victim_bounce);
    }
    false
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_8029287C (8029287C): grabbable (it_8026B390), stopped, at rest.
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

/// itHarisen_UnkMotion1_Phys: it_80272860 alone, no spin.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itHarisen_UnkMotion8_Phys -> Item_ApplyFallingPhysics.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itHarisen_UnkMotion0_Coll: it_8026D62C, off the floor it falls
/// (it_8029290C).
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itHarisen_UnkMotion1_Coll / UnkMotion7_Coll -> it_8026E15C(it_8029287C).
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
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

/// itHarisen_UnkMotion8_Coll (80292AE8): with a nonzero xD4C the dropped
/// fan lands as a thrown one (it_8026E15C); with zero the floor pass
/// it_8026DF34 removes it on the first floor it touches. Nothing writes
/// xD4C for the fan and Item_80268B5C does not clear it: it is whatever
/// the pooled Item struct last held (retail traces show 0, 0xF1F1F1F1,
/// 0xD8BAB3BF, ... for different fans), which the port cannot know. Both
/// passes move the fan alike until it touches a surface; a touch fails
/// closed.
fn dropped_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_bits(ctx.map) & 0xF != 0 {
        unimplemented!(
            "itHarisen_UnkMotion8_Coll (itharisen.c:139-147): a dropped fan's landing reads uninitialized xD4C"
        );
    }
    false
}

/// itHarisen_UnkMotion9_Coll -> it_8026E8C4(it_8029287C, it_8029290C).
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => come_to_rest(item, ctx.assets),
        SlidePass::Airborne => change(item, motion::FALL, ANIM_UPDATE, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}
