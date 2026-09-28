//! Peach's turnip, itpeachturnip.c (802BD2F8..802BD9F0): pulled straight
//! into her hand, thrown with the shared item throws, rebounding off what
//! it hits and gone on the floor.
use crate::{no_collision, no_physics};
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem, TurnipState,
};
use melee_types::ItemKind;

pub struct PeachTurnip;

/// it_803F74A8's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [1, -1, 2, 2, -1];
/// itPeachTurnipAttributes: x0 lifetime, x4 face count, then eight
/// (odds, damage) pairs.
pub const SPECIAL_ATTRIBUTES: u32 = 18;

mod motion {
    /// In the hand straight after the pull.
    pub const HELD: u16 = 0;
    /// Rebounding after a hit: falls through everything until it expires.
    pub const REBOUND: u16 = 1;
    pub const THROWN: u16 = 2;
    pub const DROPPED: u16 = 3;
    /// In a hand again after lying on the ground.
    pub const HELD_AGAIN: u16 = 4;
}
/// it_8026DAA8's result bits a thrown turnip reacts to: a ceiling or wall
/// bounces it, a floor ends it.
const SURFACE_BITS: u32 = 0xE;
const FLOOR_BITS: u32 = 0x3;

/// itPeachTurnipAttributes, read from the special attribute words.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x8[face].x4_damage (an int).
    fn damage(&self, face: i32) -> i32 {
        self.0[3 + 2 * face as usize].to_bits() as i32
    }
}

static STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: held,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: count_down,
        physics: rebound_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: count_down,
        physics: flight_physics,
        collision: flight_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: count_down,
        physics: flight_physics,
        collision: flight_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: held,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for PeachTurnip {
    const KIND: ItemKind = ItemKind::PeachTurnip;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802BD4AC (802BD4AC) for a turnip, before Item_8026AB54: its
    /// lifetime (it_80275158) and the face the pull drew (it_802BD32C, in
    /// the spawn's argument), whose damage the throw will use. xDCD b2
    /// (muted bounces, it_8026BD0C) has no port consumer.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        item.life_timer = a.lifetime();
        item.half_life = a.lifetime() * common.half_life_scale;
        let face = spawn.spawn_argument;
        // it_80274594 after Item_8026AB54 restores the model scale; xDE0
        // keeps it.
        item.scratch = ItemScratch::Turnip(TurnipState {
            face,
            damage: a.damage(face),
            picked_up: false,
            scale: item.scale,
        });
    }
    /// itPeachTurnip_Logic56_PickedUp (802BD5AC): the face, the hold state
    /// (4 once it has been held before), then its first frame.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        pose_face(item);
        let again = turnip(item).picked_up;
        let state = if again {
            motion::HELD_AGAIN
        } else {
            motion::HELD
        };
        change(item, state, ANIM_UPDATE, ctx.assets);
        turnip_mut(item).picked_up = true;
        item.advance_animation(ctx.assets);
    }
    /// itPeachTurnip_Logic56_Thrown (802BD6E4): the flight state, the face,
    /// hitbox 0 at the face's damage (it_80272460), then it_80274484 at the
    /// scale the turnip had when pulled (xDE0), which leaves the hitbox at
    /// its command size.
    fn thrown(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, ctx.assets);
        pose_face(item);
        let damage = turnip(item).damage;
        item.set_hitbox_damage(0, damage as u32);
        let scale = turnip(item).scale;
        item.rescale(scale);
    }
    /// itPeachTurnip_Logic56_Dropped (802BD86C).
    fn dropped(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, ctx.assets);
        pose_face(item);
    }
    /// itPeachTurnip_Logic56_DmgDealt (802BD8AC): it pops back and falls.
    fn damage_dealt(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(ctx.victim_bounce);
        change(item, motion::REBOUND, ANIM_UPDATE, ctx.assets);
        false
    }
    /// itPeachTurnip_Logic56_Clanked.
    fn clanked(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(ctx.victim_bounce);
        false
    }
    /// itPeachTurnip_Logic56_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itPeachTurnip_Logic56_HitShield.
    fn hit_shield(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(ctx.victim_bounce);
        false
    }
    /// itPeachTurnip_Logic56_ShieldBounced -> itColl_BounceOffShield.
    /// itPeachTurnip_Logic56_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(ctx.shield_normal);
        false
    }
}

/// it_80273670(item, 0, xDD8): the face.
fn pose_face(item: &mut ItemCore) {
    let face = turnip(item).face;
    item.pose_article_frame(face as f32);
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[usize::from(motion)], flags, assets);
}

fn turnip(item: &ItemCore) -> &TurnipState {
    match &item.scratch {
        ItemScratch::Turnip(state) => state,
        _ => panic!("turnip scratch"),
    }
}
fn turnip_mut(item: &mut ItemCore) -> &mut TurnipState {
    match &mut item.scratch {
        ItemScratch::Turnip(state) => state,
        _ => panic!("turnip scratch"),
    }
}

/// itPeachturnip_UnkMotion4_Anim: nothing while held.
fn held(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// decrease_lifetimer: gone when the lifetime runs out.
fn count_down(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itPeachturnip_UnkMotion1_Phys -> it_80272860: gravity only.
fn rebound_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itPeachturnip_UnkMotion3_Phys -> Item_ApplyFallingPhysics: gravity, then
/// the falling spin.
fn flight_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itPeachturnip_UnkMotion3_Coll (802BD814): a wall or ceiling bounces it
/// into the rebound (it_80276FC4); a floor ends it.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let bits = item.air_contact_bits(ctx.map);
    if bits & SURFACE_BITS != 0 {
        item.bounce_off_surfaces(bits, ctx.map, ctx.assets);
        change(item, motion::REBOUND, ANIM_UPDATE, ctx.assets);
    } else if bits & FLOOR_BITS != 0 {
        return true;
    }
    false
}
