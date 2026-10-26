//! The Ray Gun (It_Kind_L_Gun, itlgun.c 8028E74C..8028EB5C) and its ray
//! (It_Kind_L_Gun_Ray, itlgunray.c 80298168..802986A8).
//!
//! The holder's LGunShoot accessory (ftCo_800CE1D4) fires a ray from the
//! muzzle while the gun has shots left (xD4C, ItLGunAttr x0 at spawn); each
//! shot plays the gun's recoil (state 3, a 40-frame timer). A thrown or
//! dropped gun without shots vanishes on the first floor it touches.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, RayState,
    SpawnItem,
};
use melee_types::ItemKind;

/// It_Kind_L_Gun.
pub struct RayGun;
/// It_Kind_L_Gun_Ray.
pub struct RayGunRay;

/// it_803F5CC8's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [-1, -1, -1, 0, 1, -1];
/// ItLGunAttr: x0 (shots), x4..xC (the muzzle, in the hand joint's frame).
pub const SPECIAL_ATTRIBUTES: u32 = 4;
/// it_803F64F8's anim_id column.
pub const RAY_ARTICLE_STATES: [i32; 1] = [0];
/// ItLGunRayAttr: x0 speed, x4 lifetime, x8 the model's longest stretch.
pub const RAY_SPECIAL_ATTRIBUTES: u32 = 3;

mod motion {
    pub const GROUNDED: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const RECOIL: u16 = 3;
    pub const FLYING: u16 = 4;
    pub const SLIDING: u16 = 5;
}

/// it_8028E938: the recoil's length in frames (xDD4_itemVar.lgun.timer).
const RECOIL_FRAMES: i32 = 40;
/// Item_UpdateRayAnimation's scale divisor for the Ray Gun's ray.
const RAY_SCALE_DIVISOR: f32 = 7.0;

/// it_8028E774 (8028E774): the muzzle, in the hand joint's frame
/// (ItLGunAttr x4).
pub fn muzzle_offset(assets: &ItemAssets) -> hsd_types::Vec3 {
    let a = &assets.special_attributes;
    hsd_types::Vec3::new(a[1], a[2], a[3])
}

static STATES: [ItemStateRow; 6] = [
    row(motion::GROUNDED, no_animation, no_physics, grounded_collision),
    row(motion::FALL, no_animation, gravity_physics, flying_collision),
    row(motion::HELD, no_animation, no_physics, no_collision),
    row(motion::RECOIL, recoil_animation, no_physics, no_collision),
    row(motion::FLYING, no_animation, falling_physics, flying_collision),
    row(motion::SLIDING, no_animation, no_physics, sliding_collision),
];

const fn row(
    state: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state as usize],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for RayGun {
    const KIND: ItemKind = ItemKind::LGun;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itLGun_Logic16_Spawned (8028E70C): the shots (x0), the recoil timer
    /// cleared, then the fall (it_8028E860).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ammo = assets.special_attributes[0].to_bits() as i32;
        item.scratch = ItemScratch::Tools(0);
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itLGun_Logic16_PickedUp: state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itLGun_Logic16_Dropped: state 4.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FLYING, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itLGun_Logic16_Thrown: state 4.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FLYING, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itLGun_Logic16_EnteredAir: state 5.
    const ENTERED_AIR: Option<EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itLGun_Logic16_EnteredAir (itlgun.c:186): a Ray Gun sliding down a slope is not witnessed")
    });
    /// itLGun_Logic16_DmgDealt / Clanked / HitShield: itColl_BounceOffVictim.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itLGun_Logic16_Reflected: it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// itLGun_Logic16_ShieldBounced: itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // it_8028E79C (8028E79C)'s count: one shot fewer (the ray itself
            // is the holder's spawn request).
            ItemControl::Fire => {
                if item.ammo > 0 {
                    item.ammo -= 1;
                }
            }
            // it_8028E938 (8028E938): the recoil, 40 frames of state 3.
            ItemControl::Recoil => {
                item.scratch = ItemScratch::Tools(RECOIL_FRAMES);
                change(item, motion::RECOIL, ANIM_UPDATE, assets);
            }
            other => unimplemented!("itlgun.c: control {other:?}"),
        }
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_8028E7D8 (8028E7D8): stopped, grabbable, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = hsd_types::Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itLgun_UnkMotion3_Anim (8028E96C): the recoil counts down, then the gun
/// is held still again (state 2).
fn recoil_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let ItemScratch::Tools(timer) = &mut item.scratch else {
        unreachable!("the Ray Gun's recoil timer")
    };
    *timer -= 1;
    if *timer <= 0 {
        change(item, motion::HELD, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itLgun_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itLgun_UnkMotion4_Phys -> Item_ApplyFallingPhysics.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itLgun_UnkMotion0_Coll: it_8026D62C, off the floor it falls
/// (it_8028E860).
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itLgun_UnkMotion4_Coll (states 1 and 4): with shots left it lands
/// (it_8026E15C(it_8028E7D8)); empty, the floor pass it_8026DF34 removes it.
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo == 0 {
        return item.air_pass(ctx.map);
    }
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            come_to_rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itLgun_UnkMotion5_Coll: it_8026E8C4(it_8028E7D8, it_8028E860), reached
/// only through the guarded EnteredAir.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itLgun_UnkMotion5_Coll (itlgun.c:200): a Ray Gun sliding down a slope is not witnessed")
}

/// ItLGunRayAttr.
struct RayAttributes<'a>(&'a [f32]);
impl RayAttributes<'_> {
    fn speed(&self) -> f32 {
        self.0[0]
    }
    fn lifetime(&self) -> f32 {
        self.0[1]
    }
    fn max_scale(&self) -> f32 {
        self.0[2]
    }
}

