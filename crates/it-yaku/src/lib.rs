//! The stage material item (ityaku.c, 802E6AEC..802E70A8): an invisible,
//! intangible item a stage creates on one of its JObjs (grMaterial_801C8CFC)
//! to carry that JObj's hitboxes. Its states play the stage's `ALDYakuAll`
//! scripts; the stage switches state (grMaterial_801C8D98 or a direct
//! Item_80268E5C) and the item follows the JObj every physics step.
//!
//! Jungle Japes' Klaptrap carries its bite this way; its callbacks
//! (yaku.x14 grGarden_80203498, x18 and x1C null) do nothing. Yoshi's
//! Island's nine blocks take hits on a capsule the stage sets
//! (grMaterial_801C8DE0); their hit-taken callback (x18,
//! grYorster_80202428) runs in the scene once this kind's DmgReceived has.
use hsd_types::Vec3;
use melee_it::{desc::YAKU_STATES, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct Yaku;

/// it_803F8C8C: every state plays its own article state, with no
/// animation or collision callback and it_2E6A_UnkMotion19_Phys.
static STATES: [ItemStateRow; YAKU_STATES] = {
    let mut rows = [ItemStateRow {
        animation_id: 0,
        animation: no_animation,
        physics: follow_anchor,
        collision: no_collision,
    }; YAKU_STATES];
    let mut state = 0;
    while state < YAKU_STATES {
        rows[state].animation_id = state as i32;
        state += 1;
    }
    rows
};

impl ItemLogic for Yaku {
    const KIND: ItemKind = ItemKind::PokemonRandom;
    const STATES: &'static [ItemStateRow] = &STATES;

    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }

    /// it_2E6A_Logic117_DmgReceived (802E6F7C): the percent resets
    /// (xC9C = 0) and it_8027B798 turns the item to face the hit
    /// (xCCC); the velocity it computes is only the stage callback's.
    fn damage_received(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        item.damage_percent = 0;
        item.facing = item.hit_direction;
        false
    }

    /// it_802E6AEC after Item_80268B18: the model at unit scale, no blast
    /// zone (xDCC b3), intangible (it_802756D0), then state `state`.
    fn spawned(item: &mut ItemCore, assets: &melee_it::desc::ItemAssets) {
        item.scratch = ItemScratch::Yaku(YakuAnchor {
            position: item.position,
            scale: Vec3::new(1.0, 1.0, 1.0),
            rotation: Vec3::ZERO,
        });
        item.model_scale = Vec3::new(1.0, 1.0, 1.0);
        item.blast_zone_checked = false;
        item.hurt_intangible = true;
        item.change_motion_with(0, 0, state_change::ANIM_UPDATE, assets);
    }
}

/// it_802E6AEC's SpawnItem: kind 0xA0 at the JObj's world position, no
/// owner, facing 0.
pub fn spawn(position: Vec3) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: None,
        kind: ItemKind::PokemonRandom,
        // Item_802674AC: It_PKind_Random holds as kind 3.
        hold_kind: 3,
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: Vec3::ZERO,
        facing: 0.0,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        // x44_flag.b0 = arg5 (0 for a JObj anchor).
        initial_collision: false,
        auxiliary_flags: [0; 3],
        ground_or_air: GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

/// The stage moved the JObj: what the next physics step copies.
pub fn set_anchor(item: &mut ItemCore, anchor: YakuAnchor) {
    item.scratch = ItemScratch::Yaku(anchor);
}

/// Item_80268E5C(item, state, ITEM_ANIM_UPDATE) from the stage.
pub fn change_state(item: &mut ItemCore, state: u16, assets: &melee_it::desc::ItemAssets) {
    item.change_motion_with(state, i32::from(state), state_change::ANIM_UPDATE, assets);
}

/// grMaterial_801C8DE0 (0x801C8DE0): the first hurt capsule's offsets
/// and radius.
pub fn set_hurt_capsule(item: &mut ItemCore, offsets: [Vec3; 2], radius: f32) {
    item.hurt_override = Some((offsets, radius));
}

/// grMaterial_801C8E08 / 801C8E28 (it_802756E0 / it_802756D0): the
/// capsules take hits again, or stop.
pub fn set_hurt_enabled(item: &mut ItemCore, enabled: bool) {
    item.hurt_intangible = !enabled;
}

/// grMaterial_801C8E68 (0x801C8E68): `ground_or_air` as the stage sets it.
pub fn set_ground_or_air(item: &mut ItemCore, ground_or_air: GroundOrAir) {
    item.ground_or_air = ground_or_air;
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_2E6A_UnkMotion19_Phys (802E6D80): the JObj's world translation
/// becomes the item's position, and its scale and rotation the model's.
fn follow_anchor(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let ItemScratch::Yaku(anchor) = item.scratch else {
        panic!("stage material item scratch")
    };
    item.position = anchor.position;
    item.model_scale = anchor.scale;
    item.rotation = anchor.rotation;
}
