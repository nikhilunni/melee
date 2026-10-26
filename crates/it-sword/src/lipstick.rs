//! The Lip's Stick (It_Kind_LipStick, itlipstick.c 80295748..80295D60),
//! its spores (It_Kind_LipStick_Spore, itlipstickspore.c 8029A114..
//! 8029A748) and the flower a spore's hit grows on a fighter's head
//! (It_Kind_F_Flower_Flame, itfflowerflame.c 8029A748..8029AD58).
//!
//! The holder's LipstickSwing accessory (ft_800CDB9C) blows a spore while
//! the stick has some left (xD4C, from the attributes at spawn); a thrown
//! stick with spores left leaves one at its first contact (it_8029A218)
//! and lands like any thrown item, an empty one vanishes on the first
//! floor it touches.
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore,
    ItemEvent, ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
    SlidePass, SpawnItem,
};
use melee_types::ItemKind;

/// It_Kind_LipStick.
pub struct LipStick;
/// It_Kind_LipStick_Spore.
pub struct LipStickSpore;
/// It_Kind_F_Flower_Flame: the flower on a fighter's head.
pub struct Flower;

/// it_803F6310's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [0, 0, 0, 1, 1, 0];
/// itLipstickAttributes: x0 (spores), x4..xC (where a spore leaves the
/// stick, in the hand joint's frame).
pub const SPECIAL_ATTRIBUTES: u32 = 4;
/// it_803F6680's anim_id column.
pub const SPORE_ARTICLE_STATES: [i32; 2] = [0, 1];
/// itLipstickSporeAttributes: x0..x18.
pub const SPORE_SPECIAL_ATTRIBUTES: u32 = 7;
/// it_803F66xx (itfflowerflame.c)'s anim_id column.
pub const FLOWER_ARTICLE_STATES: [i32; 5] = [0, 0, 0, 0, 0];
/// The flower's attributes: x0 (its lifetime).
pub const FLOWER_SPECIAL_ATTRIBUTES: u32 = 1;

mod motion {
    pub const GROUNDED: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    pub const DROPPED: u16 = 4;
    pub const SLIDING: u16 = 5;
}

/// it_80295748 (80295748): where a spore leaves the stick, in the hand
/// joint's frame (itLipstickAttributes x4).
pub fn spore_offset(assets: &ItemAssets) -> Vec3 {
    let a = &assets.special_attributes;
    Vec3::new(a[1], a[2], a[3])
}

static STATES: [ItemStateRow; 6] = [
    row(0, no_animation, no_physics, grounded_collision),
    row(1, no_animation, gravity_physics, falling_collision),
    row(2, held_animation, no_physics, no_collision),
    row(3, no_animation, flying_physics, thrown_collision),
    row(4, no_animation, flying_physics, dropped_collision),
    row(5, no_animation, no_physics, sliding_collision),
];

const fn row(
    state: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for LipStick {
    const KIND: ItemKind = ItemKind::LipStick;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itLipstick_Logic23_Spawned (802956E8): the spores (x0), command
    /// variable 0 clear, the fall (it_8029583C).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ammo = assets.special_attributes[0].to_bits() as i32;
        item.command_variables[0] = 0;
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itLipstick_Logic23_Thrown: state 3.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itLipstick_Logic23_Dropped: state 4.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    const ENTERED_AIR: Option<EnteredAir> =
        Some(|item, assets| change(item, motion::SLIDING, ANIM_UPDATE, assets));
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
    /// itLipstick_Logic23_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
    /// it_80295770 (80295770)'s count: one spore fewer (the spore itself
    /// is the holder's spawn request).
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Fire => {
                if item.ammo > 0 {
                    item.ammo -= 1;
                }
            }
            other => unimplemented!("itlipstick.c: control {other:?}"),
        }
    }
}

