//! Final Cutter's beam (It_Kind_Kirby_CBeam), itkirbycutterbeam.c
//! (8029BAB8..8029C4D4).
//!
//! Kirby's landing sends it out ahead of him (it_8029BAB8). It starts on
//! the floor and runs along it at the floor's angle, slowing by attribute
//! xC each frame; past an edge it flies on level with what speed it has.
//! Its lifetime, a wall in its way, a clank, a shield or an absorber end
//! it; a hit does not. A reflector or a reflecting shield turns it round.
//!
//! Recorded on Final Destination and on Yoshi's Story (the wall from its
//! cloud, the descent down its slopes). What those scenes do not reach
//! fails closed: a landing from the air, a clank, an absorber and a
//! glancing shield.
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    CutterBeamState, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_lb::trigf::atan2f;
use melee_types::{mp::collide, GroundOrAir, ItemKind};

pub struct CutterBeam;

/// ftData.x48_items index (ftKb_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;
/// it_803F6798's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itKirbyCutterBeamAttributes: four words.
pub const SPECIAL_ATTRIBUTES: u32 = 4;

/// The speed never drops below this (retail @191).
const MINIMUM_SPEED: f32 = 0.01;
/// The model's child is hidden for the last frames of the lifetime
/// (retail @192).
const HIDDEN_FRAMES: f32 = 5.0;
/// A descent along the floor gains this fraction of itself (retail @190).
const DESCENT_BOOST: f32 = 0.05;
/// Leaving the floor lifts the beam and its collision start by this much
/// (retail @236).
const LIFT_OFF: f32 = 0.001;

/// itKirbyCutterBeamAttributes (itkirbycutterbeam.h).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the launch speed.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x8: the lifetime, in frames.
    fn lifetime(&self) -> f32 {
        self.0[2]
    }
    /// xC: the speed lost each frame.
    fn deceleration(&self) -> f32 {
        self.0[3]
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

fn state(item: &mut ItemCore) -> &mut CutterBeamState {
    match &mut item.scratch {
        ItemScratch::CutterBeam(state) => state,
        _ => panic!("Cutter beam scratch missing"),
    }
}

/// The velocity along the slope (8029BBDC..8029BBFC and 8029BD10..8029BD2C:
/// two fmuls for x, fneg then fmuls for y).
fn slope_velocity(item: &mut ItemCore) {
    let CutterBeamState { angle, speed, .. } = *state(item);
    item.velocity.x = item.facing * (speed * gekko_math::msl::cosf(angle));
    item.velocity.y = -speed * gekko_math::msl::sinf(angle);
}

/// The floor's angle signed by the facing (8029BE40: fmuls).
fn floor_angle(item: &ItemCore) -> f32 {
    let normal = item
        .collision
        .as_ref()
        .expect("Cutter beam map collision")
        .floor
        .normal;
    item.facing * atan2f(normal.x, normal.y)
}

/// The angle brought into 0..=tau, a turn at a time in double precision
/// (8029C078..8029C0B4: fadd / fsub, frsp).
fn wrap_angle(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + std::f64::consts::TAU) as f32;
    }
    while f64::from(angle) > std::f64::consts::TAU {
        angle = (f64::from(angle) - std::f64::consts::TAU) as f32;
    }
    angle
}

/// The angle a turned beam takes: the floor's on the ground; in the air the
/// velocity's direction, half a turn on when it faces left (8029C038..68:
/// the double 0 or M_PI plus atan2f, rounded), then wrapped.
fn turned_angle(item: &ItemCore) -> f32 {
    let angle = if item.ground_or_air == GroundOrAir::Ground {
        floor_angle(item)
    } else {
        let base = if item.facing == 1.0 {
            0.0
        } else {
            std::f64::consts::PI
        };
        (base + f64::from(atan2f(item.velocity.y, item.velocity.x))) as f32
    };
    wrap_angle(angle)
}

/// HSD_JObjSetRotationX(angle) and HSD_JObjSetRotationY(M_PI_2 * facing)
/// on the model root (8029C154: fmul in double, frsp).
fn turn_model(item: &mut ItemCore) {
    let angle = turned_angle(item);
    state(item).angle = angle;
    item.rotation.x = angle;
    item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
}

