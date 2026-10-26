//! The Starman (It_Kind_Star, itstar.c 80284634..8028482C): it bounces
//! along, turning at walls; a fighter it touches becomes invincible for
//! its x0 frames (ftColl_80077C60's power-up branch, ftColl_8007B7FC), and
//! the Starman is gone (its DmgDealt).
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

/// It_Kind_Star.
pub struct Starman;

/// it_803F57A0's anim_id column.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// The Starman's attributes: x0 (the invincibility's frames), x10 / x14
/// (the bounce's speed).
pub const SPECIAL_ATTRIBUTES: u32 = 6;

/// Item_80268E5C's ITEM_UNK_0x1 (the bounce keeps its hitboxes too).
const UNK_0X1: u32 = 1;
/// Item_8026AE84's bounce sound.
const BOUNCE_SOUND: u32 = 0xFA;

/// it_80272818 (80272818): the frames of invincibility the Starman gives
/// (its attributes' x0).
pub fn invincibility_frames(assets: &ItemAssets) -> i32 {
    assets.special_attributes[0].to_bits() as i32
}

fn bounce_speed(assets: &ItemAssets) -> (f32, f32) {
    (assets.special_attributes[4], assets.special_attributes[5])
}

/// xDD4: the direction it bounces along.
fn direction(item: &ItemCore) -> f32 {
    match item.scratch {
        ItemScratch::Starman { direction, .. } => direction,
        _ => unreachable!("a Starman without its direction"),
    }
}

fn set_direction(item: &mut ItemCore, value: f32) {
    if let ItemScratch::Starman { direction, .. } = &mut item.scratch {
        *direction = value;
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation: no_animation,
    physics: gravity_physics,
    collision: bounce_collision,
}];

impl ItemLogic for Starman {
    const KIND: ItemKind = ItemKind::Star;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// No PickedUp callback: Item_IsGrabbable never admits it.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itStar_Logic10_Spawned (80284634) before its draw: the touch flag
    /// (it_8026B390) and state 0 (it_802846D4).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.grabbable = true;
        item.scratch = ItemScratch::Starman {
            direction: 1.0,
            frames: invincibility_frames(assets),
        };
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    }
    /// itStar_Logic10_Spawned's draw, the creation's last: HSD_Randi(2)
    /// picks the direction (+1 on nonzero), and the bounce's speed along
    /// it (x10 * direction, fmuls) and up (x14).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        _common: &ItemCommonData,
        _spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let direction = if rng.randi(2) != 0 { 1.0 } else { -1.0 };
        set_direction(item, direction);
        let (along, up) = bounce_speed(assets);
        item.velocity = hsd_types::Vec3::new(along * direction, up, 0.0);
    }
    /// itStar_Logic10_DmgDealt: the touch was its power-up; it is gone.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// itStar_UnkMotion0_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itStar_UnkMotion0_Coll (80284734): an airborne pass (it_8026DA08); a
/// floor bounces it up again (x10 * direction, fmuls 80284784; x14) and
/// restarts state 0 keeping its hitboxes (flags 0x11); a wall turns it
/// (fneg of the direction and the speed, guarded); a ceiling stops its
/// rise (guarded). Any contact plays the bounce sound.
fn bounce_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let mut touched = false;
    if item.air_pass(ctx.map) {
        let (along, up) = bounce_speed(ctx.assets);
        item.velocity.x = along * direction(item);
        item.velocity.y = up;
        item.change_motion_with(0, ARTICLE_STATES[0], UNK_0X1 | HIT_PRESERVE, ctx.assets);
        touched = true;
    }
    if item.wall_bits() != 0 {
        unimplemented!("itStar_UnkMotion0_Coll (itstar.c:66-70): a Starman turned by a wall is not witnessed");
    }
    if item.ceiling_bits() != 0 {
        unimplemented!("itStar_UnkMotion0_Coll (itstar.c:71-76): a Starman under a ceiling is not witnessed");
    }
    if touched {
        item.sound_requests.push(BOUNCE_SOUND);
    }
    false
}
