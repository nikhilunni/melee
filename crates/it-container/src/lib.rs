//! Random item containers: the Capsule (itcapsule.c), with what every
//! container shares: opening into the contents table (it_8026F3D4, which
//! the scene runs on an [`ItemEvent::Release`]) or exploding.
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ContainerState, ItemAnimationContext,
    ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext, ItemPhysicsContext, ItemScratch,
};

mod capsule;
pub use capsule::Capsule;
mod barrel;
pub use barrel::Barrel;
/// The Barrel's article tables, for the scene's asset loading.
pub mod barrel_assets {
    pub use crate::barrel::{ARTICLE_STATES, SPECIAL_ATTRIBUTES};
}
mod crate_box;
pub use crate_box::Box;
/// The Box's article tables, for the scene's asset loading.
pub mod box_assets {
    pub use crate::crate_box::{ARTICLE_STATES, SPECIAL_ATTRIBUTES};
}
/// The Capsule's article tables, for the scene's asset loading.
pub mod capsule_assets {
    pub use crate::capsule::{ARTICLE_STATES, SPECIAL_ATTRIBUTES};
}

/// it_80272C08's efSync_Spawn id: the blast.
const EXPLOSION_EFFECT: u16 = 0x410;
/// Item_80268E5C(gobj, explosion, 2): the blast's state change.
const EXPLOSION_MOTION_FLAGS: u32 = ANIM_UPDATE;

/// What opening a container did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opening {
    Opened,
    Exploded,
}

fn container(item: &ItemCore) -> &ContainerState {
    match &item.scratch {
        ItemScratch::Container(state) => state,
        _ => unreachable!("a container's scratch"),
    }
}
fn container_mut(item: &mut ItemCore) -> &mut ContainerState {
    match &mut item.scratch {
        ItemScratch::Container(state) => state,
        _ => unreachable!("a container's scratch"),
    }
}

/// The event callback's random stream, for a callback that draws.
fn with_event_rng<R>(context: &ItemEventContext<'_>, f: impl FnOnce(&mut HsdRng) -> R) -> R {
    let cell = context.rng.expect("a container's event callback draws");
    let mut rng = cell.get();
    let result = f(&mut rng);
    cell.set(rng);
    result
}

/// The open-or-explode draw the Capsule, Box and Barrel make
/// (it_8027CF30): HSD_Randi(`odds`) == 0 explodes, otherwise `contents`
/// items come out (it_8026F3D4) and the container goes.
fn open_or_explode(
    item: &mut ItemCore,
    assets: &ItemAssets,
    rng: &mut HsdRng,
    odds: i32,
    contents: u8,
    explode: impl FnOnce(&mut ItemCore, &ItemAssets),
) -> Opening {
    if rng.randi(odds) == 0 {
        explode(item, assets);
        container_mut(item).opened = false;
        Opening::Exploded
    } else {
        release(item, contents);
        container_mut(item).opened = true;
        Opening::Opened
    }
}

/// it_8026F3D4 (8026F3D4) as a request: `count` items at the container's
/// ECB centre (it_8026BB88), each facing from its position on the stage
/// plane (it_8026F5C8).
fn release(item: &mut ItemCore, count: u8) {
    let ecb = item.collision.as_ref().expect("a container's collision").ecb;
    // it_8026BB88: 0.5 * (top + bottom), then added to the position.
    let rise = 0.5 * (ecb.top.y + ecb.bottom.y);
    let position = Vec3::new(item.position.x, item.position.y + rise, item.position.z);
    let previous_position = Vec3::new(item.position.x, item.position.y, 0.0);
    item.events.push(ItemEvent::Release {
        count,
        same_kind: false,
        position,
        previous_position,
    });
}

/// What a container's blast shares (it_8027CFE8, the Box's it_80286AA4):
/// no longer grabbable (it_8026B3A8), the model hidden, the blast effect
/// (it_80272C08, which also ends its hitlag), hitting its owner and kin
/// (it_80275444), taking no hits (it_802756D0), the explosion lifetime
/// without a destroy effect (it_8027518C) and a radial gust
/// (lb_800119DC). How it stops and its state change are the kind's.
fn explode(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    item.hidden = true;
    item.events.push(ItemEvent::Effect {
        id: EXPLOSION_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.hurt_intangible = true;
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.mark_exploding();
    // lb_800119DC(&pos, 0x78, 1.0, 0.02, 60 * M_PI / 180).
    item.events.push(ItemEvent::Gust {
        center: item.position,
        frames: 0x78,
        strength: 1.0,
        decay: 0.02,
        phase_step: std::f32::consts::FRAC_PI_3,
    });
}

/// it_802751D8: the explosion's lifetime.
fn explosion_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
