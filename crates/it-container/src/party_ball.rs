//! Party Ball (itkusudama.c): the witnessed neutral left-facing fall and rest.
//! Opening, carrying and other contacts remain explicit reachability guards.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, AirLanding, EnteredAir, ItemAnimationContext,
    ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::{mp::collide, ItemKind};

pub struct PartyBall;
/// Only REST/FALL's -1 articles are loaded. Retail's other rows have
/// animations 0/1/-1/2/2/3/4; their callbacks fail closed before entry.
/// In particular their color scripts (opcodes 24/25) remain unsupported.
pub const ARTICLE_STATES: [i32; 9] = [-1; 9];
/// itKusudamaAttributes, x0..x2C.
pub const SPECIAL_ATTRIBUTES: u32 = 12;
const REST: u16 = 0;
const FALL: u16 = 2;
/// it_802897C8: snap within ten degrees, otherwise ease by five degrees.
const PITCH_SNAP_DISTANCE: f32 = 0.17453292;
const PITCH_EASING_STEP: f32 = 0.08726646;
/// it_8028A190's tail, after Item_80268E5C flushes queued joint effects.
const REST_PITCH: u8 = 0;
const UNSUPPORTED: ItemStateRow = ItemStateRow {
    animation_id: -1,
    animation: unsupported_animation,
    physics: unsupported_physics,
    collision: unsupported_collision,
};
static STATES: [ItemStateRow; 9] = [
    ItemStateRow {
        animation_id: -1,
        animation: ease_pitch,
        physics: crate::no_physics,
        collision: rest_collision,
    },
    UNSUPPORTED,
    ItemStateRow {
        animation_id: -1,
        animation: ease_pitch,
        physics: fall_physics,
        collision: fall_collision,
    },
    UNSUPPORTED,
    UNSUPPORTED,
    UNSUPPORTED,
    UNSUPPORTED,
    UNSUPPORTED,
    UNSUPPORTED,
];

impl ItemLogic for PartyBall {
    const KIND: ItemKind = ItemKind::Kusudama;
    const STATES: &'static [ItemStateRow] = &STATES;
    const ENTERED_AIR: Option<EnteredAir> = Some(unsupported_entered_air);
    /// itKusudama_Logic4_Spawned (80289784), then it_8028A3CC.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        if item.facing != -1.0 {
            unimplemented!("itKusudama_Logic4_Spawned: positive facing, unwitnessed");
        }
        if item.owner.is_some()
            || item.held
            || item.velocity.x != 0.0
            || item.velocity.y != 0.0
            || item.velocity.z != 0.0
        {
            unimplemented!("itKusudama_Logic4_Spawned: owned/held/non-neutral fall, unwitnessed");
        }
        item.ecb_follows_rotation = false;
        item.command_variables[..3].fill(0);
        item.grabbable = false;
        item.change_motion_with(FALL, -1, ANIM_UPDATE, assets);
        set_rest_pitch(item);
    }
    fn resumed(item: &mut ItemCore, step: u8, _assets: &ItemAssets, _rng: &mut gekko_math::HsdRng) {
        assert_eq!(step, REST_PITCH);
        assert_eq!(item.motion, REST);
        set_rest_pitch(item);
    }
    fn picked_up(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) {
        unimplemented!("itKusudama_Logic4_PickedUp (itkusudama.c:493), unwitnessed")
    }
    fn thrown(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) {
        unimplemented!("itKusudama_Logic4_Thrown (itkusudama.c:505), unwitnessed")
    }
    fn dropped(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) {
        unimplemented!("itKusudama_Logic4_Dropped (itkusudama.c:548), unwitnessed")
    }
    fn destroyed(_: &mut ItemCore) {
        unimplemented!("Party Ball destruction, unwitnessed")
    }
    fn damage_dealt(_: &mut ItemCore, _: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKusudama_Logic4_DmgDealt (itkusudama.c:676), unwitnessed")
    }
    fn damage_received(_: &mut ItemCore, _: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKusudama_Logic4_DmgReceived (itkusudama.c:709), unwitnessed")
    }
    fn clanked(_: &mut ItemCore, _: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKusudama_Logic4_Clanked (itkusudama.c:690), unwitnessed")
    }
    fn reflected(_: &mut ItemCore, _: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKusudama_Logic4_Reflected (itkusudama.c:704), unwitnessed")
    }
    fn hit_shield(_: &mut ItemCore, _: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKusudama_Logic4_HitShield (itkusudama.c:699), unwitnessed")
    }
}