/// DmgDealt / Clanked / HitShield: states 3 and 4 bounce off the victim.
fn bounce_if_flying(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    if item.motion == motion::THROWN || item.motion == motion::DROPPED {
        item.bounce_off_victim(context.victim_bounce);
    }
    false
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_802957AC (802957AC): grabbable, stopped, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = Vec3::ZERO;
    change(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itLipstick_UnkMotion2_Anim: in the hand, command variable 0 is raised
/// every frame (a throw then leaves a spore at the first contact).
fn held_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.command_variables[0] = 1;
    false
}

/// itLipstick_UnkMotion1_Phys: it_80272860 alone.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itLipstick_UnkMotion4_Phys -> Item_ApplyFallingPhysics.
fn flying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itLipstick_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// it_8026E15C(it_802957AC).
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    landed(item, landing, ctx.assets)
}

fn landed(item: &mut ItemCore, landing: AirLanding, assets: &ItemAssets) -> bool {
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            come_to_rest(item, assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itLipstick_UnkMotion3_Coll (80295B04): with spores left, the first
/// surface it touches with command variable 0 raised leaves a spore for
/// its owner (it_8029A218), then it bounces and lands as any thrown item
/// (it_80276FC4, it_8026DBC8); empty, the floor pass it_8026DF34 removes
/// it.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo == 0 {
        return item.air_pass(ctx.map);
    }
    let bits = item.air_contact_bits(ctx.map);
    if bits & 0xF == 0 {
        return false;
    }
    if item.command_variables[0] != 0 {
        if item.owner.is_none() {
            unimplemented!("it_8029A218 (itlipstick.c:131): a spore for an ownerless stick");
        }
        item.events.push(ItemEvent::StickSpore {
            previous_position: item.position,
            facing: item.facing,
        });
        item.command_variables[0] = 0;
    }
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.land_on_contact(bits, ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    landed(item, landing, ctx.assets)
}

/// itLipstick_UnkMotion4_Coll: with spores left it lands as any thrown
/// item (it_8026E15C); empty, it_8026DF34 removes it on a floor.
fn dropped_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo != 0 {
        return falling_collision(item, ctx);
    }
    item.air_pass(ctx.map)
}

/// it_8026E8C4(it_802957AC, it_8029583C).
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => come_to_rest(item, ctx.assets),
        SlidePass::Airborne => change(item, motion::FALL, ANIM_UPDATE, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}

/// itLipstickSporeAttributes.
struct SporeAttributes<'a>(&'a [f32]);
impl SporeAttributes<'_> {
    /// x0: the speed scale.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x4 / x8: the weak and strong spore's speed.
    fn weak_speed(&self) -> f32 {
        self.0[1]
    }
    fn strong_speed(&self) -> f32 {
        self.0[2]
    }
    /// xC / x10: the weak and strong spore's lifetime.
    fn weak_lifetime(&self) -> f32 {
        self.0[3]
    }
    fn strong_lifetime(&self) -> f32 {
        self.0[4]
    }
    /// x14 / x18: the weak and strong spore's damage.
    fn weak_damage(&self) -> u32 {
        self.0[5].to_bits()
    }
    fn strong_damage(&self) -> u32 {
        self.0[6].to_bits()
    }
}

/// The swing's spore (SpawnItem's variant 0, it_8029A114) or a thrown
/// stick's (variant 1, it_8029A218).
pub const SWING_SPORE: i32 = 0;
pub const CONTACT_SPORE: i32 = 1;
/// it_8029A498's lifetime for a contact spore.
const CONTACT_SPORE_LIFETIME: f32 = 5.0;
/// efSync_Spawn ids: the swing's puff (efasync.c:773-779, generator 0x1D8
/// turned to the facing) and the contact's (efasync.c:780-782, 0x1FB).
pub const SWING_SPORE_EFFECT: u16 = 0x440;
pub const CONTACT_SPORE_EFFECT: u16 = 0x441;

static SPORE_STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: SPORE_ARTICLE_STATES[0],
        animation: lifetime_animation,
        physics: spore_physics,
        collision: spore_collision,
    },
    ItemStateRow {
        animation_id: SPORE_ARTICLE_STATES[1],
        animation: lifetime_animation,
        physics: still_physics,
        collision: no_collision,
    },
];