static RAY_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: RAY_ARTICLE_STATES[0],
    animation: ray_animation,
    physics: ray_physics,
    collision: ray_collision,
}];

impl ItemLogic for RayGunRay {
    const KIND: ItemKind = ItemKind::LGunRay;
    const STATES: &'static [ItemStateRow] = &RAY_STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_80298168's set-up once Item_80268B18 returns: it_802982AC (the
    /// lifetime, ungrabbable, state 0), then the ray's scale 0, its angle
    /// (0 facing right, pi left), speed x0, the segment's start at the
    /// spawn position (the holder's ECB centre), and velocity x0 * facing.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = RayAttributes(&assets.special_attributes);
        // it_80275158: both timers.
        item.life_timer = a.lifetime();
        item.half_life = a.lifetime() * common.half_life_scale;
        item.grabbable = false;
        item.change_motion_with(0, RAY_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.scratch = ItemScratch::Ray(RayState {
            scale: 0.0,
            angle: if item.facing == 1.0 {
                0.0
            } else {
                core::f32::consts::PI
            },
            speed: a.speed(),
            previous_position: spawn.position,
        });
        item.velocity.x = a.speed() * item.facing;
    }
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itLGunRay_Logic35_Absorbed (itlgunray.c:102): an absorbed Ray Gun ray is not witnessed")
    }
    /// itLGunRay_Logic35_Reflected (802986B0): the facing reversed and the
    /// model turned to it (M_PI_2 * facing, a double rounded), the speed
    /// times the reflector's (xC70), then Item_ResetRayAfterReflection: the
    /// stretch back to 1e-3 and the angle half a turn on (double adds,
    /// rounded each time it is stored) into 0..tau.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.facing = -item.facing;
        item.rotation.y = (core::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
        let ItemScratch::Ray(ray) = &mut item.scratch else {
            unreachable!("the ray's scratch")
        };
        ray.speed *= context.reflected_speed;
        ray.scale = 1e-3;
        ray.angle = wrap_stored((f64::from(ray.angle) + core::f64::consts::PI) as f32);
        item.model_scale.z = ray.scale;
        item.joints_dirty = true;
        false
    }
    /// itLGunRay_Logic35_ShieldBounced -> Item_BounceRayOffShield: the
    /// velocity mirrored off the shield (lbVector_Mirror, 8000DC84..9C:
    /// fmadds), the stretch back to 1e-3 and the angle taken from the new
    /// velocity (atan2f) into 0..tau.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let n = context.shield_normal;
        let dot = gekko_math::fma::fmadds(n.x, item.velocity.x, n.y * item.velocity.y);
        let factor = dot * -2.0;
        item.velocity.x = gekko_math::fma::fmadds(n.x, factor, item.velocity.x);
        item.velocity.y = gekko_math::fma::fmadds(n.y, factor, item.velocity.y);
        let ItemScratch::Ray(ray) = &mut item.scratch else {
            unreachable!("the ray's scratch")
        };
        ray.scale = 1e-3;
        ray.angle = wrap_stored(melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x));
        false
    }
}

/// The ray's angle into 0..tau, each step a double add or subtract rounded
/// as it is stored.
fn wrap_stored(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + core::f64::consts::TAU) as f32;
    }
    while f64::from(angle) > core::f64::consts::TAU {
        angle = (f64::from(angle) - core::f64::consts::TAU) as f32;
    }
    angle
}

/// itLgunray_UnkMotion0_Anim (80298300) -> Item_UpdateRayAnimation with the
/// divisor 7: velocity from the angle, the model faces it and stretches
/// toward x8; true once the lifetime (it_80273130) runs out.
fn ray_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let max_scale = RayAttributes(&ctx.assets.special_attributes).max_scale();
    let ItemScratch::Ray(ray) = &mut item.scratch else {
        unreachable!("the ray's scratch")
    };
    item.velocity.x = ray.speed * gekko_math::msl::cosf(ray.angle);
    item.velocity.y = ray.speed * gekko_math::msl::sinf(ray.angle);
    item.velocity.z = 0.0;
    item.facing = if item.velocity.x > 0.0 { 1.0 } else { -1.0 };
    // M_PI_2 * facing and M_PI + atan2f are double before the JObj stores.
    item.rotation.y = (core::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
    let vel_x = if item.facing == 1.0 {
        -item.velocity.x
    } else {
        item.velocity.x
    };
    item.rotation.x =
        (core::f64::consts::PI + f64::from(melee_lb::trigf::atan2f(item.velocity.y, vel_x))) as f32;
    ray.scale += gekko_math::msl::fabsf(ray.speed) / RAY_SCALE_DIVISOR;
    if ray.scale > max_scale {
        ray.scale = max_scale;
    }
    if ray.scale < 1e-5 {
        ray.scale = 1e-3;
    }
    item.model_scale.z = ray.scale;
    item.joints_dirty = true;
    // it_80273130.
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itLgunray_UnkMotion0_Phys: this tick's segment starts where it is.
fn ray_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let ItemScratch::Ray(ray) = &mut item.scratch else {
        unreachable!("the ray's scratch")
    };
    ray.previous_position = item.position;
}

/// itLgunray_UnkMotion0_Coll -> it_8029C4D4: the segment it moved along
/// this tick touching the stage ends it.
fn ray_collision(_item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    ctx.stage_contact
}