/// it_8028A190: the literal M_PI_2_F multiply by -1 (8028A1EC), no FMA.
fn set_rest_pitch(item: &mut ItemCore) {
    assert_eq!(item.facing, -1.0);
    item.rotation.x = -std::f32::consts::FRAC_PI_2;
}
/// it_802897C8 (802897C8): easing remains active in FALL and throughout REST.
/// Retail subtracts, compares the absolute difference, then adds/subtracts
/// 0.08726646f separately; there are no fused multiply-add instructions.
fn ease_pitch(item: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    assert_eq!(item.facing, -1.0);
    let target = -std::f32::consts::FRAC_PI_2;
    let difference = item.rotation.x - target;
    let distance = if difference < 0.0 {
        -difference
    } else {
        difference
    };
    item.rotation.x = if distance > PITCH_SNAP_DISTANCE {
        if item.rotation.x < target {
            item.rotation.x + PITCH_EASING_STEP
        } else {
            item.rotation.x - PITCH_EASING_STEP
        }
    } else {
        target
    };
    false
}
/// itKusudama_UnkMotion2_Phys -> it_80272860 (8028A4E8).
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}
fn guard_contacts(item: &ItemCore) {
    let collision = item.collision.as_ref().expect("Party Ball collision");
    if collision.env_flags as u32
        & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK | collide::CEILING_MASK)
        != 0
    {
        unimplemented!("itKusudama_UnkMotion2_Coll: wall/ceiling contact, unwitnessed");
    }
    if collision.env_flags as u32 & collide::FLOOR_MASK != 0
        && (collision.floor.normal.x != 0.0 || collision.floor.normal.y <= 0.0)
    {
        unimplemented!("itKusudama_UnkMotion2_Coll: sloped floor, unwitnessed");
    }
}
/// itKusudama_UnkMotion2_Coll -> it_8026E15C (8028A518).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("Party Ball landing RNG");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    guard_contacts(item);
    match landing {
        AirLanding::Airborne => {}
        AirLanding::Landed => {
            // it_8028A190: velocity/grabbability, then change motion. That
            // call flushes the bounce JointEffect using the pre-pitch root.
            item.velocity = Vec3::ZERO;
            item.grabbable = true;
            item.change_motion_with(REST, -1, ANIM_UPDATE, ctx.assets);
            item.defer_callback(REST_PITCH);
        }
        AirLanding::Sliding => unimplemented!("Party Ball sliding, unwitnessed"),
        AirLanding::Broken => unimplemented!("Party Ball broken landing, unwitnessed"),
    }
    false
}
/// itKusudama_UnkMotion0_Coll -> it_8026D62C (8028A2C4).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        unimplemented!("itKusudama_UnkMotion0_Coll: floor departure, unwitnessed");
    }
    guard_contacts(item);
    false
}
fn unsupported_entered_air(_: &mut ItemCore, _: &ItemAssets) {
    unimplemented!("Party Ball EnteredAir/sliding, unwitnessed")
}
fn unsupported_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("Party Ball motion 1/3..8 (itkusudama.c), unwitnessed")
}
fn unsupported_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {
    unimplemented!("Party Ball motion 1/3..8 (itkusudama.c), unwitnessed")
}
fn unsupported_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("Party Ball motion 1/3..8 (itkusudama.c), unwitnessed")
}