impl ItemLogic for LipStickSpore {
    const KIND: ItemKind = ItemKind::LipStickSpore;
    const STATES: &'static [ItemStateRow] = &SPORE_STATES;
    /// it_8029A31C (8029A31C) for the swing's spore: not grabbable, the
    /// spin stopped (it_80274740), state 0, then by the swing's strength
    /// (xDD4, the spawn argument): 0 the strong spore's lifetime, speed
    /// and damage, 1 the weak one's, 2 the weak one's standing still; it
    /// slows by the speed over the lifetime each frame (xDD8). Its ECB
    /// stays upright (xDCE b7). it_8029A498 (8029A498) for a contact
    /// spore: lifetime 5, state 1.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.grabbable = false;
        item.reset_spin();
        if spawn.spawn_variant == CONTACT_SPORE {
            set_lifetime(item, CONTACT_SPORE_LIFETIME, common);
            item.change_motion_with(1, SPORE_ARTICLE_STATES[1], ANIM_UPDATE, assets);
            item.ecb_follows_rotation = false;
            return;
        }
        item.change_motion_with(0, SPORE_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        let a = SporeAttributes(&assets.special_attributes);
        let (lifetime, speed, damage) = match spawn.spawn_argument {
            0 => (a.strong_lifetime(), Some(a.strong_speed()), a.strong_damage()),
            1 => (a.weak_lifetime(), Some(a.weak_speed()), a.weak_damage()),
            2 => (a.weak_lifetime(), None, a.weak_damage()),
            other => unreachable!("it_8029A31C: spore strength {other}"),
        };
        set_lifetime(item, lifetime, common);
        let slowing = match speed {
            Some(speed) => {
                // retail 8029A3A0..: fmuls (x0 * x4/x8), fmuls by the facing.
                item.velocity.x = item.facing * (a.speed() * speed);
                // fdivs, then fabs.
                gekko_math::msl::fabsf(item.velocity.x / lifetime)
            }
            None => {
                item.velocity.x = 0.0;
                0.0
            }
        };
        item.scratch = ItemScratch::Float(slowing);
        item.set_hitbox_damage(0, damage);
        item.ecb_follows_rotation = false;
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
        true
    }
    /// itLipstickSpore_Logic37_Reflected: it_80273030, then the model faces
    /// its new velocity.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        item.rotation.z = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        false
    }
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
}

/// it_80275158: both timers.
fn set_lifetime(item: &mut ItemCore, lifetime: f32, common: &ItemCommonData) {
    item.life_timer = lifetime;
    item.half_life = lifetime * common.half_life_scale;
}

/// it_80273130 (80273130): the lifetime runs out.
fn lifetime_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itLipstickspore_UnkMotion0_Phys (8029A540): a moving spore slows by xDD8
/// toward zero, and stops once slower than that.
fn spore_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let ItemScratch::Float(slowing) = item.scratch else {
        unreachable!("a spore's slowing")
    };
    if item.velocity.x == 0.0 || slowing == 0.0 {
        return;
    }
    if item.velocity.x > 0.0 {
        item.velocity.x -= slowing;
    } else {
        item.velocity.x += slowing;
    }
    if gekko_math::msl::fabsf(item.velocity.x) < slowing {
        item.velocity.x = 0.0;
    }
}

/// itLipstickspore_UnkMotion1_Phys: itResetVelocity.
fn still_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity = Vec3::ZERO;
}

/// itLipstickspore_UnkMotion0_Coll -> it_8026DFB0: any floor, wall or
/// ceiling ends it.
fn spore_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) & 0xF != 0
}

mod flower_motion {
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
}

