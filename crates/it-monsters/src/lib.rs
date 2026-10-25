//! Crate-monster constructors (itzako, itoldkuri, itleadead). No lifecycle
//! callback is admitted by the terminal birth captures.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, FacingConstructor, ItemAnimationContext,
    ItemCollisionContext, ItemCore, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
    MonsterBehavior, MonsterState,
};
use melee_types::ItemKind;

/// it_803F8320: only the constructor's motion0 is available.
pub const KURIBOH_ARTICLE_STATES: [i32; 1] = [-1];
/// it_803F8EB0: constructor motion10 plays article1. Other rows remain unavailable.
pub const REDEAD_ARTICLE_STATES: [i32; 11] = [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 1];
pub const KURIBOH_SPECIAL_ATTRIBUTES: u32 = 4;
pub const REDEAD_SPECIAL_ATTRIBUTES: u32 = 8;

pub struct Kuriboh;
pub struct ReDead;

/// Constructor-row continuation guards: itOldkuri_UnkMotion0_Anim
/// (802D74D0, itoldkuri.c:82) and itLeadead_UnkMotion10_Anim
/// (802E9354, itleadead.c:353); no later row in either state table is admitted.
fn unavailable_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itoldkuri/itleadead: monster Animation beyond the constructor capture")
}
/// itOldkuri_UnkMotion0_Phys (802D74D8, itoldkuri.c:87) /
/// itLeadead_UnkMotion10_Phys (802E935C, itleadead.c:358).
fn unavailable_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {
    unimplemented!("itoldkuri/itleadead: monster Physics beyond the constructor capture")
}
/// itOldkuri_UnkMotion0_Coll (802D7550, itoldkuri.c:102) /
/// itLeadead_UnkMotion10_Coll (802E938C, itleadead.c:365).
fn unavailable_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itoldkuri/itleadead: monster Collision beyond the constructor capture")
}
const UNAVAILABLE: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation: unavailable_animation,
    physics: unavailable_physics,
    collision: unavailable_collision,
};
const REDEAD_ROWS: [ItemStateRow; 11] = {
    let mut rows = [UNAVAILABLE; 11];
    rows[10].animation_id = 1;
    rows
};

/// it_8027B730 (8027B730), before the constructor's own live facing query.
fn begin_kuriboh(item: &mut ItemCore, _: &ItemAssets) {
    begin(
        item,
        MonsterBehavior::Kuriboh {
            walking_speed: 0.0,
            spawn_timer: 0,
        },
    );
}
fn begin_redead(item: &mut ItemCore, _: &ItemAssets) {
    begin(
        item,
        MonsterBehavior::ReDead {
            target: None,
            attack_timer: 0,
            release_timer: 0,
            attempts: 0,
            hurt_offsets: [Vec3::ZERO; 2],
            hurt_radius: 0.0,
            collision_box: Default::default(),
        },
    );
}
fn begin(item: &mut ItemCore, behavior: MonsterBehavior) {
    assert!(
        item.hold_kind == 6
            && item.owner.is_none()
            && item.ground_or_air == melee_types::GroundOrAir::Air
            && item.velocity == Vec3::ZERO,
        "it_8026F5C8: only the unowned airborne zero-velocity crate constructor is witnessed"
    );
    item.scratch = ItemScratch::Monster(MonsterState {
        joint: None,
        joint_index: -1,
        floor_normal: Vec3::new(0.0, 1.0, 0.0),
        interpolation: Vec3::ZERO,
        behavior,
    });
    // it_80275228 / it_80275240, after the shared scratch stores.
    item.pushed_by_fighters = false;
    item.pushed_by_open_palm = false;
}

/// it_802D73F0 -> it_802D747C -> it_802D848C, after the second facing query.
fn finish_kuriboh(item: &mut ItemCore, assets: &ItemAssets) {
    item.slide_mode = 0;
    item.grabbable = false; // xDC8 x15
    item.strikes_kindred_items = false; // it_8027542C
                                        // it_8027CAD8: zero only X/Y, enable owner/kindred hits, then stop spin.
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.reset_spin();
    item.owner = None;
    // it_8027C0A8 returns without a floor; the flat-air spawn is witnessed.
    assert!(
        item.ground_or_air == melee_types::GroundOrAir::Air
            && item
                .collision
                .as_ref()
                .is_none_or(|c| c.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK == 0),
        "it_8027C0A8: grounded monster constructor is not witnessed"
    );
    item.hurt_intangible = false; // it_802756E0
    item.change_motion_with(0, -1, ANIM_UPDATE, assets);
    item.jumped_on = Some(unavailable_jumped_on); // it_802D848C, after state change.
}

