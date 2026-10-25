//! it/kinds/itklap.c: the attached Kongo Klaptrap riding its log. Its looping
//! bite never ends, so the row switch and the falls stay guarded.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, MonsterBehavior, MonsterState,
    StageParent,
};
use melee_types::ItemKind;

pub struct Klaptrap;
pub const ARTICLE_STATES: [i32; 5] = [0, 1, 2, 3, 2];
/// Attached body lies five units below the Ground root (itklap.c:131).
const PARENT_HEIGHT_OFFSET: f64 = 5.0;

/// itKlap_UnkMotion1_Anim (802E1968, itklap.c:101): never ends the item.
fn animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
/// itKlap_UnkMotion1_Phys (802E1970, itklap.c:106).
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.article_animation_ended(ctx.assets) {
        unimplemented!("itKlap_UnkMotion1_Phys: ended animation/Randi100 transition");
    }
}
/// itKlap_UnkMotion1_Coll (802E19FC, itklap.c:119).
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let expected =
        parent(item).unwrap_or_else(|| unimplemented!("it_802E1C84: NULL Klaptrap Ground parent"));
    let view = ctx
        .stage_parent
        .expect("Klaptrap collision requires live Ground parent");
    assert_eq!(view.parent, expected);
    let mut position = view.translation;
    // retail 802E1A94/1A9C: double fsub, frsp.
    position.y = (f64::from(position.y) - PARENT_HEIGHT_OFFSET) as f32;
    item.position = position;
    item.set_root_translation(position);
    item.rotation = view.rotation;
    let matrix = ctx
        .assets
        .pose
        .as_ref()
        .expect("Klaptrap authored pose")
        .bone_matrix(item.article_state, item.pose_steps, 1, item.root_srt());
    if matrix.0[2][3] >= 0.0 {
        unimplemented!("it_802E1C84: Klaptrap detaches at child Z >= 0");
    }
    false
}
// it_803F8940 (itklap.c:18..48) maps the refused rows to
// itKlap_UnkMotion1/2/3/4_Phys and corresponding Coll bodies; only row0
// has the complete born-tick physics/collision witness.
fn unavailable_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {
    unimplemented!("itklap.c: unwitnessed Klaptrap motion physics")
}
fn unavailable_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itklap.c: unwitnessed Klaptrap motion collision")
}
const UNAVAILABLE: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation,
    physics: unavailable_physics,
    collision: unavailable_collision,
};
const ROWS: [ItemStateRow; 5] = {
    let mut rows = [UNAVAILABLE; 5];
    rows[0] = ItemStateRow {
        animation_id: 0,
        animation,
        physics,
        collision,
    };
    rows
};
/// it_802E1820 (itklap.c:52), with it_8027B730's actual shared scratch stores.
fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
    assert!(
        item.owner.is_none() && item.hold_kind == 4,
        "Klaptrap stage birth context"
    );
    item.scratch = ItemScratch::Monster(MonsterState {
        joint: None,
        joint_index: -1,
        floor_normal: Vec3::new(0.0, 1.0, 0.0),
        turn_frames: 0,
        bounces: 0,
        last_voice: 0,
        behavior: MonsterBehavior::Klaptrap { parent: None },
    });
    item.pushed_by_fighters = false;
    item.pushed_by_open_palm = false;
    item.facing = 0.0;
    item.slide_mode = 0;
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    item.change_motion_with(0, 0, melee_it::state_change::ANIM_UPDATE, assets);
}
fn parent(item: &ItemCore) -> Option<StageParent> {
    let ItemScratch::Monster(MonsterState {
        behavior: MonsterBehavior::Klaptrap { parent },
        ..
    }) = &item.scratch
    else {
        panic!("Klaptrap scratch")
    };
    *parent
}
/// it_8027B5B0 tail, then it_802E18B4 publishes its Ground parent.
pub fn attach_parent(item: &mut ItemCore, parent: StageParent) {
    let ItemScratch::Monster(state) = &mut item.scratch else {
        panic!("Klaptrap scratch")
    };
    state.joint = None;
    state.joint_index = -1;
    state.last_voice = 0;
    item.collision
        .as_mut()
        .expect("Klaptrap collision")
        .x34_flags
        .b1234 = 3;
    let MonsterBehavior::Klaptrap { parent: slot } = &mut state.behavior else {
        panic!("Klaptrap scratch")
    };
    *slot = Some(parent);
    item.pushable = false; // it_8026BDB4: xDC8 x1A.
}
impl ItemLogic for Klaptrap {
    const KIND: ItemKind = ItemKind::Klap;
    const STATES: &'static [ItemStateRow] = &ROWS;
    const CONSTRUCTOR_READS_MAP: bool = true;
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        spawned(item, assets)
    }
    fn stage_parent(item: &ItemCore) -> Option<StageParent> {
        parent(item)
    }
    fn pickup_possible(_: &ItemCore) -> bool {
        false
    }
    fn destroyed(_: &mut ItemCore) {
        unimplemented!("itKlap_Logic10_Destroyed (802E1874, itklap.c:64): release Ground parent")
    }
    fn damage_received(_: &mut ItemCore, _: &ItemEventContext<'_>) -> bool {
        unimplemented!("it_2725_Logic10_DmgReceived (802E1E5C, itklap.c:203)")
    }
}
