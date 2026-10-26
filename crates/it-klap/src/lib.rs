//! it/kinds/itklap.c: Kongo Jungle's Klaptrap, born on the path rider
//! (map 11). It rides until its model's child passes the screen plane
//! (z >= 0), then falls turning upright; a hit sends it tumbling. Either
//! way it falls out of the blast zone, and its end removes the rider
//! (itKlap_Logic10_Destroyed, the scene's half). Each time its looping
//! animation wraps it may switch between riding and biting. The barrel's
//! catch (row 3) and a rider gone first stay guarded.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemCollisionContext,
    ItemCore, ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
    MonsterBehavior, MonsterState, StageParent,
};
use melee_types::ItemKind;

pub struct Klaptrap;
pub const ARTICLE_STATES: [i32; 5] = [0, 1, 2, 3, 2];
/// itKlap_UnkMotion1_Phys: at a wrap, HSD_Randi(100) below this switches.
const SWITCH_ODDS: i32 = 30;
/// Attached body lies five units below the Ground root (itklap.c:131).
const PARENT_HEIGHT_OFFSET: f64 = 5.0;
/// itKlap_UnkMotion2_Phys: upright (pi/2 about x) ...
const UPRIGHT: f32 = 1.570_796_4;
/// ... approached two degrees a frame.
const UPRIGHT_STEP: f32 = 0.034_906_585;
/// it_802E1E94: a tumble turns up to five degrees a frame each way
/// ((r - 0.5) * 10 degrees).
const TUMBLE_SPIN: f32 = 0.174_532_92;
/// The model's child whose world position decides the detach.
const CHILD_BONE: usize = 1;

mod motion {
    pub const RIDE: u16 = 0;
    pub const BITE: u16 = 1;
    pub const FALL: u16 = 2;
    pub const TUMBLE: u16 = 4;
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[usize::from(motion)], ANIM_UPDATE, assets);
}

