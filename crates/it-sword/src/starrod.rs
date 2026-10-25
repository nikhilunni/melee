//! The Star Rod (It_Kind_StarRod, itstarrod.c 80292334..80292770) and its
//! star (It_Kind_StarRod_Star, itstarrodstar.c 802988E4..80298CE0). The
//! holder's StarRodSwing accessory (ft_800CD914) fires a star while the rod
//! has shots left (xD4C, from the attributes at spawn); a thrown or dropped
//! rod without shots vanishes on the first floor it touches.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemStateRow, SlidePass, SpawnItem,
};
use melee_types::ItemKind;

/// It_Kind_StarRod.
pub struct StarRod;
/// It_Kind_StarRod_Star.
pub struct StarRodStar;

/// it_803F5F90's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [-1, -1, -1, 0, 0, -1];
/// StarRodAttributes: x0 (shots), x4..xC (the star's offset in the hand).
pub const SPECIAL_ATTRIBUTES: u32 = 4;
/// it_803F6530's anim_id column.
pub const STAR_ARTICLE_STATES: [i32; 1] = [0];
/// StarRodStarAttrs: x0..x1C.
pub const STAR_SPECIAL_ATTRIBUTES: u32 = 8;

mod motion {
    pub const GROUNDED: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    pub const DROPPED: u16 = 4;
    pub const SLIDING: u16 = 5;
}

/// it_80292394 (80292394): where the star leaves the rod, in the hand
/// joint's frame (StarRodAttributes x4).
pub fn star_offset(assets: &ItemAssets) -> hsd_types::Vec3 {
    let a = &assets.special_attributes;
    hsd_types::Vec3::new(a[1], a[2], a[3])
}

static STATES: [ItemStateRow; 6] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: no_physics,
        collision: grounded_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: gravity_physics,
        collision: falling_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: no_animation,
        physics: falling_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: no_animation,
        physics: falling_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: no_animation,
        physics: no_physics,
        collision: sliding_collision,
    },
];

impl ItemLogic for StarRod {
    const KIND: ItemKind = ItemKind::StarRod;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itStarRod_Logic22_Spawned: the shots (x0), then the fall
    /// (it_80292488).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ammo = assets.special_attributes[0].to_bits() as i32;
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itStarRod_Logic22_Thrown: state 3; ungrabbable until it rests.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itStarRod_Logic22_Dropped: state 4.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    const ENTERED_AIR: Option<EnteredAir> =
        Some(|item, assets| change(item, motion::SLIDING, ANIM_UPDATE, assets));
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        bounce_if_flying(item, context)
    }
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
    /// it_802923BC (802923BC)'s count: one shot fewer (the star itself is
    /// the holder's spawn request).
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Fire => {
                if item.ammo > 0 {
                    item.ammo -= 1;
                }
            }
            other => unimplemented!("itstarrod.c: control {other:?}"),
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

/// it_802923F8 (802923F8): grabbable, stopped, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = hsd_types::Vec3::ZERO;
    change(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itStarrod_UnkMotion1_Phys: it_80272860 alone.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itStarrod_UnkMotion4_Phys -> Item_ApplyFallingPhysics.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itStarrod_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// it_8026E15C(it_802923F8).
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
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

/// itStarrod_UnkMotion3_Coll / UnkMotion4_Coll: with shots left it lands
/// as any thrown item; empty, the floor pass it_8026DF34 removes it.
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo != 0 {
        return falling_collision(item, ctx);
    }
    item.air_pass(ctx.map)
}

/// it_8026E8C4(it_802923F8, it_80292488).
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => come_to_rest(item, ctx.assets),
        SlidePass::Airborne => change(item, motion::FALL, ANIM_UPDATE, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}

/// StarRodStarAttrs.
struct StarAttributes<'a>(&'a [f32]);
impl StarAttributes<'_> {
    fn int(&self, index: usize) -> u32 {
        self.0[index].to_bits()
    }
    /// x0: the speed scale.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x4 / x8: the weak and strong star's speed.
    fn star_speed(&self, strong: bool) -> f32 {
        self.0[if strong { 2 } else { 1 }]
    }
    /// xC / x10: the weak and strong star's lifetime.
    fn lifetime(&self, strong: bool) -> f32 {
        self.0[if strong { 4 } else { 3 }]
    }
    /// x14 / x18: the weak and strong star's damage.
    fn damage(&self, strong: bool) -> u32 {
        self.int(if strong { 6 } else { 5 })
    }
    /// x1C: the spin per frame, signed by the facing.
    fn spin(&self) -> f32 {
        self.0[7]
    }
}

static STAR_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: STAR_ARTICLE_STATES[0],
    animation: star_animation,
    physics: no_physics,
    collision: star_collision,
}];

impl ItemLogic for StarRodStar {
    const KIND: ItemKind = ItemKind::StarRodStar;
    const STATES: &'static [ItemStateRow] = &STAR_STATES;
    /// it_802989C8 (802989C8): ungrabbable, the spin reset, state 0, then
    /// the weak or strong star (SpawnItem's argument, the holder's
    /// cmd_vars[1]): lifetime, speed and damage; the model spins at x1C,
    /// and its ECB stays upright (xDCE b7).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = StarAttributes(&assets.special_attributes);
        let strong = spawn.spawn_argument != 0;
        item.grabbable = false;
        item.reset_spin();
        item.change_motion_with(0, STAR_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        // it_80275158: both timers.
        let lifetime = a.lifetime(strong);
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        // retail 802989C8: fmuls (x0 * x4/x8), then fmuls by the facing.
        item.velocity.x = item.facing * (a.speed() * a.star_speed(strong));
        item.set_hitbox_damage(0, a.damage(strong));
        item.scratch = melee_it::ItemScratch::Float(a.spin() * item.facing);
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
    /// itStarRodStar_Logic36_Reflected: it_80273030, then the model faces
    /// its new velocity.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        item.rotation.z = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        false
    }
    /// itStarRodStar_Logic36_ShieldBounced: likewise after the bounce.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        item.rotation.z = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        false
    }
}

/// itStarrodstar_UnkMotion0_Anim: the lifetime runs out, else the model
/// turns by the spin (HSD_JObjAddRotationZ).
fn star_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        return true;
    }
    if let melee_it::ItemScratch::Float(spin) = item.scratch {
        item.rotation.z += spin;
    }
    item.joints_dirty = true;
    false
}

/// itStarrodstar_UnkMotion0_Coll: it_8026DFB0, any floor, wall or ceiling
/// ends it.
fn star_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) & 0xF != 0
}