/// it_802E8BCC -> it_802E9308. Only the recorded left-facing birth is admitted.
fn finish_redead(item: &mut ItemCore, assets: &ItemAssets) {
    if item.facing != -1.0 {
        unimplemented!("it_802E8BCC: positive-facing ReDead constructor is not witnessed");
    }
    // it_8027C56C: retail 0x8027C668 fmul, 0x8027C66C frsp.
    // M_PI_2 is promoted to double before the product/conversion.
    item.rotation = Vec3::new(
        0.0,
        (core::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32,
        0.0,
    );
    item.slide_mode = 0;
    item.grabbable = false;
    item.strikes_kindred_items = false; // it_8027542C
    melee_mp::set_facing_dir(item.collision.as_mut().expect("initial collision"), -1);
    item.spin_ignores_facing = true;
    let attrs = assets
        .redead_constructor
        .expect("typed ReDead constructor attributes");
    let ItemScratch::Monster(base) = &mut item.scratch else {
        unreachable!("monster scratch")
    };
    // it_80271590 copies only capsule offsets/radius; no stale state/bone bytes.
    let (hurt_offsets, hurt_radius) = assets
        .hurtboxes
        .first()
        .map_or(([Vec3::ZERO; 2], 0.0), |hurt| (hurt.offsets, hurt.radius));
    base.behavior = MonsterBehavior::ReDead {
        target: None,
        attack_timer: 0,
        release_timer: attrs.release_timer,
        attempts: 0,
        hurt_offsets,
        hurt_radius,
        collision_box: item.collision_box,
    };
    // retail 0x802E9338: fmuls; no contraction with the state change.
    item.velocity.x *= attrs.horizontal_scale;
    item.change_motion_with(10, 1, ANIM_UPDATE, assets);
}

/// it_802D8098 (802D8098, itoldkuri.c:429), installed by it_802D848C.
fn unavailable_jumped_on(_: &mut ItemCore) -> bool {
    unimplemented!("it_802D8098: jumped-on monster lifecycle is not witnessed")
}
/// it_2725_Logic9_Destroyed (8027C8B0, itzako.c:494) /
/// itLeadead_Logic1_Destroyed (802EA41C, itleadead.c:849).
fn unavailable_destroyed(_: &mut ItemCore) {
    unimplemented!("itzako/itleadead: destroyed monster lifecycle is not witnessed")
}
/// it_2725_Logic0_DmgReceived (802D7F38, itoldkuri.c:405) /
/// it_802E8CD8 (802E8CD8, itleadead.c:167).
fn unavailable_damage(_: &mut ItemCore, _: &melee_it::ItemEventContext<'_>) -> bool {
    unimplemented!(
        "it_2725_Logic0_DmgReceived/it_802E8CD8: monster damage lifecycle is not witnessed"
    )
}

// NonNULL pickup/drop/throw entries remain unavailable: itOldKuri_Logic0_PickedUp
// (802D7C14, itoldkuri.c:292), it_2725_Logic0_Dropped / Thrown (c:311/324).
impl ItemLogic for Kuriboh {
    const KIND: ItemKind = ItemKind::Kuriboh;
    const STATES: &'static [ItemStateRow] = &[UNAVAILABLE];
    const FACING_CONSTRUCTOR: Option<FacingConstructor> = Some(FacingConstructor {
        before_query: begin_kuriboh,
        after_query: finish_kuriboh,
    });
    fn spawned(_: &mut ItemCore, _: &ItemAssets) {
        unimplemented!("it_802D73F0: required live constructor context")
    }
    fn pickup_possible(_: &ItemCore) -> bool {
        false
    }
    fn picked_up(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) {
        unimplemented!("itOldKuri_Logic0_PickedUp/itLeadead_Logic1_PickedUp: monster pickup lifecycle is not witnessed")
    }
    fn destroyed(item: &mut ItemCore) {
        unavailable_destroyed(item)
    }
    fn damage_received(item: &mut ItemCore, context: &melee_it::ItemEventContext<'_>) -> bool {
        unavailable_damage(item, context)
    }
}
// NonNULL pickup/drop/throw entries remain unavailable: itLeadead_Logic1_PickedUp
// (802EA0E0, itleadead.c:737), itLeadead_Logic1_Dropped / Thrown (c:752/758).
impl ItemLogic for ReDead {
    const KIND: ItemKind = ItemKind::Leadead;
    const STATES: &'static [ItemStateRow] = &REDEAD_ROWS;
    const FACING_CONSTRUCTOR: Option<FacingConstructor> = Some(FacingConstructor {
        before_query: begin_redead,
        after_query: finish_redead,
    });
    fn spawned(_: &mut ItemCore, _: &ItemAssets) {
        unimplemented!("it_802E8BCC: required live constructor context")
    }
    fn pickup_possible(_: &ItemCore) -> bool {
        false
    }
    fn picked_up(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) {
        unimplemented!("itOldKuri_Logic0_PickedUp/itLeadead_Logic1_PickedUp: monster pickup lifecycle is not witnessed")
    }
    fn destroyed(item: &mut ItemCore) {
        unavailable_destroyed(item)
    }
    fn damage_received(item: &mut ItemCore, context: &melee_it::ItemEventContext<'_>) -> bool {
        unavailable_damage(item, context)
    }
}
