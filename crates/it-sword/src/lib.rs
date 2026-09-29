//! Beam Sword (It_Kind_Sword), itsword.c (80284E10..80285FC4): a battering
//! item (it_8026B30C kind 2) swung through the fighter's SwordSwing states.
//! Ported: the spawn into a fall, lying on the ground, the hold, throwing
//! and dropping (a thrown sword bounces off what it hits) and landing.
//!
//! The blade's glow (it_80284E30 and the x4..x54 timers behind it) only
//! scales joints 3 and 6 of the model and feeds the swing's afterimage
//! (it_802852B8); no game state reads it, so it is not modelled.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct Sword;

/// it_803F5800's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [0, 0, 0, 1, 0];
/// itSword_UnkArticle1: x0..x18 (the afterimage bytes at x1C are not read).
pub const SPECIAL_ATTRIBUTES: u32 = 7;

mod motion {
    /// At rest on the floor (itSword_UnkMotion0).
    pub const GROUNDED: u16 = 0;
    /// Falling after the spawn or off a ledge (itSword_UnkMotion1).
    pub const FALL: u16 = 1;
    /// In a fighter's hand (itSword_UnkMotion2).
    pub const HELD: u16 = 2;
    /// Thrown or dropped (itSword_UnkMotion3's callbacks, article state 1).
    pub const THROWN: u16 = 3;
}

/// itSword_UnkArticle1 xC: the spawn's upward speed.
fn spawn_speed(assets: &ItemAssets) -> f32 {
    assets.special_attributes[3]
}

static STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: no_physics,
        collision: grounded_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: falling_physics,
        collision: falling_collision,
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
        physics: falling_physics,
        collision: falling_collision,
    },
    // State 4: carried off the floor at rest (itSword_Logic12_EnteredAir).
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: no_animation,
        physics: no_physics,
        collision: sliding_collision,
    },
];

impl ItemLogic for Sword {
    const KIND: ItemKind = ItemKind::Sword;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itSword_Logic12_Spawned (80285338): grabbable (it_8026B390), rising
    /// at xC, then the fall (it_802855F8).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.grabbable = true;
        item.velocity = Vec3::new(0.0, spawn_speed(assets), 0.0);
        start_falling(item, assets);
    }
    /// itSword_Logic12_PickedUp (80285840): the hold state.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
        item.land_count = 0;
    }
    /// itSword_Logic12_Thrown (80285CE4).
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        leave_hand(item, context.assets);
    }
    /// itSword_Logic12_Dropped (80285C1C): as a throw.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        leave_hand(item, context.assets);
    }
    /// itSword_Logic12_EnteredAir (80285E80): state 4 (the rest is visual).
    fn entered_air(_item: &mut ItemCore) {
        unimplemented!("itSword_Logic12_EnteredAir: a resting Beam Sword carried off the floor");
    }
    /// itSword_Logic12_DmgDealt (8028602C): a thrown sword pops back.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::THROWN {
            item.bounce_off_victim(context.victim_bounce);
        }
        false
    }
    /// itSword_Logic12_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// itSword_Logic12_Clanked -> itColl_BounceOffVictim.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itSword_Logic12_HitShield -> itColl_BounceOffVictim.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itSword_Logic12_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// itSword_Logic12_Thrown / _Dropped: grabbable again (it_8026B390), then
/// the thrown state.
fn leave_hand(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, assets);
}

/// it_802855F8 (802855F8): the fall, landings counted afresh.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALL, ANIM_UPDATE, assets);
    item.land_count = 0;
}

/// it_80285424 (80285424): at rest on the floor.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.land_count = 0;
    change(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itSword_UnkMotion3_Phys -> Item_ApplyFallingPhysics: gravity, then the
/// falling spin.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itSword_UnkMotion0_Coll (802855CC): it_8026D62C, off the floor it falls.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        start_falling(item, ctx.assets);
    }
    false
}

/// itSword_UnkMotion3_Coll (802857D8) -> it_8026E15C: bounces off walls and
/// ceilings; a settled landing rests (it_80285424); a thrown sword may
/// break on its first landing.
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne => false,
        AirLanding::Landed => {
            come_to_rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itSword_UnkMotion4_Coll (80285F78) -> it_8026E8C4: on the floor it
/// rests, off it it falls.
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ground_collision_with_slide(ctx.map, ctx.assets) {
        come_to_rest(item, ctx.assets);
    } else {
        start_falling(item, ctx.assets);
    }
    false
}