static FLOWER_STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: FLOWER_ARTICLE_STATES[0],
        animation: flower_rest_animation,
        physics: no_physics,
        collision: flower_rest_collision,
    },
    ItemStateRow {
        animation_id: FLOWER_ARTICLE_STATES[1],
        animation: flower_falling_animation,
        physics: gravity_physics,
        collision: flower_fall_collision,
    },
    ItemStateRow {
        animation_id: FLOWER_ARTICLE_STATES[2],
        animation: flower_held_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: FLOWER_ARTICLE_STATES[3],
        animation: flower_falling_animation,
        physics: gravity_physics,
        collision: flower_thrown_collision,
    },
    ItemStateRow {
        animation_id: FLOWER_ARTICLE_STATES[4],
        animation: flower_falling_animation,
        physics: no_physics,
        collision: flower_slide_collision,
    },
];

/// The flower's joints its fighter sizes (ftCommon_8007FDA0): joint 1's
/// scale (it_8029A89C) and joint 2's translation from its rest
/// (it_8029A8F4).
const FLOWER_SCALED_JOINT: usize = 1;
const FLOWER_MOVED_JOINT: usize = 2;

impl ItemLogic for Flower {
    const KIND: ItemKind = ItemKind::FFlowerFlame;
    const STATES: &'static [ItemStateRow] = &FLOWER_STATES;
    /// itFFlowerFlame_Logic41_Spawned: not grabbable.
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.grabbable = false;
    }
    /// it_8029A748 (8029A748) after the spawn: its lifetime (x0), and the
    /// rest translation of joint 2 kept (xDD4) before the fighter sizes it.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        set_lifetime(item, assets.special_attributes[0], common);
    }
    /// itFFlowerFlame_Logic41_PickedUp: state 2, not grabbable.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        flower_change(item, flower_motion::HELD, context.assets);
        item.grabbable = false;
    }
    /// itFFlowerFlame_Logic41_Dropped: the fall (state 1, flags 6), not
    /// grabbable.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(
            flower_motion::FALL,
            FLOWER_ARTICLE_STATES[1],
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
        item.grabbable = false;
    }
    const ENTERED_AIR: Option<EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itFFlowerFlame_Logic41_EnteredAir (itfflowerflame.c:213): a flower sliding down a slope is not witnessed")
    });
    /// it_8029A89C / it_8029A8F4 from the fighter (ftCommon_8007FDA0): joint
    /// 1 at `scale`, joint 2 at its rest translation plus `offset`.
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::FlowerSize { scale, offset } => {
                let rest = assets
                    .visual
                    .model
                    .child
                    .as_ref()
                    .and_then(|joint| joint.child.as_ref())
                    .expect("the flower's joint 2")
                    .position;
                item.joint_scale = Some((FLOWER_SCALED_JOINT, Vec3::new(scale, scale, scale)));
                item.joint_translation = Some((
                    FLOWER_MOVED_JOINT,
                    Vec3::new(rest.x + offset.x, rest.y + offset.y, rest.z + offset.z),
                ));
            }
            other => unimplemented!("itfflowerflame.c: control {other:?}"),
        }
    }
}

fn flower_change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, FLOWER_ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// itFflowerflame_UnkMotion0_Anim: the joints as the fighter set them.
fn flower_rest_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// itFflowerflame_UnkMotion2_Anim: joint 1's scale again.
fn flower_held_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// itFflowerflame_UnkMotion1_Anim / 3 / 4: the joints again, then the
/// lifetime (it_80273130).
fn flower_falling_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    lifetime_animation(item, ctx)
}

/// itFflowerflame_UnkMotion0_Coll: it_8026D62C, off the floor it falls
/// (it_8029AB90).
fn flower_rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        flower_change(item, flower_motion::FALL, ctx.assets);
        item.grabbable = false;
    }
    false
}

/// itFflowerflame_UnkMotion1_Coll -> it_8026DF34: the floor removes it.
fn flower_fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_pass(ctx.map)
}

/// itFflowerflame_UnkMotion3_Coll -> it_8026DA08 (state 3 is entered by
/// nothing ported).
fn flower_thrown_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itFflowerflame_UnkMotion3_Coll (itfflowerflame.c:205): a thrown flower")
}

/// itFflowerflame_UnkMotion4_Coll (reached only through the guarded
/// EnteredAir).
fn flower_slide_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itFflowerflame_UnkMotion4_Coll (itfflowerflame.c:231): a sliding flower")
}