/// it_3F2F.c's Logic7 row.
impl ItemLogic for CutterBeam {
    const KIND: ItemKind = ItemKind::KirbyCBeam;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_8029BAB8 ends with it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    /// it_8029BB90 (8029BB90) once Item_80268B18 returns: the owner's root
    /// angle, the attribute speed along it, the lifetime, motion 0, and the
    /// beam counts as grounded. The angle (ftKb_AttackDashAir_800F21C0,
    /// Kirby's root X rotation) rides in `spawn_argument` as float bits:
    /// SpawnItem.x40, which this kind's spawn leaves 0.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        item.scratch = ItemScratch::CutterBeam(CutterBeamState {
            angle: f32::from_bits(spawn.spawn_argument as u32),
            speed: a.speed(),
            ..Default::default()
        });
        slope_velocity(item);
        item.velocity.z = 0.0;
        // it_80275158: both timers.
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
        // it_802762B0.
        item.land_on_floor();
        let (facing, position) = (item.facing, item.position);
        let s = state(item);
        // 8029BC38: fmuls.
        s.direction = Vec3::new(
            facing * gekko_math::msl::sinf(s.angle),
            gekko_math::msl::cosf(s.angle),
            0.0,
        );
        s.previous_position = position;
    }
    /// itKirbyCutterBeam_Logic7_DmgDealt: the beam goes on through.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    /// it_2725_Logic7_Reflected (8029BFCC): facing and velocity reversed
    /// (three fneg), then the model turned to the new course.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.facing = -item.facing;
        item.velocity.x = -item.velocity.x;
        item.velocity.y = -item.velocity.y;
        turn_model(item);
        false
    }
    /// itKirbyCutterBeam_Logic7_Clanked ends the beam. No recorded hitbox
    /// has clanked with it.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKirbyCutterBeam_Logic7_Clanked (itkirbycutterbeam.c:162-165)")
    }
    /// itKirbyCutterBeam_Logic7_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itKirbyCutterBeam_Logic7_Absorbed ends the beam. No absorber has met
    /// it.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itKirbyCutterBeam_Logic7_Absorbed (itkirbycutterbeam.c:167-170)")
    }
    /// it_2725_Logic7_ShieldBounced (8029C208): the velocity mirrored off
    /// the shield and the model turned as a reflection turns it. Only an
    /// airborne item glances off a shield, and the beam is airborne only
    /// past a floor's end, where no shield has stood.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("it_2725_Logic7_ShieldBounced (itkirbycutterbeam.c:205-243)")
    }
}

/// itKirbycutterbeam_UnkMotion0_Anim (8029BC88) -> it_80273130: the
/// lifetime counts down; the beam ends when it is spent.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itKirbycutterbeam_UnkMotion0_Phys (8029BCA8): the velocity along the
/// slope at the current speed, a descent on the floor a twentieth faster
/// (retail 0x8029BD54: fmadds), then the speed worn down to its minimum.
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let position = item.position;
    state(item).previous_position = position;
    slope_velocity(item);
    if item.ground_or_air == GroundOrAir::Ground && item.velocity.y < 0.0 {
        // retail 0x8029BD54: fmadds(@190, vel.y, vel.y), a descent sped up
        // by a twentieth (itkirbycutterbeam.c:93-97).
        item.velocity.y =
            gekko_math::fma::fmadds(DESCENT_BOOST, item.velocity.y, item.velocity.y);
    }
    item.velocity.z = 0.0;
    let deceleration = Attributes(&ctx.assets.special_attributes).deceleration();
    let s = state(item);
    s.speed -= deceleration;
    if s.speed < MINIMUM_SPEED {
        s.speed = MINIMUM_SPEED;
    }
    // HSD_JObjSetFlagsAll / ClearFlagsAll(child, JOBJ_HIDDEN): the blade is
    // the model's only drawn part.
    item.hidden = item.life_timer <= HIDDEN_FRAMES;
}

/// itKirbycutterbeam_UnkMotion0_Coll (8029BDD4): on the floor it_8026D564,
/// taking the floor's angle or, past the floor's end, leaving it a hair
/// higher; in the air it_8026DA08, landing on a floor it meets. The model
/// root turns to the angle. A wall in its way ends it: the one it flies
/// into (itkirbycutterbeam.c:144-151; recorded flying right into the side
/// of Yoshi's Story, and the same test mirrored flying left), or either
/// wall when it has no horizontal speed (:138-143), which no beam has had.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ground_or_air == GroundOrAir::Ground {
        if item.ground_pass(ctx.map) {
            let angle = floor_angle(item);
            state(item).angle = angle;
        } else {
            item.enter_air();
            // 8029BE60 / 8029BE6C: fadds.
            item.position.y += LIFT_OFF;
            item.collision
                .as_mut()
                .expect("Cutter beam map collision")
                .last_pos
                .y += LIFT_OFF;
        }
    } else if item.air_pass(ctx.map) {
        // it_802762B0, then the floor's angle. No recorded beam has left a
        // floor's end and come down on another.
        unimplemented!(
            "itKirbycutterbeam_UnkMotion0_Coll (itkirbycutterbeam.c:128-133): the airborne beam \
             landing on a floor"
        );
    }
    item.rotation.x = state(item).angle;
    let env = item
        .collision
        .as_ref()
        .expect("Cutter beam map collision")
        .env_flags as u32;
    let speed = item.velocity.x;
    if speed == 0.0 {
        if env & collide::WALL_MASK != 0 {
            unimplemented!(
                "itKirbycutterbeam_UnkMotion0_Coll (itkirbycutterbeam.c:138-143): a wall beside \
                 a beam with no horizontal speed"
            );
        }
        false
    } else if speed > 0.0 {
        env & collide::LEFT_WALL_MASK != 0
    } else {
        env & collide::RIGHT_WALL_MASK != 0
    }
}