/// itKlap_UnkMotion1/2/3/4_Anim: never end the item.
fn animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
/// itKlap_UnkMotion1_Phys (802E1970, itklap.c:106): when the animation
/// wraps (lb_8000B134), three times in ten it switches between riding
/// (it_802E1C4C to the bite) and biting (it_802E1930 back), velocity zeroed.
fn ride_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if !item.article_animation_rewound(ctx.assets) {
        return;
    }
    let mut rng = ctx.rng.get();
    let draw = rng.randi(100);
    ctx.rng.set(rng);
    if draw < SWITCH_ODDS {
        item.velocity.y = 0.0;
        item.velocity.x = 0.0;
        let next = if item.motion == motion::RIDE {
            motion::BITE
        } else {
            motion::RIDE
        };
        change(item, next, ctx.assets);
    }
}
/// itKlap_UnkMotion1_Coll (802E19FC, itklap.c:119): follow the rider five
/// below its root, then let go once the model's child is at or past z 0.
fn ride_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
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
    let child = child_position(item, ctx.assets);
    if child.z >= 0.0 {
        detach(item, child, ctx.assets);
    }
    false
}
/// lb_8000B1CC on the model's child.
fn child_position(item: &ItemCore, assets: &ItemAssets) -> Vec3 {
    let matrix = assets
        .pose
        .as_ref()
        .expect("Klaptrap authored pose")
        .bone_matrix(item.article_state, item.pose_steps, CHILD_BONE, item.root_srt());
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}
/// it_802E1C84 (802E1C84): still, checked against the blast zones
/// (xDCC b3), at the child's place, falling.
fn detach(item: &mut ItemCore, child: Vec3, assets: &ItemAssets) {
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    item.blast_zone_checked = true;
    item.position = child;
    change(item, motion::FALL, assets);
}
/// itKlap_UnkMotion2_Phys (802E1D0C): gravity, and the body turns upright.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.y -= ctx.assets.fall_acceleration;
    let mut rotation = item.rotation.x;
    let difference = UPRIGHT - rotation;
    if difference > UPRIGHT_STEP {
        rotation += UPRIGHT_STEP;
    } else if difference < -UPRIGHT_STEP {
        rotation -= UPRIGHT_STEP;
    } else {
        rotation = UPRIGHT;
    }
    item.rotation.x = rotation;
}
/// itKlap_UnkMotion2_Coll / 4_Coll: no map collision.
fn no_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    false
}
/// itKlap_UnkMotion4_Phys (802E1F78): gravity and the tumble's turns,
/// each wrapped at a full turn (compared in double: `2 * M_PI`).
fn tumble_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    const TURN: f64 = 2.0 * core::f64::consts::PI;
    item.velocity.y -= ctx.assets.fall_acceleration;
    let [spin_y, spin_z] = spins(item);
    let wrap = |angle: f32| -> f32 {
        if f64::from(angle) > TURN {
            (f64::from(angle) - TURN) as f32
        } else if f64::from(angle) < -TURN {
            (f64::from(angle) + TURN) as f32
        } else {
            angle
        }
    };
    item.rotation.y = wrap(item.rotation.y + spin_y);
    item.rotation.z = wrap(item.rotation.z + spin_z);
}
// it_803F8940 row 3 (the barrel's hold, it_802E20D8, guarded at the catch).
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
    rows[motion::RIDE as usize] = ItemStateRow {
        animation_id: 0,
        animation,
        physics: ride_physics,
        collision: ride_collision,
    };
    rows[motion::BITE as usize] = ItemStateRow {
        animation_id: 1,
        animation,
        physics: ride_physics,
        collision: ride_collision,
    };
    rows[motion::FALL as usize] = ItemStateRow {
        animation_id: 2,
        animation,
        physics: fall_physics,
        collision: no_collision,
    };
    rows[motion::TUMBLE as usize] = ItemStateRow {
        animation_id: 2,
        animation,
        physics: tumble_physics,
        collision: no_collision,
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
        behavior: MonsterBehavior::Klaptrap {
            parent: None,
            spins: [0.0; 2],
        },
    });
    item.pushed_by_fighters = false;
    item.pushed_by_open_palm = false;
    item.facing = 0.0;
    item.slide_mode = 0;
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    change(item, motion::RIDE, assets);
}
fn behavior(item: &ItemCore) -> (&Option<StageParent>, &[f32; 2]) {
    let ItemScratch::Monster(MonsterState {
        behavior: MonsterBehavior::Klaptrap { parent, spins },
        ..
    }) = &item.scratch
    else {
        panic!("Klaptrap scratch")
    };
    (parent, spins)
}
fn parent(item: &ItemCore) -> Option<StageParent> {
    *behavior(item).0
}
fn spins(item: &ItemCore) -> [f32; 2] {
    *behavior(item).1
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
    let MonsterBehavior::Klaptrap { parent: slot, .. } = &mut state.behavior else {
        panic!("Klaptrap scratch")
    };
    *slot = Some(parent);
    item.pushable = false; // it_8026BDB4: xDC8 x1A.
}
/// it_2725_Logic10_DmgReceived (802E1E5C, itklap.c:203) -> it_802E1E94:
/// any hit launches it (it_8027B798) tumbling, turning a random amount
/// each frame about y and z (two HSD_Randf; retail 802E1F04 / 1F2C: fsubs
/// then fmuls).
fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    item.blast_zone_checked = true;
    item.enter_air();
    let (velocity, _) = item.knockback_launch(&context.launch);
    item.velocity = velocity;
    item.enter_air();
    let rng = context.rng.expect("it_802E1E94 draws");
    let mut r = rng.get();
    let spin_y = (r.randf() - 0.5) * TUMBLE_SPIN;
    let spin_z = (r.randf() - 0.5) * TUMBLE_SPIN;
    rng.set(r);
    if let ItemScratch::Monster(MonsterState {
        behavior: MonsterBehavior::Klaptrap { spins, .. },
        ..
    }) = &mut item.scratch
    {
        *spins = [spin_y, spin_z];
    }
    change(item, motion::TUMBLE, context.assets);
    false
}
impl Klaptrap {
    /// The Ground a Klaptrap rides (itKlap x20).
    pub fn stage_parent_of(item: &ItemCore) -> Option<StageParent> {
        parent(item)
    }
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
    /// itKlap_Logic10_Destroyed (802E1874, itklap.c:61): the scene removes
    /// the rider it names (grKongo_801D8058) before the item's GObj goes.
    fn destroyed(_: &mut ItemCore) {}
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        damage_received(item, context)
    }
}
